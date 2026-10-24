// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
//! Explicit native reference prefixes; no reconstructed archive map or ownership.
use crate::{BinaryReader, ByteSpan, FramedString};
use serde::Serialize;

const COMPONENT_MARKER: [u8; 16] = [
    0x7d, 0xc3, 0x94, 0x25, 0xad, 0x49, 0xb2, 0x54, 0x7d, 0xc3, 0x94, 0x25, 0xad, 0x49, 0xb2, 0x54,
];

/// Saved document-handle evidence from a contiguous, explicit native reference prefix.
/// A path can be stale, external or blank. It is not proof of the current owner.
#[derive(Clone, Debug, Serialize)]
pub struct NativeDocumentReference {
    /// Exact decoded stream containing the prefix.
    pub source_stream: String,
    /// Complete checked prefix through the document CString, not an object boundary.
    pub prefix_span: ByteSpan,
    /// Native Config-N index when the source is an exact mate-list stream.
    /// Visual-state configuration/display-state ownership remains unknown.
    pub configuration_index: Option<u32>,
    /// Five saved header words, without interpreting counts, stamps or object IDs.
    pub raw_header: [u32; 5],
    /// Explicit saved surface/edge representation class, not a resolved map slot.
    pub entity_class: String,
    /// Exact saved document-handle text and its CString offset. Blank stays unknown.
    pub document: FramedString,
}

fn declaration(reader: &mut BinaryReader<'_>) -> Option<String> {
    if reader.u16()? != 0xffff || reader.u16()? != 1 {
        return None;
    }
    let length = usize::from(reader.u16()?);
    let name = reader.take(length)?;
    if name.is_empty()
        || !name
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
    {
        return None;
    }
    String::from_utf8(name.to_vec()).ok()
}

impl NativeDocumentReference {
    /// Read the supported `moFaceRef_c` prefix from an exact decoded-stream offset.
    /// Every class in the chain must be explicitly declared with schema 1. Reused
    /// tags, unknown classes, malformed strings and incomplete prefixes return None.
    /// No guessed archive slots, object lengths or native entity IDs are assigned.
    pub fn parse_prefix(bytes: &[u8], source_stream: &str, offset: usize) -> Option<Self> {
        let mut reader = BinaryReader::at(bytes, offset)?;
        if declaration(&mut reader)? != "moFaceRef_c" {
            return None;
        }
        let raw_header = reader.words::<5>()?;
        if raw_header[..2] != [1, 0] || reader.take(16)? != COMPONENT_MARKER || reader.u16()? != 0 {
            return None;
        }
        let entity_class = declaration(&mut reader)?;
        if !entity_class.starts_with("mo")
            || !(entity_class.ends_with("SurfIdRep_c")
                || entity_class.ends_with("SurfIdRep2_c")
                || entity_class.ends_with("EdgeIdRep_c"))
            || reader.u16()? != 0
        {
            return None;
        }
        for expected in ["moFR_c", "moExtObject_c", "moCStringHandle_c"] {
            if declaration(&mut reader)? != expected {
                return None;
            }
        }
        let document = reader.string()?;
        Some(Self {
            source_stream: source_stream.into(),
            prefix_span: ByteSpan {
                offset,
                length: reader.position() - offset,
            },
            configuration_index: crate::mates::source_index(source_stream),
            raw_header,
            entity_class,
            document,
        })
    }
}

#[cfg(test)]
mod tests;
