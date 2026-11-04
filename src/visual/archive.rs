// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
//! Sequential visual archive profile. Unknown payloads stop map interpretation.
use super::*;
use crate::{
    ArchiveContext, ArchiveEntry, ArchiveError, NativeExternalObjectProfile,
    NativeFaceReferenceProfile, NativeSubtreeObject, NativeSubtreeReader,
};

/// Explicit display-state key row, independent of its human-readable prefix.
#[derive(Clone, Debug, Serialize)]
#[non_exhaustive]
pub struct VisualArchiveKey {
    /// Archive-local state identifier.
    pub id: u32,
    /// Exact saved text and frame offset; names can predate configuration renames.
    pub key: NativeString,
    /// Saved configuration index; absent configurations remain unresolved.
    pub configuration_index: u32,
    /// Two uninterpreted flags following the configuration index.
    pub raw_flags: [u32; 2],
}
/// One component's property link for an explicit saved display-state ID.
#[derive(Clone, Debug, Serialize)]
#[non_exhaustive]
pub struct VisualArchiveState {
    /// Identifier in the key table, not a list position.
    pub key_id: u32,
    /// Property object slot, or None for an explicitly null/absent property.
    pub property_object: Option<u32>,
}
/// Sequentially decoded visual owner, without a native topology assignment.
#[derive(Clone, Debug, Serialize)]
#[non_exhaustive]
pub struct VisualArchiveComponent {
    /// Archive-local object slot.
    pub object_index: u32,
    /// Exact runtime class name.
    pub class_name: String,
    /// Saved feature source ID and timestamp for moCompFeature_c.
    pub source_identity: Option<[u32; 2]>,
    /// Saved native face-reference object for body/face owners.
    pub reference_object: Option<u32>,
    /// Class-specific raw owner words following the native reference.
    pub raw_owner_tail: Vec<u32>,
    /// Common component bytes, retained without guessed field semantics.
    pub raw_common: Vec<u8>,
    /// Explicit state/property links, including null property links.
    pub states: Vec<VisualArchiveState>,
}
/// First unsupported sequential payload; later markers are never used to resume.
#[derive(Clone, Debug, Serialize)]
#[non_exhaustive]
pub struct VisualArchiveFailure {
    /// Decoded stream offset at which decoding stopped.
    pub offset: usize,
    /// Concrete missing payload or framing check.
    pub message: String,
}
/// Checked visual archive map and owner/state relationships for one stream.
#[derive(Clone, Debug, Default, Serialize)]
#[non_exhaustive]
pub struct VisualArchive {
    /// True only after every root payload and the supported trailer were consumed.
    pub complete: bool,
    /// Raw key-table header words.
    pub raw_key_header: [u32; 2],
    /// Configuration index, selected state ID, and an uninterpreted word.
    pub configuration_rows: Vec<[u32; 3]>,
    /// Key records in saved order.
    pub keys: Vec<VisualArchiveKey>,
    /// Complete component payloads, never resumed after a failure.
    pub components: Vec<VisualArchiveComponent>,
    /// Complete property payloads keyed by archive object slot.
    pub properties: BTreeMap<u32, NativeAppearance>,
    /// Shared class/object map in exact registration order.
    pub map: Vec<ArchiveEntry>,
    /// Complete native reference payloads sharing this exact object map.
    pub reference_objects: BTreeMap<u32, NativeSubtreeObject>,
    /// Trailing saved labels and their existing object references.
    pub clients: Vec<(NativeString, Option<u32>)>,
    /// First unsupported payload or malformed record.
    pub failure: Option<VisualArchiveFailure>,
}
#[derive(Clone, Copy)]
enum Object {
    Key,
    Holder,
    Component,
    Property,
    PropertyAttribute(Option<u32>),
    StateAttribute,
    BaseComponent,
}
struct Walker<'a> {
    cursor: Reader<'a>,
    subtree: NativeSubtreeReader,
    objects: BTreeMap<u32, Object>,
    report: VisualArchive,
    pointers: usize,
}
impl<'a> Walker<'a> {
    fn error(&self, message: &str) -> VisualArchiveFailure {
        VisualArchiveFailure {
            offset: self.cursor.at,
            message: message.into(),
        }
    }
    fn checked<T>(&self, value: Option<T>) -> Result<T, VisualArchiveFailure> {
        value.ok_or_else(|| self.error("truncated or unsupported visual payload"))
    }
    fn word(&mut self) -> Result<u32, VisualArchiveFailure> {
        let value = self.cursor.u32();
        self.checked(value)
    }
    fn count(&mut self) -> Result<usize, VisualArchiveFailure> {
        let n = self.word()? as usize;
        if n > 100_000 || n > self.cursor.remaining().len() / 2 {
            return Err(self.error("visual count exceeds remaining bytes or profile limit"));
        }
        Ok(n)
    }
    fn text(&mut self) -> Result<NativeString, VisualArchiveFailure> {
        let value = self.cursor.string();
        self.checked(value)
    }
    fn bytes(&mut self, n: usize) -> Result<&'a [u8], VisualArchiveFailure> {
        let value = self.cursor.take(n);
        self.checked(value)
    }
    fn archive_error(error: ArchiveError) -> VisualArchiveFailure {
        VisualArchiveFailure {
            offset: error.offset,
            message: error.message.into(),
        }
    }
    fn object(&mut self, depth: usize) -> Result<Option<u32>, VisualArchiveFailure> {
        if depth > 64 || self.pointers >= 100_000 {
            return Err(self.error("visual pointer/depth limit exceeded"));
        }
        self.pointers += 1;
        let header = self
            .subtree
            .archive_mut()
            .read_object(&mut self.cursor)
            .map_err(Self::archive_error)?;
        let Some(index) = header.object_index() else {
            return Ok(None);
        };
        let name = match header
            .class_index()
            .and_then(|i| self.subtree.archive().entry(i))
        {
            Some(ArchiveEntry::Class {
                name, schema: 1, ..
            }) => name.clone(),
            _ => return Err(self.error("unsupported visual class schema")),
        };
        if !header.is_new() {
            if !self.objects.contains_key(&index) {
                return Err(self.error("visual reference points to an incomplete payload"));
            }
            return Ok(Some(index));
        }
        let object = match name.as_str() {
            "visStateKeyData_c" => {
                self.report.raw_key_header = [self.word()?, self.word()?];
                if self.report.raw_key_header[0] != 5 || self.report.raw_key_header[1] > 1 {
                    return Err(self.error("unsupported key-table header"));
                }
                let n = self.count()?;
                for _ in 0..n {
                    let row = [self.word()?, self.word()?, self.word()?];
                    if self
                        .report
                        .configuration_rows
                        .iter()
                        .any(|r| r[0] == row[0])
                    {
                        return Err(self.error("duplicate visual configuration index"));
                    }
                    self.report.configuration_rows.push(row);
                }
                let n = self.count()?;
                for _ in 0..n {
                    let id = self.word()?;
                    if id == 0 || self.report.keys.iter().any(|k| k.id == id) {
                        return Err(self.error("duplicate or zero visual state ID"));
                    }
                    let key = self.text()?;
                    let configuration_index = self.word()?;
                    let raw_flags = [self.word()?, self.word()?];
                    self.report.keys.push(VisualArchiveKey {
                        id,
                        key,
                        configuration_index,
                        raw_flags,
                    });
                }
                Object::Key
            }
            "moCompHolder_c" => {
                let nested = self.object(depth + 1)?;
                if !matches!(
                    nested.and_then(|i| self.objects.get(&i)),
                    Some(Object::Component)
                ) {
                    return Err(self.error("holder does not contain a supported component"));
                }
                Object::Holder
            }
            "moUnitComponent_c" | "moCompFeature_c" | "moCompSolidBody_c" | "moCompFace_c" => {
                let base = if name != "moUnitComponent_c" {
                    let header = self
                        .subtree
                        .archive_mut()
                        .read_object(&mut self.cursor)
                        .map_err(Self::archive_error)?;
                    if !header.is_new()
                        || !matches!(header.class_index().and_then(|i| self.subtree.archive().entry(i)), Some(ArchiveEntry::Class { name, schema: 1, .. }) if name == "moUnitComponent_c")
                    {
                        return Err(self.error("unsupported visual base-component object"));
                    }
                    header.object_index()
                } else {
                    None
                };
                let raw_common = self.bytes(79)?.to_vec();
                if self.word()? != 18000 {
                    return Err(self.error("unsupported component payload version"));
                }
                if let Some(index) = base {
                    self.objects.insert(index, Object::BaseComponent);
                }
                let source_identity = if name == "moCompFeature_c" {
                    Some([self.word()?, self.word()?])
                } else {
                    None
                };
                let mut raw_owner_tail = Vec::new();
                let reference_object =
                    if matches!(name.as_str(), "moCompSolidBody_c" | "moCompFace_c") {
                        if name == "moCompFace_c" && self.bytes(1)? != [0] {
                            return Err(self.error("unsupported face-owner flag"));
                        }
                        let pointer = self
                            .subtree
                            .read_face_reference(
                                &mut self.cursor,
                                "ThirdPtyStore/VisualStates",
                                NativeFaceReferenceProfile::VisualStatesTwoWords,
                            )
                            .map_err(Self::archive_error)?;
                        let n = if name == "moCompSolidBody_c" { 8 } else { 3 };
                        for _ in 0..n {
                            raw_owner_tail.push(self.word()?);
                        }
                        if !matches!(raw_owner_tail[0], 13000 | 18000) {
                            return Err(self.error("unsupported derived component version"));
                        }
                        if name == "moCompFace_c" && self.bytes(2)? != [0; 2] {
                            return Err(self.error("unsupported face-owner trailer"));
                        }
                        pointer.header.object_index()
                    } else {
                        None
                    };
                let n = self.count()?;
                let mut states = Vec::new();
                for _ in 0..n {
                    let key_id = self.word()?;
                    if !self.report.keys.iter().any(|k| k.id == key_id)
                        || states
                            .iter()
                            .any(|s: &VisualArchiveState| s.key_id == key_id)
                    {
                        return Err(self.error("unknown or duplicate component state ID"));
                    }
                    let count = self.count()?;
                    let mut property_object = None;
                    let mut property_attribute = false;
                    for _ in 0..count {
                        let attribute = self.object(depth + 1)?;
                        match attribute.and_then(|i| self.objects.get(&i)) {
                            Some(Object::PropertyAttribute(property)) if !property_attribute => {
                                property_object = *property;
                                property_attribute = true;
                            }
                            Some(Object::StateAttribute) => {}
                            _ => return Err(self.error("unknown or duplicate component attribute")),
                        }
                    }
                    states.push(VisualArchiveState {
                        key_id,
                        property_object,
                    });
                }
                self.report.components.push(VisualArchiveComponent {
                    object_index: index,
                    class_name: name,
                    source_identity,
                    reference_object,
                    raw_owner_tail,
                    raw_common,
                    states,
                });
                Object::Component
            }
            "visStateVisualPropertiesAttribute_c" => {
                if self.word()? != 1 {
                    return Err(self.error("unsupported property attribute payload"));
                }
                let property = self.object(depth + 1)?;
                if property.is_some_and(|i| !matches!(self.objects.get(&i), Some(Object::Property)))
                {
                    return Err(self.error("property attribute points to a different object type"));
                }
                Object::PropertyAttribute(property)
            }
            "visStateVisualStateAttribute_c" => {
                let words = [
                    self.word()?,
                    self.word()?,
                    self.word()?,
                    self.word()?,
                    self.word()?,
                ];
                if words[0] != 1 || words[1] > 1 || words[2..] != [6, 1, 0] {
                    return Err(self.error("unsupported visual-state attribute payload"));
                }
                Object::StateAttribute
            }
            "moVisualProperties_c" => {
                let at = self.cursor.at;
                let definition =
                    super::definition(self.cursor.bytes, at, "ThirdPtyStore/VisualStates")
                        .ok_or_else(|| self.error("unsupported property prefix"))?;
                let native = definition
                    .native
                    .ok_or_else(|| self.error("missing native property"))?;
                let extension = native
                    .extension
                    .as_ref()
                    .ok_or_else(|| self.error("unsupported property extension"))?;
                self.cursor.at = extension.prefix_end;
                // Complete observed property trailer. Preserve optical fields above;
                // opaque scalar regions are consumed without treating them as pointers.
                self.bytes(4)?;
                for _ in 0..3 {
                    self.text()?;
                }
                self.bytes(92)?;
                self.text()?;
                self.bytes(8)?;
                self.text()?;
                self.bytes(8)?;
                self.text()?;
                self.bytes(24)?;
                self.report.properties.insert(index, native);
                Object::Property
            }
            _ => return Err(self.error(&format!("unsupported sequential visual payload: {name}"))),
        };
        self.objects.insert(index, object);
        Ok(Some(index))
    }
    fn root(&mut self) -> Result<(), VisualArchiveFailure> {
        if [self.word()?, self.word()?] != [18000, 0] {
            return Err(self.error("unsupported visual archive root"));
        }
        let key = self.object(0)?;
        if !matches!(key.and_then(|i| self.objects.get(&i)), Some(Object::Key)) || self.word()? != 0
        {
            return Err(self.error("unsupported visual archive key pointer or separator"));
        }
        let n = self.count()?;
        for _ in 0..n {
            let component = self.object(0)?;
            if !matches!(
                component.and_then(|i| self.objects.get(&i)),
                Some(Object::Holder | Object::Component)
            ) {
                return Err(self.error("root entry is not a supported component"));
            }
        }
        if self.word()? != 0 {
            return Err(self.error("unsupported visual archive separator"));
        }
        let n = self.count()?;
        for _ in 0..n {
            let label = self.text()?;
            let header = self
                .subtree
                .archive_mut()
                .read_object(&mut self.cursor)
                .map_err(Self::archive_error)?;
            if header.is_new() {
                return Err(self.error("unsupported new object in visual client table"));
            }
            self.report.clients.push((label, header.object_index()));
        }
        if !self.cursor.remaining().is_empty() {
            return Err(self.error("unsupported visual archive trailer"));
        }
        Ok(())
    }
}
/// Decode only by sequential consumption; never seed the map from scanned markers.
pub(super) fn parse(bytes: &[u8]) -> VisualArchive {
    let mut walker = Walker {
        cursor: Reader::new(bytes),
        subtree: NativeSubtreeReader::new(
            ArchiveContext::new(),
            NativeExternalObjectProfile::ThreeStrings,
        ),
        objects: BTreeMap::new(),
        report: VisualArchive::default(),
        pointers: 0,
    };
    match walker.root() {
        Ok(()) => walker.report.complete = true,
        Err(error) => walker.report.failure = Some(error),
    }
    walker.report.reference_objects = walker.subtree.objects().clone();
    walker.report.map = std::mem::take(walker.subtree.archive_mut()).into_entries();
    walker.report
}

#[cfg(test)]
mod tests;
