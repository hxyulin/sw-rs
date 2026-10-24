// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
// Surface-reference grammar adapted from cadmpeg tessellation.rs at
// 4df21312c62cc1207a4c2274e29b4118fd232ec6; see NOTICE-cadmpeg.md.
//! Saved surface-reference tokens, without inferred B-Rep ownership or stability.
use crate::{BinaryReader, ByteSpan, FramedString};
use serde::Serialize;

/// Numeric identity carried by the supported saved surface-reference profile.
/// This is document/context local, not a SolidWorks API persistent-reference blob.
#[derive(Clone, Debug, Serialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SavedSurfaceIdentity {
    /// Positive feature source ID, excluding absent zero/FFFFFFFF markers.
    pub feature_source_id: u32,
    /// Surface ID local to that feature; zero is retained as a valid saved value.
    pub local_surface_id: u32,
    /// Ordered signed-i32 tail fields represented by their original u32 bit patterns.
    /// Empty and nonempty tails are distinct identities; never discard this suffix.
    pub trailing_fields: Vec<u32>,
}
/// Supported interpretation of one framed surface-reference string.
#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SurfaceReferenceStatus {
    /// Source/local IDs and every tail field fit the supported numeric grammar.
    Complete,
    /// Valid source/local IDs but an opaque tail; no complete face identity.
    OpaqueTail,
    /// Surface-looking record with absent/invalid required numeric fields.
    Invalid,
}
/// A recognized surface-reference string, including incomplete and invalid values.
#[derive(Clone, Debug, Serialize)]
pub struct SavedSurfaceReference {
    /// Original framed string and decoded-stream offset, without normalization.
    pub source: FramedString,
    /// Exact saved reference-class name; no class/object-map ID is inferred.
    pub class_name: String,
    /// Supported positive source ID, or None for missing/invalid/reserved values.
    pub feature_source_id: Option<u32>,
    /// Supported local surface number; independent of tessellation table order.
    pub local_surface_id: Option<u32>,
    /// Original comma-separated tail tokens, excluding one terminal empty token.
    pub tail_tokens: Vec<String>,
    /// Complete supported numeric tuple, never itself proof of an existing face.
    pub identity: Option<SavedSurfaceIdentity>,
    /// Whether the complete trimmed text ends with the native terminal comma.
    pub terminal_comma: bool,
    /// Explicit complete/incomplete/invalid interpretation.
    pub status: SurfaceReferenceStatus,
}
impl SavedSurfaceReference {
    /// Interpret a framed `mo*SurfIdRep_c` string while retaining its exact source.
    /// Unrecognized class families return None; malformed recognized records remain
    /// Invalid. Parsing a tuple does not establish its configuration or entity owner.
    ///
    /// ```
    /// let raw = sw_rs::FramedString {
    ///     value: "moSimpleSurfIdRep_c,7,2,-1,".into(), offset: 64,
    /// };
    /// let saved = sw_rs::SavedSurfaceReference::parse(&raw).unwrap();
    /// assert_eq!(saved.identity.unwrap().trailing_fields, [u32::MAX]);
    /// assert_eq!(saved.source.offset, 64);
    /// ```
    pub fn parse(source: &FramedString) -> Option<Self> {
        let text = source.value.trim();
        let mut fields = text.split(',');
        let class_name = fields.next()?;
        if !class_name.starts_with("mo") || !class_name.ends_with("SurfIdRep_c") {
            return None;
        }
        let feature_source_id = fields
            .next()
            .and_then(|v| v.parse::<u32>().ok())
            .filter(|&v| v != 0 && v != u32::MAX);
        let local_surface_id = fields.next().and_then(|v| v.parse::<u32>().ok());
        let mut tail_tokens = fields.map(str::to_owned).collect::<Vec<_>>();
        let terminal_comma = text.ends_with(',');
        if tail_tokens.last().is_some_and(|v| v.trim().is_empty()) {
            tail_tokens.pop();
        }
        let numbers = tail_tokens
            .iter()
            .map(|v| {
                v.parse::<i32>()
                    .map(|v| u32::from_ne_bytes(v.to_ne_bytes()))
            })
            .collect::<Result<Vec<_>, _>>()
            .ok();
        let (status, identity) = match (feature_source_id, local_surface_id, numbers) {
            (Some(feature_source_id), Some(local_surface_id), Some(trailing_fields)) => (
                SurfaceReferenceStatus::Complete,
                Some(SavedSurfaceIdentity {
                    feature_source_id,
                    local_surface_id,
                    trailing_fields,
                }),
            ),
            (Some(_), Some(_), None) => (SurfaceReferenceStatus::OpaqueTail, None),
            _ => (SurfaceReferenceStatus::Invalid, None),
        };
        Some(Self {
            source: source.clone(),
            class_name: class_name.into(),
            feature_source_id,
            local_surface_id,
            tail_tokens,
            identity,
            terminal_comma,
            status,
        })
    }
}

/// Agreement of duplicated saved references; no native face/body ownership is implied.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum SurfaceIdentityResolution {
    /// No supported surface-reference string was recovered; not proof of no identity.
    NotDecoded,
    /// At least one recognized string has invalid required fields.
    InvalidReference,
    /// At least one reference has an opaque suffix; complete resolution is blocked.
    OpaqueTail,
    /// Complete tuples disagree, including in tail length or tail values.
    Conflicting,
    /// Every reference carries the same complete numeric tuple.
    Consistent {
        /// Agreed tuple; still needs a unique native topology target and scope.
        identity: SavedSurfaceIdentity,
    },
}

/// References recovered within a validated cache table's adjacent metadata window.
#[derive(Clone, Debug, Serialize)]
pub struct CachedSurfaceReferences {
    /// Inspected decoded-stream window, not a validated serialized-object boundary.
    pub metadata_span: ByteSpan,
    /// Source-order references, retaining duplicates, opaque tails and invalid tokens.
    pub references: Vec<SavedSurfaceReference>,
}

/// One cache table and its independently requested surface-reference inspection.
#[derive(Clone, Debug, Serialize)]
pub struct CachedSurfaceRecord {
    /// Exact decoded carrier stream.
    pub source_stream: String,
    /// Descriptor-table byte offset in that carrier; not a native face ID.
    pub source_offset: usize,
    /// Bounded adjacent metadata evidence, including absent/invalid references.
    pub surface_references: CachedSurfaceReferences,
}

/// Opt-in cached surface evidence for one source and canonical configuration.
#[derive(Clone, Debug, Serialize)]
pub struct CachedSurfaceData {
    /// Versioned interchange schema.
    pub schema: &'static str,
    /// Original file SHA-256; protects graph composition from cross-file IDs.
    pub source_sha256: String,
    /// Native index of the geometry's single canonical configuration.
    pub configuration_index: u32,
    /// Exact saved configuration name, retaining empty/unknown values.
    pub configuration_name: String,
    /// Table-local references, without copying vertices or triangles.
    pub records: Vec<CachedSurfaceRecord>,
}
impl CachedSurfaceReferences {
    /// Compare all recovered tuples. Invalid or opaque records block complete agreement.
    /// Identical source/local pairs with different suffixes remain conflicting.
    pub fn identity_resolution(&self) -> SurfaceIdentityResolution {
        if self.references.is_empty() {
            return SurfaceIdentityResolution::NotDecoded;
        }
        if self
            .references
            .iter()
            .any(|r| r.status == SurfaceReferenceStatus::Invalid)
        {
            return SurfaceIdentityResolution::InvalidReference;
        }
        if self
            .references
            .iter()
            .any(|r| r.status == SurfaceReferenceStatus::OpaqueTail)
        {
            return SurfaceIdentityResolution::OpaqueTail;
        }
        let Some(first) = self.references.first().and_then(|r| r.identity.as_ref()) else {
            return SurfaceIdentityResolution::InvalidReference;
        };
        if self
            .references
            .iter()
            .all(|r| r.identity.as_ref() == Some(first))
        {
            SurfaceIdentityResolution::Consistent {
                identity: first.clone(),
            }
        } else {
            SurfaceIdentityResolution::Conflicting
        }
    }
    /// Common feature source ID only when every recovered reference agrees.
    /// Opaque tails may retain feature-level evidence, but invalid records block it.
    pub fn feature_source_id(&self) -> Option<u32> {
        let first = self.references.first()?.feature_source_id?;
        self.references
            .iter()
            .all(|r| {
                r.status != SurfaceReferenceStatus::Invalid && r.feature_source_id == Some(first)
            })
            .then_some(first)
    }
    pub(crate) fn inspect(bytes: &[u8], span: ByteSpan) -> Self {
        let mut out = Self {
            metadata_span: span,
            references: Vec::new(),
        };
        let Some(end) = span
            .offset
            .checked_add(span.length)
            .filter(|&end| end <= bytes.len())
        else {
            return out;
        };
        let bounded = &bytes[..end];
        let mut at = span.offset;
        while end.saturating_sub(at) >= 4 {
            let mut reader = BinaryReader::at(bounded, at).expect("bounded offset");
            if let Some(source) = reader.string() {
                if let Some(reference) = SavedSurfaceReference::parse(&source) {
                    out.references.push(reference);
                }
                at = reader.position();
            } else {
                at += 1;
            }
        }
        out
    }
}

#[cfg(test)]
mod tests;
