// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
// Ported from openswx internal/cmgrhdr_reader.h.
// Copyright (c) 2026 openswx contributors; see LICENSE-openswx.
use std::collections::BTreeMap;
struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
    archive: crate::ArchiveContext,
}
impl Reader<'_> {
    fn take(&mut self, n: usize) -> Option<&[u8]> {
        let end = self.pos.checked_add(n)?;
        let slice = self.bytes.get(self.pos..end)?;
        self.pos = end;
        Some(slice)
    }
    fn u8(&mut self) -> Option<u8> {
        Some(self.take(1)?[0])
    }
    fn u16(&mut self) -> Option<u16> {
        Some(u16::from_le_bytes(self.take(2)?.try_into().ok()?))
    }
    fn u32(&mut self) -> Option<u32> {
        Some(u32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }
    fn string(&mut self) -> Option<String> {
        let first = self.u8()?;
        if first != 255 {
            return String::from_utf8(self.take(first as usize)?.to_vec()).ok();
        }
        let second = self.u8()?;
        if second != 254 && second != 255 {
            return String::from_utf8(self.take(second as usize)?.to_vec()).ok();
        }
        let third = self.u8()?;
        let len = if second == 254 && third == 255 {
            match self.u8()? {
                254 => self.u16()? as usize,
                255 => self.u32()? as usize,
                n => n as usize,
            }
        } else {
            third as usize
        };
        let raw = self.take(len.checked_mul(2)?)?;
        String::from_utf16(
            &raw.as_chunks::<2>()
                .0
                .iter()
                .map(|b| u16::from_le_bytes([b[0], b[1]]))
                .collect::<Vec<_>>(),
        )
        .ok()
    }
    fn class(&mut self, expected: &str) -> Option<()> {
        let mut cursor = crate::BinaryReader::at(self.bytes, self.pos)?;
        let header = self.archive.read_object(&mut cursor).ok()?;
        if !header.is_new
            || !matches!(self.archive.entry(header.class_index?),
            Some(crate::ArchiveEntry::Class { name, schema: 0 | 1, .. }) if name == expected)
        {
            return None;
        }
        self.pos = cursor.position();
        Some(())
    }
}
pub(crate) fn names(data: &[u8], version: u32) -> BTreeMap<u32, (String, Option<u32>)> {
    let mut r = Reader {
        bytes: data,
        pos: 0,
        archive: crate::ArchiveContext::new(),
    };
    let mut output = BTreeMap::new();
    let mut parse = || -> Option<()> {
        r.class("dmConfigMgrHeader_c")?;
        let count = match r.u16()? {
            65535 => r.u32()?,
            n => u32::from(n),
        };
        for _ in 0..count {
            r.class("dmConfigHeader_c")?;
            r.u32()?;
            let name = r.string()?;
            let id = r.u32()?;
            output.insert(id, (name, None));
            if version >= 1341 {
                r.u32()?;
            }
            if version >= 2037 {
                r.string()?;
                let parent = r.u32()?;
                output.get_mut(&id)?.1 = (parent != u32::MAX).then_some(parent);
            }
            if version >= 2136 {
                r.u32()?;
            }
            if version >= 2610 {
                r.string()?;
                r.string()?;
            }
            for gate in [3928, 11142, 12155, 12216] {
                if version >= gate {
                    r.u32()?;
                }
            }
        }
        Some(())
    };
    let _ = parse();
    output
}

/// Decode the count-prefixed MFC string pairs in the saved ReplaceRef stream.
/// Reject incomplete/unknown layouts atomically; partial pairs are not mappings.
pub(crate) fn replacement_references(data: &[u8]) -> Option<Vec<crate::ReferenceReplacement>> {
    let mut r = Reader {
        bytes: data,
        pos: 0,
        archive: crate::ArchiveContext::new(),
    };
    let count = r.u32()? as usize;
    if count > data.len().saturating_sub(4) / 2 {
        return None;
    }
    let mut pairs = Vec::new();
    for _ in 0..count {
        let original_path = r.string()?;
        let replacement_path = r.string()?;
        if original_path.is_empty() || replacement_path.is_empty() {
            return None;
        }
        pairs.push(crate::ReferenceReplacement {
            original_path,
            replacement_path,
        });
    }
    (r.pos == data.len()).then_some(pairs)
}
