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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeExternalObjectProfile {
    /// Two saved CStrings, 17 raw scalar bytes, a CString, then 12 raw bytes.
    TwoStrings,
    /// Three saved CStrings, 18 raw scalar bytes, a CString, then 16 raw bytes.
    ThreeStrings,
    /// Three saved CStrings, 18 raw scalar bytes, a CString, then 12 raw bytes.
    /// Observed in controlled SW2022 Part references; select from boundary evidence.
    ThreeStringsShortTail,
    /// Three saved CStrings, 17 raw scalar bytes, a CString, then 12 raw bytes.
    /// The shorter pre-configuration block has no inferred version threshold.
    ThreeStringsShortBlock,
}

/// Explicitly selected scalar trailer of the supported complete face-reference layout.
/// These profiles do not establish a document-version gate or apply to edge objects.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeFaceReferenceProfile {
    /// Three zero words following the surface array.
    ThreeWords,
    /// Two zero words, one saved raw word, then two zero words.
    FiveWords,
}

/// Explicit scalar trailer of the supported complete edge-reference layout.
/// Neither profile establishes a version gate or applies to other edge headers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeEdgeReferenceProfile {
    /// Two zero words following the surface array.
    TwoWords,
    /// Two zero words, one saved raw word, then a zero word.
    FourWords,
}

/// Explicit initial scalar framing of moHeader_c; no version threshold is inferred.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeModelHeaderProfile {
    /// The two string-array pointers immediately follow the root header.
    DirectArrays,
    /// One raw u32 precedes the two string-array pointers.
    LeadingWord,
}

/// Explicit scalar extension after the common complete model-header tail.
/// Sizes are observed layouts, not inferred document-version thresholds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeModelHeaderTailProfile {
    /// No extension; the established direct-array/two-string layout.
    NoExtension,
    /// Seven saved words, starting with one, then checked zero/one values.
    Extension28,
    /// Eight saved words, starting with one, then checked zero/one values.
    Extension32,
    /// Sixteen saved words, starting with two, then checked zero/one values.
    Extension64,
    /// Seventeen saved words, starting with zero, one or two, then zero/one values.
    Extension68,
}

/// Shared checked fields of a model-header prefix and a supported complete header.
#[derive(Clone, Debug, Serialize)]
pub struct NativeModelHeaderFields {
    /// Explicit initial framing profile.
    pub profile: NativeModelHeaderProfile,
    /// Optional saved scalar, retained without interpreting its bits.
    pub raw_header_word: Option<u32>,
    /// Two independently serialized su_CStringArray pointers, in saved order.
    pub string_arrays: [NativeArchivePointer; 2],
    /// Saved suObList of moLogs_c records.
    pub logs: NativeArchivePointer,
    /// Two scalar words between the log list and external-object list.
    pub raw_before_external: [u32; 2],
    /// Saved moExtObjectList_c, without assigning current dependencies.
    pub external_objects: NativeArchivePointer,
}

/// Remaining fields of a supported complete model-header layout.
#[derive(Clone, Debug, Serialize)]
pub struct NativeModelHeaderTail {
    /// Explicit scalar extension profile.
    pub profile: NativeModelHeaderTailProfile,
    /// Exact source interval after the shared prefix, including nested definitions.
    pub span: ByteSpan,
    /// Saved scalar before the following external-object pointer.
    pub raw_before_reference: u32,
    /// Following saved reference; no current document or occurrence is assigned.
    pub following_external_object: NativeArchivePointer,
    /// Forty-two scalar bytes before the trailing list pointer.
    pub raw_scalar_block: Vec<u8>,
    /// Supported trailing suObList pointer; only null/empty lists are decoded.
    pub saved_list: NativeArchivePointer,
    /// Three words before the optional block; the last is a checked 0/1 presence word.
    pub raw_optional_prefix: [u32; 3],
    /// Eighty saved bytes when the presence word is one; meanings remain unassigned.
    pub raw_optional_block: Option<Vec<u8>>,
    /// Three common final scalar words before any extension, with no flag meanings assigned.
    pub raw_final_words: [u32; 3],
    /// Extension words after the common final words; meanings remain unassigned.
    pub raw_extension_words: Vec<u32>,
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
    /// Runtime class established by the archive map with the supported saved schema.
    pub class_name: String,
    /// False for reference-array and model-header prefixes with unread parent fields.
    pub complete_payload: bool,
    /// Raw saved fields and pointer occurrences; no current ownership is inferred.
    pub payload: NativeSubtreePayload,
}

/// Supported reference-subtree fields; numeric words retain their native scope.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum NativeSubtreePayload {
    /// Sequential moHeader_c fields through its external-object list.
    /// Later header fields remain unread, so this definition is incomplete.
    ModelHeaderPrefix {
        /// Checked fields, flattened in JSON to preserve the prefix representation.
        #[serde(flatten)]
        header: Box<NativeModelHeaderFields>,
    },
    /// Complete supported header, including the following reference and selected tail.
    ModelHeader {
        /// Shared checked prefix fields.
        #[serde(flatten)]
        header: Box<NativeModelHeaderFields>,
        /// Checked tail fields and their exact source interval.
        tail: Box<NativeModelHeaderTail>,
    },
    /// Schema-zero su_CStringArray payload, preserving blanks and duplicate strings.
    StringArray {
        /// Counted UTF-16 CStrings in source order; no author/owner roles inferred.
        strings: Vec<FramedString>,
    },
    /// Saved suObList of log pointers, retaining nulls and repeated objects.
    LogList {
        /// Ordered moLogs_c pointer occurrences.
        records: Vec<NativeArchivePointer>,
    },
    /// A trailing suObList with a checked zero count and no element-type inference.
    EmptyObjectList,
    /// Saved moLogs_c payload, without joining its word to current features.
    LogRecord {
        /// Ordered moStamp_c pointers, retaining nulls and backreferences.
        stamps: Vec<NativeArchivePointer>,
        /// Saved scalar following the stamp collection.
        raw_word: u32,
        /// Exact saved record label.
        label: FramedString,
    },
    /// Saved moStamp_c bytes and text; timestamp/user semantics remain unassigned.
    Stamp {
        /// Ten fixed scalar bytes before the CString.
        raw_prefix: [u8; 10],
        /// Exact saved stamp label.
        label: FramedString,
    },
    /// Complete supported moExtObjectList_c collection.
    ExternalObjectList {
        /// Ordered external-object pointers, without resolving current owners.
        records: Vec<NativeArchivePointer>,
    },
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
        /// Saved leading scalar byte; supported values are 2, 3 and 5.
        raw_header_byte: u8,
        /// Saved 16-bit prefix word; supported values are zero and 0x0100.
        raw_prefix_word: u16,
        /// Raw word following the saved byte and 16-bit prefix word.
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
        /// Derived and linear-pattern representations contain a nested surface pointer.
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
    /// Complete supported moFaceRef_c payload, including its checked scalar trailer.
    FaceReference {
        /// Explicit trailer profile, without inferring a version threshold.
        profile: NativeFaceReferenceProfile,
        /// Five saved header words; this layout requires word three to equal 768.
        raw_header: [u32; 5],
        /// Surface pointer occurrences, retaining nulls and duplicates.
        records: Vec<NativeArchivePointer>,
        /// Exact three or five saved trailer words; the middle word remains raw.
        raw_tail: Vec<u32>,
    },
    /// Complete supported moEdgeRef_c payload, including its checked scalar trailer.
    EdgeReference {
        /// Explicit trailer profile, without inferring a version threshold.
        profile: NativeEdgeReferenceProfile,
        /// Five saved header words; this layout requires word three to equal 512.
        raw_header: [u32; 5],
        /// Surface pointer occurrences, retaining nulls and duplicates.
        records: Vec<NativeArchivePointer>,
        /// Exact two or four saved trailer words; the third word remains raw.
        raw_tail: Vec<u32>,
    },
}

#[derive(Clone, Copy)]
enum Role {
    HeaderPrefix(NativeModelHeaderProfile),
    Header(NativeModelHeaderProfile, NativeModelHeaderTailProfile),
    Strings,
    LogList,
    EmptyList,
    Log,
    Stamp,
    ExternalList,
    Array,
    Face(NativeFaceReferenceProfile),
    Edge(NativeEdgeReferenceProfile),
    Surface,
    Feature,
    External,
    Handle,
}
impl Role {
    fn accepts(self, name: &str) -> bool {
        match self {
            Self::HeaderPrefix(_) | Self::Header(_, _) => name == "moHeader_c",
            Self::Strings => name == "su_CStringArray",
            Self::LogList | Self::EmptyList => name == "suObList",
            Self::Log => name == "moLogs_c",
            Self::Stamp => name == "moStamp_c",
            Self::ExternalList => name == "moExtObjectList_c",
            Self::Array => matches!(name, "moFaceRef_c" | "moEdgeRef_c"),
            Self::Face(_) => name == "moFaceRef_c",
            Self::Edge(_) => name == "moEdgeRef_c",
            Self::Surface => matches!(
                name,
                "moSimpleSurfIdRep_c"
                    | "moFromSktEntSurfIdRep_c"
                    | "moFilletSurfIdRep_c"
                    | "moEndFaceSurfIdRep_c"
                    | "moDerivedSurfIdRep_c"
                    | "moEndFace3IntSurfIdRep_c"
                    | "moFromSktEnt3IntSurfIdRep_c"
                    | "moThinOffsetSurfIdRep_c"
                    | "moLPatternSurfIdRep_c"
            ),
            Self::Feature => name == "moFR_c",
            Self::External => name == "moExtObject_c",
            Self::Handle => name == "moCStringHandle_c",
        }
    }
    fn schema(self) -> u16 {
        if matches!(self, Self::Strings) { 0 } else { 1 }
    }
}

/// Sequential decoder for supported native model-header and reference profiles.
///
/// The caller must supply a genuine sequential archive context and exact cursor.
/// An empty context is valid only for a fresh archive. Scanned declarations,
/// candidate tag correlations and guessed initial slots must never seed this map.
/// Object backreferences require a completely decoded typed value in this reader.
/// Array-prefix reads leave the parent trailer unread. The separate face/edge reads
/// consume a selected supported trailer before advancing to a sibling. No complete
/// mate/feature stream walk is supplied.
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
    pub(crate) fn into_definitions(
        self,
    ) -> (Vec<ArchiveEntry>, BTreeMap<u32, NativeSubtreeObject>) {
        (self.archive.into_entries(), self.objects)
    }
    /// Read moHeader_c sequentially through its external-object list.
    /// Select direct string arrays or the optional leading scalar word explicitly.
    /// This supports the two schema-zero string arrays, log/stamp collections,
    /// two scalar words and external-object collection at the exact cursor.
    /// Later header fields remain unread and the root stays incomplete.
    /// Collections support short counts only; unknown layouts stop transactionally.
    pub fn read_model_header_prefix(
        &mut self,
        cursor: &mut BinaryReader<'_>,
        source_stream: &str,
        profile: NativeModelHeaderProfile,
    ) -> Result<NativeArchivePointer, ArchiveError> {
        self.read(cursor, source_stream, Role::HeaderPrefix(profile))
    }
    /// Read the complete observed DirectArrays/TwoStrings model-header layout.
    /// LeadingWord headers and nonempty trailing lists remain unsupported here.
    /// The optional 80-byte block remains raw; no ownership or version is inferred.
    pub fn read_model_header(
        &mut self,
        cursor: &mut BinaryReader<'_>,
        source_stream: &str,
        profile: NativeModelHeaderProfile,
    ) -> Result<NativeArchivePointer, ArchiveError> {
        self.read_model_header_with_tail(
            cursor,
            source_stream,
            profile,
            NativeModelHeaderTailProfile::NoExtension,
        )
    }
    /// Read a complete header with an explicitly selected scalar extension.
    /// Supported direct-array combinations are TwoStrings/NoExtension,
    /// ThreeStringsShortBlock/Extension28 and ThreeStringsShortTail/Extension32.
    /// Leading-word combinations are
    /// LeadingWord/ThreeStringsShortTail/Extension32 or Extension64, and
    /// LeadingWord/ThreeStrings/Extension68. Complete leading-word headers require
    /// a saved leading value of one. Unknown combinations and extension values stop.
    pub fn read_model_header_with_tail(
        &mut self,
        cursor: &mut BinaryReader<'_>,
        source_stream: &str,
        profile: NativeModelHeaderProfile,
        tail: NativeModelHeaderTailProfile,
    ) -> Result<NativeArchivePointer, ArchiveError> {
        let supported = matches!(
            (profile, self.profile, tail),
            (
                NativeModelHeaderProfile::DirectArrays,
                NativeExternalObjectProfile::TwoStrings,
                NativeModelHeaderTailProfile::NoExtension
            ) | (
                NativeModelHeaderProfile::DirectArrays,
                NativeExternalObjectProfile::ThreeStringsShortBlock,
                NativeModelHeaderTailProfile::Extension28
            ) | (
                NativeModelHeaderProfile::DirectArrays,
                NativeExternalObjectProfile::ThreeStringsShortTail,
                NativeModelHeaderTailProfile::Extension32
            ) | (
                NativeModelHeaderProfile::LeadingWord,
                NativeExternalObjectProfile::ThreeStringsShortTail,
                NativeModelHeaderTailProfile::Extension32
                    | NativeModelHeaderTailProfile::Extension64
            ) | (
                NativeModelHeaderProfile::LeadingWord,
                NativeExternalObjectProfile::ThreeStrings,
                NativeModelHeaderTailProfile::Extension68
            )
        );
        if !supported {
            return Err(error(
                cursor.position(),
                "unsupported complete model-header profile",
            ));
        }
        self.read(cursor, source_stream, Role::Header(profile, tail))
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
    /// Read a complete supported face reference with header word three equal to 768.
    /// Select the trailer from established enclosing-layout evidence. Neither profile
    /// applies to edge references or establishes configuration/topology ownership.
    pub fn read_face_reference(
        &mut self,
        cursor: &mut BinaryReader<'_>,
        source_stream: &str,
        profile: NativeFaceReferenceProfile,
    ) -> Result<NativeArchivePointer, ArchiveError> {
        self.read(cursor, source_stream, Role::Face(profile))
    }
    /// Read a complete supported edge reference with saved header word three equal to 512.
    /// Select the trailer from established enclosing-layout evidence. Other saved
    /// edge-header variants remain unsupported; no topology ownership is inferred.
    pub fn read_edge_reference(
        &mut self,
        cursor: &mut BinaryReader<'_>,
        source_stream: &str,
        profile: NativeEdgeReferenceProfile,
    ) -> Result<NativeArchivePointer, ArchiveError> {
        self.read(cursor, source_stream, Role::Edge(profile))
    }
    /// Read a supported surface, including recursive Derived and linear-pattern records.
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
            Some(ArchiveEntry::Class { name, schema, .. })
                if *schema == role.schema() && role.accepts(name) =>
            {
                name.clone()
            }
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
            let profile_matches = match role {
                Role::Header(profile, tail_profile) => matches!(
                    &self.objects[&index].payload,
                    NativeSubtreePayload::ModelHeader {header,tail}
                    if header.profile == profile && tail.profile == tail_profile
                ),
                Role::HeaderPrefix(profile) => match &self.objects[&index].payload {
                    NativeSubtreePayload::ModelHeaderPrefix { header }
                    | NativeSubtreePayload::ModelHeader { header, .. } => header.profile == profile,
                    _ => false,
                },
                Role::EmptyList => match &self.objects[&index].payload {
                    NativeSubtreePayload::EmptyObjectList => true,
                    NativeSubtreePayload::LogList { records } => records.is_empty(),
                    _ => false,
                },
                Role::Face(profile) => matches!(
                    &self.objects[&index].payload,
                    NativeSubtreePayload::FaceReference { profile: saved, .. } if *saved == profile
                ),
                Role::Edge(profile) => matches!(
                    &self.objects[&index].payload,
                    NativeSubtreePayload::EdgeReference { profile: saved, .. } if *saved == profile
                ),
                _ => true,
            };
            if !profile_matches {
                return Err(error(
                    offset,
                    "referenced object payload does not match the selected layout",
                ));
            }
        } else {
            let payload = self.payload(cursor, source, &class_name, role, depth)?;
            self.objects.insert(
                index,
                NativeSubtreeObject {
                    source_stream: source.into(),
                    span: ByteSpan {
                        offset,
                        length: cursor.position() - offset,
                    },
                    complete_payload: !matches!(role, Role::Array | Role::HeaderPrefix(_)),
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
        role: Role,
        depth: usize,
    ) -> Result<NativeSubtreePayload, ArchiveError> {
        match class {
            "moHeader_c" => {
                let profile = match role {
                    Role::HeaderPrefix(profile) | Role::Header(profile, _) => profile,
                    _ => return Err(error(cursor.position(), "unsupported model-header role")),
                };
                let header = Box::new(self.model_header_fields(
                    cursor,
                    source,
                    profile,
                    depth,
                    matches!(role, Role::Header(_, _)),
                )?);
                if matches!(role, Role::HeaderPrefix(_)) {
                    return Ok(NativeSubtreePayload::ModelHeaderPrefix { header });
                }
                let Role::Header(_, tail_profile) = role else {
                    return Err(error(
                        cursor.position(),
                        "unsupported complete model-header role",
                    ));
                };
                let at = cursor.position();
                let raw_before_reference = words::<1>(cursor)?[0];
                let following_external_object =
                    self.pointer(cursor, source, Role::External, depth + 1)?;
                let raw_scalar_block = raw_bytes(cursor, 42)?;
                let saved_list = self.pointer(cursor, source, Role::EmptyList, depth + 1)?;
                let optional_at = cursor.position();
                let raw_optional_prefix = words::<3>(cursor)?;
                let raw_optional_block = match raw_optional_prefix[2] {
                    0 => None,
                    1 => Some(raw_bytes(cursor, 80)?),
                    _ => {
                        return Err(error(
                            optional_at + 8,
                            "unsupported model-header optional block flag",
                        ));
                    }
                };
                let raw_final_words = words::<3>(cursor)?;
                let raw_extension_words = header_extension(cursor, tail_profile)?;
                let tail = Box::new(NativeModelHeaderTail {
                    profile: tail_profile,
                    span: ByteSpan {
                        offset: at,
                        length: cursor.position() - at,
                    },
                    raw_before_reference,
                    following_external_object,
                    raw_scalar_block,
                    saved_list,
                    raw_optional_prefix,
                    raw_optional_block,
                    raw_final_words,
                    raw_extension_words,
                });
                Ok(NativeSubtreePayload::ModelHeader { header, tail })
            }
            "su_CStringArray" => {
                let at = cursor.position();
                let count = short_count(cursor)?;
                if count > cursor.remaining().len() / 4 {
                    return Err(error(at, "string count exceeds remaining bytes"));
                }
                let strings = (0..count).map(|_| text(cursor)).collect::<Result<_, _>>()?;
                Ok(NativeSubtreePayload::StringArray { strings })
            }
            "suObList" => {
                if matches!(role, Role::EmptyList) {
                    let at = cursor.position();
                    if short_count(cursor)? != 0 {
                        return Err(error(
                            at,
                            "nonempty trailing model-header list is unsupported",
                        ));
                    }
                    return Ok(NativeSubtreePayload::EmptyObjectList);
                }
                Ok(NativeSubtreePayload::LogList {
                    records: self.pointer_array(cursor, source, Role::Log, depth + 1)?,
                })
            }
            "moLogs_c" => Ok(NativeSubtreePayload::LogRecord {
                stamps: self.pointer_array(cursor, source, Role::Stamp, depth + 1)?,
                raw_word: words::<1>(cursor)?[0],
                label: text(cursor)?,
            }),
            "moStamp_c" => {
                let at = cursor.position();
                let raw_prefix = cursor
                    .take(10)
                    .and_then(|bytes| bytes.try_into().ok())
                    .ok_or_else(|| error(at, "truncated stamp scalar prefix"))?;
                Ok(NativeSubtreePayload::Stamp {
                    raw_prefix,
                    label: text(cursor)?,
                })
            }
            "moExtObjectList_c" => Ok(NativeSubtreePayload::ExternalObjectList {
                records: self.pointer_array(cursor, source, Role::External, depth + 1)?,
            }),
            "moCStringHandle_c" => Ok(NativeSubtreePayload::StringHandle {
                text: text(cursor)?,
            }),
            "moExtObject_c" => {
                let handles = [
                    self.pointer(cursor, source, Role::Handle, depth + 1)?,
                    self.pointer(cursor, source, Role::Handle, depth + 1)?,
                ];
                let at = cursor.position();
                let raw_header_byte = cursor.take(1).map(|bytes| bytes[0]);
                let raw_prefix_word = cursor.u16();
                if !matches!(raw_header_byte, Some(2 | 3 | 5))
                    || !matches!(raw_prefix_word, Some(0 | 0x0100))
                {
                    return Err(error(at, "unsupported external-object scalar prefix"));
                }
                let raw_header_byte = raw_header_byte.expect("checked supported scalar byte");
                let raw_prefix_word = raw_prefix_word.expect("checked supported prefix word");
                let raw_header_word = words::<1>(cursor)?[0];
                let count = match self.profile {
                    NativeExternalObjectProfile::TwoStrings => 2,
                    NativeExternalObjectProfile::ThreeStrings
                    | NativeExternalObjectProfile::ThreeStringsShortTail
                    | NativeExternalObjectProfile::ThreeStringsShortBlock => 3,
                };
                let saved_strings = (0..count)
                    .map(|_| text(cursor))
                    .collect::<Result<Vec<_>, _>>()?;
                let (before, after) = match self.profile {
                    NativeExternalObjectProfile::TwoStrings => (17, 12),
                    NativeExternalObjectProfile::ThreeStrings => (18, 16),
                    NativeExternalObjectProfile::ThreeStringsShortTail => (18, 12),
                    NativeExternalObjectProfile::ThreeStringsShortBlock => (17, 12),
                };
                let raw_before_configuration = raw_bytes(cursor, before)?;
                let configuration_text = text(cursor)?;
                let raw_tail = raw_bytes(cursor, after)?;
                Ok(NativeSubtreePayload::ExternalObject {
                    profile: self.profile,
                    handles,
                    raw_header_byte,
                    raw_prefix_word,
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
                let required_header = match role {
                    Role::Face(_) => Some(768),
                    Role::Edge(_) => Some(512),
                    _ => None,
                };
                if required_header.is_some_and(|word| raw_header[3] != word) {
                    return Err(error(at + 12, "unsupported complete reference header"));
                }
                let at = cursor.position();
                if cursor.take(16) != Some(COMPONENT_MARKER.as_slice()) || cursor.u16() != Some(0) {
                    return Err(error(at, "unsupported reference-array marker"));
                }
                let count = usize::try_from(raw_header[2])
                    .map_err(|_| error(count_offset, "reference count exceeds platform range"))?;
                let records = self.pointer_list(
                    cursor,
                    source,
                    Role::Surface,
                    depth + 1,
                    count,
                    count_offset,
                )?;
                if let Role::Face(profile) = role {
                    let (count, raw_middle) = match profile {
                        NativeFaceReferenceProfile::ThreeWords => (3, false),
                        NativeFaceReferenceProfile::FiveWords => (5, true),
                    };
                    let raw_tail = reference_tail(cursor, count, raw_middle)?;
                    return Ok(NativeSubtreePayload::FaceReference {
                        profile,
                        raw_header,
                        records,
                        raw_tail,
                    });
                }
                if let Role::Edge(profile) = role {
                    let (count, raw_middle) = match profile {
                        NativeEdgeReferenceProfile::TwoWords => (2, false),
                        NativeEdgeReferenceProfile::FourWords => (4, true),
                    };
                    let raw_tail = reference_tail(cursor, count, raw_middle)?;
                    return Ok(NativeSubtreePayload::EdgeReference {
                        profile,
                        raw_header,
                        records,
                        raw_tail,
                    });
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
                let nested = if matches!(class, "moDerivedSurfIdRep_c" | "moLPatternSurfIdRep_c") {
                    Some(self.pointer(cursor, source, Role::Surface, depth + 1)?)
                } else {
                    None
                };
                let count = match class {
                    "moEndFace3IntSurfIdRep_c" | "moFromSktEnt3IntSurfIdRep_c" => 3,
                    "moEndFaceSurfIdRep_c" | "moLPatternSurfIdRep_c" => 2,
                    _ => 1,
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
    fn pointer_array(
        &mut self,
        cursor: &mut BinaryReader<'_>,
        source: &str,
        role: Role,
        depth: usize,
    ) -> Result<Vec<NativeArchivePointer>, ArchiveError> {
        let at = cursor.position();
        let count = short_count(cursor)?;
        self.pointer_list(cursor, source, role, depth, count, at)
    }
    fn model_header_fields(
        &mut self,
        cursor: &mut BinaryReader<'_>,
        source: &str,
        profile: NativeModelHeaderProfile,
        depth: usize,
        complete: bool,
    ) -> Result<NativeModelHeaderFields, ArchiveError> {
        let at = cursor.position();
        let raw_header_word = match profile {
            NativeModelHeaderProfile::DirectArrays => None,
            NativeModelHeaderProfile::LeadingWord => Some(words::<1>(cursor)?[0]),
        };
        if complete
            && profile == NativeModelHeaderProfile::LeadingWord
            && raw_header_word != Some(1)
        {
            return Err(error(at, "unsupported leading model-header word"));
        }
        Ok(NativeModelHeaderFields {
            profile,
            raw_header_word,
            string_arrays: [
                self.pointer(cursor, source, Role::Strings, depth + 1)?,
                self.pointer(cursor, source, Role::Strings, depth + 1)?,
            ],
            logs: self.pointer(cursor, source, Role::LogList, depth + 1)?,
            raw_before_external: words(cursor)?,
            external_objects: self.pointer(cursor, source, Role::ExternalList, depth + 1)?,
        })
    }
    fn pointer_list(
        &mut self,
        cursor: &mut BinaryReader<'_>,
        source: &str,
        role: Role,
        depth: usize,
        count: usize,
        count_offset: usize,
    ) -> Result<Vec<NativeArchivePointer>, ArchiveError> {
        if count > cursor.remaining().len() / 2
            || count > self.max_pointers.saturating_sub(self.pointers)
        {
            return Err(error(
                count_offset,
                "reference count exceeds remaining bytes or pointer budget",
            ));
        }
        (0..count)
            .map(|_| self.pointer(cursor, source, role, depth))
            .collect()
    }
}
fn short_count(cursor: &mut BinaryReader<'_>) -> Result<usize, ArchiveError> {
    let at = cursor.position();
    match cursor.u16() {
        Some(u16::MAX) => Err(error(
            at,
            "extended subtree collection count is unsupported",
        )),
        Some(count) => Ok(usize::from(count)),
        None => Err(error(at, "truncated subtree collection count")),
    }
}
fn header_extension(
    cursor: &mut BinaryReader<'_>,
    profile: NativeModelHeaderTailProfile,
) -> Result<Vec<u32>, ArchiveError> {
    let at = cursor.position();
    let count = match profile {
        NativeModelHeaderTailProfile::NoExtension => return Ok(Vec::new()),
        NativeModelHeaderTailProfile::Extension28 => 7,
        NativeModelHeaderTailProfile::Extension32 => 8,
        NativeModelHeaderTailProfile::Extension64 => 16,
        NativeModelHeaderTailProfile::Extension68 => 17,
    };
    let bytes = cursor
        .take(count * 4)
        .ok_or_else(|| error(at, "truncated model-header scalar extension"))?;
    let values: Vec<_> = bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|word| u32::from_le_bytes(*word))
        .collect();
    let first_valid = match profile {
        NativeModelHeaderTailProfile::Extension28 | NativeModelHeaderTailProfile::Extension32 => {
            values[0] == 1
        }
        NativeModelHeaderTailProfile::Extension64 => values[0] == 2,
        NativeModelHeaderTailProfile::Extension68 => values[0] <= 2,
        NativeModelHeaderTailProfile::NoExtension => false,
    };
    if !first_valid || values[1..].iter().any(|&word| word > 1) {
        return Err(error(
            at,
            "unsupported model-header scalar extension values",
        ));
    }
    Ok(values)
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

// Face and edge trailers share zero checks but differ in length. Keeping the
// checks together prevents accidentally consuming a face-sized tail for an edge.
fn reference_tail(
    cursor: &mut BinaryReader<'_>,
    count: usize,
    raw_middle: bool,
) -> Result<Vec<u32>, ArchiveError> {
    let at = cursor.position();
    let bytes = cursor
        .take(count * 4)
        .ok_or_else(|| error(at, "truncated reference scalar trailer"))?;
    let values: Vec<_> = bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|word| u32::from_le_bytes(*word))
        .collect();
    if values
        .iter()
        .enumerate()
        .any(|(index, &word)| word != 0 && !(raw_middle && index == 2))
    {
        return Err(error(at, "unsupported reference scalar trailer"));
    }
    Ok(values)
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
