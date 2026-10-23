// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
// Ported from openswx document.cc and internal/xml_readers.cc.
// Copyright (c) 2026 openswx contributors; see LICENSE-openswx.
use crate::{Container, Diagnostic, config, xml};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

/// Cached values copied from source tokens; units, frame, and freshness are unverified.
#[derive(Debug, Serialize)]
pub struct MassProperties {
    /// Saved center-of-gravity tokens (not guaranteed metres).
    pub center_of_gravity: [f64; 3],
    /// Saved volume token.
    pub volume: f64,
    /// Saved surface-area token.
    pub surface_area: f64,
    /// Saved mass token (not guaranteed kilograms).
    pub mass: f64,
    /// Saved diagonal inertia tokens; not validated physical inertials.
    pub moments_of_inertia: [f64; 3],
    /// Saved cross-inertia tokens; sign and frame are unverified.
    pub products_of_inertia: [f64; 3],
}
/// One saved component occurrence; external paths are retained without resolution.
#[derive(Debug, Serialize)]
pub struct Component {
    /// Occurrence name, preserving Unicode.
    pub name: String,
    /// Source path, often a Windows path.
    pub path: String,
    /// Referenced child configuration.
    pub ref_config: String,
    /// User-defined component reference.
    pub component_reference: String,
    /// Saved suppression flag (abbreviated records count as suppressed).
    pub is_suppressed: bool,
    /// Saved hidden flag.
    pub is_hidden: bool,
    /// Saved BOM exclusion flag.
    pub exclude_from_bom: bool,
    /// Saved 16-value transform, column-major according to openswx.
    /// None means absent or malformed; an identity is never invented.
    pub transform: Option<[f64; 16]>,
}
/// A saved configuration, identified by its native Config-N index.
#[derive(Debug, Serialize)]
pub struct Configuration {
    /// Native configuration index.
    pub index: u32,
    /// Resolved source name; empty when unknown.
    pub name: String,
    /// Saved parent configuration index, when present.
    pub parent_index: Option<u32>,
    /// Saved user properties.
    pub properties: BTreeMap<String, String>,
    /// Unvalidated saved mass properties.
    pub mass_properties: Option<MassProperties>,
    /// Direct occurrences for assemblies, without recursive file loading.
    pub components: Vec<Component>,
}
/// A partial metadata view of a modern SolidWorks document.
#[derive(Debug, Serialize)]
pub struct Document {
    /// Extension-derived kind: part, assembly, or drawing.
    pub document_type: String,
    /// Native model version (not a calendar year); zero when unknown.
    pub version: u32,
    /// Saved global custom and core properties.
    pub global_properties: BTreeMap<String, String>,
    /// Saved configurations; drawings currently expose raw streams only.
    pub configurations: Vec<Configuration>,
    /// Document PNG preview bytes, without decoding the image.
    pub preview_png: Vec<u8>,
    /// Names of decoded streams; use Container for the underlying bytes.
    pub stream_names: Vec<String>,
    /// Container and metadata diagnostics.
    pub diagnostics: Vec<Diagnostic>,
}
pub(crate) fn build(mut container: Container, filename: &str) -> Document {
    let streams = &container.streams;
    let get = |name: &str| streams.get(name).map(Vec::as_slice).unwrap_or_default();
    let version = streams
        .keys()
        .filter_map(|name| {
            name.strip_prefix("_MO_VERSION_")?
                .split_once('/')?
                .0
                .parse::<u32>()
                .ok()
        })
        .max()
        .unwrap_or(0);
    let kind = if filename.to_ascii_lowercase().ends_with(".sldasm") {
        "assembly"
    } else if filename.to_ascii_lowercase().ends_with(".slddrw") {
        "drawing"
    } else {
        "part"
    };
    let custom = xml::properties(get("docProps/custom.xml"), false);
    let mut global = custom.clone();
    for (k, v) in xml::core(get("docProps/core.xml")) {
        global.entry(k).or_insert(v);
    }
    let info = xml::properties(get("docProps/ISolidWorksInformation.xml"), true);
    let manager = if streams.contains_key("Contents/CMgrHdr2") {
        get("Contents/CMgrHdr2")
    } else {
        get("Contents/CMgrHdr")
    };
    let names = config::names(manager, version);
    let tree = xml::tree(get("swXmlContents/COMPINSTANCETREE"));
    let mut indices: BTreeSet<u32> = streams
        .keys()
        .filter_map(|s| {
            let suffix = s
                .strip_prefix("docProps/Config-")
                .or_else(|| s.strip_prefix("Contents/Config-"))?;
            suffix
                .chars()
                .take_while(char::is_ascii_digit)
                .collect::<String>()
                .parse()
                .ok()
        })
        .collect();
    indices.extend(names.keys().copied());
    if indices.is_empty() {
        indices.insert(0);
    }
    let mut configurations = Vec::new();
    let mut seen = BTreeSet::new();
    if kind != "drawing" {
        for index in indices {
            let (mut name, parent_index) = names.get(&index).cloned().unwrap_or_default();
            if name.is_empty() {
                name = tree.names.get(&index).cloned().unwrap_or_default();
            }
            if name.is_empty() {
                name = xml::root_attribute(
                    get(&format!("docProps/Config-{index}-Cutlist-Properties.xml")),
                    "Name",
                );
            }
            if name.is_empty() && index == 0 {
                name = info
                    .get("SW-Configuration Name")
                    .cloned()
                    .unwrap_or_default();
            }
            if !name.is_empty() && !seen.insert(name.clone()) {
                continue;
            }
            let mut properties = custom.clone();
            properties.extend(xml::properties(
                get(&format!("docProps/Config-{index}-Properties.xml")),
                false,
            ));
            if name.is_empty() {
                name = properties.get("Configuration").cloned().unwrap_or_default();
            }
            configurations.push(Configuration {
                index,
                name,
                parent_index,
                properties,
                mass_properties: info
                    .get(&format!("SW-MassProp-Config-{index}"))
                    .and_then(|s| xml::mass(s)),
                components: if kind == "assembly" {
                    xml::components(&tree, index)
                } else {
                    Vec::new()
                },
            });
        }
        let unnamed: Vec<_> = configurations
            .iter()
            .enumerate()
            .filter(|(_, c)| c.name.is_empty())
            .map(|(i, _)| i)
            .collect();
        if let [i] = unnamed.as_slice() {
            if let Some(name) = info
                .get("SW-Configuration Name")
                .filter(|n| !seen.contains(*n))
            {
                configurations[*i].name = name.clone();
            } else {
                let bases: Vec<_> = configurations
                    .iter()
                    .filter_map(|c| {
                        let s = c.name.strip_suffix('>')?;
                        let (prefix, base) = s.rsplit_once('<')?;
                        (!prefix.is_empty()
                            && !base.is_empty()
                            && !seen.contains(base)
                            && (prefix.ends_with(' ') || !base.contains(' ')))
                        .then_some(base.to_owned())
                    })
                    .collect();
                if let [name] = bases.as_slice() {
                    configurations[*i].name = name.clone();
                }
            }
        }
    }
    // Surface malformed XML instead of silently claiming complete extraction.
    for (name, data) in streams {
        if (name.ends_with(".xml") || name.ends_with("COMPINSTANCETREE"))
            && xml::parse(data).is_none()
        {
            container.diagnostics.push(Diagnostic {
                offset: 0,
                message: format!("invalid or unsupported XML encoding: {name}"),
            });
        }
    }
    container.diagnostics.push(Diagnostic {offset: 0, message: "partial metadata reader: geometry, mates, cut lists and drawing semantics are not decoded; cached physical units remain unverified".into()});
    let preview = get("PreviewPNG");
    let magic = b"\x89PNG\r\n\x1a\n";
    let preview_png = preview
        .windows(8)
        .position(|w| w == magic)
        .map(|p| preview[p..].to_vec())
        .unwrap_or_default();
    Document {
        document_type: kind.into(),
        version,
        global_properties: global,
        configurations,
        preview_png,
        stream_names: streams.keys().cloned().collect(),
        diagnostics: container.diagnostics,
    }
}
