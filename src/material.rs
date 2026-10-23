// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
//! Saved MATERIALTREE definitions and explicit configuration references.
//!
//! The profile is based on locally inspected saved XML, not a published native
//! file specification. Definitions can be stale or unassigned. Saved swatches are
//! metadata only: appearances do not identify physical material or body ownership.
//! See https://help.solidworks.com/2016/english/SolidWorks/sldworks/c_appearances_overview.htm.
use crate::Diagnostic;
use serde::Serialize;

pub(crate) const STREAM: &str = "swXmlContents/MATERIALTREE";

/// Saved physical-material records, without inferred body ownership or freshness.
#[derive(Clone, Debug, Serialize)]
pub struct MaterialData {
    /// Decoded container stream carrying every record in this collection.
    pub source_stream: String,
    /// Saved definitions, including definitions without a known assignment.
    pub definitions: Vec<SavedMaterial>,
    /// Saved configuration references, including incomplete or unresolved records.
    pub assignments: Vec<MaterialAssignment>,
    /// False: this profile does not decode body-level material ownership.
    pub body_assignments_decoded: bool,
}

/// One saved material definition, which alone does not establish an assignment.
#[derive(Clone, Debug, Serialize)]
pub struct SavedMaterial {
    /// Exact saved name, often a database and label separated by `|`.
    pub name: String,
    /// Exact saved `matid`, absent for some custom definitions.
    pub material_id: Option<String>,
    /// Outer-to-inner saved classification names; no material family is inferred.
    pub classifications: Vec<String>,
    /// Saved density token, if exactly one physicalproperties/DENS record exists.
    pub density: Option<MaterialDensity>,
    /// Exact saved swatch RGB attribute, not a resolved face/body appearance.
    pub saved_swatch_rgb: Option<String>,
    /// Element path in `MaterialData::source_stream`, using one-based sibling indices.
    pub source_path: String,
}

/// A density token from a saved material definition, not a validated physical value.
#[derive(Clone, Debug, Serialize)]
pub struct MaterialDensity {
    /// Exact `value` attribute; units and cache freshness are not certified.
    pub value: String,
    /// Finite positive numeric interpretation, absent for invalid tokens.
    pub numeric_value: Option<f64>,
    /// Saved localized display name, when present.
    pub display_name: Option<String>,
    /// Explicit source units only; never filled using a guessed convention.
    pub units: Option<String>,
    /// Element path of the DENS record in the source stream.
    pub source_path: String,
}

/// An explicit saved configuration-to-material reference, never a color inference.
#[derive(Clone, Debug, Serialize)]
pub struct MaterialAssignment {
    /// Exact saved configuration name; empty for an incomplete record.
    pub configuration: String,
    /// Exact saved material key; empty for an incomplete record.
    pub material_reference: String,
    /// Index into `MaterialData::definitions` only when the key matches uniquely.
    pub definition_index: Option<usize>,
    /// Element path of the configuration reference in the source stream.
    pub source_path: String,
}

fn path(node: roxmltree::Node<'_, '_>) -> String {
    let mut parts = Vec::new();
    for n in node.ancestors().filter(|n| n.is_element()) {
        let name = n.tag_name().name();
        let index = n
            .prev_siblings()
            .filter(|s| *s != n && s.is_element() && s.tag_name() == n.tag_name())
            .count()
            + 1;
        parts.push(format!("{name}[{index}]"));
    }
    parts.reverse();
    format!("/{}", parts.join("/"))
}

fn text(bytes: &[u8]) -> Option<String> {
    if bytes.starts_with(&[0xff, 0xfe]) || bytes.starts_with(&[0xfe, 0xff]) {
        let little = bytes[0] == 0xff;
        let payload = &bytes[2..];
        if !payload.len().is_multiple_of(2) {
            return None;
        }
        let units = payload
            .as_chunks::<2>()
            .0
            .iter()
            .map(|b| {
                if little {
                    u16::from_le_bytes([b[0], b[1]])
                } else {
                    u16::from_be_bytes([b[0], b[1]])
                }
            })
            .collect::<Vec<_>>();
        String::from_utf16(&units).ok()
    } else {
        std::str::from_utf8(bytes)
            .ok()
            .map(|s| s.trim_start_matches('\u{feff}').to_owned())
    }
}

pub(crate) fn parse(bytes: &[u8], diagnostics: &mut Vec<Diagnostic>) -> Option<MaterialData> {
    let warn = |diagnostics: &mut Vec<Diagnostic>, message: &str| {
        diagnostics.push(Diagnostic {
            offset: 0,
            message: format!("{STREAM}: {message}"),
        });
    };
    let Some(text) = text(bytes) else {
        warn(diagnostics, "invalid UTF-8/UTF-16 material XML encoding");
        return None;
    };
    let Ok(doc) = roxmltree::Document::parse(&text) else {
        warn(diagnostics, "malformed material XML");
        return None;
    };
    let root = doc.root_element();
    if root.tag_name().name() != "materials"
        || root.tag_name().namespace() != Some("http://www.solidworks.com/sldmaterials")
    {
        warn(diagnostics, "unsupported material XML root/namespace");
        return None;
    }
    let mut definitions = Vec::new();
    for n in root.descendants().filter(|n| {
        n.has_tag_name("material") && n.parent().is_some_and(|p| p.has_tag_name("classification"))
    }) {
        let mut classifications = n
            .ancestors()
            .filter(|p| p.has_tag_name("classification"))
            .map(|p| p.attribute("name").unwrap_or_default().to_owned())
            .collect::<Vec<_>>();
        classifications.reverse();
        let densities = n
            .children()
            .filter(|p| p.has_tag_name("physicalproperties"))
            .flat_map(|p| p.children())
            .filter(|p| p.has_tag_name("DENS"))
            .collect::<Vec<_>>();
        let density = if let [d] = densities.as_slice() {
            let value = d.attribute("value").unwrap_or_default().to_owned();
            let numeric_value = value
                .trim()
                .parse::<f64>()
                .ok()
                .filter(|v| v.is_finite() && *v > 0.0);
            if numeric_value.is_none() {
                warn(diagnostics, &format!("invalid density at {}", path(*d)));
            }
            Some(MaterialDensity {
                value,
                numeric_value,
                display_name: d.attribute("displayname").map(str::to_owned),
                units: d.attribute("units").map(str::to_owned),
                source_path: path(*d),
            })
        } else {
            if densities.len() > 1 {
                warn(diagnostics, &format!("ambiguous density at {}", path(n)));
            }
            None
        };
        definitions.push(SavedMaterial {
            name: n.attribute("name").unwrap_or_default().to_owned(),
            material_id: n.attribute("matid").map(str::to_owned),
            classifications,
            density,
            saved_swatch_rgb: n
                .children()
                .find(|p| p.has_tag_name("swatchcolor"))
                .and_then(|p| p.attribute("RGB"))
                .map(str::to_owned),
            source_path: path(n),
        });
    }
    let mut assignments = Vec::new();
    for n in root
        .children()
        .filter(|n| n.has_tag_name("configurationslist"))
        .flat_map(|n| n.children())
        .filter(|n| n.has_tag_name("configuration"))
    {
        let configuration = n.attribute("name").unwrap_or_default().to_owned();
        let material_reference = n.attribute("material").unwrap_or_default().to_owned();
        let matches = definitions
            .iter()
            .enumerate()
            .filter(|(_, d)| {
                let key = match &d.material_id {
                    Some(id) => format!("{}|{id}", d.name),
                    None => d.name.clone(),
                };
                !d.name.is_empty() && !configuration.is_empty() && key == material_reference
            })
            .map(|(i, _)| i)
            .collect::<Vec<_>>();
        let definition_index = if let [i] = matches.as_slice() {
            Some(*i)
        } else {
            None
        };
        assignments.push(MaterialAssignment {
            configuration,
            material_reference,
            definition_index,
            source_path: path(n),
        });
    }
    let unresolved = assignments
        .iter()
        .filter(|a| a.definition_index.is_none())
        .count();
    if unresolved > 0 {
        warn(
            diagnostics,
            &format!(
                "{unresolved} incomplete, missing or ambiguous material references; no assignment inferred"
            ),
        );
    }
    if !definitions.is_empty() {
        warn(
            diagnostics,
            "saved material definitions may be unassigned or stale; density units/freshness and body-level assignments are unverified",
        );
    }
    Some(MaterialData {
        body_assignments_decoded: false,
        source_stream: STREAM.into(),
        definitions,
        assignments,
    })
}
