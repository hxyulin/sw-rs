// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
// Keyword feature interpretation adapted from cadmpeg history/mod.rs;
// see NOTICE-cadmpeg.md. Source XML remains authoritative.
//! Saved feature-tree identities, independent of feature evaluation or topology.
use crate::{ByteSpan, StreamInspection, XmlAttribute};
use serde::Serialize;
use std::collections::BTreeMap;

/// A saved feature-tree entry, including sketches, folders and unknown tags.
#[derive(Clone, Debug, Serialize)]
pub struct SavedFeature {
    /// Index within this catalog, in saved XML preorder, not regeneration order.
    pub index: usize,
    /// Closest containing feature record; not a geometric dependency.
    pub parent: Option<usize>,
    /// Original structural XML element index.
    pub element_index: usize,
    /// Complete source element range in decoded-stream bytes.
    pub span: ByteSpan,
    /// Exact saved element tag.
    pub element: String,
    /// Raw unqualified `id`, preserving reserved/invalid tokens and duplicates.
    pub saved_id: Option<String>,
    /// Positive numeric ID, excluding reserved zero and FFFFFFFF values.
    /// Document-local evidence; never a face ID or a globally persistent key.
    pub source_id: Option<u32>,
    /// Exact unqualified saved name, when present.
    pub name: Option<String>,
    /// Exact saved type token; localized values are not interpreted as operation types.
    pub saved_type: Option<String>,
    /// All original decoded attributes, including unknown and namespaced values.
    pub attributes: Vec<XmlAttribute>,
}

/// Partial saved feature catalog. Absence is distinct from a successfully empty tree.
#[derive(Clone, Debug, Serialize)]
pub struct FeatureCatalog {
    /// Versioned interchange schema.
    pub schema: &'static str,
    /// Exact decoded source stream.
    pub source_stream: String,
    /// `decoded`, `absent`, `invalid_xml`, or `unsupported_root`.
    pub status: &'static str,
    /// Saved feature-tree entries; no suppressed entries are discarded.
    pub records: Vec<SavedFeature>,
    /// Parsing limitations or XML errors, without failing unrelated metadata.
    pub diagnostics: Vec<String>,
    /// False: tree nesting/order does not establish regeneration dependencies.
    pub dependencies_decoded: bool,
}
pub(crate) fn attribute<'a>(attributes: &'a [XmlAttribute], name: &str) -> Option<&'a str> {
    attributes
        .iter()
        .find(|a| a.namespace.is_none() && a.name == name)
        .map(|a| a.value.as_str())
}
impl FeatureCatalog {
    pub(crate) fn parse(inspection: Option<&StreamInspection>) -> Self {
        let mut out = Self {
            schema: "sw-rs-features-v1",
            source_stream: "swXmlContents/KeyWords".into(),
            status: "absent",
            records: Vec::new(),
            diagnostics: Vec::new(),
            dependencies_decoded: false,
        };
        let Some(inspection) = inspection else {
            return out;
        };
        out.status = "invalid_xml";
        if inspection.structure_status != "xml" {
            out.diagnostics.push(
                inspection
                    .xml_error
                    .clone()
                    .unwrap_or_else(|| "Unsupported KeyWords encoding/layout".into()),
            );
            return out;
        }
        out.status = "unsupported_root";
        if !inspection
            .xml_elements
            .first()
            .is_some_and(|e| e.name == "Keywords" && e.namespace.is_none())
        {
            out.diagnostics
                .push("Expected unqualified Keywords root; no feature identities inferred".into());
            return out;
        }
        out.status = "decoded";
        let mut ancestors = BTreeMap::new();
        for element in &inspection.xml_elements {
            let parent = element
                .parent
                .and_then(|p| ancestors.get(&p).copied().flatten());
            if element.namespace.is_none()
                && !matches!(
                    element.name.as_str(),
                    "Keywords" | "Configuration" | "Dimension"
                )
            {
                let index = out.records.len();
                let saved_id = attribute(&element.attributes, "id").map(str::to_owned);
                let source_id = saved_id
                    .as_deref()
                    .and_then(|s| s.parse::<u32>().ok())
                    .filter(|id| *id != 0 && *id != u32::MAX);
                out.records.push(SavedFeature {
                    index,
                    parent,
                    element_index: element.index,
                    span: element.span,
                    element: element.name.clone(),
                    saved_id,
                    source_id,
                    name: attribute(&element.attributes, "Name").map(str::to_owned),
                    saved_type: attribute(&element.attributes, "Type").map(str::to_owned),
                    attributes: element.attributes.clone(),
                });
                ancestors.insert(element.index, Some(index));
            } else {
                ancestors.insert(element.index, parent);
            }
        }
        out
    }
}
