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
