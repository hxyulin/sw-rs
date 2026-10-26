// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
// Ported from openswx internal/cmgrhdr_reader.h.
// Copyright (c) 2026 openswx contributors; see LICENSE-openswx.
use crate::BinaryReader;
use std::collections::BTreeMap;
struct Reader<'a> {
    cursor: BinaryReader<'a>,
    archive: crate::ArchiveContext,
}
impl Reader<'_> {
    fn u8(&mut self) -> Option<u8> {
        Some(self.cursor.take(1)?[0])
    }
    fn string(&mut self) -> Option<String> {
        let first = self.u8()?;
        if first != 255 {
            return String::from_utf8(self.cursor.take(usize::from(first))?.to_vec()).ok();
        }
        let second = self.u8()?;
        if second != 254 && second != 255 {
            return String::from_utf8(self.cursor.take(usize::from(second))?.to_vec()).ok();
        }
        let third = self.u8()?;
        let len = if second == 254 && third == 255 {
            match self.u8()? {
                254 => usize::from(self.cursor.u16()?),
                255 => usize::try_from(self.cursor.u32()?).ok()?,
                n => usize::from(n),
            }
        } else {
            usize::from(third)
        };
        let raw = self.cursor.take(len.checked_mul(2)?)?;
        let units = raw
            .as_chunks::<2>()
            .0
            .iter()
            .map(|b| u16::from_le_bytes(*b));
        char::decode_utf16(units)
            .collect::<Result<String, _>>()
            .ok()
    }
    fn class(&mut self, expected: &str) -> Option<()> {
        let mut cursor = self.cursor;
        let header = self.archive.read_object(&mut cursor).ok()?;
        if !header.is_new()
            || !matches!(self.archive.entry(header.class_index()?),
            Some(crate::ArchiveEntry::Class { name, schema: 0 | 1, .. }) if name == expected)
        {
            return None;
        }
        self.cursor = cursor;
        Some(())
    }
}
pub(crate) fn names(data: &[u8], version: u32) -> BTreeMap<u32, (String, Option<u32>)> {
    let mut r = Reader {
        cursor: BinaryReader::new(data),
        archive: crate::ArchiveContext::new(),
    };
    let mut output = BTreeMap::new();
    let mut parse = || -> Option<()> {
        r.class("dmConfigMgrHeader_c")?;
        let count = match r.cursor.u16()? {
            65535 => r.cursor.u32()?,
            n => u32::from(n),
        };
        for _ in 0..count {
            r.class("dmConfigHeader_c")?;
            r.cursor.u32()?;
            let name = r.string()?;
            let id = r.cursor.u32()?;
            output.insert(id, (name, None));
            if version >= 1341 {
                r.cursor.u32()?;
            }
            if version >= 2037 {
                r.string()?;
                let parent = r.cursor.u32()?;
                output.get_mut(&id)?.1 = (parent != u32::MAX).then_some(parent);
            }
            if version >= 2136 {
                r.cursor.u32()?;
            }
            if version >= 2610 {
                r.string()?;
                r.string()?;
            }
            for gate in [3928, 11142, 12155, 12216] {
                if version >= gate {
                    r.cursor.u32()?;
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
        cursor: BinaryReader::new(data),
        archive: crate::ArchiveContext::new(),
    };
    let count = usize::try_from(r.cursor.u32()?).ok()?;
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
    r.cursor.remaining().is_empty().then_some(pairs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replacement_strings_preserve_supported_legacy_frames() {
        let value = "路径😀";
        let units: Vec<_> = value.encode_utf16().collect();
        let mut frames = vec![vec![255, 254, units.len() as u8]];
        frames.push(vec![255, 255, units.len() as u8]);
        frames.push(vec![255, 254, 255, units.len() as u8]);
        let mut wide_count = vec![255, 254, 255, 254];
        wide_count.extend((units.len() as u16).to_le_bytes());
        frames.push(wide_count);
        let mut long_count = vec![255, 254, 255, 255];
        long_count.extend((units.len() as u32).to_le_bytes());
        frames.push(long_count);
        for mut frame in frames {
            frame.extend(units.iter().flat_map(|unit| unit.to_le_bytes()));
            let mut bytes = 1_u32.to_le_bytes().to_vec();
            bytes.extend(&frame);
            bytes.extend(&frame);
            let pairs = replacement_references(&bytes).expect("complete replacement pair");
            assert_eq!(pairs[0].original_path, value, "frame {frame:?}");
            assert_eq!(pairs[0].replacement_path, value, "frame {frame:?}");
            bytes.pop();
            assert!(replacement_references(&bytes).is_none(), "frame {frame:?}");
        }
        for frame in [vec![3, b'a', b'b', b'c'], vec![255, 3, b'a', b'b', b'c']] {
            let mut reader = Reader {
                cursor: BinaryReader::new(&frame),
                archive: crate::ArchiveContext::new(),
            };
            assert_eq!(reader.string().as_deref(), Some("abc"), "frame {frame:?}");
        }
    }

    #[test]
    fn replacement_pairs_reject_invalid_unicode_empty_paths_and_trailing_bytes() {
        for data in [
            vec![1, 0, 0, 0, 255, 254, 1, 0, 216, 1, b'x'],
            vec![1, 0, 0, 0, 1, 255, 1, b'x'],
            vec![1, 0, 0, 0, 0, 1, b'x'],
            vec![1, 0, 0, 0, 1, b'x', 1, b'y', 0],
        ] {
            assert!(replacement_references(&data).is_none(), "input {data:?}");
        }
    }
}
