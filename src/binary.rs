// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
// String framing extracted from sw-rs visual.rs; see NOTICE-cadmpeg.md.
//! Checked borrowed binary cursor for native record decoders.
use serde::Serialize;

/// Exact framed UTF-16 string with its byte offset in a decoded source stream.
#[derive(Clone, Debug, Serialize)]
pub struct FramedString {
    /// Exact saved Unicode value, including blank values and original separators.
    pub value: String,
    /// Offset of the CString frame in the source slice passed to the cursor.
    pub offset: usize,
}

/// A checked little-endian cursor over caller-owned bytes; reads never copy payloads.
/// Primitive failed reads leave the position unchanged. Compound reads are transactional.
/// Float-array helpers require finite values; scalar float reads preserve raw IEEE values.
#[derive(Clone, Copy, Debug)]
pub struct BinaryReader<'a> {
    pub(crate) bytes: &'a [u8],
    pub(crate) at: usize,
}
impl<'a> BinaryReader<'a> {
    /// Start at byte zero of a borrowed stream.
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, at: 0 }
    }
    /// Start at an exact stream offset, or return None when it is out of bounds.
    pub fn at(bytes: &'a [u8], offset: usize) -> Option<Self> {
        (offset <= bytes.len()).then_some(Self { bytes, at: offset })
    }
    /// Current byte offset in the original borrowed slice.
    pub fn position(&self) -> usize {
        self.at
    }
    /// Remaining bytes without allocation.
    pub fn remaining(&self) -> &'a [u8] {
        &self.bytes[self.at..]
    }
    /// Borrow the next byte range, advancing only on success.
    pub fn take(&mut self, len: usize) -> Option<&'a [u8]> {
        let end = self.at.checked_add(len)?;
        let bytes = self.bytes.get(self.at..end)?;
        self.at = end;
        Some(bytes)
    }
    /// Read an unsigned little-endian 16-bit word.
    pub fn u16(&mut self) -> Option<u16> {
        Some(u16::from_le_bytes(self.take(2)?.try_into().ok()?))
    }
    /// Read an unsigned little-endian 32-bit word.
    pub fn u32(&mut self) -> Option<u32> {
        Some(u32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }
    /// Read an unsigned little-endian 64-bit word.
    pub fn u64(&mut self) -> Option<u64> {
        Some(u64::from_le_bytes(self.take(8)?.try_into().ok()?))
    }
    /// Read a raw IEEE single-precision value, including nonfinite values.
    pub fn f32(&mut self) -> Option<f32> {
        self.u32().map(f32::from_bits)
    }
    /// Read a raw IEEE double-precision value, including nonfinite values.
    pub fn f64(&mut self) -> Option<f64> {
        self.u64().map(f64::from_bits)
    }
    /// Read a fixed number of words, rolling back if the complete array is unavailable.
    pub fn words<const N: usize>(&mut self) -> Option<[u32; N]> {
        let mut trial = *self;
        let mut values = [0; N];
        for value in &mut values {
            *value = trial.u32()?;
        }
        *self = trial;
        Some(values)
    }
    /// Read finite doubles, rolling back on truncation or a nonfinite value.
    pub fn doubles<const N: usize>(&mut self) -> Option<[f64; N]> {
        let mut trial = *self;
        let mut values = [0.; N];
        for value in &mut values {
            *value = trial.f64()?;
        }
        if !values.iter().all(|v| v.is_finite()) {
            return None;
        }
        *self = trial;
        Some(values)
    }
    /// Read finite floats, rolling back on truncation or a nonfinite value.
    pub fn floats<const N: usize>(&mut self) -> Option<[f32; N]> {
        let mut trial = *self;
        let values = trial.words::<N>()?.map(f32::from_bits);
        if !values.iter().all(|v| v.is_finite()) {
            return None;
        }
        *self = trial;
        Some(values)
    }
    /// Borrow complete counted CString code-unit bytes without decoding their text.
    /// Advances only for an in-bounds supported frame; UTF-16 validity is unchecked.
    pub(crate) fn string_bytes(&mut self) -> Option<&'a [u8]> {
        let mut trial = *self;
        if trial.take(3)? != [0xff, 0xfe, 0xff] {
            return None;
        }
        let count = match trial.take(1)?[0] {
            254 => usize::from(trial.u16()?),
            255 => usize::try_from(trial.u32()?).ok()?,
            count => usize::from(count),
        };
        let bytes = trial.take(count.checked_mul(2)?)?;
        *self = trial;
        Some(bytes)
    }
    /// Read the supported native UTF-16 CString framing with short/u16/u32 counts.
    /// Checks the complete byte span before allocating, rejects invalid UTF-16,
    /// and rolls back on failure. Offsets refer to the original stream slice.
    pub fn string(&mut self) -> Option<FramedString> {
        let mut trial = *self;
        let offset = trial.at;
        let bytes = trial.string_bytes()?;
        // Decode directly from borrowed code units; no intermediate Vec<u16>.
        let units = bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|b| u16::from_le_bytes(*b));
        let value = char::decode_utf16(units)
            .collect::<Result<String, _>>()
            .ok()?;
        *self = trial;
        Some(FramedString { value, offset })
    }
}
