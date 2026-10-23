// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
// Ported from openswx internal/xml_readers.cc.
// Copyright (c) 2026 openswx contributors; see LICENSE-openswx.
use crate::{Component, MassProperties};
use roxmltree::{Document, Node};
use std::collections::BTreeMap;
pub(crate) fn parse(bytes: &[u8]) -> Option<Document<'_>> {
    Document::parse(std::str::from_utf8(bytes).ok()?).ok()
}
fn attr(n: Node<'_, '_>, name: &str) -> String {
    n.attribute(name).unwrap_or_default().to_owned()
}
pub(crate) fn properties(bytes: &[u8], all: bool) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let Some(doc) = parse(bytes) else {
        return out;
    };
    for section in doc
        .root_element()
        .children()
        .filter(|n| n.has_tag_name("propertySection"))
    {
        if !all && section.attribute("name") != Some("UserDefinedProperties") {
            continue;
        }
        for prop in section.children().filter(|n| n.has_tag_name("property")) {
            let name = attr(prop, "name");
            if name.is_empty() {
                continue;
            }
            let value = [
                "lpstr", "lpwstr", "i4", "i2", "ui4", "ui2", "r8", "bool", "date", "filetime",
            ]
            .iter()
            .find_map(|tag| {
                prop.children()
                    .find(|n| n.is_element() && n.tag_name().name() == *tag)
                    .map(|n| n.text().unwrap_or_default().trim().to_owned())
            })
            .unwrap_or_default();
            out.entry(name).or_insert(value);
        }
    }
    out
}
pub(crate) fn core(bytes: &[u8]) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let Some(doc) = parse(bytes) else {
        return out;
    };
    for (tag, key) in [
        ("creator", "Author"),
        ("lastModifiedBy", "LastSavedBy"),
        ("title", "Title"),
        ("subject", "Subject"),
        ("keywords", "Keywords"),
        ("description", "Comments"),
        ("created", "CreateDateTime"),
        ("modified", "LastSaveDateTime"),
    ] {
        if let Some(value) = doc
            .root_element()
            .children()
            .find(|n| n.is_element() && n.tag_name().name() == tag)
            .and_then(|n| n.text())
        {
            out.insert(key.into(), value.trim().into());
        }
    }
    out
}
pub(crate) fn root_attribute(bytes: &[u8], name: &str) -> String {
    parse(bytes)
        .map(|d| attr(d.root_element(), name))
        .unwrap_or_default()
}
fn numbers<const N: usize>(text: &str, comma: bool) -> Option<[f64; N]> {
    let values: Vec<f64> = text
        .split(|c: char| c.is_whitespace() || (comma && c == ','))
        .filter(|s| !s.is_empty())
        .map(str::parse)
        .collect::<Result<_, _>>()
        .ok()?;
    if values.iter().any(|v| !v.is_finite()) {
        return None;
    }
    values.try_into().ok()
}
pub(crate) fn mass(text: &str) -> Option<MassProperties> {
    let values: Vec<f64> = text
        .split(|c: char| c == ',' || c.is_whitespace())
        .filter(|s| !s.is_empty())
        .map(str::parse)
        .collect::<Result<_, _>>()
        .ok()?;
    if !(12..=14).contains(&values.len()) || values.iter().any(|v| !v.is_finite()) {
        return None;
    }
    Some(MassProperties {
        center_of_gravity: values[0..3].try_into().ok()?,
        volume: values[3],
        surface_area: values[4],
        mass: values[5],
        moments_of_inertia: values[6..9].try_into().ok()?,
        products_of_inertia: values[9..12].try_into().ok()?,
    })
}
#[derive(Default)]
pub(crate) struct Tree<'a> {
    doc: Option<Document<'a>>,
    pub names: BTreeMap<u32, String>,
}
pub(crate) fn tree(bytes: &[u8]) -> Tree<'_> {
    let doc = parse(bytes);
    let mut names = BTreeMap::new();
    if let Some(d) = &doc {
        for m in d.descendants().filter(|n| n.has_tag_name("swModel")) {
            if let (Some(id), Some(name)) = (
                m.attribute("swConfigurationId")
                    .and_then(|s| s.parse::<u32>().ok()),
                m.attribute("swConfigurationName").filter(|s| !s.is_empty()),
            ) {
                names.entry(id).or_insert(name.to_owned());
            }
        }
    }
    Tree { doc, names }
}
pub(crate) fn components(tree: &Tree<'_>, index: u32) -> Vec<Component> {
    let Some(doc) = &tree.doc else {
        return Vec::new();
    };
    let root = doc.root_element();
    let files: Vec<_> = root
        .children()
        .filter(|n| n.has_tag_name("swHeader"))
        .flat_map(|n| n.children())
        .filter(|n| n.has_tag_name("swFile"))
        .collect();
    let models: Vec<_> = root
        .children()
        .filter(|n| n.has_tag_name("swModelList"))
        .flat_map(|n| n.children())
        .filter(|n| n.has_tag_name("swModel"))
        .collect();
    let top = files.first().and_then(|n| n.attribute("id"));
    let Some(model) = models.iter().find(|m| {
        m.attribute("swConfigurationId")
            .and_then(|s| s.parse::<u32>().ok())
            == Some(index)
            && top.is_none_or(|t| m.attribute("swFileRef") == Some(t))
            && m.children().any(|n| n.has_tag_name("swReference"))
    }) else {
        return Vec::new();
    };
    model
        .children()
        .filter(|n| n.has_tag_name("swReference"))
        .map(|r| {
            let child = models.iter().find(|m| {
                m.attribute("id") == r.attribute("swModelRef")
                    && r.attribute("swModelRef").is_some()
            });
            let path = child
                .and_then(|m| m.attribute("swFileRef"))
                .and_then(|id| files.iter().find(|f| f.attribute("id") == Some(id)))
                .map(|f| attr(*f, "swPath"))
                .unwrap_or_default();
            let abbreviated = r.attribute("swSuppressed").is_none();
            let flag = |name| matches!(r.attribute(name), Some("YES" | "TRUE" | "yes" | "true"));
            let mut ref_config = attr(r, "swConfigurationName");
            if ref_config.is_empty() {
                ref_config = child
                    .map(|m| attr(*m, "swConfigurationName"))
                    .unwrap_or_default();
            }
            Component {
                name: attr(r, "swName"),
                path,
                ref_config,
                component_reference: attr(r, "swComponentReference"),
                is_suppressed: abbreviated || flag("swSuppressed"),
                is_hidden: abbreviated || flag("swHidden"),
                exclude_from_bom: abbreviated || flag("swExcludeFromBOM"),
                transform: r.attribute("swTransform").and_then(|s| numbers(s, false)),
            }
        })
        .collect()
}
