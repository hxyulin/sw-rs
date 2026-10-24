// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
//! Structural XML inspection with offsets in original UTF-8/UTF-16 stream bytes.
use super::{ByteSpan, StreamInspection, XmlAttribute, XmlElement, XmlReference, XmlText};
use std::{
    borrow::Cow,
    collections::{BTreeMap, HashMap},
};
const SW_NAMESPACE: &str = "http://www.solidworks.com/sw2003/schema";
struct Source<'a> {
    text: Cow<'a, str>,
    start: usize,
    encoding: &'static str,
    checkpoints: Vec<(usize, usize)>,
}
impl<'a> Source<'a> {
    fn decode(bytes: &'a [u8], start: usize) -> Result<Self, String> {
        let payload = &bytes[start..];
        let endian = if payload.starts_with(&[255, 254]) {
            Some((true, 2))
        } else if payload.starts_with(&[254, 255]) {
            Some((false, 2))
        } else if payload.starts_with(b"<\0") {
            Some((true, 0))
        } else if payload.starts_with(b"\0<") {
            Some((false, 0))
        } else {
            None
        };
        if let Some((little, bom)) = endian {
            let raw = &payload[bom..];
            if !raw.len().is_multiple_of(2) {
                return Err("odd UTF-16 XML byte length".into());
            }
            let units = raw.as_chunks::<2>().0.iter().map(|b| {
                if little {
                    u16::from_le_bytes(*b)
                } else {
                    u16::from_be_bytes(*b)
                }
            });
            let text = char::decode_utf16(units)
                .collect::<Result<String, _>>()
                .map_err(|e| e.to_string())?;
            // Sparse checkpoints avoid a source-offset entry for every UTF-8 byte.
            let mut checkpoints = Vec::new();
            let mut source = start + bom;
            for (count, (offset, ch)) in text.char_indices().enumerate() {
                if count.is_multiple_of(128) {
                    checkpoints.push((offset, source));
                }
                source += ch.len_utf16() * 2;
            }
            checkpoints.push((text.len(), source));
            Ok(Self {
                text: Cow::Owned(text),
                start,
                encoding: if little { "utf16_le" } else { "utf16_be" },
                checkpoints,
            })
        } else {
            let text = std::str::from_utf8(payload).map_err(|e| e.to_string())?;
            Ok(Self {
                text: Cow::Borrowed(text),
                start,
                encoding: "utf8",
                checkpoints: Vec::new(),
            })
        }
    }
    fn offset(&self, utf8: usize) -> usize {
        if self.checkpoints.is_empty() {
            return self.start + utf8;
        }
        let index = self
            .checkpoints
            .partition_point(|(offset, _)| *offset <= utf8)
            - 1;
        let (offset, source) = self.checkpoints[index];
        source + self.text[offset..utf8].encode_utf16().count() * 2
    }
    fn span(&self, range: std::ops::Range<usize>) -> ByteSpan {
        ByteSpan::between(self.offset(range.start), self.offset(range.end))
    }
}
pub(super) fn inspect(name: &str, bytes: &[u8], out: &mut StreamInspection) -> bool {
    let hinted =
        name.ends_with(".xml") || name.ends_with(".rels") || name.starts_with("swXmlContents/");
    let candidate = hinted
        || bytes.starts_with(&[0xef, 0xbb, 0xbf])
        || bytes.starts_with(&[255, 254])
        || bytes.starts_with(&[254, 255])
        || bytes
            .iter()
            .copied()
            .find(|b| !b.is_ascii_whitespace())
            .is_some_and(|b| b == b'<');
    if !candidate {
        return false;
    }
    // Observed KeyWords wrapper: retain the leading byte as opaque; no flag/length
    // meaning is assigned to it. Do not search arbitrary binary data for XML.
    let start = usize::from(
        name == "swXmlContents/KeyWords" && bytes.get(1..).is_some_and(|b| b.starts_with(b"<?xml")),
    );
    let source = match Source::decode(bytes, start) {
        Ok(source) => source,
        Err(error) => {
            if hinted {
                out.structure_status = "invalid_xml";
                out.xml_error = Some(error);
            }
            return hinted;
        }
    };
    let doc = match roxmltree::Document::parse(&source.text) {
        Ok(doc) => doc,
        Err(error) => {
            if hinted {
                out.structure_status = "invalid_xml";
                out.xml_error = Some(error.to_string());
            }
            return hinted;
        }
    };
    out.structure_status = "xml";
    out.xml_encoding = Some(source.encoding);
    out.xml_payload_span = Some(ByteSpan::between(start, bytes.len()));
    out.opaque_prefix = (start > 0).then_some(ByteSpan::between(0, start));
    let mut parents = HashMap::new();
    let mut ids = BTreeMap::<String, Vec<usize>>::new();
    for node in doc.descendants().filter(|n| n.is_element()) {
        let index = out.xml_elements.len();
        let parent = node.parent().and_then(|p| parents.get(&p.id()).copied());
        parents.insert(node.id(), index);
        if node.tag_name().namespace() == Some(SW_NAMESPACE)
            && let Some(id) = node.attribute("id")
        {
            ids.entry(id.into()).or_default().push(index);
        }
        out.xml_elements.push(XmlElement {
            index,
            parent,
            name: node.tag_name().name().into(),
            namespace: node.tag_name().namespace().map(str::to_owned),
            span: source.span(node.range()),
            attributes: node
                .attributes()
                .map(|a| XmlAttribute {
                    name: a.name().into(),
                    namespace: a.namespace().map(str::to_owned),
                    value: a.value().into(),
                })
                .collect(),
        });
    }
    for node in doc.descendants().filter(|n| n.is_text()) {
        out.xml_text.push(XmlText {
            parent: node.parent().and_then(|p| parents.get(&p.id()).copied()),
            span: source.span(node.range()),
            value: node.text().unwrap_or_default().into(),
        });
    }
    for element in &out.xml_elements {
        if element.namespace.as_deref() != Some(SW_NAMESPACE) {
            continue;
        }
        for attr in element.attributes.iter().filter(|a| {
            a.namespace.is_none() && matches!(a.name.as_str(), "swFileRef" | "swModelRef")
        }) {
            out.xml_references.push(XmlReference {
                from_element: element.index,
                attribute: attr.name.clone(),
                reference: attr.value.clone(),
                candidate_elements: ids.get(&attr.value).cloned().unwrap_or_default(),
            });
        }
    }
    true
}
