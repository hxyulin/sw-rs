// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
//! Authored native-prefix framing; no proprietary document contents.
use super::*;

fn class(out: &mut Vec<u8>, name: &str) {
    out.extend([255, 255, 1, 0]);
    out.extend((name.len() as u16).to_le_bytes());
    out.extend(name.as_bytes());
}
fn prefix(path: &str) -> Vec<u8> {
    let mut out = Vec::new();
    class(&mut out, "moFaceRef_c");
    for word in [1u32, 0, 490, 0x0304, 0x12345678] {
        out.extend(word.to_le_bytes());
    }
    out.extend(COMPONENT_MARKER);
    out.extend([0; 2]);
    class(&mut out, "moFromSktEntSurfIdRep_c");
    out.extend([0; 2]);
    for name in ["moFR_c", "moExtObject_c", "moCStringHandle_c"] {
        class(&mut out, name);
    }
    let units = path.encode_utf16().collect::<Vec<_>>();
    out.extend([255, 254, 255, 254]);
    out.extend((units.len() as u16).to_le_bytes());
    out.extend(units.into_iter().flat_map(u16::to_le_bytes));
    out
}

#[test]
fn exact_prefix_keeps_scope_raw_words_unicode_and_checked_source_spans() {
    let path = format!("C:\\原始\\{}\\Part.SLDPRT", "😀".repeat(160));
    let encoded = prefix(&path);
    let mut bytes = vec![0x51; 13];
    bytes.extend(&encoded);
    bytes.extend(b"opaque remainder, not part of the decoded prefix");
    let reference =
        NativeDocumentReference::parse_prefix(&bytes, "Contents/Config-17-MatesList", 13).unwrap();
    assert_eq!(reference.configuration_index, Some(17));
    assert_eq!(reference.source_stream, "Contents/Config-17-MatesList");
    assert_eq!(reference.raw_header, [1, 0, 490, 0x0304, 0x12345678]);
    assert_eq!(reference.prefix_span.slice(&bytes).unwrap(), encoded);
    assert_eq!(reference.document.value, path);
    let mut string = BinaryReader::at(&bytes, reference.document.offset).unwrap();
    assert_eq!(string.string().unwrap().value, path);
    assert_eq!(
        string.position(),
        reference.prefix_span.offset + reference.prefix_span.length
    );
    assert_eq!(reference.entity_class, "moFromSktEntSurfIdRep_c");
    let blank = prefix("");
    let blank =
        NativeDocumentReference::parse_prefix(&blank, "ThirdPtyStore/VisualStates", 0).unwrap();
    assert_eq!(blank.configuration_index, None);
    assert!(blank.document.value.is_empty());
}

#[test]
fn truncation_corruption_and_unknown_class_reuse_never_resume_a_prefix() {
    let bytes = prefix("../source/Part.SLDPRT");
    for end in 0..bytes.len() {
        assert!(
            NativeDocumentReference::parse_prefix(&bytes[..end], "ThirdPtyStore/VisualStates", 0)
                .is_none()
        );
    }
    assert!(NativeDocumentReference::parse_prefix(&bytes, "s", usize::MAX).is_none());
    let declaration_len = 6 + "moFaceRef_c".len();
    for at in [
        0,
        2,
        declaration_len,
        declaration_len + 4,
        declaration_len + 20,
        declaration_len + 36,
    ] {
        let mut bad = bytes.clone();
        bad[at] ^= 1;
        assert!(NativeDocumentReference::parse_prefix(&bad, "s", 0).is_none());
    }
    for name in [
        "moFromSktEntSurfIdRep_c",
        "moFR_c",
        "moExtObject_c",
        "moCStringHandle_c",
    ] {
        let at = bytes
            .windows(name.len())
            .position(|w| w == name.as_bytes())
            .unwrap()
            - 6;
        let mut reused = bytes[..at].to_vec();
        reused.extend(0x8017u16.to_le_bytes());
        reused.extend(&bytes[at + 6 + name.len()..]);
        assert!(NativeDocumentReference::parse_prefix(&reused, "s", 0).is_none());
    }
    // Unknown declaration followed by a familiar string is not admitted by proximity.
    let at = bytes
        .windows(9)
        .position(|w| w == b"moFR_c\xff\xff\x01")
        .unwrap();
    let mut wrong = bytes.clone();
    wrong[at] = b'x';
    assert!(NativeDocumentReference::parse_prefix(&wrong, "s", 0).is_none());
}

#[test]
fn invalid_unicode_and_forged_counted_string_fail_without_interpreting_paths() {
    let bytes = prefix("../not-opened/Part.SLDPRT");
    let reference =
        NativeDocumentReference::parse_prefix(&bytes, "ThirdPtyStore/VisualStates", 0).unwrap();
    assert_eq!(reference.document.value, "../not-opened/Part.SLDPRT");
    let at = reference.document.offset;
    let mut bad = bytes[..at].to_vec();
    bad.extend([255, 254, 255, 1, 0, 216]);
    assert!(NativeDocumentReference::parse_prefix(&bad, "s", 0).is_none());
    let mut bad = bytes[..at].to_vec();
    bad.extend([255, 254, 255, 255]);
    bad.extend(u32::MAX.to_le_bytes());
    assert!(NativeDocumentReference::parse_prefix(&bad, "s", 0).is_none());
}

#[test]
fn graph_ignores_prefix_lookalikes_inside_cstrings_but_keeps_adjacent_prefixes() {
    let mut inner = prefix("inner.SLDPRT");
    if !inner.len().is_multiple_of(2) {
        inner.push(0);
    }
    let units = inner
        .as_chunks::<2>()
        .0
        .iter()
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .collect::<Vec<_>>();
    let hidden = String::from_utf16(&units).unwrap();
    let mut bytes = prefix(&hidden);
    let second = bytes.len();
    bytes.extend(prefix("next.SLDPRT"));
    let container = crate::Container {
        streams: std::collections::BTreeMap::from([("ThirdPtyStore/VisualStates".into(), bytes)]),
        diagnostics: vec![],
    };
    let doc = crate::document::build_borrowed(&container, "part.SLDPRT");
    let mut graph =
        crate::ReferenceGraph::build("hash", &crate::FeatureCatalog::parse(None), &doc, &[]);
    graph.native_document_references(&container);
    let references = graph
        .nodes
        .iter()
        .filter_map(|node| match &node.record {
            crate::ReferenceRecord::NativeDocumentReference { reference } => Some(reference),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(references.len(), 2);
    assert_eq!(references[0].document.value, hidden);
    assert_eq!(references[1].document.value, "next.SLDPRT");
    assert_eq!(references[1].prefix_span.offset, second);
}

fn string(out: &mut Vec<u8>, value: &str) {
    let units = value.encode_utf16().collect::<Vec<_>>();
    out.extend([255, 254, 255, 254]);
    out.extend((units.len() as u16).to_le_bytes());
    out.extend(units.into_iter().flat_map(u16::to_le_bytes));
}

fn handle_frame(tag: u16) -> Vec<u8> {
    let mut bytes = Vec::new();
    class(&mut bytes, "moExtFeature_c");
    class(&mut bytes, "moCStringHandle_c");
    string(&mut bytes, "different/stale/assembly.SLDASM");
    bytes.extend(tag.to_le_bytes());
    string(&mut bytes, "saved configuration text");
    bytes
}

fn configuration(tag: u16) -> Vec<u8> {
    let mut bytes = Vec::new();
    class(&mut bytes, "moAssembly_c");
    bytes.extend([0x41; 19]); // Opaque assembly data is never used to assign map slots.
    bytes.extend(handle_frame(tag));
    bytes
}

fn reused_handle(path: &str, tag: u16) -> Vec<u8> {
    let bytes = prefix(path);
    let name = b"moCStringHandle_c";
    let at = bytes.windows(name.len()).position(|w| w == name).unwrap() - 6;
    let mut result = bytes[..at].to_vec();
    result.extend(tag.to_le_bytes());
    result.extend(&bytes[at + 6 + name.len()..]);
    result
}

fn graph(streams: Vec<(&str, Vec<u8>)>) -> crate::ReferenceGraph {
    let container = crate::Container {
        streams: streams.into_iter().map(|(n, b)| (n.into(), b)).collect(),
        diagnostics: vec![],
    };
    let doc = crate::document::build_borrowed(&container, "assembly.SLDASM");
    let mut graph =
        crate::ReferenceGraph::build("hash", &crate::FeatureCatalog::parse(None), &doc, &[]);
    graph.native_document_references(&container);
    graph
}

fn candidates(graph: &crate::ReferenceGraph) -> Vec<&NativeDocumentReferenceCandidate> {
    graph
        .nodes
        .iter()
        .filter_map(|node| match &node.record {
            crate::ReferenceRecord::NativeDocumentReferenceCandidate { reference } => {
                Some(reference.as_ref())
            }
            _ => None,
        })
        .collect()
}

#[test]
fn candidate_keeps_both_source_spans_and_the_unresolved_class_without_assigning_ownership() {
    let mate = reused_handle("../外部/part😀.SLDPRT", 0x8010);
    let config = configuration(0x8010);
    let report = graph(vec![
        ("Contents/Config-17-MatesList", mate.clone()),
        ("Contents/Config-17", config.clone()),
    ]);
    assert!(!report.native_object_graph_decoded && !report.topology_ownership_decoded);
    assert_eq!(report.native_reference_stops.len(), 1);
    let references = candidates(&report);
    assert_eq!(references.len(), 1);
    let reference = references[0];
    assert_eq!(reference.configuration_index, 17);
    assert_eq!(reference.document.value, "../外部/part😀.SLDPRT");
    assert_eq!(reference.prefix_span.slice(&mate), Some(mate.as_slice()));
    assert_eq!(reference.unresolved_class.stage, "moCStringHandle_c");
    assert_eq!(reference.unresolved_class.raw_tag, Some(0x8010));
    assert_eq!(
        reference.unresolved_class.offset + 2,
        reference.document.offset
    );
    assert_eq!(reference.class_evidence.source_stream, "Contents/Config-17");
    let evidence = &reference.class_evidence;
    assert_eq!(
        evidence.frame_span.slice(&config),
        Some(handle_frame(0x8010).as_slice())
    );
    assert_eq!(
        evidence.class_span.slice(&config).unwrap()[6..],
        *b"moCStringHandle_c"
    );
    assert_eq!(
        BinaryReader::at(&config, evidence.tag_offset)
            .unwrap()
            .u16(),
        Some(0x8010)
    );
    for span in evidence.string_spans {
        let mut cursor = BinaryReader::at(&config, span.offset).unwrap();
        assert!(cursor.string().is_some());
        assert_eq!(cursor.position(), span.offset + span.length);
    }
    let link = report
        .links
        .iter()
        .find(|l| l.relation == crate::ReferenceRelation::NativeDocumentHandleCandidate)
        .unwrap();
    assert_eq!(link.resolution, crate::ReferenceResolution::Unresolved);
    assert!(link.candidates.is_empty());
    assert_eq!(link.source_offset, Some(reference.document.offset));
    // Legacy callers still only receive fully explicit prefixes.
    assert!(
        NativeDocumentReference::parse_prefix(&mate, "Contents/Config-17-MatesList", 0).is_none()
    );
}

#[test]
fn candidate_requires_one_matching_frame_in_the_exact_configuration() {
    let mate = reused_handle("same.SLDPRT", 0x8010);
    let mut ambiguous = configuration(0x8010);
    ambiguous.extend(handle_frame(0x8010));
    let mut other_context = configuration(0x8010);
    other_context.extend(handle_frame(0x8011));
    let mut malformed_duplicate = configuration(0x8010);
    let mut truncated_frame = handle_frame(0x8010);
    truncated_frame.pop();
    malformed_duplicate.extend(truncated_frame);
    let mut wrong_schema = configuration(0x8010);
    let at = wrong_schema
        .windows(17)
        .position(|w| w == b"moCStringHandle_c")
        .unwrap();
    wrong_schema[at - 4] = 2;
    let mut wrong_root = configuration(0x8010);
    wrong_root[6] = b'x';
    let invalid = vec![
        vec![],
        vec![("Contents/Config-18", configuration(0x8010))],
        vec![("Contents/Config-17-ModelHeader", configuration(0x8010))],
        vec![("Contents/CMgr", configuration(0x8010))],
        vec![("Contents/Config-17", configuration(0x8011))],
        vec![("Contents/Config-17", ambiguous)],
        vec![("Contents/Config-17", other_context)],
        vec![("Contents/Config-17", malformed_duplicate)],
        vec![("Contents/Config-17", wrong_schema)],
        vec![("Contents/Config-17", wrong_root)],
    ];
    for mut streams in invalid {
        streams.push(("Contents/Config-17-MatesList", mate.clone()));
        let report = graph(streams);
        assert!(candidates(&report).is_empty());
        assert_eq!(report.native_reference_stops.len(), 1);
    }
    let report = graph(vec![
        ("ThirdPtyStore/VisualStates", mate),
        ("Contents/Config-17", configuration(0x8010)),
    ]);
    assert!(candidates(&report).is_empty());
}

#[test]
fn candidate_rejects_object_tags_invalid_strings_and_earlier_unknown_classes() {
    for tag in [0, 1, 0x7fff, 0x8000, 0xffff] {
        let report = graph(vec![
            ("Contents/Config-17-MatesList", reused_handle("part", tag)),
            ("Contents/Config-17", configuration(tag)),
        ]);
        assert!(candidates(&report).is_empty());
    }
    let bytes = reused_handle("part", 0x8010);
    for end in 0..bytes.len() {
        let report = graph(vec![
            ("Contents/Config-17-MatesList", bytes[..end].to_vec()),
            ("Contents/Config-17", configuration(0x8010)),
        ]);
        assert!(candidates(&report).is_empty());
    }
    let stop = NativeDocumentReference::inspect_prefix(&bytes, "s", 0).unwrap_err();
    let mut invalid = bytes[..stop.offset + 2].to_vec();
    invalid.extend([255, 254, 255, 1, 0, 216]); // Unpaired surrogate.
    let report = graph(vec![
        ("Contents/Config-17-MatesList", invalid),
        ("Contents/Config-17", configuration(0x8010)),
    ]);
    assert!(candidates(&report).is_empty());
    for name in ["moFromSktEntSurfIdRep_c", "moFR_c", "moExtObject_c"] {
        let mut invalid = bytes.clone();
        let at = invalid
            .windows(name.len())
            .position(|w| w == name.as_bytes())
            .unwrap();
        invalid[at] = b'x';
        let report = graph(vec![
            ("Contents/Config-17-MatesList", invalid),
            ("Contents/Config-17", configuration(0x8010)),
        ]);
        assert!(candidates(&report).is_empty());
        assert_eq!(report.native_reference_stops.len(), 1);
        assert_ne!(report.native_reference_stops[0].stage, "moCStringHandle_c");
    }
}

#[test]
fn candidate_blank_text_and_embedded_markers_never_create_owner_links_or_nested_prefixes() {
    let mut inner = prefix("hidden.SLDPRT");
    if !inner.len().is_multiple_of(2) {
        inner.push(0);
    }
    let units = inner
        .as_chunks::<2>()
        .0
        .iter()
        .map(|b| u16::from_le_bytes(*b))
        .collect::<Vec<_>>();
    let hidden = String::from_utf16(&units).unwrap();
    let mut bytes = reused_handle(&hidden, 0x8010);
    let next = bytes.len();
    bytes.extend(reused_handle("", 0x8010));
    let report = graph(vec![
        ("Contents/Config-17-MatesList", bytes),
        ("Contents/Config-17", configuration(0x8010)),
    ]);
    let references = candidates(&report);
    assert_eq!(references.len(), 2);
    assert_eq!(references[0].document.value, hidden);
    assert_eq!(references[1].prefix_span.offset, next);
    assert!(references[1].document.value.is_empty());
    assert_eq!(report.native_reference_stops.len(), 2);
    assert!(
        report
            .links
            .iter()
            .filter(|l| l.relation == crate::ReferenceRelation::NativeDocumentHandleCandidate)
            .all(|l| l.resolution == crate::ReferenceResolution::Unresolved
                && l.candidates.is_empty())
    );
}

#[test]
fn stopping_diagnostics_keep_exact_offsets_and_failed_field_types() {
    let bytes = prefix("part");
    let name = b"moFR_c";
    let at = bytes.windows(name.len()).position(|w| w == name).unwrap() - 6;
    let mut invalid = bytes[..at].to_vec();
    invalid.extend(0x8123u16.to_le_bytes());
    invalid.extend(&bytes[at + 6 + name.len()..]);
    let stop = NativeDocumentReference::inspect_prefix(&invalid, "stream", 0).unwrap_err();
    assert_eq!(stop.stage, "moFR_c");
    assert_eq!(stop.offset, at);
    assert_eq!(stop.raw_tag, Some(0x8123));
    assert_eq!(stop.prefix_offset, 0);
    assert_eq!(stop.source_stream, "stream");
    assert!(stop.explicit_class_candidate.is_none());
    let mut invalid = bytes.clone();
    invalid[6 + "moFaceRef_c".len()] = 2;
    let stop = NativeDocumentReference::inspect_prefix(&invalid, "s", 0).unwrap_err();
    assert_eq!(stop.stage, "reference_header");
    assert!(stop.raw_tag.is_none());
    let stop = NativeDocumentReference::inspect_prefix(&bytes, "s", usize::MAX).unwrap_err();
    assert_eq!(stop.offset, usize::MAX);
    assert!(stop.raw_tag.is_none());
}

#[test]
fn route_assembly_context_and_literal_configuration_names_keep_their_scope() {
    let mut route = Vec::new();
    class(&mut route, "moRouteAssembly_c");
    route.extend(handle_frame(0x8010));
    let report = graph(vec![
        (
            "Contents/Config-00-MatesList",
            reused_handle("saved", 0x8010),
        ),
        ("Contents/Config-00", route),
        ("Contents/Config-0", configuration(0x8011)),
    ]);
    let refs = candidates(&report);
    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0].class_evidence.root_class, "moRouteAssembly_c");
    assert_eq!(refs[0].class_evidence.source_stream, "Contents/Config-00");
    assert_eq!(refs[0].configuration_index, 0);
    let report = graph(vec![
        (
            "Contents/Config-00-MatesList",
            reused_handle("saved", 0x8010),
        ),
        ("Contents/Config-0", configuration(0x8010)),
    ]);
    assert!(candidates(&report).is_empty());
}

#[test]
fn failed_counted_strings_do_not_expose_nested_candidate_markers() {
    let mut inner = reused_handle("hidden", 0x8010);
    if !inner.len().is_multiple_of(2) {
        inner.push(0);
    }
    let mut units = inner
        .as_chunks::<2>()
        .0
        .iter()
        .map(|b| u16::from_le_bytes(*b))
        .collect::<Vec<_>>();
    units.push(0xd800); // Structurally counted, invalid UTF-16 at the end.
    let mut outer = reused_handle("", 0x8010);
    let at = NativeDocumentReference::inspect_prefix(&outer, "s", 0)
        .unwrap_err()
        .offset
        + 2;
    outer.truncate(at);
    outer.extend([255, 254, 255, 254]);
    outer.extend((units.len() as u16).to_le_bytes());
    outer.extend(units.into_iter().flat_map(u16::to_le_bytes));
    let report = graph(vec![
        ("Contents/Config-17-MatesList", outer),
        ("Contents/Config-17", configuration(0x8010)),
    ]);
    assert!(candidates(&report).is_empty());
    assert_eq!(report.native_reference_stops.len(), 1);

    let mut inner = handle_frame(0x8010);
    if !inner.len().is_multiple_of(2) {
        inner.push(0);
    }
    let hidden = String::from_utf16(
        &inner
            .as_chunks::<2>()
            .0
            .iter()
            .map(|b| u16::from_le_bytes(*b))
            .collect::<Vec<_>>(),
    )
    .unwrap();
    let mut context = Vec::new();
    class(&mut context, "moAssembly_c");
    class(&mut context, "moExtFeature_c");
    class(&mut context, "moCStringHandle_c");
    string(&mut context, &hidden);
    context.extend([0, 0]); // Outer frame fails after a complete counted string.
    assert!(NativeHandleClassEvidence::scan_unique(&context, "Contents/Config-17").is_none());
}
