// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
//! Authored headers validate the fresh shared map, schema-zero arrays and logs.
use super::super::*;
use super::{class, external_tail, tag, text, words};

fn strings_class(bytes: &mut Vec<u8>) {
    let at = bytes.len();
    class(bytes, "su_CStringArray");
    bytes[at + 2] = 0;
}
fn fixture(profile: NativeExternalObjectProfile) -> Vec<u8> {
    let mut bytes = Vec::new();
    class(&mut bytes, "moHeader_c"); // 1/2
    strings_class(&mut bytes); // 3/4
    tag(&mut bytes, 3);
    for value in ["saved user😀", "saved user😀", ""] {
        text(&mut bytes, value);
    }
    tag(&mut bytes, 0x8003); // second array, object 5
    tag(&mut bytes, 1);
    text(&mut bytes, "");
    class(&mut bytes, "suObList"); // 6/7
    tag(&mut bytes, 2);
    class(&mut bytes, "moLogs_c"); // 8/9
    tag(&mut bytes, 1);
    class(&mut bytes, "moStamp_c"); // 10/11
    bytes.extend([1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    text(&mut bytes, "Created");
    words(&mut bytes, &[0]);
    text(&mut bytes, "saved root");
    tag(&mut bytes, 0x8008); // log object 12
    tag(&mut bytes, 2);
    tag(&mut bytes, 11); // reuse complete stamp, no bytes or new slot
    tag(&mut bytes, 0x800a); // stamp object 13
    bytes.extend([10, 9, 8, 7, 6, 5, 4, 3, 2, 1]);
    text(&mut bytes, "Modified");
    words(&mut bytes, &[227]);
    text(&mut bytes, "a saved name");
    words(&mut bytes, &[0x11223344, 0xaabbccdd]);
    class(&mut bytes, "moExtObjectList_c"); // 14/15
    tag(&mut bytes, 2);
    class(&mut bytes, "moExtObject_c"); // 16/17
    class(&mut bytes, "moCStringHandle_c"); // 18/19
    text(&mut bytes, "../saved/零件.SLDPRT");
    tag(&mut bytes, 0x8012); // handle object 20
    text(&mut bytes, "零件");
    external_tail(&mut bytes, profile);
    tag(&mut bytes, 17); // repeat complete external object
    bytes
}

#[test]
fn model_header_prefix_uses_real_serialization_order_and_retains_the_unread_tail() {
    for profile in [
        NativeExternalObjectProfile::TwoStrings,
        NativeExternalObjectProfile::ThreeStrings,
        NativeExternalObjectProfile::ThreeStringsShortTail,
    ] {
        let mut bytes = fixture(profile);
        let prefix_end = bytes.len();
        words(&mut bytes, &[17, 23]); // unknown enclosing fields remain unread
        let mut reader = NativeSubtreeReader::new(ArchiveContext::new(), profile);
        let mut cursor = BinaryReader::new(&bytes);
        let root = reader
            .read_model_header_prefix(
                &mut cursor,
                "Contents/Config-0-ModelHeader",
                NativeModelHeaderProfile::DirectArrays,
            )
            .unwrap();
        assert_eq!(root.header.object_index, Some(2));
        assert_eq!(cursor.position(), prefix_end);
        assert_eq!(reader.archive().entries().len(), 20);
        let root = &reader.objects()[&2];
        assert!(!root.complete_payload);
        assert_eq!(
            root.span,
            ByteSpan {
                offset: 0,
                length: prefix_end
            }
        );
        let NativeSubtreePayload::ModelHeaderPrefix { header } = &reader.objects()[&2].payload
        else {
            panic!()
        };
        let NativeModelHeaderFields {
            string_arrays,
            logs,
            raw_before_external,
            external_objects,
            ..
        } = header.as_ref();
        assert_eq!(string_arrays[0].header.class_index, Some(3));
        assert_eq!(string_arrays[1].header.class_index, Some(3));
        assert_eq!(string_arrays[1].header.object_index, Some(5));
        assert_eq!(logs.header.object_index, Some(7));
        assert_eq!(raw_before_external, &[0x11223344, 0xaabbccdd]);
        assert_eq!(external_objects.header.object_index, Some(15));
        assert!(
            matches!(reader.archive().entry(3), Some(ArchiveEntry::Class {schema: 0, name, ..}) if name == "su_CStringArray")
        );
        let NativeSubtreePayload::StringArray { strings } = &reader.objects()[&4].payload else {
            panic!()
        };
        assert_eq!(
            strings.iter().map(|s| s.value.as_str()).collect::<Vec<_>>(),
            ["saved user😀", "saved user😀", ""]
        );
        let NativeSubtreePayload::LogRecord {
            stamps,
            raw_word,
            label,
        } = &reader.objects()[&12].payload
        else {
            panic!()
        };
        assert_eq!(stamps[0].header.object_index, Some(11));
        assert!(!stamps[0].header.is_new);
        assert_eq!(stamps[1].header.object_index, Some(13));
        assert_eq!(*raw_word, 227);
        assert_eq!(label.value, "a saved name");
        let NativeSubtreePayload::Stamp { raw_prefix, label } = &reader.objects()[&13].payload
        else {
            panic!()
        };
        assert_eq!(raw_prefix, &[10, 9, 8, 7, 6, 5, 4, 3, 2, 1]);
        assert_eq!(label.value, "Modified");
        let NativeSubtreePayload::ExternalObjectList { records } = &reader.objects()[&15].payload
        else {
            panic!()
        };
        assert_eq!(
            records[0].header.object_index,
            records[1].header.object_index
        );
        assert!(!records[1].header.is_new);
        let saved = serde_json::to_value(reader.objects()).unwrap();
        let later = reader
            .read_external_object(&mut BinaryReader::new(&[17, 0]), "Later")
            .unwrap();
        assert!(!later.header.is_new);
        assert_eq!(later.source_stream, "Later");
        assert_eq!(
            reader.objects()[&17].source_stream,
            "Contents/Config-0-ModelHeader"
        );
        reader
            .read_string_handle(&mut BinaryReader::new(&[19, 0]), "Later")
            .unwrap();
        assert_eq!(serde_json::to_value(reader.objects()).unwrap(), saved);
        let mut backref = BinaryReader::new(&[2, 0]);
        assert!(
            reader
                .read_model_header_prefix(
                    &mut backref,
                    "Later",
                    NativeModelHeaderProfile::DirectArrays
                )
                .is_err()
        );
        assert_eq!(backref.position(), 0);
    }
}

#[test]
fn model_header_truncation_and_invalid_schema_count_or_text_are_transactional() {
    let profile = NativeExternalObjectProfile::TwoStrings;
    let bytes = fixture(profile);
    let mut reader = NativeSubtreeReader::new(ArchiveContext::new(), profile);
    for end in 0..bytes.len() {
        let mut cursor = BinaryReader::new(&bytes[..end]);
        assert!(
            reader
                .read_model_header_prefix(
                    &mut cursor,
                    "Header",
                    NativeModelHeaderProfile::DirectArrays
                )
                .is_err(),
            "end={end}"
        );
        assert_eq!(cursor.position(), 0);
        assert!(reader.archive().entries().is_empty());
        assert!(reader.objects().is_empty());
    }
    let class_name = bytes
        .windows(15)
        .position(|s| s == b"su_CStringArray")
        .unwrap();
    let mut wrong_schema = bytes.clone();
    wrong_schema[class_name - 4] = 1;
    let mut extended_count = bytes.clone();
    extended_count[class_name + 15..class_name + 17].copy_from_slice(&u16::MAX.to_le_bytes());
    let mut excessive_count = bytes.clone();
    excessive_count[class_name + 15..class_name + 17].copy_from_slice(&65534_u16.to_le_bytes());
    let mut invalid_text = bytes.clone();
    let frame = bytes
        .windows(4)
        .position(|s| s == [255, 254, 255, 254])
        .unwrap();
    invalid_text[frame + 6..frame + 8].copy_from_slice(&0xdc00_u16.to_le_bytes());
    for bad in [wrong_schema, extended_count, excessive_count, invalid_text] {
        let mut cursor = BinaryReader::new(&bad);
        assert!(
            reader
                .read_model_header_prefix(
                    &mut cursor,
                    "Header",
                    NativeModelHeaderProfile::DirectArrays
                )
                .is_err()
        );
        assert_eq!(cursor.position(), 0);
        assert!(reader.archive().entries().is_empty());
        assert!(reader.objects().is_empty());
    }
    reader
        .read_model_header_prefix(
            &mut BinaryReader::new(&bytes),
            "Header",
            NativeModelHeaderProfile::DirectArrays,
        )
        .unwrap();
    let saved = serde_json::to_value(reader.objects()).unwrap();
    // Preserve previous typed definitions when a later root fails.
    let mut wrong = bytes.clone();
    wrong[2] = 2;
    assert!(
        reader
            .read_model_header_prefix(
                &mut BinaryReader::new(&wrong),
                "Later",
                NativeModelHeaderProfile::DirectArrays
            )
            .is_err()
    );
    assert_eq!(serde_json::to_value(reader.objects()).unwrap(), saved);
    assert_eq!(reader.archive().entries().len(), 20);
}

#[test]
fn model_header_collections_obey_pointer_and_depth_budgets() {
    let profile = NativeExternalObjectProfile::TwoStrings;
    let bytes = fixture(profile);
    for (pointers, depth) in [(1, 64), (13, 64), (100, 2)] {
        let mut reader =
            NativeSubtreeReader::with_limits(ArchiveContext::new(), profile, pointers, depth);
        let mut cursor = BinaryReader::new(&bytes);
        assert!(
            reader
                .read_model_header_prefix(
                    &mut cursor,
                    "Header",
                    NativeModelHeaderProfile::DirectArrays
                )
                .is_err()
        );
        assert_eq!(cursor.position(), 0);
        assert!(reader.archive().entries().is_empty());
        assert!(reader.objects().is_empty());
    }
    let mut reader = NativeSubtreeReader::with_limits(ArchiveContext::new(), profile, 14, 3);
    reader
        .read_model_header_prefix(
            &mut BinaryReader::new(&bytes),
            "Header",
            NativeModelHeaderProfile::DirectArrays,
        )
        .unwrap();
    assert!(
        reader
            .read_string_handle(&mut BinaryReader::new(&[19, 0]), "Later")
            .is_err()
    );
}

#[test]
fn model_header_preserves_nulls_and_complete_string_array_backreferences() {
    let mut bytes = Vec::new();
    class(&mut bytes, "moHeader_c");
    strings_class(&mut bytes);
    tag(&mut bytes, 0);
    tag(&mut bytes, 4); // complete first array reused as second
    tag(&mut bytes, 0); // no log list
    words(&mut bytes, &[0, 0]);
    tag(&mut bytes, 0); // no external-object list
    let mut reader = NativeSubtreeReader::new(
        ArchiveContext::new(),
        NativeExternalObjectProfile::TwoStrings,
    );
    let mut cursor = BinaryReader::new(&bytes);
    reader
        .read_model_header_prefix(
            &mut cursor,
            "Header",
            NativeModelHeaderProfile::DirectArrays,
        )
        .unwrap();
    assert_eq!(cursor.position(), bytes.len());
    let NativeSubtreePayload::ModelHeaderPrefix { header } = &reader.objects()[&2].payload else {
        panic!()
    };
    let NativeModelHeaderFields {
        string_arrays,
        logs,
        external_objects,
        ..
    } = header.as_ref();
    assert_eq!(string_arrays[0].header.object_index, Some(4));
    assert_eq!(string_arrays[1].header.object_index, Some(4));
    assert!(!string_arrays[1].header.is_new);
    assert_eq!(logs.header.object_index, None);
    assert_eq!(external_objects.header.object_index, None);
}

#[test]
fn model_header_leading_word_is_explicit_and_does_not_change_map_slots() {
    let external = NativeExternalObjectProfile::ThreeStringsShortTail;
    let direct = fixture(external);
    let mut bytes = direct.clone();
    bytes.splice(16..16, 0xfedcba98_u32.to_le_bytes());
    let mut reader = NativeSubtreeReader::new(ArchiveContext::new(), external);
    let mut cursor = BinaryReader::new(&bytes);
    reader
        .read_model_header_prefix(&mut cursor, "Header", NativeModelHeaderProfile::LeadingWord)
        .unwrap();
    assert_eq!(cursor.position(), bytes.len());
    assert_eq!(reader.archive().entries().len(), 20);
    let NativeSubtreePayload::ModelHeaderPrefix { header } = &reader.objects()[&2].payload else {
        panic!()
    };
    let NativeModelHeaderFields {
        profile,
        raw_header_word,
        ..
    } = header.as_ref();
    assert!(matches!(profile, NativeModelHeaderProfile::LeadingWord));
    assert_eq!(*raw_header_word, Some(0xfedcba98));
    for (data, wrong) in [
        (bytes, NativeModelHeaderProfile::DirectArrays),
        (direct, NativeModelHeaderProfile::LeadingWord),
    ] {
        let mut reader = NativeSubtreeReader::new(ArchiveContext::new(), external);
        let mut cursor = BinaryReader::new(&data);
        assert!(
            reader
                .read_model_header_prefix(&mut cursor, "Header", wrong)
                .is_err()
        );
        assert_eq!(cursor.position(), 0);
        assert!(reader.archive().entries().is_empty());
        assert!(reader.objects().is_empty());
    }
}

fn complete_fixture(optional: bool, empty_list: bool) -> (Vec<u8>, usize) {
    let mut bytes = fixture(NativeExternalObjectProfile::TwoStrings);
    let marker = bytes
        .windows(7)
        .position(|s| s == [2, 0, 0, 0x78, 0x56, 0x34, 0x12])
        .unwrap();
    bytes[marker] = 3; // preserve the second observed leading scalar value
    words(&mut bytes, &[0x01020304]);
    tag(&mut bytes, 17); // following saved external object reuses a complete value
    bytes.extend(0..42_u8);
    if empty_list {
        tag(&mut bytes, 0x8006); // generic suObList class, new empty object 21
        tag(&mut bytes, 0);
    } else {
        tag(&mut bytes, 0);
    }
    let flag_at = bytes.len() + 8;
    words(&mut bytes, &[123, 456, u32::from(optional)]);
    if optional {
        bytes.extend([0xff; 80]);
    }
    words(&mut bytes, &[u32::MAX; 3]);
    (bytes, flag_at)
}

#[test]
fn complete_model_header_checks_optional_blocks_and_supports_root_backreferences() {
    for optional in [false, true] {
        for empty_list in [false, true] {
            let (mut bytes, _) = complete_fixture(optional, empty_list);
            let end = bytes.len();
            tag(&mut bytes, 2);
            let mut reader = NativeSubtreeReader::new(
                ArchiveContext::new(),
                NativeExternalObjectProfile::TwoStrings,
            );
            let mut cursor = BinaryReader::new(&bytes);
            reader
                .read_model_header(
                    &mut cursor,
                    "Header",
                    NativeModelHeaderProfile::DirectArrays,
                )
                .unwrap();
            assert_eq!(cursor.position(), end);
            assert!(reader.objects()[&2].complete_payload);
            assert_eq!(
                reader.objects()[&2].span,
                ByteSpan {
                    offset: 0,
                    length: end
                }
            );
            let NativeSubtreePayload::ModelHeader { header, tail } = &reader.objects()[&2].payload
            else {
                panic!()
            };
            assert_eq!(header.profile, NativeModelHeaderProfile::DirectArrays);
            assert_eq!(
                tail.span.offset,
                fixture(NativeExternalObjectProfile::TwoStrings).len()
            );
            assert_eq!(tail.span.offset + tail.span.length, end);
            assert_eq!(tail.raw_before_reference, 0x01020304);
            assert_eq!(tail.following_external_object.header.object_index, Some(17));
            assert!(!tail.following_external_object.header.is_new);
            assert_eq!(tail.raw_scalar_block, (0..42_u8).collect::<Vec<_>>());
            assert_eq!(tail.raw_optional_prefix, [123, 456, u32::from(optional)]);
            assert_eq!(tail.raw_optional_block, optional.then(|| vec![0xff; 80]));
            assert_eq!(tail.raw_final_words, [u32::MAX; 3]);
            let NativeSubtreePayload::ExternalObject {
                raw_header_byte, ..
            } = &reader.objects()[&17].payload
            else {
                panic!()
            };
            assert_eq!(*raw_header_byte, 3);
            if empty_list {
                assert!(matches!(
                    reader.objects()[&21].payload,
                    NativeSubtreePayload::EmptyObjectList
                ));
            }
            let saved = serde_json::to_value(reader.objects()).unwrap();
            let root = reader
                .read_model_header(&mut cursor, "Later", NativeModelHeaderProfile::DirectArrays)
                .unwrap();
            assert_eq!(root.header.object_index, Some(2));
            assert!(!root.header.is_new);
            assert_eq!(cursor.position(), bytes.len());
            // A prefix read may reference a complete header, without downgrading it.
            reader
                .read_model_header_prefix(
                    &mut BinaryReader::new(&[2, 0]),
                    "Later",
                    NativeModelHeaderProfile::DirectArrays,
                )
                .unwrap();
            let mut wrong = BinaryReader::new(&[2, 0]);
            assert!(
                reader
                    .read_model_header_prefix(
                        &mut wrong,
                        "Later",
                        NativeModelHeaderProfile::LeadingWord
                    )
                    .is_err()
            );
            assert_eq!(wrong.position(), 0);
            assert_eq!(serde_json::to_value(reader.objects()).unwrap(), saved);
        }
    }
}

#[test]
fn complete_model_header_failures_roll_back_and_never_upgrade_an_unread_prefix() {
    let profile = NativeExternalObjectProfile::TwoStrings;
    let (bytes, flag_at) = complete_fixture(true, true);
    let mut reader = NativeSubtreeReader::new(ArchiveContext::new(), profile);
    for end in 0..bytes.len() {
        let mut cursor = BinaryReader::new(&bytes[..end]);
        assert!(
            reader
                .read_model_header(
                    &mut cursor,
                    "Header",
                    NativeModelHeaderProfile::DirectArrays
                )
                .is_err(),
            "end={end}"
        );
        assert_eq!(cursor.position(), 0);
        assert!(reader.archive().entries().is_empty());
        assert!(reader.objects().is_empty());
    }
    let mut bad_flag = bytes.clone();
    bad_flag[flag_at..flag_at + 4].copy_from_slice(&2_u32.to_le_bytes());
    let list_at = fixture(profile).len() + 4 + 2 + 42;
    let mut nonempty = bytes.clone();
    nonempty[list_at + 2..list_at + 4].copy_from_slice(&1_u16.to_le_bytes());
    let mut wrong_backref = bytes.clone();
    wrong_backref.splice(list_at..list_at + 4, 7_u16.to_le_bytes()); // nonempty earlier log list
    for bad in [&bad_flag, &nonempty, &wrong_backref] {
        let mut cursor = BinaryReader::new(bad);
        assert!(
            reader
                .read_model_header(
                    &mut cursor,
                    "Header",
                    NativeModelHeaderProfile::DirectArrays
                )
                .is_err()
        );
        assert_eq!(cursor.position(), 0);
        assert!(reader.archive().entries().is_empty());
        assert!(reader.objects().is_empty());
    }
    reader
        .read_model_header(
            &mut BinaryReader::new(&bytes),
            "Header",
            NativeModelHeaderProfile::DirectArrays,
        )
        .unwrap();
    let saved = serde_json::to_value(reader.objects()).unwrap();
    assert!(
        reader
            .read_model_header(
                &mut BinaryReader::new(&bad_flag),
                "Later",
                NativeModelHeaderProfile::DirectArrays
            )
            .is_err()
    );
    assert_eq!(serde_json::to_value(reader.objects()).unwrap(), saved);
    assert_eq!(reader.archive().entries().len(), 21);
    let mut prefix = NativeSubtreeReader::new(ArchiveContext::new(), profile);
    prefix
        .read_model_header_prefix(
            &mut BinaryReader::new(&bytes),
            "Header",
            NativeModelHeaderProfile::DirectArrays,
        )
        .unwrap();
    let mut pointer = BinaryReader::new(&[2, 0]);
    assert!(
        prefix
            .read_model_header(
                &mut pointer,
                "Later",
                NativeModelHeaderProfile::DirectArrays
            )
            .is_err()
    );
    assert_eq!(pointer.position(), 0);
    assert!(!prefix.objects()[&2].complete_payload);
}

#[test]
fn complete_model_headers_require_the_established_profiles_and_pointer_budget() {
    let (bytes, _) = complete_fixture(true, true);
    for (external, header) in [
        (
            NativeExternalObjectProfile::TwoStrings,
            NativeModelHeaderProfile::LeadingWord,
        ),
        (
            NativeExternalObjectProfile::ThreeStrings,
            NativeModelHeaderProfile::DirectArrays,
        ),
        (
            NativeExternalObjectProfile::ThreeStringsShortTail,
            NativeModelHeaderProfile::DirectArrays,
        ),
    ] {
        let mut reader = NativeSubtreeReader::new(ArchiveContext::new(), external);
        let mut cursor = BinaryReader::new(&bytes);
        assert!(
            reader
                .read_model_header(&mut cursor, "Header", header)
                .is_err()
        );
        assert_eq!(cursor.position(), 0);
        assert!(reader.archive().entries().is_empty());
    }
    let mut limited = NativeSubtreeReader::with_limits(
        ArchiveContext::new(),
        NativeExternalObjectProfile::TwoStrings,
        15,
        64,
    );
    assert!(
        limited
            .read_model_header(
                &mut BinaryReader::new(&bytes),
                "Header",
                NativeModelHeaderProfile::DirectArrays
            )
            .is_err()
    );
    assert!(limited.archive().entries().is_empty());
    let mut exact = NativeSubtreeReader::with_limits(
        ArchiveContext::new(),
        NativeExternalObjectProfile::TwoStrings,
        16,
        3,
    );
    exact
        .read_model_header(
            &mut BinaryReader::new(&bytes),
            "Header",
            NativeModelHeaderProfile::DirectArrays,
        )
        .unwrap();
}

fn leading_fixture(
    profile: NativeModelHeaderTailProfile,
) -> (Vec<u8>, usize, NativeExternalObjectProfile) {
    let external = if profile == NativeModelHeaderTailProfile::Extension68 {
        NativeExternalObjectProfile::ThreeStrings
    } else {
        NativeExternalObjectProfile::ThreeStringsShortTail
    };
    let mut bytes = fixture(external);
    bytes.splice(16..16, 1_u32.to_le_bytes());
    words(&mut bytes, &[101]);
    tag(&mut bytes, 17);
    bytes.extend([0; 42]);
    tag(&mut bytes, 0);
    words(&mut bytes, &[0, 0, 1]);
    bytes.extend([0; 80]);
    words(&mut bytes, &[u32::MAX; 3]);
    let extension_at = bytes.len();
    let (count, first) = match profile {
        NativeModelHeaderTailProfile::Extension32 => (8, 1),
        NativeModelHeaderTailProfile::Extension64 => (16, 2),
        NativeModelHeaderTailProfile::Extension68 => (17, 2),
        _ => panic!(),
    };
    let mut values = vec![0; count];
    values[0] = first;
    values[count - 1] = 1;
    words(&mut bytes, &values);
    (bytes, extension_at, external)
}
#[test]
fn complete_leading_headers_preserve_extensions_and_exact_following_object_boundaries() {
    for profile in [
        NativeModelHeaderTailProfile::Extension32,
        NativeModelHeaderTailProfile::Extension64,
        NativeModelHeaderTailProfile::Extension68,
    ] {
        let (mut bytes, extension_at, external) = leading_fixture(profile);
        let end = bytes.len();
        tag(&mut bytes, 2);
        let mut reader = NativeSubtreeReader::new(ArchiveContext::new(), external);
        let mut cursor = BinaryReader::new(&bytes);
        reader
            .read_model_header_with_tail(
                &mut cursor,
                "Header",
                NativeModelHeaderProfile::LeadingWord,
                profile,
            )
            .unwrap();
        assert_eq!(cursor.position(), end);
        let object = &reader.objects()[&2];
        assert!(object.complete_payload);
        let NativeSubtreePayload::ModelHeader { header, tail } = &object.payload else {
            panic!()
        };
        assert_eq!(header.raw_header_word, Some(1));
        assert_eq!(tail.profile, profile);
        assert_eq!(tail.raw_extension_words.len() * 4, end - extension_at);
        assert_eq!(tail.raw_extension_words.last(), Some(&1));
        assert_eq!(tail.span.offset + tail.span.length, end);
        let saved = serde_json::to_value(reader.objects()).unwrap();
        let reused = reader
            .read_model_header_with_tail(
                &mut cursor,
                "Later",
                NativeModelHeaderProfile::LeadingWord,
                profile,
            )
            .unwrap();
        assert!(!reused.header.is_new);
        assert_eq!(cursor.position(), bytes.len());
        let other = if profile == NativeModelHeaderTailProfile::Extension32 {
            NativeModelHeaderTailProfile::Extension64
        } else {
            NativeModelHeaderTailProfile::Extension32
        };
        let mut wrong = BinaryReader::new(&[2, 0]);
        assert!(
            reader
                .read_model_header_with_tail(
                    &mut wrong,
                    "Later",
                    NativeModelHeaderProfile::LeadingWord,
                    other
                )
                .is_err()
        );
        assert_eq!(wrong.position(), 0);
        assert_eq!(serde_json::to_value(reader.objects()).unwrap(), saved);
    }
}
#[test]
fn leading_header_extensions_reject_truncation_unknown_values_and_unknown_initial_words() {
    for profile in [
        NativeModelHeaderTailProfile::Extension32,
        NativeModelHeaderTailProfile::Extension64,
        NativeModelHeaderTailProfile::Extension68,
    ] {
        let (bytes, extension_at, external) = leading_fixture(profile);
        let mut reader = NativeSubtreeReader::new(ArchiveContext::new(), external);
        for end in 0..bytes.len() {
            let mut cursor = BinaryReader::new(&bytes[..end]);
            assert!(
                reader
                    .read_model_header_with_tail(
                        &mut cursor,
                        "Header",
                        NativeModelHeaderProfile::LeadingWord,
                        profile
                    )
                    .is_err(),
                "end={end}"
            );
            assert_eq!(cursor.position(), 0);
            assert!(reader.archive().entries().is_empty());
            assert!(reader.objects().is_empty());
        }
        for offset in [16, extension_at, extension_at + 4, bytes.len() - 4] {
            let mut bad = bytes.clone();
            bad[offset..offset + 4].copy_from_slice(&3_u32.to_le_bytes());
            let mut cursor = BinaryReader::new(&bad);
            assert!(
                reader
                    .read_model_header_with_tail(
                        &mut cursor,
                        "Header",
                        NativeModelHeaderProfile::LeadingWord,
                        profile
                    )
                    .is_err()
            );
            assert_eq!(cursor.position(), 0);
            assert!(reader.archive().entries().is_empty());
            assert!(reader.objects().is_empty());
        }
        reader
            .read_model_header_with_tail(
                &mut BinaryReader::new(&bytes),
                "Header",
                NativeModelHeaderProfile::LeadingWord,
                profile,
            )
            .unwrap();
        let saved = serde_json::to_value(reader.objects()).unwrap();
        let mut bad = bytes.clone();
        *bad.last_mut().unwrap() = 2;
        assert!(
            reader
                .read_model_header_with_tail(
                    &mut BinaryReader::new(&bad),
                    "Later",
                    NativeModelHeaderProfile::LeadingWord,
                    profile
                )
                .is_err()
        );
        assert_eq!(serde_json::to_value(reader.objects()).unwrap(), saved);
    }
}

#[test]
fn direct_headers_preserve_new_external_prefix_values_and_three_string_block_framing() {
    for (external, tail_profile, count, byte, prefix_word) in [
        (
            NativeExternalObjectProfile::ThreeStringsShortBlock,
            NativeModelHeaderTailProfile::Extension28,
            7,
            2_u8,
            0x0100_u16,
        ),
        (
            NativeExternalObjectProfile::ThreeStringsShortTail,
            NativeModelHeaderTailProfile::Extension32,
            8,
            5_u8,
            0_u16,
        ),
    ] {
        let mut bytes = fixture(external);
        let scalar = bytes
            .windows(7)
            .position(|s| s == [2, 0, 0, 0x78, 0x56, 0x34, 0x12])
            .unwrap();
        bytes[scalar] = byte;
        bytes[scalar + 1..scalar + 3].copy_from_slice(&prefix_word.to_le_bytes());
        words(&mut bytes, &[101]);
        tag(&mut bytes, 17);
        bytes.extend([0; 42]);
        tag(&mut bytes, 0);
        words(&mut bytes, &[0; 3]);
        words(&mut bytes, &[u32::MAX; 3]);
        let extension_at = bytes.len();
        let mut extension = vec![0; count];
        extension[0] = 1;
        words(&mut bytes, &extension);
        let mut reader = NativeSubtreeReader::new(ArchiveContext::new(), external);
        let mut cursor = BinaryReader::new(&bytes);
        reader
            .read_model_header_with_tail(
                &mut cursor,
                "Header",
                NativeModelHeaderProfile::DirectArrays,
                tail_profile,
            )
            .unwrap();
        assert_eq!(cursor.position(), bytes.len());
        let NativeSubtreePayload::ModelHeader { tail, .. } = &reader.objects()[&2].payload else {
            panic!()
        };
        assert_eq!(tail.raw_extension_words, extension);
        let NativeSubtreePayload::ExternalObject {
            raw_header_byte,
            raw_prefix_word,
            raw_before_configuration,
            saved_strings,
            ..
        } = &reader.objects()[&17].payload
        else {
            panic!()
        };
        assert_eq!((*raw_header_byte, *raw_prefix_word), (byte, prefix_word));
        assert_eq!(saved_strings.len(), 3);
        assert_eq!(
            raw_before_configuration.len(),
            if external == NativeExternalObjectProfile::ThreeStringsShortBlock {
                17
            } else {
                18
            }
        );
        for end in 0..bytes.len() {
            let mut reader = NativeSubtreeReader::new(ArchiveContext::new(), external);
            let mut cursor = BinaryReader::new(&bytes[..end]);
            assert!(
                reader
                    .read_model_header_with_tail(
                        &mut cursor,
                        "Header",
                        NativeModelHeaderProfile::DirectArrays,
                        tail_profile
                    )
                    .is_err(),
                "end={end}"
            );
            assert_eq!(cursor.position(), 0);
            assert!(reader.archive().entries().is_empty());
            assert!(reader.objects().is_empty());
        }
        for offset in [scalar + 1, extension_at, extension_at + 4] {
            let mut bad = bytes.clone();
            if offset == scalar + 1 {
                bad[offset..offset + 2].copy_from_slice(&0x0200_u16.to_le_bytes());
            } else {
                bad[offset..offset + 4].copy_from_slice(&3_u32.to_le_bytes());
            }
            let mut reader = NativeSubtreeReader::new(ArchiveContext::new(), external);
            assert!(
                reader
                    .read_model_header_with_tail(
                        &mut BinaryReader::new(&bad),
                        "Header",
                        NativeModelHeaderProfile::DirectArrays,
                        tail_profile
                    )
                    .is_err()
            );
            assert!(reader.archive().entries().is_empty());
            assert!(reader.objects().is_empty());
        }
    }
}
