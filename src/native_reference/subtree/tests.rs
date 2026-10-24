// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
//! Authored archives exercise complete payload consumption and typed backreferences.
use super::*;
fn class(b: &mut Vec<u8>, name: &str) {
    b.extend([255, 255, 1, 0]);
    b.extend((name.len() as u16).to_le_bytes());
    b.extend(name.as_bytes());
}
fn tag(b: &mut Vec<u8>, n: u16) {
    b.extend(n.to_le_bytes());
}
fn words(b: &mut Vec<u8>, ns: &[u32]) {
    for n in ns {
        b.extend(n.to_le_bytes());
    }
}
fn text(b: &mut Vec<u8>, s: &str) {
    let units = s.encode_utf16().collect::<Vec<_>>();
    b.extend([255, 254, 255, 254]);
    tag(b, units.len() as u16);
    b.extend(units.into_iter().flat_map(u16::to_le_bytes));
}
fn external_tail(b: &mut Vec<u8>, profile: NativeExternalObjectProfile) {
    b.extend([2, 0, 0]);
    words(b, &[0x12345678]);
    text(b, "");
    text(b, "saved note");
    match profile {
        NativeExternalObjectProfile::TwoStrings => b.push(0x7f),
        NativeExternalObjectProfile::ThreeStrings => {
            text(b, "third");
            tag(b, 0x1234);
        }
    }
    words(b, &[0xcccccccc, 2, 3, 4]);
    text(b, "配置😀");
    words(b, &[5, 6, 7]);
    if matches!(profile, NativeExternalObjectProfile::ThreeStrings) {
        words(b, &[8]);
    }
}
fn array_header(b: &mut Vec<u8>, name: &str, count: u32) {
    class(b, name);
    words(b, &[1, 0, count, 512, 0xaabbccdd]);
    b.extend(COMPONENT_MARKER);
    tag(b, 0);
}
fn fr_backref(b: &mut Vec<u8>, external: u16) {
    tag(b, 0x8005);
    tag(b, external);
    words(b, &[33, 0x87654321]);
}
fn edge(profile: NativeExternalObjectProfile) -> Vec<u8> {
    let mut b = Vec::new();
    array_header(&mut b, "moEdgeRef_c", 4); // root class/object 1/2
    class(&mut b, "moFromSktEntSurfIdRep_c");
    tag(&mut b, 0); // 3/4
    class(&mut b, "moFR_c"); // 5/6
    class(&mut b, "moExtObject_c"); // 7/8
    class(&mut b, "moCStringHandle_c");
    text(&mut b, "../原始/Part.SLDPRT"); // 9/10
    tag(&mut b, 0x8009);
    text(&mut b, "Part"); // handle object 11
    external_tail(&mut b, profile);
    words(&mut b, &[33, 0x87654321]);
    words(&mut b, &[4]);
    class(&mut b, "moEndFaceSurfIdRep_c");
    tag(&mut b, 0); // 12/13
    fr_backref(&mut b, 8); // FR object 14, external object reused
    words(&mut b, &[0, 99]);
    class(&mut b, "moDerivedSurfIdRep_c");
    tag(&mut b, 0); // 15/16
    fr_backref(&mut b, 8); // FR object 17
    class(&mut b, "moFilletSurfIdRep_c");
    tag(&mut b, 0); // 18/19
    fr_backref(&mut b, 8); // FR object 20
    words(&mut b, &[12]);
    words(&mut b, &[123]);
    tag(&mut b, 0); // array null retained
    b
}
fn face(profile: NativeExternalObjectProfile) -> Vec<u8> {
    let mut b = Vec::new();
    array_header(&mut b, "moFaceRef_c", 3); // 21/22
    tag(&mut b, 4); // reuse entire earlier surface, no payload
    class(&mut b, "moSimpleSurfIdRep_c");
    tag(&mut b, 0); // 23/24
    tag(&mut b, 0x8005); // FR object 25
    tag(&mut b, 0x8007); // external object 26
    tag(&mut b, 10);
    tag(&mut b, 11); // both handle objects reused, no new strings
    external_tail(&mut b, profile);
    words(&mut b, &[23, 0x99999999]);
    words(&mut b, &[175]);
    tag(&mut b, 0);
    b
}

#[test]
fn array_prefixes_reuse_classes_objects_and_handles_at_authored_serialization_points() {
    for profile in [
        NativeExternalObjectProfile::TwoStrings,
        NativeExternalObjectProfile::ThreeStrings,
    ] {
        let mut reader = NativeSubtreeReader::new(ArchiveContext::new(), profile);
        let edge = edge(profile);
        let mut cursor = BinaryReader::new(&edge);
        let root = reader
            .read_reference_array_prefix(&mut cursor, "Features")
            .unwrap();
        assert_eq!(cursor.position(), edge.len());
        assert_eq!(root.header.object_index, Some(2));
        assert_eq!(reader.archive().entries().len(), 20);
        assert!(!reader.objects()[&2].complete_payload);
        assert_eq!(
            reader.objects()[&2].span.slice(&edge),
            Some(edge.as_slice())
        );
        let NativeSubtreePayload::ReferenceArrayPrefix { records, .. } =
            &reader.objects()[&2].payload
        else {
            panic!()
        };
        assert_eq!(records.len(), 4);
        assert_eq!(records[3].header.object_index, None);
        let NativeSubtreePayload::Surface {
            nested, raw_tail, ..
        } = &reader.objects()[&16].payload
        else {
            panic!()
        };
        assert_eq!(nested.as_ref().unwrap().header.object_index, Some(19));
        assert_eq!(raw_tail, &[123]);
        let NativeSubtreePayload::FeatureReference {
            external_object, ..
        } = &reader.objects()[&14].payload
        else {
            panic!()
        };
        assert_eq!(external_object.header.object_index, Some(8));
        assert!(!external_object.header.is_new);
        let before = serde_json::to_value(&reader.objects()[&8]).unwrap();
        let face = face(profile);
        let mut cursor = BinaryReader::new(&face);
        let root = reader
            .read_reference_array_prefix(&mut cursor, "Mates")
            .unwrap();
        assert_eq!(cursor.position(), face.len());
        assert_eq!(root.header.object_index, Some(22));
        assert_eq!(reader.archive().entries().len(), 26);
        assert_eq!(serde_json::to_value(&reader.objects()[&8]).unwrap(), before);
        let NativeSubtreePayload::ReferenceArrayPrefix { records, .. } =
            &reader.objects()[&22].payload
        else {
            panic!()
        };
        assert_eq!(records[0].header.object_index, Some(4));
        assert!(!records[0].header.is_new);
        assert_eq!(records[0].source_stream, "Mates");
        assert_eq!(reader.objects()[&4].source_stream, "Features");
        let NativeSubtreePayload::ExternalObject {
            handles,
            raw_before_configuration,
            raw_tail,
            ..
        } = &reader.objects()[&26].payload
        else {
            panic!()
        };
        assert_eq!(handles[0].header.object_index, Some(10));
        assert_eq!(handles[1].header.object_index, Some(11));
        assert!(handles.iter().all(|p| !p.header.is_new));
        assert_eq!(
            raw_before_configuration.len(),
            match profile {
                NativeExternalObjectProfile::TwoStrings => 17,
                _ => 18,
            }
        );
        assert_eq!(
            raw_tail.len(),
            match profile {
                NativeExternalObjectProfile::TwoStrings => 12,
                _ => 16,
            }
        );
        let NativeSubtreePayload::StringHandle { text } = &reader.objects()[&10].payload else {
            panic!()
        };
        assert_eq!(text.value, "../原始/Part.SLDPRT");
    }
}

#[test]
fn truncation_rolls_back_new_values_map_cursor_and_preserves_prior_stream() {
    let profile = NativeExternalObjectProfile::TwoStrings;
    let edge = edge(profile);
    let face = face(profile);
    for end in 0..edge.len() {
        let mut reader = NativeSubtreeReader::new(ArchiveContext::new(), profile);
        let mut cursor = BinaryReader::new(&edge[..end]);
        assert!(
            reader
                .read_reference_array_prefix(&mut cursor, "s")
                .is_err(),
            "end={end}"
        );
        assert_eq!(cursor.position(), 0);
        assert!(reader.archive().entries().is_empty());
        assert!(reader.objects().is_empty());
    }
    let mut reader = NativeSubtreeReader::new(ArchiveContext::new(), profile);
    reader
        .read_reference_array_prefix(&mut BinaryReader::new(&edge), "first")
        .unwrap();
    let saved = serde_json::to_value(reader.objects()).unwrap();
    for end in 0..face.len() {
        let mut cursor = BinaryReader::new(&face[..end]);
        assert!(
            reader
                .read_reference_array_prefix(&mut cursor, "second")
                .is_err()
        );
        assert_eq!(cursor.position(), 0);
        assert_eq!(reader.archive().entries().len(), 20);
        assert_eq!(serde_json::to_value(reader.objects()).unwrap(), saved);
    }
    reader
        .read_reference_array_prefix(&mut BinaryReader::new(&face), "second")
        .unwrap();
    assert_eq!(reader.archive().entries().len(), 26);
}

#[test]
fn missing_payload_wrong_types_unknown_classes_and_cycles_stop_without_resuming() {
    let profile = NativeExternalObjectProfile::TwoStrings;
    let mut bytes = Vec::new();
    class(&mut bytes, "moExtObject_c");
    let mut archive = ArchiveContext::new();
    archive.read_object(&mut BinaryReader::new(&bytes)).unwrap();
    let mut reader = NativeSubtreeReader::new(archive, profile);
    let mut cursor = BinaryReader::new(&[2, 0]);
    assert_eq!(
        reader
            .read_external_object(&mut cursor, "s")
            .unwrap_err()
            .message,
        "referenced subtree payload is missing or still in progress"
    );
    assert_eq!(cursor.position(), 0);
    assert_eq!(reader.archive().entries().len(), 2);
    let mut wrong = Vec::new();
    class(&mut wrong, "moEndFace3IntSurfIdRep_c");
    wrong.extend([0; 64]);
    assert!(
        reader
            .read_surface(&mut BinaryReader::new(&wrong), "s")
            .is_err()
    );
    let mut cyclic = Vec::new();
    class(&mut cyclic, "moDerivedSurfIdRep_c");
    tag(&mut cyclic, 0);
    tag(&mut cyclic, 0);
    tag(&mut cyclic, 2);
    words(&mut cyclic, &[0]);
    let mut reader = NativeSubtreeReader::new(ArchiveContext::new(), profile);
    assert!(
        reader
            .read_surface(&mut BinaryReader::new(&cyclic), "s")
            .is_err()
    );
    assert!(reader.archive().entries().is_empty());
    reader
        .read_reference_array_prefix(&mut BinaryReader::new(&edge(profile)), "s")
        .unwrap();
    let mut cursor = BinaryReader::new(&[8, 0]);
    assert!(reader.read_surface(&mut cursor, "s").is_err()); // external object is not a surface
    assert_eq!(cursor.position(), 0);
}

#[test]
fn pointer_count_depth_and_invalid_field_budgets_are_transactional() {
    let profile = NativeExternalObjectProfile::TwoStrings;
    for (pointers, depth) in [(0, 64), (1, 64), (1000, 1)] {
        let mut reader =
            NativeSubtreeReader::with_limits(ArchiveContext::new(), profile, pointers, depth);
        let bytes = edge(profile);
        let mut cursor = BinaryReader::new(&bytes);
        assert!(
            reader
                .read_reference_array_prefix(&mut cursor, "s")
                .is_err()
        );
        assert_eq!(cursor.position(), 0);
        assert!(reader.archive().entries().is_empty());
    }
    let mut forged = Vec::new();
    array_header(&mut forged, "moEdgeRef_c", u32::MAX);
    let mut reader = NativeSubtreeReader::new(ArchiveContext::new(), profile);
    assert!(
        reader
            .read_reference_array_prefix(&mut BinaryReader::new(&forged), "s")
            .is_err()
    );
    let mut bad = edge(profile);
    let at = bad.windows(6).position(|b| b == b"moFR_c").unwrap() - 4;
    bad[at] = 2;
    assert!(
        reader
            .read_reference_array_prefix(&mut BinaryReader::new(&bad), "s")
            .is_err()
    );
    let mut bad = edge(profile);
    let at = bad
        .windows(17)
        .position(|b| b == b"moCStringHandle_c")
        .unwrap()
        + 17;
    bad.splice(at..at + 6, [255, 254, 255, 1, 0, 216]);
    assert!(
        reader
            .read_reference_array_prefix(&mut BinaryReader::new(&bad), "s")
            .is_err()
    );
    assert!(reader.archive().entries().is_empty() && reader.objects().is_empty());
}

#[test]
fn wide_class_and_object_tags_use_the_supplied_archive_without_guessing_slots() {
    let profile = NativeExternalObjectProfile::TwoStrings;
    let mut archive = ArchiveContext::new();
    // Authored MapObject calls precede this segment. These are real registrations,
    // not a seed inferred from a saved stream's leading word.
    for _ in 0..32766 {
        archive.map_external_object().unwrap();
    }
    let mut bytes = Vec::new();
    class(&mut bytes, "moExtObject_c");
    class(&mut bytes, "moCStringHandle_c");
    text(&mut bytes, "shared");
    tag(&mut bytes, 0x7fff);
    words(&mut bytes, &[32770]);
    external_tail(&mut bytes, profile);
    let mut reader = NativeSubtreeReader::new(archive, profile);
    let mut cursor = BinaryReader::new(&bytes);
    let first = reader.read_external_object(&mut cursor, "first").unwrap();
    assert_eq!(first.header.class_index, Some(32767));
    assert_eq!(first.header.object_index, Some(32768));
    assert_eq!(cursor.position(), bytes.len());
    let mut next = Vec::new();
    tag(&mut next, 0x7fff);
    words(&mut next, &[0x8000_7fff]);
    // New second external object, using the existing string-handle class once.
    tag(&mut next, 0x7fff);
    words(&mut next, &[0x8000_8001]);
    text(&mut next, "second");
    tag(&mut next, 0x7fff);
    words(&mut next, &[32770]);
    external_tail(&mut next, profile);
    let root = reader
        .read_external_object(&mut BinaryReader::new(&next), "second")
        .unwrap();
    assert_eq!(root.header.object_index, Some(32771));
    assert!(root.header.is_new);
    let mut backref = Vec::new();
    tag(&mut backref, 0x7fff);
    words(&mut backref, &[32768]);
    let root = reader
        .read_external_object(&mut BinaryReader::new(&backref), "third")
        .unwrap();
    assert_eq!(root.header.object_index, Some(32768));
    assert!(!root.header.is_new);
    assert_eq!(reader.objects().len(), 4);
}

#[test]
fn wrong_external_profile_and_scalar_marker_cannot_skip_to_later_objects() {
    let mut reader = NativeSubtreeReader::new(
        ArchiveContext::new(),
        NativeExternalObjectProfile::ThreeStrings,
    );
    let bytes = edge(NativeExternalObjectProfile::TwoStrings);
    let mut cursor = BinaryReader::new(&bytes);
    assert!(
        reader
            .read_reference_array_prefix(&mut cursor, "s")
            .is_err()
    );
    assert_eq!(cursor.position(), 0);
    let mut bytes = edge(NativeExternalObjectProfile::ThreeStrings);
    let at = bytes
        .windows(7)
        .position(|w| w == [2, 0, 0, 0x78, 0x56, 0x34, 0x12])
        .unwrap();
    bytes[at] = 3;
    assert!(
        reader
            .read_reference_array_prefix(&mut BinaryReader::new(&bytes), "s")
            .is_err()
    );
    assert!(reader.archive().entries().is_empty());
    assert!(reader.objects().is_empty());
}

#[test]
fn incomplete_array_roots_cannot_be_used_as_complete_backreference_values() {
    let profile = NativeExternalObjectProfile::TwoStrings;
    let mut reader = NativeSubtreeReader::new(ArchiveContext::new(), profile);
    reader
        .read_reference_array_prefix(&mut BinaryReader::new(&edge(profile)), "s")
        .unwrap();
    let mut cursor = BinaryReader::new(&[2, 0]);
    assert!(
        reader
            .read_reference_array_prefix(&mut cursor, "later")
            .is_err()
    );
    assert_eq!(cursor.position(), 0);
    assert!(!reader.objects()[&2].complete_payload);
    let mut limited = NativeSubtreeReader::with_limits(ArchiveContext::new(), profile, 1, 64);
    assert!(
        limited
            .read_reference_array_prefix(&mut BinaryReader::new(&edge(profile)), "s")
            .is_err()
    );
    // A failed root must not spend the only pointer allowance.
    limited
        .read_surface(&mut BinaryReader::new(&[0, 0]), "s")
        .unwrap();
    assert!(
        limited
            .read_surface(&mut BinaryReader::new(&[0, 0]), "s")
            .is_err()
    );
}

#[test]
fn enclosing_decoders_share_the_map_and_record_handles_before_later_reuse() {
    let profile = NativeExternalObjectProfile::TwoStrings;
    let mut reader = NativeSubtreeReader::new(ArchiveContext::new(), profile);
    let mut configuration = Vec::new();
    class(&mut configuration, "AuthoredEnclosingClass"); // class/object 1/2
    class(&mut configuration, "moCStringHandle_c"); // class/object 3/4
    text(&mut configuration, "saved earlier in configuration");
    let mut cursor = BinaryReader::new(&configuration);
    reader.archive_mut().read_object(&mut cursor).unwrap();
    let handle = reader
        .read_string_handle(&mut cursor, "Configuration")
        .unwrap();
    assert_eq!(handle.header.object_index, Some(4));
    assert_eq!(cursor.position(), configuration.len());
    // An authored, explicit external-object mapping in the enclosing call sequence.
    assert_eq!(reader.archive_mut().map_external_object().unwrap(), 5);
    let mut external = Vec::new();
    class(&mut external, "moExtObject_c"); // class/object 6/7
    tag(&mut external, 4);
    tag(&mut external, 4);
    external_tail(&mut external, profile);
    let root = reader
        .read_external_object(&mut BinaryReader::new(&external), "Later")
        .unwrap();
    assert_eq!(root.header.object_index, Some(7));
    let NativeSubtreePayload::ExternalObject { handles, .. } = &reader.objects()[&7].payload else {
        panic!()
    };
    assert!(
        handles
            .iter()
            .all(|handle| handle.header.object_index == Some(4) && !handle.header.is_new)
    );
    assert_eq!(reader.objects()[&4].source_stream, "Configuration");
    assert_eq!(reader.objects().len(), 2);
}
