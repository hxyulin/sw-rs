// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
//! Configuration-local framing evidence, never an inferred archive dictionary.
use super::{NativeReferenceStop, declaration};
use crate::{BinaryReader, ByteSpan, FramedString};
use serde::Serialize;

/// Contiguous configuration framing supporting a reused handle-class candidate.
/// Tag equality does not establish the archive's serialization call sequence.
#[derive(Clone, Debug, Serialize)]
pub struct NativeHandleClassEvidence {
    /// Exact matching `Contents/Config-N` stream, not a model-header or other config.
    pub source_stream: String,
    /// Explicit schema-1 assembly root class at decoded offset zero.
    pub root_class: String,
    /// Checked moExtFeature/handle declarations, strings and intervening tag.
    /// This is a framing interval, not a complete serialized object boundary.
    pub frame_span: ByteSpan,
    /// Explicit schema-1 moCStringHandle_c declaration inside the frame.
    pub class_span: ByteSpan,
    /// Saved short class-reference tag between the strings, not an assigned slot.
    pub raw_tag: u16,
    /// Exact decoded-stream position of that tag.
    pub tag_offset: usize,
    /// Exact CString frame spans. Text is not used to infer owners or copied here.
    pub string_spans: [ByteSpan; 2],
}

/// Saved text following a reused handle tag, supported by configuration framing.
/// The handle class and archive continuity remain candidates, not decoded facts.
#[derive(Clone, Debug, Serialize)]
pub struct NativeDocumentReferenceCandidate {
    /// Exact mate-list or matching resolved-feature source stream.
    pub source_stream: String,
    /// Explicit face/edge root class. Face is omitted in JSON for compatibility.
    #[serde(skip_serializing_if = "super::is_face")]
    pub reference_class: String,
    /// Checked prefix framing through the candidate CString; not an object boundary.
    pub prefix_span: ByteSpan,
    /// Native index from the exact supported configuration stream name.
    pub configuration_index: u32,
    /// Opaque saved header words, without assigning CAD identities.
    pub raw_header: [u32; 5],
    /// Explicit entity-representation class preceding the unknown handle class.
    pub entity_class: String,
    /// Saved candidate text and its CString offset, without opening or rewriting it.
    pub document: FramedString,
    /// The class tag still requiring a verified archive context.
    pub unresolved_class: NativeReferenceStop,
    /// Unique matching configuration frame; not a recovered class dictionary.
    pub class_evidence: NativeHandleClassEvidence,
}

impl NativeHandleClassEvidence {
    pub(crate) fn scan_unique(bytes: &[u8], source_stream: &str) -> Option<Self> {
        let mut root = BinaryReader::new(bytes);
        let root_class = declaration(&mut root)
            .filter(|name| matches!(name.as_str(), "moAssembly_c" | "moRouteAssembly_c"))?;
        let marker = b"\xff\xff\x01\0\x0e\0moExtFeature_c";
        let mut frame = None;
        let mut seen = false;
        let mut end = 0;
        for (offset, window) in bytes.windows(marker.len()).enumerate() {
            if offset < end || window != marker {
                continue;
            }
            if seen {
                return None;
            }
            seen = true;
            frame = Self::at(bytes, source_stream, &root_class, offset, &mut end);
        }
        frame
    }

    fn at(
        bytes: &[u8],
        source_stream: &str,
        root_class: &str,
        offset: usize,
        protected_end: &mut usize,
    ) -> Option<Self> {
        let mut cursor = BinaryReader::at(bytes, offset)?;
        if declaration(&mut cursor)? != "moExtFeature_c" {
            return None;
        }
        let class_offset = cursor.position();
        if declaration(&mut cursor)? != "moCStringHandle_c" {
            return None;
        }
        let class_span = ByteSpan {
            offset: class_offset,
            length: cursor.position() - class_offset,
        };
        let first = string_span(&mut cursor, protected_end)?;
        let tag_offset = cursor.position();
        let raw_tag = cursor.u16()?;
        if !(0x8001..0xffff).contains(&raw_tag) {
            return None;
        }
        let second = string_span(&mut cursor, protected_end)?;
        Some(Self {
            source_stream: source_stream.into(),
            root_class: root_class.into(),
            frame_span: ByteSpan {
                offset,
                length: cursor.position() - offset,
            },
            class_span,
            raw_tag,
            tag_offset,
            string_spans: [first, second],
        })
    }
}

// Do not rescan declaration-looking bytes in a counted string, even when its
// UTF-16 is invalid or the later frame fails. Each string is checked at most once.
fn string_span(cursor: &mut BinaryReader<'_>, protected_end: &mut usize) -> Option<ByteSpan> {
    let offset = cursor.position();
    let raw = cursor.string_bytes()?;
    *protected_end = cursor.position();
    let units = raw
        .as_chunks::<2>()
        .0
        .iter()
        .map(|b| u16::from_le_bytes(*b));
    if char::decode_utf16(units).any(|c| c.is_err()) {
        return None;
    }
    Some(ByteSpan {
        offset,
        length: cursor.position() - offset,
    })
}
