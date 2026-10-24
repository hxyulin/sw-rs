// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
//! Authored mate streams; no proprietary CAD fixtures.
use super::*;
fn text(out: &mut Vec<u8>, value: &str) {
    let units: Vec<_> = value.encode_utf16().collect();
    out.extend([255, 254, 255]);
    if units.len() < 254 {
        out.push(units.len() as u8);
    } else {
        out.push(255);
        out.extend((units.len() as u32).to_le_bytes());
    }
    out.extend(units.into_iter().flat_map(u16::to_le_bytes));
}
fn class(out: &mut Vec<u8>, name: &str) {
    out.extend([255, 255, 1, 0]);
    out.extend((name.len() as u16).to_le_bytes());
    out.extend(name.as_bytes());
}
fn header(count: u16) -> Vec<u8> {
    [
        193_u32.to_le_bytes().as_slice(),
        count.to_le_bytes().as_slice(),
    ]
    .concat()
}
fn record(out: &mut Vec<u8>, kind: Option<&str>, name: &str, identifier: u32) -> usize {
    let start = out.len();
    if let Some(kind) = kind {
        class(out, kind);
    } else {
        out.extend(0x80c1_u16.to_le_bytes());
    }
    out.extend(0x800c_u16.to_le_bytes());
    text(out, name);
    out.extend([0, 1, 0, 0, 0, 0, 0, 0x40]);
    out.extend(identifier.to_le_bytes());
    out.extend([0; 4]);
    text(out, "note@not-an-endpoint");
    start
}
fn reference(out: &mut Vec<u8>, descriptor: &str, path: &str) {
    text(out, descriptor);
    out.extend(1_u32.to_le_bytes());
    text(out, path);
}

#[test]
fn multiline_selection_descriptors_keep_all_rows_without_choosing_an_endpoint() {
    let mut bytes = header(1);
    record(&mut bytes, Some("moMateCoincident"), "edited name", 27);
    let descriptor = "moSimpleSurfIdRep_c,7,2, \nmoFromSktEntSurfIdRep_c,8,4, \n";
    reference(&mut bytes, descriptor, "Part-2@Root");
    let data = parse_stream(&bytes, "Contents/Config-7-MatesList", 7);
    let selection = &data.records[0].reference_candidates[0];
    assert_eq!(
        selection.geometry_descriptor.as_ref().unwrap().value,
        descriptor
    );
    assert!(selection.surface_reference().is_none());
    let surfaces = selection.surface_references();
    assert_eq!(surfaces.len(), 2);
    assert_eq!(surfaces[0].feature_source_id, Some(7));
    assert_eq!(surfaces[1].feature_source_id, Some(8));
    let doc = crate::metadata_from_container(
        crate::Container {
            streams: BTreeMap::from([("Contents/Config-7-MatesList".into(), bytes)]),
            diagnostics: vec![],
        },
        "part.SLDASM",
    );
    let graph =
        crate::ReferenceGraph::build("hash", &crate::FeatureCatalog::parse(None), &doc, &[]);
    let link = graph
        .links
        .iter()
        .find(|l| l.relation == crate::ReferenceRelation::MateSelection)
        .unwrap();
    assert!(link.selection_surface_reference.is_none());
    assert_eq!(link.selection_surface_references.len(), 2);
    assert_eq!(
        link.selection_descriptor.as_ref().unwrap().value,
        descriptor
    );
    assert!(link.candidates.is_empty());
    assert_eq!(link.resolution, crate::ReferenceResolution::Unresolved);
    assert!(!graph.topology_ownership_decoded);
}
#[test]
fn names_types_and_reference_graph_keep_occurrences_and_hyperedges_distinct() {
    let mut bytes = header(2);
    record(
        &mut bytes,
        Some("moMateWidth"),
        "not a localized type name",
        27,
    );
    for path in ["A-1@Root", "A-2@Root", "B-1@Root/Leaf-1@B", "B-2@Root"] {
        reference(&mut bytes, "moSimpleSurfIdRep_c,70,227,", path);
    }
    reference(&mut bytes, "moSimpleSurfIdRep_c,70,227,", "A-1@Root");
    let next = record(&mut bytes, None, "renamed mate", 31);
    reference(&mut bytes, "moSimpleSurfIdRep_c,70,227,", "A-1@Root");
    let data = parse_stream(&bytes, "Contents/Config-7-MatesList", 7);
    assert!(data.inventory_matches_declared_count);
    assert_eq!(data.configuration_index, 7);
    assert_eq!(data.records[0].kind, Some(MateKind::Width));
    assert_eq!(data.records[0].name.value, "not a localized type name");
    assert_eq!(data.records[0].evidence_end, next);
    assert_eq!(data.records[0].reference_candidates.len(), 5);
    assert!(
        data.records[0].reference_candidates[0]
            .geometry_descriptor
            .is_some()
    );
    assert_eq!(data.records[1].class_reference, Some(0x80c1));
    assert_eq!(data.records[1].kind, None);
    let graph = data.reference_graph();
    assert_eq!(graph.len(), 4);
    assert_eq!(graph[0].reference, "A-1@Root");
    assert_eq!(graph[0].record_indices, [0, 1]);
    assert!(!data.entity_ownership_decoded);
    assert!(!data.parameters_decoded);
}
#[test]
fn nested_dimension_parameters_are_not_extra_mates_and_types_are_not_guessed() {
    let mut bytes = header(2);
    record(&mut bytes, Some("moMateDistanceDim_c"), "same name", 91);
    record(&mut bytes, Some("moLengthParameter_c"), "D1", u32::MAX);
    record(&mut bytes, Some("moMateFutureUnknown"), "same name", 92);
    let data = parse_stream(&bytes, "Contents/Config-0-MatesList", 0);
    assert!(data.inventory_matches_declared_count);
    assert_eq!(data.records.len(), 2);
    assert_eq!(data.records[0].kind, Some(MateKind::Distance));
    assert_eq!(
        data.records[1].class_name.as_deref(),
        Some("moMateFutureUnknown")
    );
    assert_eq!(data.records[1].kind, None);
    assert_ne!(data.records[0].source_offset, data.records[1].source_offset);
}
#[test]
fn stream_scope_empty_missing_and_unsupported_are_distinct() {
    let data = parse(&BTreeMap::from([
        ("Contents/Config-0-MatesList".into(), header(0)),
        ("Contents/Config-9-MatesList".into(), vec![0; 3]),
        ("Contents/Config-10-MatesList".into(), header(u16::MAX)),
        ("Contents/Config-bad-MatesList".into(), header(0)),
        ("Contents/Config-11-MatesList-extra".into(), header(0)),
    ]));
    assert_eq!(data.configurations.len(), 3);
    let empty = data
        .configurations
        .iter()
        .find(|c| c.configuration_index == 0)
        .unwrap();
    assert_eq!(empty.declared_count, Some(0));
    assert!(empty.inventory_matches_declared_count);
    for unsupported in data
        .configurations
        .iter()
        .filter(|c| c.configuration_index != 0)
    {
        assert!(!unsupported.inventory_matches_declared_count);
        assert_eq!(unsupported.diagnostics[0].code, "unsupported_mate_header");
    }
    assert!(parse(&BTreeMap::new()).configurations.is_empty());
}
#[test]
fn truncation_and_oversized_strings_remain_partial_without_panics() {
    let mut bytes = header(1);
    record(&mut bytes, Some("moMateConcentric"), "同心_名称", 42);
    reference(&mut bytes, "moSimpleSurfIdRep_c,1,2,", "A-1@Root");
    for end in 0..bytes.len() {
        let data = parse_stream(&bytes[..end], "Contents/Config-0-MatesList", 0);
        assert!(!data.entity_ownership_decoded);
    }
    let mut malformed = header(1);
    malformed.extend([255, 254, 255, 255]);
    malformed.extend(u32::MAX.to_le_bytes());
    let data = parse_stream(&malformed, "Contents/Config-0-MatesList", 0);
    assert!(!data.inventory_matches_declared_count);
    assert!(
        data.diagnostics
            .iter()
            .any(|d| d.code == "mate_count_mismatch")
    );
}
#[test]
fn long_unicode_references_round_trip_and_notes_and_file_paths_are_excluded() {
    let mut bytes = header(1);
    record(&mut bytes, Some("moMateLock_c"), "锁定", 42);
    let value = format!("{}-1@Root", "零件".repeat(150));
    reference(&mut bytes, "moSimpleSurfIdRep_c,1,2,", &value);
    text(&mut bytes, "C:\\email@example.com\\part.SLDPRT");
    let data = parse_stream(&bytes, "Contents/Config-0-MatesList", 0);
    assert_eq!(data.records[0].reference_candidates.len(), 1);
    let reference = &data.records[0].reference_candidates[0].reference;
    assert_eq!(reference.value, value);
    assert_eq!(
        native_string_at(&bytes, reference.offset).unwrap().0.value,
        value
    );
    assert_eq!(
        serde_json::to_value(&data).unwrap()["records"][0]["kind"],
        "lock"
    );
}

#[test]
fn metadata_exposes_native_configuration_indices_and_mate_coverage() {
    let mut bytes = header(1);
    record(&mut bytes, Some("moMateConcentric"), "Concentric1", 42);
    let doc = crate::metadata_from_container(
        crate::Container {
            streams: BTreeMap::from([
                ("Contents/Config-7-MatesList".into(), bytes),
                ("Contents/Config-2-MatesList".into(), header(0)),
            ]),
            diagnostics: Vec::new(),
        },
        "test.SLDASM",
    );
    let json = serde_json::to_value(doc).unwrap();
    assert_eq!(json["configurations"][0]["index"], 2);
    assert_eq!(json["configurations"][1]["index"], 7);
    assert_eq!(json["mates"]["configurations"][1]["configuration_index"], 7);
    assert_eq!(
        json["mates"]["configurations"][1]["records"][0]["kind"],
        "concentric"
    );
    assert_eq!(
        json["mates"]["configurations"][1]["entity_ownership_decoded"],
        false
    );
}
