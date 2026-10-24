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
