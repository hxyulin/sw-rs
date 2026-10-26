// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
//! Authored whole streams exercise strict profile selection and scope isolation.
use super::*;
fn header(layout: NativeModelHeaderLayout) -> Vec<u8> {
    let mut bytes = vec![255, 255, 1, 0, 10, 0];
    bytes.extend(b"moHeader_c");
    if layout.header_profile == NativeModelHeaderProfile::LeadingWord {
        bytes.extend(1_u32.to_le_bytes());
    }
    bytes.extend([0; 16]); // null string arrays/log list, two words, null external list
    bytes.extend([0; 74]); // following word/reference, scalar block, null list, no optional block
    let (count, first) = match layout.tail_profile {
        NativeModelHeaderTailProfile::NoExtension => (0, 0),
        NativeModelHeaderTailProfile::Extension28 => (7, 1),
        NativeModelHeaderTailProfile::Extension32 => (8, 1),
        NativeModelHeaderTailProfile::Extension64 => (16, 2),
        NativeModelHeaderTailProfile::Extension68 => (17, 2),
    };
    for i in 0..count {
        bytes.extend(if i == 0 { first } else { 0_u32 }.to_le_bytes());
    }
    bytes
}
#[test]
fn selects_only_the_unique_layout_that_consumes_a_complete_header_and_eof() {
    for (index, &layout) in LAYOUTS.iter().enumerate() {
        assert!(!LAYOUTS[..index].contains(&layout));
        let bytes = header(layout);
        let result = inspect_stream(&bytes, "Contents/Config-7-ModelHeader", Some(7), &LAYOUTS);
        assert_eq!(result.status, NativeModelHeaderStatus::Decoded);
        let decoding = result.decoding.unwrap();
        assert_eq!(decoding.layout, layout);
        assert_eq!(decoding.root.header.object_index(), Some(2));
        assert_eq!(decoding.archive.len(), 2);
        assert_eq!(decoding.objects.len(), 1);
        assert!(decoding.objects[&2].complete_payload);
        assert_eq!(decoding.objects[&2].span.length, bytes.len());
        assert_eq!(
            result
                .attempts
                .iter()
                .filter(|a| matches!(a.outcome, NativeModelHeaderAttemptOutcome::Complete))
                .count(),
            1
        );
    }
}
#[test]
fn trailing_bytes_truncation_and_null_roots_never_select_a_partial_or_empty_graph() {
    let valid = header(LAYOUTS[0]);
    let mut trailing = valid.clone();
    trailing.extend([4, 5, 6]);
    let result = inspect_stream(&trailing, "Header", None, &LAYOUTS);
    assert_eq!(result.status, NativeModelHeaderStatus::Unsupported);
    assert!(result.decoding.is_none());
    assert!(
        matches!(result.attempts[0].outcome,NativeModelHeaderAttemptOutcome::TrailingBytes{offset,length:3} if offset==valid.len())
    );
    for bytes in [&valid[..valid.len() - 1], &[0, 0][..], &[][..]] {
        let result = inspect_stream(bytes, "Header", None, &LAYOUTS);
        assert_eq!(result.status, NativeModelHeaderStatus::Unsupported);
        assert!(result.decoding.is_none());
        assert!(
            result
                .attempts
                .iter()
                .all(|a| matches!(a.outcome, NativeModelHeaderAttemptOutcome::Rejected { .. }))
        );
    }
}
#[test]
fn multiple_complete_attempts_expose_no_chosen_graph() {
    // An injected competing catalog exercises the fail-closed selection rule.
    // Production catalog uniqueness is checked by the per-layout regression above.
    let result = inspect_stream(
        &header(LAYOUTS[0]),
        "Header",
        None,
        &[LAYOUTS[0], LAYOUTS[0]],
    );
    assert_eq!(result.status, NativeModelHeaderStatus::Ambiguous);
    assert!(result.decoding.is_none());
    assert!(
        result
            .attempts
            .iter()
            .all(|a| matches!(a.outcome, NativeModelHeaderAttemptOutcome::Complete))
    );
}
#[test]
fn literal_stream_names_remain_separate_and_unsupported_streams_are_retained() {
    let mut container = Container {
        streams: BTreeMap::new(),
        diagnostics: vec![],
    };
    for name in [
        "Contents/Config-0-ModelHeader",
        "Contents/Config-00-ModelHeader",
        "Contents/Config-99999999999999999999-ModelHeader",
    ] {
        container.streams.insert(name.into(), header(LAYOUTS[0]));
    }
    container.streams.insert(
        "Contents/Config-1-ModelHeader".into(),
        b"unsupported".to_vec(),
    );
    for name in [
        "Contents/Config-x-ModelHeader",
        "Contents/Config--ModelHeader",
        "Contents/Config-0-ModelHeader/child",
        "Contents/Config-0-MatesList",
    ] {
        container.streams.insert(name.into(), header(LAYOUTS[0]));
    }
    let report = NativeModelHeaderReport::inspect(&container, "source-hash");
    assert_eq!(report.source_sha256, "source-hash");
    assert_eq!(report.streams.len(), 4);
    assert!(!report.native_object_graph_decoded && !report.topology_ownership_decoded);
    for stream in &report.streams {
        if let Some(decoding) = &stream.decoding {
            assert_eq!(decoding.root.source_stream, stream.source_stream);
            assert_eq!(decoding.objects[&2].source_stream, stream.source_stream);
        }
    }
    assert_eq!(report.streams[0].configuration_index, Some(0));
    assert_eq!(report.streams[1].configuration_index, Some(0));
    assert_ne!(
        report.streams[0].source_stream,
        report.streams[1].source_stream
    );
    assert_eq!(
        report.streams[2].status,
        NativeModelHeaderStatus::Unsupported
    );
    assert_eq!(report.streams[3].configuration_index, None);
}
