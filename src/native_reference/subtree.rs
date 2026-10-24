// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
//! Sequential supported reference payloads. Never seed maps from marker scans.
use super::COMPONENT_MARKER;
use crate::{
    ArchiveContext, ArchiveEntry, ArchiveError, ArchiveObjectHeader, BinaryReader, ByteSpan,
    FramedString,
};
use serde::Serialize;
use std::collections::BTreeMap;

/// Observed external-object framing profile, selected explicitly by the caller.
/// No universal document-version threshold has been established for these layouts.
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeExternalObjectProfile {
    /// Two saved CStrings, 17 raw scalar bytes, a CString, then 12 raw bytes.
    TwoStrings,
    /// Three saved CStrings, 18 raw scalar bytes, a CString, then 16 raw bytes.
    ThreeStrings,
}

/// A pointer occurrence in a decoded stream, not a persistent CAD identity.
#[derive(Clone, Debug, Serialize)]
pub struct NativeArchivePointer {
    /// Exact source stream of this occurrence, including backreferences.
    pub source_stream: String,
    /// Checked shared-map pointer header; null pointers have no object index.
    pub header: ArchiveObjectHeader,
}

/// Checked object fields with their source span and explicit payload completeness.
#[derive(Clone, Debug, Serialize)]
pub struct NativeSubtreeObject {
    /// Source of the object's definition, which may precede a later backreference.
    pub source_stream: String,
    /// Header and checked fields, including nested objects. A prefix is incomplete.
    pub span: ByteSpan,
    /// Runtime class established by the actual archive map, with schema one.
    pub class_name: String,
    /// False for face/edge array prefixes whose parent trailer remains unread.
    pub complete_payload: bool,
    /// Raw saved fields and pointer occurrences; no current ownership is inferred.
    pub payload: NativeSubtreePayload,
}

/// Supported reference-subtree fields; numeric words retain their native scope.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum NativeSubtreePayload {
    /// A string-handle object's entire payload is one checked CString.
    StringHandle {
        /// Exact saved text and frame offset in the defining source stream.
        text: FramedString,
    },
    /// External-object fields, retaining both independent saved handle pointers.
    ExternalObject {
        /// Selected explicit framing profile, without inferring a version gate.
        profile: NativeExternalObjectProfile,
        /// Document/name handle positions; either pointer may be null.
        handles: [NativeArchivePointer; 2],
        /// Raw word following the supported byte(2), word(0) prefix.
        raw_header_word: u32,
        /// Two or three exact CStrings, including blank strings.
        saved_strings: Vec<FramedString>,
        /// Profile-sized raw scalar block before the next CString.
        raw_before_configuration: Vec<u8>,
        /// Saved configuration-looking text; not current configuration ownership.
        configuration_text: FramedString,
        /// Profile-sized raw trailing scalar block completing this payload.
        raw_tail: Vec<u8>,
    },
    /// moFR_c carries an external-object pointer followed by two saved words.
    FeatureReference {
        /// Pointer to an external object, not a current component assignment.
        external_object: NativeArchivePointer,
        /// Saved words, without promoting them to feature IDs or timestamps.
        raw_words: [u32; 2],
    },
    /// A supported leaf or recursive surface-representation payload.
    Surface {
        /// Saved feature-reference pointer; null remains unknown.
        feature_reference: NativeArchivePointer,
        /// Derived representations contain a nested surface pointer.
        nested: Option<NativeArchivePointer>,
        /// Class-specific trailing words, without topology interpretation.
        raw_tail: Vec<u32>,
    },
    /// Face/edge array prefix. Its parent-object trailer is not consumed.
    ReferenceArrayPrefix {
        /// Five saved words; word two is consumed as count in this explicit profile.
        raw_header: [u32; 5],
        /// All pointer occurrences in order, retaining nulls and duplicates.
        records: Vec<NativeArchivePointer>,
    },
}

#[derive(Clone, Copy)]
enum Role {
    Array,
    Surface,
    Feature,
    External,
    Handle,
}
impl Role {
    fn accepts(self, name: &str) -> bool {
        match self {
            Self::Array => matches!(name, "moFaceRef_c" | "moEdgeRef_c"),
            Self::Surface => matches!(
                name,
                "moSimpleSurfIdRep_c"
                    | "moFromSktEntSurfIdRep_c"
                    | "moFilletSurfIdRep_c"
                    | "moEndFaceSurfIdRep_c"
                    | "moDerivedSurfIdRep_c"
            ),
            Self::Feature => name == "moFR_c",
            Self::External => name == "moExtObject_c",
            Self::Handle => name == "moCStringHandle_c",
        }
    }
}

/// Sequential decoder for the supported native reference subtree profiles.
///
/// The caller must supply a genuine sequential archive context and exact cursor.
/// An empty context is valid only for a fresh archive. Scanned declarations,
/// candidate tag correlations and guessed initial slots must never seed this map.
/// Object backreferences require a completely decoded typed value in this reader.
/// Array reads consume prefixes only: callers must decode the parent trailer before
/// advancing to its next sibling. No complete mate/feature stream walk is supplied.
/// Unknown layouts, missing payloads and cycles stop instead of skipping bytes.
/// Each public read rolls back cursor, registrations, values and budgets on error.
#[derive(Debug)]
pub struct NativeSubtreeReader {
    archive: ArchiveContext,
    objects: BTreeMap<u32, NativeSubtreeObject>,
    profile: NativeExternalObjectProfile,
    pointers: usize,
    max_pointers: usize,
    max_depth: usize,
}
impl NativeSubtreeReader {
    /// Adopt a verified context. The default budgets are 100,000 pointers and depth 64.
    pub fn new(archive: ArchiveContext, profile: NativeExternalObjectProfile) -> Self {
        Self::with_limits(archive, profile, 100_000, 64)
    }
    /// Set cumulative pointer/depth budgets. Depth is always capped at 64.
    pub fn with_limits(
        archive: ArchiveContext,
        profile: NativeExternalObjectProfile,
        max_pointers: usize,
        max_depth: usize,
    ) -> Self {
        Self {
            archive,
            objects: BTreeMap::new(),
            profile,
            pointers: 0,
            max_pointers,
            max_depth: max_depth.min(64),
        }
    }
    /// Inspect the verified shared map, including registrations from successful reads.
    pub fn archive(&self) -> &ArchiveContext {
        &self.archive
    }
    /// Share this map with a caller's sequential decoders for other native classes.
    /// Only append operations whose position is established by the payload schema.
    /// Never replace this context, mix archives, or fill guessed missing slots.
    pub fn archive_mut(&mut self) -> &mut ArchiveContext {
        &mut self.archive
    }
    /// Read a standalone handle while walking an enclosing configuration payload.
    /// Its saved value is then available to later typed object backreferences.
    pub fn read_string_handle(
        &mut self,
        cursor: &mut BinaryReader<'_>,
        source_stream: &str,
    ) -> Result<NativeArchivePointer, ArchiveError> {
        self.read(cursor, source_stream, Role::Handle)
    }
    /// Borrow checked definitions by slot. Check complete_payload before using a prefix.
    pub fn objects(&self) -> &BTreeMap<u32, NativeSubtreeObject> {
        &self.objects
    }
    /// Read a face/edge reference-array prefix at the current cursor.
    /// The parent trailer remains unread; do not resume at a scanned later header.
    pub fn read_reference_array_prefix(
        &mut self,
        cursor: &mut BinaryReader<'_>,
        source_stream: &str,
    ) -> Result<NativeArchivePointer, ArchiveError> {
        self.read(cursor, source_stream, Role::Array)
    }
    /// Read one supported surface representation, including recursive Derived records.
    pub fn read_surface(
        &mut self,
        cursor: &mut BinaryReader<'_>,
        source_stream: &str,
    ) -> Result<NativeArchivePointer, ArchiveError> {
        self.read(cursor, source_stream, Role::Surface)
    }
    /// Read an external object and both handle pointers using the selected profile.
    pub fn read_external_object(
        &mut self,
        cursor: &mut BinaryReader<'_>,
        source_stream: &str,
    ) -> Result<NativeArchivePointer, ArchiveError> {
        self.read(cursor, source_stream, Role::External)
    }
    fn read(
        &mut self,
        cursor: &mut BinaryReader<'_>,
        source: &str,
        role: Role,
    ) -> Result<NativeArchivePointer, ArchiveError> {
        let saved = *cursor;
        let count = self.archive.entries().len();
        let pointers = self.pointers;
        let result = self.pointer(cursor, source, role, 0);
        if result.is_err() {
            *cursor = saved;
            self.archive.rollback(count);
            self.objects.retain(|&slot, _| slot as usize <= count);
            self.pointers = pointers;
        }
        result
    }
    fn pointer(
        &mut self,
        cursor: &mut BinaryReader<'_>,
        source: &str,
        role: Role,
        depth: usize,
    ) -> Result<NativeArchivePointer, ArchiveError> {
        let offset = cursor.position();
        if depth > self.max_depth {
            return Err(error(offset, "native subtree depth limit exceeded"));
        }
        if self.pointers >= self.max_pointers {
            return Err(error(offset, "native subtree pointer limit exceeded"));
        }
        self.pointers += 1;
        let header = self.archive.read_object(cursor)?;
        let Some(index) = header.object_index else {
            return Ok(NativeArchivePointer {
                source_stream: source.into(),
                header,
            });
        };
        let class_name = match header.class_index.and_then(|i| self.archive.entry(i)) {
            Some(ArchiveEntry::Class {
                name, schema: 1, ..
            }) if role.accepts(name) => name.clone(),
            _ => {
                return Err(error(
                    offset,
                    "unsupported subtree class, schema or pointer type",
                ));
            }
        };
        if !header.is_new {
            if !self
                .objects
                .get(&index)
                .is_some_and(|object| object.complete_payload)
            {
                return Err(error(
                    offset,
                    "referenced subtree payload is missing or still in progress",
                ));
            }
        } else {
            let payload = self.payload(cursor, source, &class_name, depth)?;
            self.objects.insert(
                index,
                NativeSubtreeObject {
                    source_stream: source.into(),
                    span: ByteSpan {
                        offset,
                        length: cursor.position() - offset,
                    },
                    complete_payload: !matches!(role, Role::Array),
                    class_name,
                    payload,
                },
            );
        }
        Ok(NativeArchivePointer {
            source_stream: source.into(),
            header,
        })
    }
    fn payload(
        &mut self,
        cursor: &mut BinaryReader<'_>,
        source: &str,
        class: &str,
        depth: usize,
    ) -> Result<NativeSubtreePayload, ArchiveError> {
        match class {
            "moCStringHandle_c" => Ok(NativeSubtreePayload::StringHandle {
                text: text(cursor)?,
            }),
            "moExtObject_c" => {
                let handles = [
                    self.pointer(cursor, source, Role::Handle, depth + 1)?,
                    self.pointer(cursor, source, Role::Handle, depth + 1)?,
                ];
                let at = cursor.position();
                if cursor.take(1) != Some([2].as_slice()) || cursor.u16() != Some(0) {
                    return Err(error(at, "unsupported external-object scalar prefix"));
                }
                let raw_header_word = words::<1>(cursor)?[0];
                let count = match self.profile {
                    NativeExternalObjectProfile::TwoStrings => 2,
                    NativeExternalObjectProfile::ThreeStrings => 3,
                };
                let saved_strings = (0..count)
                    .map(|_| text(cursor))
                    .collect::<Result<Vec<_>, _>>()?;
                let (before, after) = match self.profile {
                    NativeExternalObjectProfile::TwoStrings => (17, 12),
                    NativeExternalObjectProfile::ThreeStrings => (18, 16),
                };
                let raw_before_configuration = raw_bytes(cursor, before)?;
                let configuration_text = text(cursor)?;
                let raw_tail = raw_bytes(cursor, after)?;
                Ok(NativeSubtreePayload::ExternalObject {
                    profile: self.profile,
                    handles,
                    raw_header_word,
                    saved_strings,
                    raw_before_configuration,
                    configuration_text,
                    raw_tail,
                })
            }
            "moFR_c" => {
                let external_object = self.pointer(cursor, source, Role::External, depth + 1)?;
                Ok(NativeSubtreePayload::FeatureReference {
                    external_object,
                    raw_words: words(cursor)?,
                })
            }
            "moFaceRef_c" | "moEdgeRef_c" => {
                let at = cursor.position();
                let raw_header = words::<5>(cursor)?;
                let count_offset = at + 8;
                if raw_header[..2] != [1, 0] {
                    return Err(error(at, "unsupported reference-array header"));
                }
                let at = cursor.position();
                if cursor.take(16) != Some(COMPONENT_MARKER.as_slice()) || cursor.u16() != Some(0) {
                    return Err(error(at, "unsupported reference-array marker"));
                }
                let count = usize::try_from(raw_header[2])
                    .map_err(|_| error(count_offset, "reference count exceeds platform range"))?;
                if count > cursor.remaining().len() / 2
                    || count > self.max_pointers.saturating_sub(self.pointers)
                {
                    return Err(error(
                        count_offset,
                        "reference count exceeds remaining bytes or pointer budget",
                    ));
                }
                let mut records = Vec::new();
                for _ in 0..count {
                    records.push(self.pointer(cursor, source, Role::Surface, depth + 1)?);
                }
                Ok(NativeSubtreePayload::ReferenceArrayPrefix {
                    raw_header,
                    records,
                })
            }
            _ => {
                let at = cursor.position();
                if cursor.u16() != Some(0) {
                    return Err(error(at, "unsupported surface scalar prefix"));
                }
                let feature_reference = self.pointer(cursor, source, Role::Feature, depth + 1)?;
                let nested = if class == "moDerivedSurfIdRep_c" {
                    Some(self.pointer(cursor, source, Role::Surface, depth + 1)?)
                } else {
                    None
                };
                let count = if class == "moEndFaceSurfIdRep_c" {
                    2
                } else {
                    1
                };
                let mut raw_tail = Vec::new();
                for _ in 0..count {
                    raw_tail.push(words::<1>(cursor)?[0]);
                }
                Ok(NativeSubtreePayload::Surface {
                    feature_reference,
                    nested,
                    raw_tail,
                })
            }
        }
    }
}
fn error(offset: usize, message: &'static str) -> ArchiveError {
    ArchiveError { offset, message }
}
fn words<const N: usize>(cursor: &mut BinaryReader<'_>) -> Result<[u32; N], ArchiveError> {
    let at = cursor.position();
    cursor
        .words()
        .ok_or_else(|| error(at, "truncated subtree words"))
}
fn text(cursor: &mut BinaryReader<'_>) -> Result<FramedString, ArchiveError> {
    let at = cursor.position();
    cursor
        .string()
        .ok_or_else(|| error(at, "invalid or truncated subtree CString"))
}

#[cfg(test)]
mod tests;

fn raw_bytes(cursor: &mut BinaryReader<'_>, count: usize) -> Result<Vec<u8>, ArchiveError> {
    let at = cursor.position();
    cursor
        .take(count)
        .map(<[u8]>::to_vec)
        .ok_or_else(|| error(at, "truncated external-object scalar block"))
}
