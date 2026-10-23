// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
//! Synthetic container and metadata regression tests.
use std::io::Write;
use sw_rs::{Container, Limits};
fn file(streams: &[(&str, &[u8])]) -> Vec<u8> {
    let mut bytes = vec![0; 16];
    bytes[7] = 3;
    for (name, data) in streams {
        let mut encoder =
            flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(data).unwrap();
        let compressed = encoder.finish().unwrap();
        bytes.extend([0x14, 0, 6, 0, 8, 0]);
        for n in [
            1,
            crc32fast::hash(data),
            compressed.len() as u32,
            data.len() as u32,
            name.len() as u32,
        ] {
            bytes.extend(n.to_le_bytes());
        }
        bytes.extend(name.bytes().map(|b| b.rotate_right(3)));
        bytes.extend(compressed);
    }
    bytes
}
#[test]
fn streams_verify_crc_first_wins_and_keep_preview() {
    let input = file(&[
        ("_MO_VERSION_13000/x", b"v"),
        ("name", b"first"),
        ("name", b"second"),
        ("PreviewPNG", b"prefix\x89PNG\r\n\x1a\nrest"),
    ]);
    let container = Container::parse(&input, Limits::default()).unwrap();
    assert_eq!(container.streams["name"], b"first");
    assert!(
        container
            .diagnostics
            .iter()
            .any(|d| d.message.contains("duplicate"))
    );
    let doc = sw_rs::parse(&input, "test.SLDPRT").unwrap();
    assert_eq!(doc.version, 13000);
    assert!(doc.preview_png.starts_with(b"\x89PNG"));
}
#[test]
fn corrupt_record_is_reported_and_next_stream_survives() {
    let mut bytes = file(&[("broken", b"bad"), ("good", b"good")]);
    bytes[26] ^= 1; // first CRC byte, marker at 16
    let c = Container::parse(&bytes, Limits::default()).unwrap();
    assert!(!c.streams.contains_key("broken"));
    assert_eq!(c.streams["good"], b"good");
    assert!(c.diagnostics.iter().any(|d| d.message.contains("CRC")));
}
#[test]
fn rejects_unsupported_and_truncated_inputs_without_panics() {
    for bytes in [b"".as_slice(), b"PK\x03\x04", b"\xd0\xcf\x11\xe0"] {
        assert!(sw_rs::parse(bytes, "x").is_err());
    }
    let bytes = file(&[("a", b"test")]);
    for end in 0..bytes.len() {
        assert!(Container::parse(&bytes[..end], Limits::default()).is_err());
    }
    for n in 0..512 {
        let junk: Vec<u8> = (0..n).map(|i| ((i * 73 + n * 11) % 256) as u8).collect();
        let _ = Container::parse(&junk, Limits::default());
    }
}
#[test]
fn budgets_bound_expansion_records_and_total_including_duplicates() {
    let input = file(&[("a", &[b'x'; 1024]), ("a", &[b'y'; 1024])]);
    for limits in [
        Limits {
            max_stream_bytes: 512,
            ..Limits::default()
        },
        Limits {
            max_file_bytes: 8,
            ..Limits::default()
        },
        Limits {
            max_total_bytes: 1500,
            ..Limits::default()
        },
        Limits {
            max_records: 1,
            ..Limits::default()
        },
    ] {
        assert!(Container::parse(&input, limits).is_err());
    }
    let mut input = file(&[("a", &[b'x'; 1024])]);
    input[34..38].copy_from_slice(&1u32.to_le_bytes()); // malicious declared decoded size
    assert!(
        Container::parse(
            &input,
            Limits {
                max_stream_bytes: 512,
                ..Limits::default()
            }
        )
        .is_err()
    );
}
#[test]
fn metadata_preserves_unicode_cached_tokens_and_missing_transform() {
    let props = br#"<properties xmlns:vt="urn:vt"><propertySection name="UserDefinedProperties"><property name="material"><vt:lpstr>steel &amp; paint</vt:lpstr></property></propertySection></properties>"#;
    let info = br#"<properties xmlns:vt="urn:vt"><propertySection><property name="SW-MassProp-Config-0"><vt:lpstr>1,2,3,4,5,6,7,8,9,10,11,12,0,0</vt:lpstr></property></propertySection></properties>"#;
    let tree = r#"<swSolidWorks><swHeader><swFile id="root" swPath="a.SLDASM"/><swFile id="child" swPath="C:\part.SLDPRT"/></swHeader><swModelList><swModel id="childModel" swFileRef="child" swConfigurationName="childConfig"/><swModel id="rootModel" swFileRef="root" swConfigurationId="0" swConfigurationName="默认"><swReference swModelRef="childModel" swName="部件"/><swReference swModelRef="childModel" swSuppressed="NO" swTransform="NaN 0"/></swModel></swModelList></swSolidWorks>"#;
    let override_props = br#"<properties xmlns:vt="urn:vt"><propertySection name="UserDefinedProperties"><property name="material"><vt:lpstr>aluminum</vt:lpstr></property></propertySection></properties>"#;
    let bytes = file(&[
        ("docProps/Config-0-Properties.xml", override_props),
        ("docProps/custom.xml", props),
        ("docProps/ISolidWorksInformation.xml", info),
        ("swXmlContents/COMPINSTANCETREE", tree.as_bytes()),
    ]);
    let doc = sw_rs::parse(&bytes, "x.SLDASM").unwrap();
    assert_eq!(doc.global_properties["material"], "steel & paint");
    let config = &doc.configurations[0];
    assert_eq!(config.name, "默认");
    assert_eq!(config.properties["material"], "aluminum");
    assert_eq!(config.mass_properties.as_ref().unwrap().mass, 6.);
    assert_eq!(config.components[0].name, "部件");
    assert_eq!(config.components[0].ref_config, "childConfig");
    assert!(config.components[0].is_suppressed);
    assert!(config.components.iter().all(|c| c.transform.is_none()));
}

#[test]
fn empty_stream_is_valid_but_missing_deflate_terminator_is_rejected() {
    let input = file(&[("empty", b"")]);
    assert!(Container::parse(&input, Limits::default()).unwrap().streams["empty"].is_empty());
    let mut input = file(&[("payload", b"abc")]);
    let size = u32::from_le_bytes(input[30..34].try_into().unwrap());
    input[30..34].copy_from_slice(&(size - 1).to_le_bytes());
    input.pop();
    assert!(Container::parse(&input, Limits::default()).is_err());
}

#[test]
fn binary_configuration_names_obey_version_gates_and_parent_ids() {
    let mut manager = Vec::new();
    fn new_class(out: &mut Vec<u8>, name: &str) {
        out.extend(65535u16.to_le_bytes());
        out.extend(0u16.to_le_bytes());
        out.extend((name.len() as u16).to_le_bytes());
        out.extend(name.bytes());
    }
    fn string(out: &mut Vec<u8>, name: &str) {
        let utf16: Vec<_> = name.encode_utf16().collect();
        out.extend([255, 254, utf16.len() as u8]);
        for c in utf16 {
            out.extend(c.to_le_bytes());
        }
    }
    new_class(&mut manager, "dmConfigMgrHeader_c");
    manager.extend(2u16.to_le_bytes());
    for (i, name, parent) in [(0u32, "默认", u32::MAX), (3, "derived", 0)] {
        new_class(&mut manager, "dmConfigHeader_c");
        manager.extend(0u32.to_le_bytes());
        string(&mut manager, name);
        manager.extend(i.to_le_bytes());
        manager.extend(0u32.to_le_bytes()); // >= 1341
        string(&mut manager, name); // >= 2037
        manager.extend(parent.to_le_bytes());
        manager.extend(0u32.to_le_bytes()); // >= 2136
        string(&mut manager, ""); // >= 2610
        string(&mut manager, "");
        manager.extend(0u32.to_le_bytes()); // >= 3928
    }
    let bytes = file(&[
        ("_MO_VERSION_4000/a", b"v"),
        ("Contents/CMgrHdr2", &manager),
    ]);
    let doc = sw_rs::parse(&bytes, "part.SLDPRT").unwrap();
    assert_eq!(doc.configurations.len(), 2);
    assert_eq!(doc.configurations[0].name, "默认");
    assert_eq!(doc.configurations[1].index, 3);
    assert_eq!(doc.configurations[1].parent_index, Some(0));
}

fn display_table() -> Vec<u8> {
    fn descriptor(out: &mut Vec<u8>, size: u32, kind: u32, count: u32, bytes: &[u8]) {
        for v in [size, kind, 2, count] {
            out.extend(v.to_le_bytes());
        }
        out.extend(bytes);
    }
    let mut table = Vec::new();
    descriptor(&mut table, 4, 8, 1, &3u32.to_le_bytes());
    let positions: Vec<u8> = [0f32, 0., 0., 1., 0., 0., 0., 1., 0.]
        .into_iter()
        .flat_map(f32::to_le_bytes)
        .collect();
    descriptor(&mut table, 12, 100, 3, &positions);
    descriptor(&mut table, 12, 100, 3, &[0; 36]);
    descriptor(&mut table, 4, 8, 4, &[0; 16]);
    descriptor(&mut table, 4, 8, 1, &4u32.to_le_bytes());
    descriptor(&mut table, 1, 8, 4, &[0; 4]);
    table
}
fn display_payload() -> Vec<u8> {
    let class = b"uoTempFaceTessData_c";
    let mut out = vec![255, 255, 1, 0];
    out.extend((class.len() as u16).to_le_bytes());
    out.extend(class);
    out.extend(1u32.to_le_bytes());
    out.extend(1u32.to_le_bytes());
    out.extend(display_table());
    out
}
#[test]
fn cached_geometry_is_byte_based_hashed_and_preserves_winding_and_units() {
    let payload = display_payload();
    let bytes = file(&[("Contents/DisplayLists", &payload)]);
    let g = sw_rs::geometry(&bytes, "synthetic.SLDPRT").unwrap();
    assert_eq!(g.source.sha256.len(), 64);
    assert_eq!(g.length_unit, "millimeter");
    let m = &g.model.tessellations[0];
    assert_eq!(m.vertices, [[0., 0., 0.], [1000., 0., 0.], [0., 1000., 0.]]);
    assert_eq!(m.triangles, [[0, 1, 2]]);
    assert_eq!(
        &payload[m.source_offset..m.source_offset + 4],
        &4u32.to_le_bytes()
    );
    assert!(sw_rs::geometry(&bytes, "assembly.SLDASM").is_err());
}
#[test]
fn cached_geometry_rejects_missing_truncated_nonfinite_and_ambiguous_data() {
    assert!(sw_rs::geometry(&file(&[("other", b"x")]), "p.SLDPRT").is_err());
    let p = display_payload();
    for end in 0..p.len() {
        assert!(
            sw_rs::geometry(&file(&[("Contents/DisplayLists", &p[..end])]), "p.SLDPRT").is_err()
        );
    }
    let mut p = display_payload();
    let start = 6 + b"uoTempFaceTessData_c".len() + 8 + 20 + 16;
    p[start..start + 4].copy_from_slice(&f32::NAN.to_le_bytes());
    assert!(sw_rs::geometry(&file(&[("Contents/DisplayLists", &p)]), "p.SLDPRT").is_err());
    let p = display_payload();
    let bytes = file(&[
        ("Contents/DisplayLists", &p),
        ("Contents/Config-1", b"x"),
        ("Contents/Config-2", b"x"),
    ]);
    assert!(
        sw_rs::geometry(&bytes, "p.SLDPRT")
            .unwrap_err()
            .to_string()
            .contains("ambiguous")
    );
}
#[test]
fn metadata_json_has_cache_compatible_com_and_no_preview_byte_arrays() {
    let info=br#"<properties xmlns:vt="urn:vt"><propertySection><property name="SW-MassProp-Config-0"><vt:lpstr>1,2,3,4,5,6,7,8,9,10,11,12</vt:lpstr></property></propertySection></properties>"#;
    let cut=br#"<Configuration Name="default"><Feature Name="bar"><CustomProperty Name="QUANTITY">2</CustomProperty></Feature></Configuration>"#;
    let bytes = file(&[
        ("docProps/ISolidWorksInformation.xml", info),
        ("docProps/Config-0-Cutlist-Properties.xml", cut),
        ("PreviewPNG", b"\x89PNG\r\n\x1a\npixels"),
    ]);
    let doc = sw_rs::parse(&bytes, "p.SLDPRT").unwrap();
    let json = serde_json::to_value(&doc).unwrap();
    assert_eq!(json["schema"], "sw-rs-metadata-v1");
    assert_eq!(json["doc_type"], "part");
    assert_eq!(
        json["configurations"][0]["mass_properties"]["center_of_gravity"],
        serde_json::json!({"x":1.,"y":2.,"z":3.})
    );
    assert_eq!(json["configurations"][0]["cut_list"][0]["quantity"], 2);
    assert!(json.get("preview_png").is_none());
    assert!(json["configurations"][0].get("preview_png").is_none());
    assert!(!doc.configurations[0].preview_png.is_empty());
}

#[test]
fn drawing_sheet_and_utf16_reference_metadata_preserve_unicode() {
    let xml = r#"<Keywords><Sheet Name="图纸" Type="Sheet"><View Name="front" Description="默认"> C:\部件.SLDPRT </View></Sheet><Sheet Name="template" Type="Sheet Format"/></Keywords>"#;
    let path: Vec<u8> = "C:\\部件🦀.SLDPRT\0"
        .encode_utf16()
        .flat_map(u16::to_le_bytes)
        .collect();
    let bytes = file(&[
        ("swXmlContents/KeyWords", xml.as_bytes()),
        ("Header2", &path),
    ]);
    let doc = sw_rs::parse(&bytes, "d.SLDDRW").unwrap();
    assert_eq!(doc.sheets.len(), 1);
    assert_eq!(doc.sheets[0].views[0].referenced_configuration, "默认");
    assert_eq!(doc.header_references, ["C:\\部件🦀.SLDPRT"]);
}

#[test]
fn incomplete_large_descriptor_table_is_rejected() {
    let count = 250_001u32;
    let mut p = display_payload();
    let table = 6 + b"uoTempFaceTessData_c".len() + 8;
    p.truncate(table);
    for n in [4u32, 8, 2, 1, count, 12, 100, 2, count] {
        p.extend(n.to_le_bytes());
    }
    p.resize(p.len() + count as usize * 12, 0);
    // No normal/auxiliary descriptors follow these positions; the table is incomplete.
    let bytes = file(&[("Contents/DisplayLists", &p)]);
    assert!(sw_rs::geometry(&bytes, "p.SLDPRT").is_err());
}

#[test]
fn local_byte_override_retains_record_integrity_and_honors_compressed_limits() {
    let input = file(&[("saved", b"display cache"), ("other", b"")]);
    let bounded = Container::parse(
        &input,
        Limits {
            max_compressed_bytes: 2,
            ..Limits::default()
        },
    )
    .unwrap();
    assert!(!bounded.streams.contains_key("saved"));
    let local = Container::parse(&input, Limits::without_byte_limits()).unwrap();
    assert_eq!(local.streams["saved"], b"display cache");
    let mut corrupt = input;
    corrupt[26] ^= 1;
    let local = Container::parse(&corrupt, Limits::without_byte_limits()).unwrap();
    assert!(!local.streams.contains_key("saved"));
    assert!(local.diagnostics.iter().any(|d| d.message.contains("CRC")));
}

#[test]
fn utf8_embedded_stream_names_and_explicit_replacement_chains_are_preserved() {
    fn string(out: &mut Vec<u8>, text: &str) {
        let units: Vec<_> = text.encode_utf16().collect();
        out.extend([255, 254, 255]);
        if units.len() < 254 {
            out.push(units.len() as u8);
        } else {
            out.push(254);
            out.extend((units.len() as u16).to_le_bytes());
        }
        for unit in units {
            out.extend(unit.to_le_bytes());
        }
    }
    let mut replacements = 2u32.to_le_bytes().to_vec();
    for path in [
        r"C:\source\部件.SLDPRT",
        r"C:\export\部件-v2.SLDPRT",
        r"C:\export\部件-v2.SLDPRT",
        r"C:\export\部件-final.SLDPRT",
    ] {
        string(&mut replacements, path);
    }
    let bytes = file(&[
        ("ImportedComp/嵌入零件.SLDPRT", b"native child"),
        ("SwDocMgrTempStorage/ReplaceRef", &replacements),
    ]);
    let container = Container::parse(&bytes, Limits::unlimited()).unwrap();
    assert_eq!(
        container.streams["ImportedComp/嵌入零件.SLDPRT"],
        b"native child"
    );
    let doc = sw_rs::metadata_from_container(container, "root.SLDASM");
    assert_eq!(doc.reference_replacements.len(), 2);
    assert_eq!(
        doc.reference_replacements[0].replacement_path,
        doc.reference_replacements[1].original_path
    );
    replacements.pop();
    let bytes = file(&[("SwDocMgrTempStorage/ReplaceRef", &replacements)]);
    let doc = sw_rs::parse(&bytes, "root.SLDASM").unwrap();
    assert!(doc.reference_replacements.is_empty());
    assert!(
        doc.diagnostics
            .iter()
            .any(|d| d.message.contains("replacement-reference"))
    );
}

#[test]
fn valid_cached_mesh_can_exceed_former_table_and_triangle_caps() {
    let triangles = 500_001u32;
    let vertices = triangles * 3;
    let mut table = Vec::new();
    let mut descriptor = |size: u32, kind: u32, count: u32, data: &[u8]| {
        for n in [size, kind, 2, count] {
            table.extend(n.to_le_bytes());
        }
        table.extend(data);
    };
    descriptor(
        4,
        8,
        triangles,
        &3u32.to_le_bytes().repeat(triangles as usize),
    );
    let positions: Vec<u8> = [0f32, 0., 0., 1., 0., 0., 0., 1., 0.]
        .into_iter()
        .flat_map(f32::to_le_bytes)
        .collect();
    descriptor(12, 100, vertices, &positions.repeat(triangles as usize));
    descriptor(12, 100, 0, &[]);
    descriptor(4, 8, 0, &[]);
    descriptor(
        4,
        8,
        triangles,
        &4u32.to_le_bytes().repeat(triangles as usize),
    );
    descriptor(1, 8, 0, &[]);
    let class = b"uoTempFaceTessData_c";
    let mut payload = vec![255, 255, 1, 0];
    payload.extend((class.len() as u16).to_le_bytes());
    payload.extend(class);
    payload.extend(triangles.to_le_bytes());
    payload.extend(triangles.to_le_bytes());
    payload.extend(table);
    let bytes = file(&[("Contents/DisplayLists", &payload)]);
    let geometry = sw_rs::geometry(&bytes, "large.SLDPRT").unwrap();
    assert_eq!(geometry.model.tessellations.len(), 1);
    assert_eq!(
        geometry.model.tessellations[0].vertices.len(),
        vertices as usize
    );
    assert_eq!(
        geometry.model.tessellations[0].triangles.len(),
        triangles as usize
    );
}
