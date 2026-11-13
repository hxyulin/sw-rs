// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
//! Supported saved mate state; no effective activation or motion inference.
use super::{MateKind, MateRecord, dword, word};
use crate::visual::native_string_at;
use serde::Serialize;

/// One saved boolean word and its exact decoded-stream source offset.
#[derive(Clone, Debug, Serialize)]
pub struct MateStateFlag {
    /// Saved value, not a derived solver or motion result.
    pub value: bool,
    /// Exact u32 field offset in the enclosing configuration's mate stream.
    pub source_offset: usize,
}

/// Alignment matched to native API readings, not the API's numerical enum values.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MateAlignment {
    /// Selected directions aligned.
    Aligned,
    /// Selected directions anti-aligned.
    AntiAligned,
    /// Saved closest-orientation mode; not a chosen geometric branch.
    Closest,
}

/// Saved alignment with the original native code and byte provenance.
#[derive(Clone, Debug, Serialize)]
pub struct MateAlignmentValue {
    /// Alignment established by the supported native framing profile.
    /// None preserves an unknown native code without choosing an orientation.
    pub value: Option<MateAlignment>,
    /// Native u16: 0 closest, 1 aligned, 2 anti-aligned; other codes stay unknown.
    /// This is not swMateAlign_e.
    pub raw_value: u16,
    /// Exact u16 offset in the enclosing configuration's mate stream.
    pub source_offset: usize,
}

/// Partial saved state for one mate in one exact Config-N stream.
#[derive(Clone, Debug, Serialize)]
pub struct MateSavedState {
    /// Saved suppression. False does not prove effective activation or a valid solve.
    pub suppression: MateStateFlag,
    /// Saved alignment when its common mate frame is supported.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alignment: Option<MateAlignmentValue>,
    /// Saved concentric rotation lock, without a decoded lock-reference phase.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rotation_lock: Option<MateStateFlag>,
    /// Exact source offset of the checked common-feature format word, currently 101.
    pub feature_format_offset: usize,
}

pub(super) fn parse(bytes: &[u8], record: &MateRecord) -> Option<MateSavedState> {
    let bytes = bytes.get(..record.evidence_end)?;
    let (_, name_end) = native_string_at(bytes, record.name.offset)?;
    let (_, body_at) = native_string_at(bytes, name_end.checked_add(16)?)?;
    let body = bytes.get(body_at..)?;
    // These five words and format 101 frame the observed common-feature state.
    // The earlier byte in the feature header is NOT an inverse suppression flag.
    if [0, 4, 8, 16]
        .into_iter()
        .any(|at| dword(body, at) != Some(0))
        || dword(body, 20)? != 101
    {
        return None;
    }
    let suppression = flag(body, 12, body_at)?;
    Some(MateSavedState {
        suppression,
        alignment: record.kind.and_then(|_| alignment(body, body_at)),
        rotation_lock: (record.kind == Some(MateKind::Concentric))
            .then(|| rotation_lock(bytes, body_at))
            .flatten(),
        feature_format_offset: body_at.checked_add(20)?,
    })
}

fn flag(bytes: &[u8], at: usize, base: usize) -> Option<MateStateFlag> {
    let value = match dword(bytes, at)? {
        0 => false,
        1 => true,
        _ => return None,
    };
    Some(MateStateFlag {
        value,
        source_offset: base.checked_add(at)?,
    })
}

fn alignment(body: &[u8], base: usize) -> Option<MateAlignmentValue> {
    // Format 101 has 55 scalar bytes, a CString, then 84 scalar bytes before
    // the common mate frame. Follow the CString; never use fixture file offsets.
    let (_, string_end) = native_string_at(body, 55)?;
    let at = string_end.checked_add(84)?;
    let frame = body.get(at..at.checked_add(15)?)?;
    if word(frame, 2)? > 2
        || frame[4] != 0
        || !(1..=4).contains(&dword(frame, 5)?)
        || dword(frame, 9)? != 0
    {
        return None;
    }
    let raw_value = word(frame, 0)?;
    let value = match raw_value {
        0 => Some(MateAlignment::Closest),
        1 => Some(MateAlignment::Aligned),
        2 => Some(MateAlignment::AntiAligned),
        _ => None,
    };
    Some(MateAlignmentValue {
        value,
        raw_value,
        source_offset: base.checked_add(at)?,
    })
}

fn rotation_lock(bytes: &[u8], body_at: usize) -> Option<MateStateFlag> {
    let tail_at = bytes.len().checked_sub(16)?;
    if tail_at < body_at.checked_add(24)? {
        return None;
    }
    let tail = bytes.get(tail_at..)?;
    // Checked class-specific tail: two empty CStrings, boolean u32, -1 u32.
    // The caller bounds bytes at the next mate header, not at the whole file EOF.
    if tail[..8] != [255, 254, 255, 0, 255, 254, 255, 0] || dword(tail, 12)? != u32::MAX {
        return None;
    }
    flag(tail, 8, tail_at)
}
