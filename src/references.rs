// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
//! Source-scoped identity links with explicit syntactic versus native certainty.
use crate::{Document, FeatureCatalog, PresentationSource, StreamInspection, XmlAttribute};
use serde::Serialize;
use std::collections::BTreeMap;

const SW_NAMESPACE: &str = "http://www.solidworks.com/sw2003/schema";

/// A source record represented in a document-local reference graph.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ReferenceRecord {
    /// Validated display table and its adjacent saved surface-reference evidence.
    CachedSurface {
        /// Native configuration index from the canonical single-config geometry.
        configuration_index: u32,
        /// Exact saved configuration name; empty remains unknown.
        configuration_name: String,
        /// Descriptor-table offset, not a native B-Rep face ID.
        descriptor_offset: usize,
        /// Bounded raw references and decoded numeric tuples.
        surface_references: crate::CachedSurfaceReferences,
    },
    /// Feature catalog record; IDs and raw attributes live in `features.records`.
    Feature {
        /// Index in this graph's feature catalog.
        feature_index: usize,
    },
    /// XML object with its original attributes; `id` is local to its stream.
    XmlElement {
        /// Original structural element index, not a persistent native ID.
        element_index: usize,
        /// Exact local XML element name.
        name: String,
        /// XML namespace, kept distinct from unqualified elements.
        namespace: Option<String>,
        /// All decoded attributes, retaining resource paths and unknown values.
        attributes: Vec<XmlAttribute>,
    },
    /// Native mate header, without claiming resolved selections or active state.
    Mate {
        /// Native Config-N index, not dropdown position.
        configuration_index: u32,
        /// Record index within that configuration's mate inventory.
        record_index: usize,
        /// Saved header identifier; distinct from XML and topology ID spaces.
        saved_identifier: u32,
        /// Exact saved header name, without using it as an identity.
        name: String,
        /// Explicit native class declaration, if decoded at this header.
        class_name: Option<String>,
        /// Known family from an explicit class, not a localized name inference.
        mate_kind: Option<crate::MateKind>,
    },
    /// Unresolved native visual owner; identity words retain their class context.
    VisualOwner {
        /// Index into `Document.visuals.unresolved_bindings`.
        binding_index: usize,
        /// Exact native serialized class.
        owner_class: String,
        /// Raw identity words; not interchangeable across owner classes.
        identity: Vec<u32>,
        /// Index in `Document.visuals.definitions`, not presentation's merged array.
        appearance_index: Option<usize>,
    },
}

/// Graph record ID plus its authoritative source locator.
#[derive(Clone, Debug, Serialize)]
pub struct ReferenceNode {
    /// Zero-based graph-local node index; stable only for this source and schema.
    pub index: usize,
    /// Decoded stream and exact source path/byte locator.
    pub source: PresentationSource,
    /// Byte offset in the decoded stream, when available.
    pub source_offset: Option<usize>,
    /// Typed source identity, never a guessed topological entity.
    pub record: ReferenceRecord,
}

/// How far a relationship has been established.
#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReferenceResolution {
    /// Validated XML nesting or unique typed, stream-local XML ID reference only.
    Resolved,
    /// Unique identity match, but timestamp/configuration/native ownership unverified.
    Candidate,
    /// Multiple matches; no first-match selection is made.
    Ambiguous,
    /// Missing, reserved, wrong-type, or undecoded reference.
    Unresolved,
}

/// Relationship between source records, separate from its confidence.
#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReferenceRelation {
    /// Agreed cache-reference source ID matching a saved feature, without topology proof.
    CachedSurfaceFeatureIdentity,
    /// Saved feature-tree nesting, not regeneration dependency.
    FeatureParent,
    /// Exact unqualified swFileRef to a same-stream swFile record.
    XmlFile,
    /// Exact unqualified swModelRef to a same-stream swModel record.
    XmlModel,
    /// Saved mate identifier, whose native identity namespace is not decoded.
    NativeMateIdentity,
    /// moCompFeature source-ID matching a saved feature; timestamp unverified.
    VisualFeatureIdentity,
    /// Native visual owner whose persistent entity identity remains undecoded.
    NativeVisualOwner,
    /// Saved mate selection string; never interpreted by splitting editable names.
    MateSelection,
}

/// One source relationship; candidates are not automatically applied ownership.
#[derive(Clone, Debug, Serialize)]
pub struct ReferenceLink {
    /// Referring graph-local node index.
    pub from: usize,
    /// Saved relationship being inspected.
    pub relation: ReferenceRelation,
    /// Exact token or decimal numeric word; None when no identity was decoded.
    pub reference: Option<String>,
    /// All matching graph-local record indices; no deduplication across source records.
    pub candidates: Vec<usize>,
    /// Resolution certainty for this link.
    pub resolution: ReferenceResolution,
    /// Concrete missing evidence, or the limited fact established by resolution.
    pub reason: String,
    /// Exact decoded source offset for the reference, when separately available.
    pub source_offset: Option<usize>,
    /// Adjacent saved geometry descriptor for mate-selection framing, if supported.
    /// Raw text and offset only; contained numbers are not decoded entity IDs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selection_descriptor: Option<crate::NativeString>,
    /// Supported numeric surface descriptor, retaining opaque or invalid fields.
    /// Does not establish the mate's endpoint or its component occurrence.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selection_surface_reference: Option<crate::SavedSurfaceReference>,
    /// Ordered rows when a mate descriptor contains more than one surface record.
    /// The singular field is absent for these frames; no row is selected as owner.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub selection_surface_references: Vec<crate::SavedSurfaceReference>,
}

/// Shared source-reference layer. No geometry or renderer assumptions are required.
#[derive(Clone, Debug, Serialize)]
pub struct ReferenceGraph {
    /// Versioned interchange schema.
    pub schema: &'static str,
    /// Original file SHA-256. Node indices must never be reused with another source.
    pub source_sha256: String,
    /// Named saved feature IDs, including reserved and duplicate records.
    pub features: FeatureCatalog,
    /// Document-local typed records with source provenance.
    pub nodes: Vec<ReferenceNode>,
    /// Resolved syntactic links and unresolved/candidate native relationships.
    pub links: Vec<ReferenceLink>,
    /// False: native object reuse/class maps are not completely decoded.
    pub native_object_graph_decoded: bool,
    /// False: no persistent B-Rep face/edge/body graph is established here.
    pub topology_ownership_decoded: bool,
}
impl ReferenceGraph {
    /// Add cache-table reference evidence from exactly the same original file.
    /// Source/local/tail tuples remain distinct; only feature-ID candidates are
    /// joined. Existing table locators are retained, making repeated calls idempotent.
    /// Does not infer B-Rep ownership, apply appearances, or resolve mate endpoints.
    pub fn with_surfaces(
        mut self,
        surfaces: &crate::CachedSurfaceData,
    ) -> Result<Self, crate::ParseError> {
        if surfaces.schema != "sw-rs-surface-references-v1" {
            return Err(crate::ParseError(
                "unsupported surface-reference schema".into(),
            ));
        }
        if self.source_sha256 != surfaces.source_sha256 {
            return Err(crate::ParseError(
                "reference graph and surface data source hashes differ".into(),
            ));
        }
        let mut feature_ids = BTreeMap::<u32, Vec<usize>>::new();
        let mut cached = std::collections::BTreeSet::new();
        for node in &self.nodes {
            match &node.record {
                ReferenceRecord::Feature { feature_index } => {
                    if let Some(id) = self
                        .features
                        .records
                        .get(*feature_index)
                        .and_then(|f| f.source_id)
                    {
                        feature_ids.entry(id).or_default().push(node.index);
                    }
                }
                ReferenceRecord::CachedSurface {
                    descriptor_offset,
                    configuration_index,
                    configuration_name,
                    ..
                } => {
                    if *configuration_index != surfaces.configuration_index
                        || *configuration_name != surfaces.configuration_name
                    {
                        return Err(crate::ParseError(
                            "cached surface configuration differs from existing graph records"
                                .into(),
                        ));
                    }
                    cached.insert((node.source.stream.clone(), *descriptor_offset));
                }
                _ => {}
            }
        }
        for table in &surfaces.records {
            if !cached.insert((table.source_stream.clone(), table.source_offset)) {
                continue;
            }
            let node = self.node(
                &table.source_stream,
                format!("table[{}]", table.source_offset),
                Some(table.source_offset),
                ReferenceRecord::CachedSurface {
                    configuration_index: surfaces.configuration_index,
                    configuration_name: surfaces.configuration_name.clone(),
                    descriptor_offset: table.source_offset,
                    surface_references: table.surface_references.clone(),
                },
            );
            self.identity_link(node,ReferenceRelation::CachedSurfaceFeatureIdentity,table.surface_references.feature_source_id(),&feature_ids,"Cache metadata agrees on the feature source ID; timestamp and native face/body ownership are not decoded");
        }
        Ok(self)
    }
    fn node(
        &mut self,
        stream: &str,
        path: String,
        offset: Option<usize>,
        record: ReferenceRecord,
    ) -> usize {
        let index = self.nodes.len();
        self.nodes.push(ReferenceNode {
            index,
            source: PresentationSource {
                stream: stream.into(),
                path,
            },
            source_offset: offset,
            record,
        });
        index
    }
    pub(crate) fn build(
        hash: &str,
        features: &FeatureCatalog,
        doc: &Document,
        xml: &[StreamInspection],
    ) -> Self {
        let mut out = Self {
            schema: "sw-rs-references-v1",
            source_sha256: hash.into(),
            features: features.clone(),
            nodes: Vec::new(),
            links: Vec::new(),
            native_object_graph_decoded: false,
            topology_ownership_decoded: false,
        };
        let mut feature_ids = BTreeMap::<u32, Vec<usize>>::new();
        let mut feature_nodes = Vec::new();
        for feature in &features.records {
            let index = out.node(
                &features.source_stream,
                format!("element[{}]", feature.element_index),
                Some(feature.span.offset),
                ReferenceRecord::Feature {
                    feature_index: feature.index,
                },
            );
            feature_nodes.push(index);
            if let Some(id) = feature.source_id {
                feature_ids.entry(id).or_default().push(index);
            }
            if let Some(parent) = feature.parent {
                out.links.push(ReferenceLink {
                    from: index,
                    relation: ReferenceRelation::FeatureParent,
                    reference: None,
                    candidates: vec![feature_nodes[parent]],
                    resolution: ReferenceResolution::Resolved,
                    reason: "Saved XML containment only; not a feature dependency".into(),
                    source_offset: Some(feature.span.offset),
                    selection_descriptor: None,
                    selection_surface_reference: None,
                    selection_surface_references: Vec::new(),
                });
            }
        }
        for inspection in xml {
            out.xml_links(inspection);
        }
        for configuration in &doc.mates.configurations {
            for (record_index, mate) in configuration.records.iter().enumerate() {
                let node = out.node(
                    &configuration.source_stream,
                    format!("byte[{}]", mate.source_offset),
                    Some(mate.source_offset),
                    ReferenceRecord::Mate {
                        configuration_index: configuration.configuration_index,
                        record_index,
                        saved_identifier: mate.saved_identifier,
                        name: mate.name.value.clone(),
                        class_name: mate.class_name.clone(),
                        mate_kind: mate.kind,
                    },
                );
                out.links.push(ReferenceLink { from: node, relation: ReferenceRelation::NativeMateIdentity, reference: Some(mate.saved_identifier.to_string()), candidates: Vec::new(), resolution: ReferenceResolution::Unresolved, reason: "Mate identifier namespace and native object references not decoded; not joined to feature IDs".into(), source_offset: None, selection_descriptor: None, selection_surface_reference: None, selection_surface_references: Vec::new() });
                for selection in &mate.reference_candidates {
                    let mut surfaces = selection.surface_references();
                    let surface = (surfaces.len() == 1).then(|| surfaces.remove(0));
                    out.links.push(ReferenceLink { from: node, relation: ReferenceRelation::MateSelection, reference: Some(selection.reference.value.clone()), candidates: Vec::new(), resolution: ReferenceResolution::Unresolved, reason: "Saved selection string retained; occurrence chain and geometric endpoint not decoded".into(), source_offset: Some(selection.reference.offset), selection_descriptor: selection.geometry_descriptor.clone(), selection_surface_reference: surface, selection_surface_references: surfaces });
                }
            }
        }
        for (binding_index, binding) in doc.visuals.unresolved_bindings.iter().enumerate() {
            // Source paths are emitted by the visual decoder as @byte[N].
            let offset = binding
                .source
                .path
                .strip_prefix("@byte[")
                .and_then(|s| s.strip_suffix(']'))
                .and_then(|s| s.parse().ok());
            let node = out.node(
                &binding.source.stream,
                binding.source.path.clone(),
                offset,
                ReferenceRecord::VisualOwner {
                    binding_index,
                    owner_class: binding.owner_class.clone(),
                    identity: binding.identity.clone(),
                    appearance_index: binding.appearance_index,
                },
            );
            if binding.owner_class == "moCompFeature_c" {
                out.identity_link(node, ReferenceRelation::VisualFeatureIdentity, binding.identity.first().copied(), &feature_ids, "Feature source-ID match only; saved timestamp, configuration and topology ownership unverified");
            } else {
                out.links.push(ReferenceLink {
                    from: node,
                    relation: ReferenceRelation::NativeVisualOwner,
                    reference: None,
                    candidates: Vec::new(),
                    resolution: ReferenceResolution::Unresolved,
                    reason: binding.reason.clone(),
                    source_offset: offset,
                    selection_descriptor: None,
                    selection_surface_reference: None,
                    selection_surface_references: Vec::new(),
                });
            }
        }
        out
    }
    fn identity_link(
        &mut self,
        from: usize,
        relation: ReferenceRelation,
        id: Option<u32>,
        ids: &BTreeMap<u32, Vec<usize>>,
        reason: &str,
    ) {
        let candidates = id.and_then(|id| ids.get(&id)).cloned().unwrap_or_default();
        let resolution = match candidates.len() {
            0 => ReferenceResolution::Unresolved,
            1 => ReferenceResolution::Candidate,
            _ => ReferenceResolution::Ambiguous,
        };
        self.links.push(ReferenceLink { from, relation, reference: id.map(|id| id.to_string()), candidates, resolution, reason: if id.is_none() { "No common valid source ID was decoded; missing, invalid or disagreeing references remain unresolved".into() } else if resolution == ReferenceResolution::Unresolved { "No supported saved feature matches this ID; absence does not establish a deleted entity".into() } else { reason.into() }, source_offset: None, selection_descriptor: None, selection_surface_reference: None, selection_surface_references: Vec::new() });
    }
    fn xml_links(&mut self, inspection: &StreamInspection) {
        let mut indices = BTreeMap::new();
        for element in &inspection.xml_elements {
            let node = self.node(
                &inspection.source_stream,
                format!("element[{}]", element.index),
                Some(element.span.offset),
                ReferenceRecord::XmlElement {
                    element_index: element.index,
                    name: element.name.clone(),
                    namespace: element.namespace.clone(),
                    attributes: element.attributes.clone(),
                },
            );
            indices.insert(element.index, node);
        }
        for reference in &inspection.xml_references {
            let (relation, target) = match reference.attribute.as_str() {
                "swFileRef" => (ReferenceRelation::XmlFile, "swFile"),
                "swModelRef" => (ReferenceRelation::XmlModel, "swModel"),
                _ => continue,
            };
            let valid = reference.candidate_elements.iter().all(|&i| {
                inspection.xml_elements[i].name == target
                    && inspection.xml_elements[i].namespace.as_deref() == Some(SW_NAMESPACE)
            });
            let resolution = match reference.candidate_elements.len() {
                0 => ReferenceResolution::Unresolved,
                1 if valid && !reference.reference.is_empty() => ReferenceResolution::Resolved,
                1 => ReferenceResolution::Unresolved,
                _ => ReferenceResolution::Ambiguous,
            };
            self.links.push(ReferenceLink {
                from: indices[&reference.from_element],
                relation,
                reference: Some(reference.reference.clone()),
                candidates: reference
                    .candidate_elements
                    .iter()
                    .map(|i| indices[i])
                    .collect(),
                resolution,
                reason: if resolution == ReferenceResolution::Resolved {
                    "Unique typed ID within this XML stream only"
                } else {
                    "Missing, duplicate, empty or wrong-type XML ID; no cross-stream/name fallback"
                }
                .into(),
                source_offset: Some(inspection.xml_elements[reference.from_element].span.offset),
                selection_descriptor: None,
                selection_surface_reference: None,
                selection_surface_references: Vec::new(),
            });
        }
    }
}

#[cfg(test)]
mod tests;
