// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
//! MFC archive-local class/object tags, not SolidWorks persistent topology IDs.
//! Independently implemented from Microsoft's TN002 and CArchive API contracts.
use crate::{BinaryReader, ByteSpan};
use serde::Serialize;

/// One shared class/object map slot; numeric identities are archive-local.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ArchiveEntry {
    /// Runtime-class declaration occupying one map slot.
    Class {
        /// Saved class schema, not the model version.
        schema: u16,
        /// Saved ASCII runtime-class name.
        name: String,
        /// Class declaration bytes in the cursor's source stream.
        source: ByteSpan,
    },
    /// Object registered before its payload is decoded, allowing cyclic references.
    Object {
        /// Slot of its declared runtime class.
        class_index: u32,
        /// Object header bytes; not a complete object boundary.
        source: ByteSpan,
    },
    /// Caller-mapped external object, occupying a slot without serialized bytes.
    ExternalObject,
}

/// Decoded object header; its class-specific payload must be read separately.
/// Obtain headers through [`ArchiveContext::read_object`]; pointer state is read-only.
/// JSON retains the `object_index`, `class_index`, `is_new` and `source` fields.
#[derive(Clone, Debug)]
pub struct ArchiveObjectHeader {
    pointer: ObjectPointer,
    /// Exact tag/declaration span; never an inferred payload boundary.
    pub source: ByteSpan,
}
#[derive(Clone, Copy, Debug)]
enum ObjectPointer {
    Null,
    External { object_index: u32 },
    Reference { object_index: u32, class_index: u32 },
    New { object_index: u32, class_index: u32 },
}
impl ArchiveObjectHeader {
    /// Map slot of this object, or None for a null pointer.
    pub fn object_index(&self) -> Option<u32> {
        match self.pointer {
            ObjectPointer::Null => None,
            ObjectPointer::External { object_index }
            | ObjectPointer::Reference { object_index, .. }
            | ObjectPointer::New { object_index, .. } => Some(object_index),
        }
    }
    /// Runtime-class slot, unknown for null and externally mapped objects.
    pub fn class_index(&self) -> Option<u32> {
        match self.pointer {
            ObjectPointer::Null | ObjectPointer::External { .. } => None,
            ObjectPointer::Reference { class_index, .. }
            | ObjectPointer::New { class_index, .. } => Some(class_index),
        }
    }
    /// Whether a newly registered object's payload follows this header.
    pub fn is_new(&self) -> bool {
        matches!(self.pointer, ObjectPointer::New { .. })
    }
}
impl Serialize for ArchiveObjectHeader {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut header = serializer.serialize_struct("ArchiveObjectHeader", 4)?;
        header.serialize_field("object_index", &self.object_index())?;
        header.serialize_field("class_index", &self.class_index())?;
        header.serialize_field("is_new", &self.is_new())?;
        header.serialize_field("source", &self.source)?;
        header.end()
    }
}

/// Invalid, missing or unsupported archive framing, with the failed header offset.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArchiveError {
    /// Cursor position before the failed operation; cursor/map remain unchanged.
    pub offset: usize,
    /// Concrete framing/map failure.
    pub message: &'static str,
}
impl std::fmt::Display for ArchiveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} at decoded byte {}", self.message, self.offset)
    }
}
impl std::error::Error for ArchiveError {}

/// Shared archive-local map for sequential class-specific decoding.
///
/// Slot zero is null; classes and objects share the subsequent slots. Callers must
/// read every intervening payload according to its actual schema. Never use this
/// map on scanned markers or resume after skipping an unknown payload: omitted
/// nested objects/classes would invalidate all subsequent slot assignments.
///
/// ```
/// // Two empty-payload objects of the same class, followed by a root backreference.
/// let bytes = [255, 255, 1, 0, 4, 0, b'N', b'o', b'd', b'e', 1, 128, 2, 0];
/// let mut cursor = sw_rs::BinaryReader::new(&bytes);
/// let mut context = sw_rs::ArchiveContext::new();
/// let root = context.read_object(&mut cursor)?;
/// assert_eq!(root.object_index(), Some(2)); // class occupies slot 1
/// let other = context.read_object(&mut cursor)?;
/// assert_eq!(other.object_index(), Some(3));
/// let reference = context.read_object(&mut cursor)?;
/// assert_eq!(reference.object_index(), root.object_index());
/// assert!(!reference.is_new()); // no payload follows a backreference
/// # Ok::<(), sw_rs::ArchiveError>(())
/// ```
#[derive(Debug, Default)]
pub struct ArchiveContext {
    entries: Vec<ArchiveEntry>,
}
enum Tag {
    Null,
    Object(u32),
    Class(u32),
    NewClass { schema: u16, name: String },
}
impl ArchiveContext {
    /// Create an empty archive context; do not reuse it across unrelated archives.
    pub fn new() -> Self {
        Self::default()
    }
    /// Inspect a nonzero map slot without treating object slots as classes.
    pub fn entry(&self, index: u32) -> Option<&ArchiveEntry> {
        self.entries
            .get(usize::try_from(index.checked_sub(1)?).ok()?)
    }
    /// All registered slots in order; slice index zero corresponds to map slot one.
    pub fn entries(&self) -> &[ArchiveEntry] {
        &self.entries
    }
    pub(crate) fn into_entries(self) -> Vec<ArchiveEntry> {
        self.entries
    }
    // Subtree readers can roll back registrations after a class-specific failure.
    pub(crate) fn rollback(&mut self, count: usize) {
        self.entries.truncate(count);
    }
    fn push(&mut self, entry: ArchiveEntry) -> u32 {
        self.entries.push(entry);
        self.entries.len() as u32
    }
    /// Mirror an explicit MapObject call for an externally owned object.
    /// Only use when its position in the serialized call sequence is known.
    /// Errors use offset zero because this operation has no source cursor.
    ///
    /// # Errors
    /// Returns an error if another slot would exceed the MFC archive index range.
    pub fn map_external_object(&mut self) -> Result<u32, ArchiveError> {
        self.capacity(1, 0)?;
        Ok(self.push(ArchiveEntry::ExternalObject))
    }
    fn capacity(&self, count: usize, offset: usize) -> Result<(), ArchiveError> {
        if self.entries.len().saturating_add(count) > 0x3fff_fffe {
            Err(ArchiveError {
                offset,
                message: "archive map index exceeds MFC format range",
            })
        } else {
            Ok(())
        }
    }
    fn tag(reader: &mut BinaryReader<'_>) -> Option<Tag> {
        let word = reader.u16()?;
        if word == 0xffff {
            let schema = reader.u16()?;
            let length = usize::from(reader.u16()?);
            let name = reader.take(length)?;
            if name.is_empty() || !name.iter().all(|b| (0x21..=0x7e).contains(b)) {
                return None;
            }
            return Some(Tag::NewClass {
                schema,
                name: std::str::from_utf8(name).ok()?.into(),
            });
        }
        let (index, class) = if word == 0x7fff {
            let value = reader.u32()?;
            let index = value & 0x7fff_ffff;
            if !(0x7fff..=0x3fff_fffe).contains(&index) {
                return None;
            }
            (index, value & 0x8000_0000 != 0)
        } else {
            (u32::from(word & 0x7fff), word & 0x8000 != 0)
        };
        if class {
            (index != 0).then_some(Tag::Class(index))
        } else if index == 0 {
            Some(Tag::Null)
        } else {
            Some(Tag::Object(index))
        }
    }
    /// Read a ReadClass/SerializeClass header without allocating an object slot.
    /// Payload/base-class serialization remains the caller's responsibility.
    ///
    /// # Errors
    /// Returns an error for truncated or invalid tags, missing or non-class slots,
    /// or an exhausted archive index range. The cursor and map remain unchanged.
    pub fn read_class(&mut self, reader: &mut BinaryReader<'_>) -> Result<u32, ArchiveError> {
        let offset = reader.position();
        let error = |message| ArchiveError { offset, message };
        let mut trial = *reader;
        let tag =
            Self::tag(&mut trial).ok_or_else(|| error("invalid or truncated archive class tag"))?;
        let index = match tag {
            Tag::NewClass { schema, name } => {
                self.capacity(1, offset)?;
                self.push(ArchiveEntry::Class {
                    schema,
                    name,
                    source: ByteSpan {
                        offset,
                        length: trial.position() - offset,
                    },
                })
            }
            Tag::Class(index) if matches!(self.entry(index), Some(ArchiveEntry::Class { .. })) => {
                index
            }
            _ => return Err(error("class tag refers to missing, null or non-class slot")),
        };
        *reader = trial;
        Ok(index)
    }
    /// Read an object pointer header and register new objects before their payload.
    /// Null/backreferences have no payload; new objects require schema-specific
    /// decoding before reading another header. Failure rolls back both cursor/map.
    ///
    /// # Errors
    /// Returns an error for truncated or invalid tags, missing or mismatched map
    /// slots, or an exhausted archive index range.
    pub fn read_object(
        &mut self,
        reader: &mut BinaryReader<'_>,
    ) -> Result<ArchiveObjectHeader, ArchiveError> {
        let offset = reader.position();
        let error = |message| ArchiveError { offset, message };
        let mut trial = *reader;
        let tag = Self::tag(&mut trial)
            .ok_or_else(|| error("invalid or truncated archive object tag"))?;
        let source = ByteSpan {
            offset,
            length: trial.position() - offset,
        };
        let pointer = match tag {
            Tag::Null => ObjectPointer::Null,
            Tag::Object(index) => match self.entry(index) {
                Some(ArchiveEntry::Object { class_index, .. }) => ObjectPointer::Reference {
                    object_index: index,
                    class_index: *class_index,
                },
                Some(ArchiveEntry::ExternalObject) => ObjectPointer::External {
                    object_index: index,
                },
                _ => return Err(error("object tag refers to missing or class slot")),
            },
            Tag::Class(index) => {
                if !matches!(self.entry(index), Some(ArchiveEntry::Class { .. })) {
                    return Err(error("reused class refers to missing or object slot"));
                }
                self.capacity(1, offset)?;
                let object = self.push(ArchiveEntry::Object {
                    class_index: index,
                    source,
                });
                ObjectPointer::New {
                    object_index: object,
                    class_index: index,
                }
            }
            Tag::NewClass { schema, name } => {
                self.capacity(2, offset)?;
                let class = self.push(ArchiveEntry::Class {
                    schema,
                    name,
                    source,
                });
                let object = self.push(ArchiveEntry::Object {
                    class_index: class,
                    source,
                });
                ObjectPointer::New {
                    object_index: object,
                    class_index: class,
                }
            }
        };
        *reader = trial;
        Ok(ArchiveObjectHeader { pointer, source })
    }
}

#[cfg(test)]
mod tests;
