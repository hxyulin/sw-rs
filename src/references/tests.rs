// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
//! Source identity and ambiguity regressions; all fixtures authored here.
use super::*;
use crate::{Container, Limits, UnresolvedVisualBinding};
use std::io::Write;

fn inspection(name: &str, xml: &[u8]) -> StreamInspection {
    crate::inventory::inspect(name, xml)
}
fn document() -> Document {
    crate::metadata_from_container(
        Container {
            streams: BTreeMap::new(),
            diagnostics: Vec::new(),
        },
        "part.SLDPRT",
    )
}
fn catalog(xml: &str) -> FeatureCatalog {
    FeatureCatalog::parse(Some(&inspection("swXmlContents/KeyWords", xml.as_bytes())))
}
#[test]
fn feature_tree_retains_duplicates_reserved_ids_unknown_attributes_and_nesting() {
    let features = catalog(
        r#"<Keywords xmlns:x="urn:other" id="77"><Configuration id="7"/><Feature id="12" Name="renamed" Type="localized" x:id="different"><Sketch id="12" Name="草图"><Dimension Name="D1">2</Dimension></Sketch><Feature id="-1"/><Feature id="4294967295"/><Feature id="0"/><Feature id="invalid"/></Feature></Keywords>"#,
    );
    assert_eq!(features.status, "decoded");
    assert_eq!(features.records.len(), 6);
    assert_eq!(features.records[0].parent, None);
    assert_eq!(features.records[1].parent, Some(0));
    assert_eq!(features.records[1].element, "Sketch");
    assert_eq!(features.records[1].name.as_deref(), Some("草图"));
    assert_eq!(features.records[0].source_id, Some(12));
    assert_eq!(features.records[1].source_id, Some(12));
    assert!(features.records[2..].iter().all(|f| f.source_id.is_none()));
    assert_eq!(features.records[2].saved_id.as_deref(), Some("-1"));
    assert!(
        features.records[0]
            .attributes
            .iter()
            .any(|a| a.namespace.as_deref() == Some("urn:other"))
    );
    assert!(!features.dependencies_decoded);
}
#[test]
fn wrapped_utf16_keyword_spans_refer_to_original_stream_bytes() {
    let text = "<Keywords><Feature id=\"9\" Name=\"纹理😀\"/></Keywords>";
    let mut bytes = vec![255, 254];
    bytes.extend(text.encode_utf16().flat_map(u16::to_le_bytes));
    let features = FeatureCatalog::parse(Some(&inspection("swXmlContents/KeyWords", &bytes)));
    let raw = features.records[0].span.slice(&bytes).unwrap();
    let decoded = String::from_utf16(
        &raw.as_chunks::<2>()
            .0
            .iter()
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
            .collect::<Vec<_>>(),
    )
    .unwrap();
    assert_eq!(decoded, "<Feature id=\"9\" Name=\"纹理😀\"/>");
    let mut wrapped = vec![0x86];
    wrapped.extend(b"<?xml version=\"1.0\"?><Keywords><Feature id=\"9\"/></Keywords>");
    let features = FeatureCatalog::parse(Some(&inspection("swXmlContents/KeyWords", &wrapped)));
    assert_eq!(
        features.records[0].span.slice(&wrapped),
        Some(b"<Feature id=\"9\"/>".as_slice())
    );
}
#[test]
fn absent_empty_invalid_and_namespaced_keyword_roots_are_distinct() {
    assert_eq!(FeatureCatalog::parse(None).status, "absent");
    assert_eq!(catalog("<Keywords/>").status, "decoded");
    assert!(catalog("<Keywords/>").records.is_empty());
    assert_eq!(catalog("<Keywords>").status, "invalid_xml");
    assert_eq!(
        catalog("<Keywords xmlns=\"urn:unknown\"/>").status,
        "unsupported_root"
    );
}
fn visual(id: u32, class: &str) -> UnresolvedVisualBinding {
    UnresolvedVisualBinding {
        owner_class: class.into(),
        identity: vec![id, 999],
        appearance_index: None,
        reason: "No decoded ownership".into(),
        source: PresentationSource {
            stream: "ThirdPtyStore/VisualStates".into(),
            path: "@byte[64]".into(),
        },
    }
}
#[test]
fn native_id_matches_remain_candidates_and_body_ids_never_become_feature_ids() {
    let features = catalog(
        "<Keywords><Feature id=\"9\" Name=\"same\"/><Feature id=\"10\" Name=\"same\"/><Feature id=\"10\" Name=\"other\"/></Keywords>",
    );
    let mut doc = document();
    doc.visuals.unresolved_bindings = vec![
        visual(9, "moCompFeature_c"),
        visual(10, "moCompFeature_c"),
        visual(8, "moCompFeature_c"),
        visual(9, "moCompSolidBody_c"),
    ];
    let graph = ReferenceGraph::build("hash", &features, &doc, &[]);
    let links = &graph.links;
    assert_eq!(
        links.iter().map(|l| l.resolution).collect::<Vec<_>>(),
        [
            ReferenceResolution::Candidate,
            ReferenceResolution::Ambiguous,
            ReferenceResolution::Unresolved,
            ReferenceResolution::Unresolved
        ]
    );
    assert_eq!(links[0].candidates, [0]);
    assert_eq!(links[1].candidates, [1, 2]);
    assert!(links[3].candidates.is_empty());
    assert_eq!(graph.nodes[3].source_offset, Some(64));
    assert!(!graph.topology_ownership_decoded);
    assert!(!graph.native_object_graph_decoded);
}
#[test]
fn xml_ids_are_stream_scoped_typed_and_never_resolved_by_first_match() {
    let a = inspection("swXmlContents/Features", br#"<swSolidWorks xmlns="http://www.solidworks.com/sw2003/schema"><swFile id="1"/><swModel id="2" swFileRef="1"/><swReference swModelRef="2"/><swReference swModelRef="1"/><swReference swModelRef="3"/><swModel id="4"/><swModel id="4"/><swReference swModelRef="4"/></swSolidWorks>"#);
    let b = inspection("swXmlContents/COMPINSTANCETREE", br#"<swSolidWorks xmlns="http://www.solidworks.com/sw2003/schema"><swModel id="3"/><swReference swFileRef="1"/></swSolidWorks>"#);
    let graph = ReferenceGraph::build("hash", &catalog("<Keywords/>"), &document(), &[a, b]);
    assert_eq!(
        graph.links.iter().map(|l| l.resolution).collect::<Vec<_>>(),
        [
            ReferenceResolution::Resolved,
            ReferenceResolution::Resolved,
            ReferenceResolution::Unresolved,
            ReferenceResolution::Unresolved,
            ReferenceResolution::Ambiguous,
            ReferenceResolution::Unresolved
        ]
    );
    assert_eq!(graph.links[4].candidates.len(), 2);
    assert!(graph.links[3].candidates.is_empty());
    assert!(graph.links[5].candidates.is_empty());
    assert_ne!(
        graph.nodes[graph.links[0].from].source.stream,
        graph.nodes[graph.links[5].from].source.stream
    );
}
fn file() -> Vec<u8> {
    let mut bytes = vec![0; 16];
    let content = b"<Keywords><Feature id=\"9\"/></Keywords>";
    let name = "swXmlContents/KeyWords";
    let mut encoder =
        flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(content).unwrap();
    let packed = encoder.finish().unwrap();
    bytes.extend([20, 0, 6, 0, 8, 0]);
    for word in [
        1,
        crc32fast::hash(content),
        packed.len() as u32,
        content.len() as u32,
        name.len() as u32,
    ] {
        bytes.extend(word.to_le_bytes());
    }
    bytes.extend(name.as_bytes());
    bytes.extend(packed);
    bytes
}
#[test]
fn reader_caches_graph_without_geometry_and_moves_requested_projections() {
    let bytes = file();
    let reader = crate::DocumentReader::open(&bytes, "part.SLDPRT", Limits::default()).unwrap();
    assert!(std::ptr::eq(
        reader.features().unwrap(),
        reader.features().unwrap()
    ));
    assert!(std::ptr::eq(
        reader.references().unwrap(),
        reader.references().unwrap()
    ));
    assert_eq!(
        reader.references().unwrap().source_sha256,
        reader.source_sha256()
    );
    let result = reader.finish().unwrap();
    assert!(result.geometry.is_none());
    assert!(result.metadata.is_some());
    assert!(result.features.is_some());
    assert!(result.references.is_some());
    let reader = crate::DocumentReader::open(b"unknown", "part.SLDPRT", Limits::default()).unwrap();
    assert!(reader.references().is_err());
}

#[test]
fn mate_identity_and_selection_evidence_keep_configuration_and_descriptor_context() {
    let mut doc = document();
    doc.mates.configurations.push(crate::ConfigurationMates {
        configuration_index: 7,
        source_stream: "Contents/Config-7-MatesList".into(),
        header_word: Some(193),
        declared_count: Some(1),
        inventory_matches_declared_count: true,
        entity_ownership_decoded: false,
        parameters_decoded: false,
        diagnostics: Vec::new(),
        records: vec![crate::MateRecord {
            name: crate::NativeString {
                offset: 12,
                value: "edited name".into(),
            },
            saved_identifier: 9,
            name_prefix_tag: 0x800c,
            class_name: Some("moMateWidth".into()),
            class_reference: None,
            kind: Some(crate::MateKind::Width),
            source_offset: 6,
            evidence_end: 200,
            reference_candidates: vec![crate::MateReferenceCandidate {
                reference: crate::NativeString {
                    offset: 150,
                    value: "A-1@Root".into(),
                },
                geometry_descriptor: Some(crate::NativeString {
                    offset: 110,
                    value: "moSimpleSurfIdRep_c,9,227,".into(),
                }),
            }],
        }],
    });
    let graph = ReferenceGraph::build(
        "hash",
        &catalog("<Keywords><Feature id=\"9\" Name=\"another name\"/></Keywords>"),
        &doc,
        &[],
    );
    assert_eq!(graph.links[0].resolution, ReferenceResolution::Unresolved);
    assert!(graph.links[0].candidates.is_empty());
    assert_eq!(graph.links[1].resolution, ReferenceResolution::Unresolved);
    assert_eq!(
        graph.links[1].selection_descriptor.as_ref().unwrap().offset,
        110
    );
    assert_eq!(graph.links[1].source_offset, Some(150));
    let surface = graph.links[1].selection_surface_reference.as_ref().unwrap();
    assert_eq!(surface.feature_source_id, Some(9));
    assert_eq!(surface.local_surface_id, Some(227));
    assert_eq!(surface.source.offset, 110);
    assert!(
        matches!(&graph.nodes[1].record, ReferenceRecord::Mate { configuration_index: 7, name, .. } if name == "edited name")
    );
    assert!(graph.links[1].candidates.is_empty());
    let mut binding = visual(9, "moCompFeature_c");
    binding.source.path = "unknown".into();
    doc.visuals.unresolved_bindings.push(binding);
    let graph = ReferenceGraph::build("hash", &catalog("<Keywords/>"), &doc, &[]);
    assert_eq!(graph.nodes.last().unwrap().source_offset, None);
}

#[test]
fn cache_feature_candidates_preserve_duplicate_ids_and_conflicting_prefixes() {
    let graph = ReferenceGraph::build(
        "hash",
        &catalog("<Keywords><Feature id=\"7\"/><Feature id=\"7\"/><Feature id=\"8\"/></Keywords>"),
        &document(),
        &[],
    );
    let record = |offset, values: &[&str]| crate::CachedSurfaceRecord {
        source_stream: "Contents/DisplayLists".into(),
        source_offset: offset,
        surface_references: crate::CachedSurfaceReferences {
            metadata_span: crate::ByteSpan {
                offset: 100,
                length: 200,
            },
            references: values
                .iter()
                .map(|v| {
                    crate::SavedSurfaceReference::parse(&crate::FramedString {
                        value: (*v).into(),
                        offset: 100,
                    })
                    .unwrap()
                })
                .collect(),
        },
    };
    let data = crate::CachedSurfaceData {
        schema: "sw-rs-surface-references-v1",
        source_sha256: "hash".into(),
        configuration_index: 7,
        configuration_name: "Alternate".into(),
        records: vec![
            record(12, &["moSimpleSurfIdRep_c,7,2,"]),
            record(
                24,
                &["moSimpleSurfIdRep_c,7,2,", "moSimpleSurfIdRep_c,8,2,"],
            ),
        ],
    };
    let graph = graph.with_surfaces(&data).unwrap();
    assert_eq!(graph.links[0].resolution, ReferenceResolution::Ambiguous);
    assert_eq!(graph.links[0].candidates, [0, 1]);
    assert_eq!(graph.links[1].resolution, ReferenceResolution::Unresolved);
    assert!(graph.links[1].candidates.is_empty());
    assert!(
        matches!(&graph.nodes[3].record,ReferenceRecord::CachedSurface {configuration_index:7,configuration_name,..} if configuration_name=="Alternate")
    );
    assert!(!graph.topology_ownership_decoded);
}
