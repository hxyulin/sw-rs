// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
use super::*;
use crate::visual::tests::{class, extension_bytes, property, string};
fn words(b: &mut Vec<u8>, values: &[u32]) {
    b.extend(values.iter().flat_map(|v| v.to_le_bytes()));
}
fn tag(b: &mut Vec<u8>, value: u16) {
    b.extend(value.to_le_bytes());
}
fn complete_property(b: &mut Vec<u8>, rgb: [u8; 3]) {
    b.extend(property(rgb, "", ""));
    extension_bytes(b);
    words(b, &[0]);
    for _ in 0..3 {
        string(b, "");
    }
    b.extend([0; 92]);
    string(b, "");
    b.extend([0; 8]);
    string(b, "");
    b.extend([0; 8]);
    string(b, "");
    b.extend([0; 24]);
}
fn authored() -> Vec<u8> {
    let mut b = Vec::new();
    words(&mut b, &[18000, 0]);
    class(&mut b, "visStateKeyData_c");
    words(&mut b, &[5, 1, 1, 0, 2, 1, 2, 1]);
    string(&mut b, "<OldName>_Red");
    words(&mut b, &[0, 0, 0, 2]);
    string(&mut b, "Blue");
    words(&mut b, &[0, 0, 0]);
    words(&mut b, &[0, 2]);
    class(&mut b, "moCompHolder_c");
    class(&mut b, "moUnitComponent_c");
    b.extend([0; 79]);
    words(&mut b, &[18000, 2, 1, 2]);
    class(&mut b, "visStateVisualPropertiesAttribute_c");
    words(&mut b, &[1]);
    class(&mut b, "moVisualProperties_c");
    complete_property(&mut b, [255, 0, 0]);
    class(&mut b, "visStateVisualStateAttribute_c");
    words(&mut b, &[1, 1, 6, 1, 0]);
    words(&mut b, &[2, 2]);
    tag(&mut b, 0x8007);
    words(&mut b, &[1]);
    tag(&mut b, 0x8009);
    complete_property(&mut b, [0, 0, 255]);
    tag(&mut b, 12);
    class(&mut b, "moCompFeature_c");
    tag(&mut b, 0x8005);
    b.extend([0; 79]);
    words(&mut b, &[18000, 49, 12345, 2, 1, 2]);
    tag(&mut b, 13);
    tag(&mut b, 12);
    words(&mut b, &[2, 2]);
    tag(&mut b, 0x8007);
    words(&mut b, &[1]);
    tag(&mut b, 0);
    tag(&mut b, 12);
    words(&mut b, &[0, 0]);
    b
}
#[test]
fn state_ids_shared_properties_and_base_object_slots_are_sequential() {
    let report = parse(&authored());
    assert!(report.complete, "{:?}", report.failure);
    assert_eq!(report.configuration_rows, [[0, 2, 1]]);
    assert_eq!(report.keys[0].configuration_index, 0);
    let unit = &report.components[0];
    let feature = &report.components[1];
    assert_eq!(unit.states.len(), 2);
    assert_eq!(feature.source_identity, Some([49, 12345]));
    let blue = unit.states[1].property_object.expect("blue property");
    assert_eq!(
        feature.states[0].property_object,
        Some(blue),
        "shared attribute points to the same property"
    );
    assert_eq!(report.properties[&blue].color_rgb, Some([0, 0, 255]));
    assert_eq!(
        feature.states[1].property_object, None,
        "explicit null property stays inherited"
    );
    assert!(
        matches!(report.map[16], ArchiveEntry::Object { class_index: 5, .. }),
        "base component consumes its own object slot"
    );
}
#[test]
fn unknown_or_truncated_payloads_never_resume_at_later_declarations() {
    let bytes = authored();
    for length in [0, 8, 100, bytes.len() - 1] {
        assert!(!parse(&bytes[..length]).complete, "truncation at {length}");
    }
    let mut unknown = bytes.clone();
    let at = unknown
        .windows(b"moUnitComponent_c".len())
        .position(|w| w == b"moUnitComponent_c")
        .unwrap();
    unknown[at] = b'X';
    let report = parse(&unknown);
    assert!(!report.complete);
    assert!(
        report.properties.is_empty(),
        "later valid-looking properties are not scanned into the map"
    );
    let mut trailing = bytes;
    trailing.push(1);
    assert!(
        !parse(&trailing).complete,
        "unconsumed bytes invalidate completeness"
    );
}

#[test]
fn untextured_selector_profiles_require_an_observed_paired_header() {
    let original = authored();
    let needle = [0_u32, 1, 0, 1, 0, 1]
        .into_iter()
        .flat_map(u32::to_le_bytes)
        .collect::<Vec<_>>();
    let at = original
        .windows(needle.len())
        .position(|w| w == needle)
        .unwrap();
    let mut alternate = original.clone();
    alternate[at + 16..at + 20].copy_from_slice(&1_u32.to_le_bytes());
    assert!(
        parse(&alternate).complete,
        "supported untextured header consumes its complete payload"
    );
    alternate[at + 16..at + 20].copy_from_slice(&2_u32.to_le_bytes());
    let report = parse(&alternate);
    assert!(!report.complete);
    assert!(
        report.components.is_empty(),
        "unknown extension does not manufacture a completed owner"
    );
}
