// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
//! Synthetic saved-material records; no source CAD or material database is embedded.
use std::io::Write;

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
fn xml(body: &str) -> String {
    format!(
        r#"<?xml version="1.0"?><m:materials xmlns:m="http://www.solidworks.com/sldmaterials">{body}</m:materials>"#
    )
}
fn document(materials: &[u8]) -> sw_rs::Document {
    let properties = br#"<properties xmlns:v="urn:v"><propertySection name="UserDefinedProperties"><property name="Configuration"><v:lpstr>test</v:lpstr></property><property name="material"><v:lpstr>custom user label</v:lpstr></property></propertySection></properties>"#;
    sw_rs::parse(
        &file(&[
            ("swXmlContents/MATERIALTREE", materials),
            ("docProps/Config-0-Properties.xml", properties),
        ]),
        "test.SLDPRT",
    )
    .unwrap()
}
const DEFINITION: &str = r#"<classification name="synthetic family"><material name="example|test alloy" matid="7"><swatchcolor RGB="abcDEF"/><shaders><pwshader2 name="carbon appearance"/></shaders><physicalproperties><DENS displayname="density" value="2700.000000"/></physicalproperties></material></classification>"#;

#[test]
fn exact_saved_configuration_reference_resolves_with_provenance() {
    let text = xml(&format!(
        "{DEFINITION}<configurationslist><configuration name=\"test\" material=\"example|test alloy|7\"/></configurationslist>"
    ));
    let doc = document(text.as_bytes());
    let materials = doc.materials.unwrap();
    assert_eq!(materials.source_stream, "swXmlContents/MATERIALTREE");
    assert!(!materials.body_assignments_decoded);
    let definition = &materials.definitions[0];
    assert_eq!(definition.name, "example|test alloy");
    assert_eq!(definition.classifications, ["synthetic family"]);
    assert_eq!(definition.saved_swatch_rgb.as_deref(), Some("abcDEF"));
    let density = definition.density.as_ref().unwrap();
    assert_eq!(density.value, "2700.000000");
    assert_eq!(density.numeric_value, Some(2700.0));
    assert_eq!(density.units, None);
    assert_eq!(
        density.source_path,
        "/materials[1]/classification[1]/material[1]/physicalproperties[1]/DENS[1]"
    );
    let assignment = doc.configurations[0].material.as_ref().unwrap();
    assert_eq!(assignment.definition_index, Some(0));
    assert_eq!(assignment.material_reference, "example|test alloy|7");
    assert_eq!(
        assignment.source_path,
        "/materials[1]/configurationslist[1]/configuration[1]"
    );
    assert_eq!(
        doc.configurations[0].properties["material"],
        "custom user label"
    );
}
#[test]
fn custom_definition_without_assignment_stays_unassigned_despite_shader() {
    let text = xml(
        r#"<classification name="碳纤维"><material name="custom|VMA"><physicalproperties><DENS value="2000.000000"/></physicalproperties><shaders><pwshader2 name="black plastic"/></shaders></material></classification><configurationslist><configuration/></configurationslist>"#,
    );
    let doc = document(text.as_bytes());
    assert!(doc.configurations[0].material.is_none());
    let materials = doc.materials.unwrap();
    assert_eq!(materials.definitions[0].classifications, ["碳纤维"]);
    assert_eq!(materials.assignments[0].definition_index, None);
    assert_eq!(materials.assignments[0].configuration, "");
}
#[test]
fn utf16_both_orders_and_utf8_bom_preserve_unicode() {
    let text =
        xml(r#"<classification name="材料"><material name="自定义|样本"/></classification>"#);
    for little in [true, false] {
        let mut bytes = if little {
            vec![0xff, 0xfe]
        } else {
            vec![0xfe, 0xff]
        };
        for unit in text.encode_utf16() {
            bytes.extend(if little {
                unit.to_le_bytes()
            } else {
                unit.to_be_bytes()
            });
        }
        assert_eq!(
            document(&bytes).materials.unwrap().definitions[0].name,
            "自定义|样本"
        );
    }
    assert_eq!(
        document(format!("\u{feff}{text}").as_bytes())
            .materials
            .unwrap()
            .definitions[0]
            .name,
        "自定义|样本"
    );
}
#[test]
fn duplicate_definition_keys_and_duplicate_assignments_are_not_selected() {
    let reference = r#"<configuration name="test" material="example|test alloy|7"/>"#;
    let doc = document(
        xml(&format!(
            "{DEFINITION}{DEFINITION}<configurationslist>{reference}</configurationslist>"
        ))
        .as_bytes(),
    );
    assert!(
        doc.configurations[0]
            .material
            .as_ref()
            .unwrap()
            .definition_index
            .is_none()
    );
    let doc = document(
        xml(&format!(
            "{DEFINITION}<configurationslist>{reference}{reference}</configurationslist>"
        ))
        .as_bytes(),
    );
    assert!(doc.configurations[0].material.is_none());
    assert!(
        doc.diagnostics
            .iter()
            .any(|d| d.message.contains("ambiguous references"))
    );
}
#[test]
fn references_do_not_fall_back_to_label_or_first_definition() {
    for reference in ["example|test alloy", "example|test alloy|8", "missing"] {
        let doc = document(xml(&format!("{DEFINITION}<configurationslist><configuration name=\"test\" material=\"{reference}\"/></configurationslist>")).as_bytes());
        assert!(
            doc.configurations[0]
                .material
                .as_ref()
                .unwrap()
                .definition_index
                .is_none()
        );
    }
    let doc = document(xml(&format!("{DEFINITION}<configurationslist><configuration name=\"other\" material=\"example|test alloy|7\"/></configurationslist>")).as_bytes());
    assert!(doc.configurations[0].material.is_none());
    assert_eq!(
        doc.materials.unwrap().assignments[0].definition_index,
        Some(0)
    );
}
#[test]
fn invalid_density_is_retained_as_token_without_numeric_claim() {
    for value in ["NaN", "inf", "-1", "0", "not a number"] {
        let doc = document(xml(&DEFINITION.replace("2700.000000", value)).as_bytes());
        let materials = doc.materials.unwrap();
        let density = materials.definitions[0].density.as_ref().unwrap();
        assert_eq!(density.value, value);
        assert_eq!(density.numeric_value, None);
        assert!(
            doc.diagnostics
                .iter()
                .any(|d| d.message.contains("invalid density"))
        );
    }
    let doc = document(
        xml(&DEFINITION.replace(
            "<DENS displayname=",
            "<DENS units=\"explicit source unit\" displayname=",
        ))
        .as_bytes(),
    );
    assert_eq!(
        doc.materials.unwrap().definitions[0]
            .density
            .as_ref()
            .unwrap()
            .units
            .as_deref(),
        Some("explicit source unit")
    );
}
#[test]
fn absent_malformed_and_unsupported_materials_have_distinct_diagnostics() {
    let absent = sw_rs::parse(&file(&[("other", b"data")]), "x.SLDPRT").unwrap();
    assert!(absent.materials.is_none());
    for bytes in [
        b"<bad".as_slice(),
        b"<materials/>",
        &[0xff, 0xfe, 0],
        &[0xff, 0xfe, 0x00, 0xd8],
    ] {
        let doc = document(bytes);
        assert!(doc.materials.is_none());
        assert!(
            doc.diagnostics
                .iter()
                .any(|d| d.message.starts_with("swXmlContents/MATERIALTREE:"))
        );
    }
    let empty = document(xml("").as_bytes());
    assert!(empty.materials.unwrap().definitions.is_empty());
}
#[test]
fn multiple_density_records_are_not_arbitrarily_selected() {
    let doc = document(
        xml(&DEFINITION.replace(
            "</physicalproperties>",
            "<DENS value=\"123\"/></physicalproperties>",
        ))
        .as_bytes(),
    );
    assert!(doc.materials.unwrap().definitions[0].density.is_none());
    assert!(
        doc.diagnostics
            .iter()
            .any(|d| d.message.contains("ambiguous density"))
    );
}
