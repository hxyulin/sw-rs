// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
//! Owned synthetic appearance fixtures; no third-party CAD or shader assets.
use super::*;
fn class(out: &mut Vec<u8>, name: &str) {
    out.extend([255, 255, 1, 0]);
    out.extend((name.len() as u16).to_le_bytes());
    out.extend(name.as_bytes());
}
fn string(out: &mut Vec<u8>, text: &str) {
    let units: Vec<_> = text.encode_utf16().collect();
    out.extend([255, 254, 255]);
    if units.len() < 254 {
        out.push(units.len() as u8);
    } else {
        out.push(254);
        out.extend((units.len() as u16).to_le_bytes());
    }
    out.extend(units.into_iter().flat_map(u16::to_le_bytes));
}
fn property(rgb: [u8; 3], texture: &str, name: &str) -> Vec<u8> {
    let mut out = vec![rgb[0], rgb[1], rgb[2], 0];
    out.extend([0; 8]);
    string(&mut out, texture);
    string(&mut out, name);
    out.extend(
        [1., 0.7, 0.5, 0.3125, 0., 0.]
            .into_iter()
            .flat_map(f64::to_le_bytes),
    );
    out
}
fn extension_bytes(out: &mut Vec<u8>) {
    out.extend(
        [0_u32, 1, 0, 1, 0, 1]
            .into_iter()
            .flat_map(u32::to_le_bytes),
    );
    string(out, "");
    out.extend([1_u32, 1, 3, 0].into_iter().flat_map(u32::to_le_bytes));
    string(out, "independent_shader");
    out.push(2);
    out.extend(0x00abcdef_u32.to_le_bytes());
    out.extend([1_f32; 10].into_iter().flat_map(f32::to_le_bytes));
    out.extend(1_u32.to_le_bytes());
    out.extend([0_f32; 5].into_iter().flat_map(f32::to_le_bytes));
    out.extend([0; 12]);
    string(out, "<SystemTexture>\\metal\\test.p2m");
    out.extend([0; 4]);
    out.extend([0.4_f32, 1.].into_iter().flat_map(f32::to_le_bytes));
    string(out, "<SystemTexture>\\test.dds");
    out.extend([0.001_f32, 0.005].into_iter().flat_map(f32::to_le_bytes));
    out.extend([0_u32, 320, 0].into_iter().flat_map(u32::to_le_bytes));
    out.extend([1_f32; 14].into_iter().flat_map(f32::to_le_bytes));
    string(out, "");
}
fn root(key: &str, value: &[u8]) -> Vec<u8> {
    let mut out = 18000_u32.to_le_bytes().to_vec();
    out.extend([0; 4]);
    class(&mut out, "visStateKeyData_c");
    out.extend(
        [5_u32, 1, 1, 0, 1, 1, 1, 1]
            .into_iter()
            .flat_map(u32::to_le_bytes),
    );
    string(&mut out, key);
    out.extend([0; 16]);
    out.extend(1_u32.to_le_bytes());
    class(&mut out, "moCompHolder_c");
    class(&mut out, "moUnitComponent_c");
    out.extend([0; 43]);
    out.extend([255; 16]);
    out.extend([0; 20]);
    out.extend([18000_u32, 1, 1, 2].into_iter().flat_map(u32::to_le_bytes));
    class(&mut out, "visStateVisualPropertiesAttribute_c");
    out.extend(1_u32.to_le_bytes());
    class(&mut out, "moVisualProperties_c");
    out.extend(value);
    out
}
fn doc(visual: &[u8], configurations: &[&str], kind: &str) -> crate::Document {
    let tree = format!(
        "<swSolidWorks><swModelList>{}</swModelList></swSolidWorks>",
        configurations
            .iter()
            .enumerate()
            .map(|(index, name)| format!(
                r#"<swModel swConfigurationId="{index}" swConfigurationName="{name}"/>"#
            ))
            .collect::<String>()
    );
    let mut streams = BTreeMap::from([
        ("swXmlContents/COMPINSTANCETREE".into(), tree.into_bytes()),
        ("ThirdPtyStore/VisualStates".into(), visual.to_vec()),
    ]);
    for index in 0..configurations.len() {
        streams.insert(format!("docProps/Config-{index}-Properties.xml"), vec![]);
    }
    crate::metadata_from_container(
        crate::Container {
            streams,
            diagnostics: vec![],
        },
        kind,
    )
}
#[test]
fn native_part_default_has_exact_state_scope_and_no_physical_assignment() {
    let native = root("<配置_甲>_显示_状态", &property([1, 2, 3], "纹理.jpg", ""));
    let doc = doc(&native, &["配置_甲"], "part.SLDPRT");
    let presentation = doc.presentation();
    assert!(presentation.physical_materials.is_empty());
    assert_eq!(presentation.appearance_bindings.len(), 1);
    let binding = &presentation.appearance_bindings[0];
    assert!(matches!(binding.target, AppearanceTarget::Document));
    assert!(matches!(&binding.configuration,PresentationScope::Named{name} if name=="配置_甲"));
    assert!(matches!(&binding.display_state,PresentationScope::Named{name} if name=="显示_状态"));
    let value = presentation.appearances[0].native.as_ref().unwrap();
    assert_eq!(value.color_rgb, Some([1, 2, 3]));
    assert_eq!(value.texture_reference.value, "纹理.jpg");
    assert_eq!(value.material_name.value, "");
    assert_eq!(value.optical_values.unwrap()[1], 0.7);
    assert!(presentation.coverage.document_appearance_bindings_decoded);
    assert!(!presentation.coverage.appearance_bindings_decoded);
    assert!(!presentation.coverage.display_states_decoded);
    assert!(
        presentation
            .visual_diagnostics
            .iter()
            .any(|d| d.code == "partial_visual_graph")
    );
}
#[test]
fn ambiguous_configuration_prefixes_and_multiple_states_never_become_global() {
    let bytes = root("A_B_State", &property([1, 2, 3], "", "label"));
    let ambiguous = doc(&bytes, &["A", "A_B"], "part.SLDPRT");
    assert!(ambiguous.visuals.bindings.is_empty());
    assert!(matches!(
        ambiguous.visuals.display_states[0].configuration,
        PresentationScope::Unknown
    ));
    let mut bytes = root("<A>_first", &property([1, 2, 3], "", ""));
    // Additional independently framed key in the same key interval remains ambiguous.
    let index = bytes
        .windows(b"moCompHolder_c".len())
        .position(|b| b == b"moCompHolder_c")
        .unwrap()
        - 6;
    let mut extra = vec![];
    string(&mut extra, "<A>_second");
    bytes.splice(index..index, extra);
    let multiple = doc(&bytes, &["A"], "part.SLDPRT");
    assert_eq!(multiple.visuals.display_states.len(), 2);
    assert!(multiple.visuals.bindings.is_empty());
}
#[test]
fn declarations_and_names_alone_do_not_assign_assembly_or_unknown_version() {
    let bytes = root(
        "<A>_State",
        &property([1, 2, 3], "", "physical-looking name"),
    );
    assert!(
        doc(&bytes, &["A"], "assembly.SLDASM")
            .visuals
            .bindings
            .is_empty()
    );
    let mut unknown = bytes.clone();
    unknown[..4].copy_from_slice(&99999_u32.to_le_bytes());
    assert!(
        doc(&unknown, &["A"], "part.SLDPRT")
            .visuals
            .bindings
            .is_empty()
    );
    let mut standalone = vec![];
    class(&mut standalone, "moVisualProperties_c");
    standalone.extend(property([1, 2, 3], "", ""));
    let data = doc(&standalone, &["A"], "part.SLDPRT").visuals;
    assert_eq!(data.definitions.len(), 1);
    assert!(data.bindings.is_empty());
    assert_eq!(data.unresolved_bindings.len(), 1);
}
#[test]
fn variable_strings_optical_bits_and_mapping_numbers_are_lossless() {
    let mut bytes = property([0x12, 0x34, 0x56], "长路径.jpg", &"名".repeat(300));
    extension_bytes(&mut bytes);
    let native = definition(&bytes, 0, "test").unwrap().native.unwrap();
    assert_eq!(native.material_name.value, "名".repeat(300));
    let extension = native.extension.unwrap();
    assert_eq!(extension.shader_identifier.value, "independent_shader");
    assert_eq!(extension.mapping_flags, [0, 320, 0]);
    assert_eq!(extension.mapping_scalars, [0.001, 0.005]);
    assert_eq!(
        extension.appearance_reference.value,
        "<SystemTexture>\\metal\\test.p2m"
    );
    let mut nan = property([1, 2, 3], "", "");
    let at = nan.len() - 48;
    nan[at..at + 8].copy_from_slice(&0x7ff8000000000123_u64.to_le_bytes());
    let native = definition(&nan, 0, "test").unwrap().native.unwrap();
    assert!(native.optical_values.is_none());
    assert_eq!(native.optical_bits[0], "7FF8000000000123");
    assert_eq!(
        serde_json::to_value(native).unwrap()["optical_bits"][0],
        "7FF8000000000123"
    );
}
#[test]
fn every_truncated_prefix_and_bad_utf16_fails_without_panicking() {
    let bytes = property([1, 2, 3], "texture", "name");
    for end in 0..bytes.len() {
        assert!(definition(&bytes[..end], 0, "test").is_none(), "{end}");
    }
    let mut bad = bytes.clone();
    bad[16..18].copy_from_slice(&0xd800_u16.to_le_bytes());
    assert!(definition(&bad, 0, "test").is_none());
    let mut oversized = vec![255, 254, 255, 255];
    oversized.extend(u32::MAX.to_le_bytes());
    assert!(
        Reader {
            bytes: &oversized,
            at: 0
        }
        .string()
        .is_none()
    );
}
fn descriptor(out: &mut Vec<u8>, size: u32, kind: u32, count: u32, data: &[u8]) {
    out.extend(
        [size, kind, 2, count]
            .into_iter()
            .flat_map(u32::to_le_bytes),
    );
    out.extend(data);
}
fn face(out: &mut Vec<u8>, color: Option<[u8; 3]>) -> usize {
    class(out, "uoTempFaceTessData_c");
    out.extend([1_u32, 1].into_iter().flat_map(u32::to_le_bytes));
    let at = out.len();
    descriptor(out, 4, 8, 1, &3_u32.to_le_bytes());
    descriptor(
        out,
        12,
        100,
        3,
        &[0_f32, 0., 0., 1., 0., 0., 0., 1., 0.]
            .into_iter()
            .flat_map(f32::to_le_bytes)
            .collect::<Vec<_>>(),
    );
    descriptor(out, 12, 100, 3, &[0; 36]);
    descriptor(out, 4, 8, 4, &[0; 16]);
    descriptor(out, 4, 8, 1, &4_u32.to_le_bytes());
    descriptor(out, 1, 8, 4, &[0; 4]);
    if let Some(color) = color {
        out.extend([0x33, 0x80]);
        out.extend(property(color, "", "face"));
    }
    at
}
#[test]
fn cache_bindings_use_exact_table_offsets_and_retain_conflicting_layers() {
    let mut bytes = vec![];
    let first = face(&mut bytes, Some([255, 0, 0]));
    let second = face(&mut bytes, None);
    class(&mut bytes, "uoBodyPropInfo_c");
    bytes.extend([0x33, 0x80]);
    bytes.extend(property([0, 0, 255], "", "body"));
    let faces = mesh::section_display_faces(&bytes);
    assert_eq!(faces.len(), 2);
    let data = cached(&bytes, "Contents/DisplayLists", &faces, &[]);
    assert_eq!(data.definitions.len(), 2);
    assert_eq!(data.bindings.len(), 3);
    assert_eq!(
        data.bindings
            .iter()
            .filter(|b| b.layer == AppearanceLayer::CachedFace)
            .count(),
        1
    );
    let body: Vec<_> = data
        .bindings
        .iter()
        .filter(|b| b.layer == AppearanceLayer::CachedBody)
        .collect();
    assert_eq!(body[0].appearance_index, body[1].appearance_index);
    for (binding, at) in [(body[0], first), (body[1], second)] {
        assert!(
            matches!(&binding.target,AppearanceTarget::Tessellation {source_stream,source_offset} if source_stream=="Contents/DisplayLists" && *source_offset==at)
        );
        assert!(matches!(binding.display_state, PresentationScope::Unknown));
    }
}
#[test]
fn source_feature_id_and_timestamp_are_not_promoted_to_guessed_face_ownership() {
    let mut bytes = vec![];
    class(&mut bytes, "moCompFeature_c");
    bytes.extend(
        [18000_u32, 71, 0x12345678, 1, 1, 2]
            .into_iter()
            .flat_map(u32::to_le_bytes),
    );
    bytes.extend([7, 128, 1, 0, 0, 0, 9, 128]);
    bytes.extend(property([0, 255, 0], "", "feature"));
    let native = doc(&bytes, &["A"], "part.SLDPRT").visuals;
    assert!(native.bindings.is_empty());
    assert_eq!(native.definitions.len(), 1);
    assert_eq!(native.unresolved_bindings[0].identity, [71, 0x12345678]);
    bytes[6 + b"moCompFeature_c".len() + 12] = 0;
    assert!(
        doc(&bytes, &["A"], "part.SLDPRT")
            .visuals
            .definitions
            .is_empty()
    );
}
#[test]
fn projection_offsets_native_indices_after_material_defaults() {
    let bytes = root("<A>_State", &property([1, 2, 3], "", ""));
    let mut doc = doc(&bytes, &["A"], "part.SLDPRT");
    doc.materials = Some(crate::MaterialData {
        source_stream: "MATERIALTREE".into(),
        definitions: vec![crate::SavedMaterial {
            name: "Unassigned".into(),
            material_id: None,
            classifications: vec![],
            density: None,
            saved_swatch_rgb: Some("abcdef".into()),
            appearance: None,
            source_path: "/material".into(),
        }],
        assignments: vec![],
        body_assignments_decoded: false,
    });
    let presentation = doc.presentation();
    assert_eq!(presentation.appearances.len(), 2);
    assert_eq!(
        presentation.appearance_bindings[0].appearance_index,
        Some(1)
    );
    assert!(matches!(
        presentation.appearances[0].origin,
        AppearanceOrigin::MaterialDefault { .. }
    ));
    assert!(matches!(
        presentation.appearances[1].origin,
        AppearanceOrigin::SavedAppearance
    ));
}
#[test]
fn conditional_legacy_texture_block_preserves_values_without_guessing_mapping() {
    let mut standard = vec![];
    extension_bytes(&mut standard);
    let mut extra = 1_u32.to_le_bytes().to_vec();
    extra.extend([12.5_f64; 13].into_iter().flat_map(f64::to_le_bytes));
    string(&mut extra, "local image reference");
    extra.extend([0; 6]);
    extra.extend([0.5_f64, 0.25].into_iter().flat_map(f64::to_le_bytes));
    extra.extend(
        [0_u32, 1, 1, 0, 1, 1]
            .into_iter()
            .flat_map(u32::to_le_bytes),
    );
    extra.extend(&standard[24..]);
    let mut bytes = property([1, 2, 3], "", "");
    bytes.extend(extra);
    let native = definition(&bytes, 0, "test").unwrap().native.unwrap();
    let extension = native.extension.unwrap();
    let block = extension.legacy_texture_block.unwrap();
    assert_eq!(block.values, [12.5; 13]);
    assert_eq!(block.additional_values, [0.5, 0.25]);
    assert_eq!(block.reference.value, "local image reference");
    assert_eq!(extension.shader_identifier.value, "independent_shader");
}
#[test]
fn body_property_slot_accepts_its_typed_reference_and_never_scans_later_objects() {
    let mut bytes = vec![0, 0];
    bytes.extend([1_u32, 6, 0, 1, 0].into_iter().flat_map(u32::to_le_bytes));
    string(&mut bytes, "Body label");
    bytes.extend([0_u32, 1, u32::MAX].into_iter().flat_map(u32::to_le_bytes));
    bytes.extend(0x8035_u16.to_le_bytes());
    bytes.extend(property([1, 2, 3], "", "body"));
    assert_eq!(
        body_property(&bytes, "test", 0)
            .unwrap()
            .native
            .unwrap()
            .color_rgb,
        Some([1, 2, 3])
    );
    let mut unknown = bytes.clone();
    unknown[2..6].copy_from_slice(&2_u32.to_le_bytes());
    unknown.extend([0x33, 0x80]);
    unknown.extend(property([255, 0, 0], "", "later unrelated object"));
    assert!(body_property(&unknown, "test", 0).is_none());
}
#[test]
fn cache_conflicts_and_unresolved_links_block_lower_layer_fallback() {
    let mut bytes = vec![];
    let offset = face(&mut bytes, Some([255, 0, 0]));
    class(&mut bytes, "uoBodyPropInfo_c");
    bytes.extend([0x33, 0x80]);
    bytes.extend(property([0, 0, 255], "", "body"));
    let data = cached(
        &bytes,
        "DisplayLists",
        &mesh::section_display_faces(&bytes),
        &[],
    );
    let doc = doc(&[], &["A"], "part.SLDPRT");
    let mut presentation = doc.presentation();
    presentation.append_visuals(&data);
    assert_eq!(
        presentation.cached_property("DisplayLists", offset),
        crate::CachedPropertyResolution::Resolved {
            appearance_index: 0,
            layer: AppearanceLayer::CachedFace
        }
    );
    assert_eq!(
        presentation.cached_property("DisplayLists", offset + 1),
        crate::CachedPropertyResolution::NotDecoded
    );
    let mut conflict = presentation.appearance_bindings[0].clone();
    conflict.appearance_index = Some(1);
    presentation.appearance_bindings.push(conflict);
    assert_eq!(
        presentation.cached_property("DisplayLists", offset),
        crate::CachedPropertyResolution::Ambiguous {
            layer: AppearanceLayer::CachedFace
        }
    );
    presentation
        .appearance_bindings
        .last_mut()
        .unwrap()
        .appearance_index = None;
    assert_eq!(
        presentation.cached_property("DisplayLists", offset),
        crate::CachedPropertyResolution::Unresolved {
            layer: AppearanceLayer::CachedFace
        }
    );
}
#[test]
fn document_lookup_keeps_state_names_exact_and_rejects_conflicting_defaults() {
    let bytes = root("<A>_State", &property([1, 2, 3], "", ""));
    let doc = doc(&bytes, &["A"], "part.SLDPRT");
    let mut presentation = doc.presentation();
    assert!(
        presentation
            .document_appearance_for_state("A", "State")
            .is_some()
    );
    assert!(
        presentation
            .document_appearance_for_state("A", "state")
            .is_none()
    );
    let mut conflict = presentation.appearance_bindings[0].clone();
    conflict.appearance_index = None;
    presentation.appearance_bindings.push(conflict);
    assert!(
        presentation
            .document_appearance_for_state("A", "State")
            .is_none()
    );
}
#[test]
fn packed_color_sentinels_remain_raw_instead_of_becoming_white_or_alpha() {
    let mut bytes = property([1, 2, 3], "", "label");
    bytes[..4].copy_from_slice(&u32::MAX.to_le_bytes());
    let value = definition(&bytes, 0, "test").unwrap();
    assert!(value.swatch_rgb.is_none());
    let native = value.native.unwrap();
    assert_eq!(native.packed_color, u32::MAX);
    assert_eq!(native.color_rgb, None);
    assert_eq!(native.material_name.value, "label");
}
