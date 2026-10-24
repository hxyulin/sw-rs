// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
//! Partial configuration-local mate inventory; no constraint solving or topology inference.
use crate::{NativeString, mesh, visual::native_string_at};
use serde::Serialize;
use std::collections::BTreeMap;

/// Semantic family established only by an explicit saved native class declaration.
/// Numeric SolidWorks API enum values are not assumed to be native file tags.
#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MateKind {
    /// Coincidence of the selected geometric entities.
    Coincident,
    /// Concentric selected entities.
    Concentric,
    /// Parallel selected entities.
    Parallel,
    /// Perpendicular selected entities.
    Perpendicular,
    /// Tangency of the selected entities.
    Tangent,
    /// Saved distance dimension; value/units are not decoded by this profile.
    Distance,
    /// Saved planar angle dimension; value/units are not decoded by this profile.
    Angle,
    /// Symmetric positioning of selected entities.
    Symmetric,
    /// Width mate, which can reference more than two entities.
    Width,
    /// Locked relative placement, not proof that either component is grounded.
    Lock,
    /// Slot mate; free/centered/dimensioned mode remains unknown.
    Slot,
    /// Limited distance; lower/upper bounds remain unknown.
    LimitDistance,
    /// Limited planar angle; lower/upper bounds remain unknown.
    LimitAngle,
}
impl MateKind {
    fn from_class(name: &str) -> Option<Self> {
        Some(match name {
            "moMateCoincident" => Self::Coincident,
            "moMateConcentric" => Self::Concentric,
            "moMateParallel" => Self::Parallel,
            "moMatePerpendicular" => Self::Perpendicular,
            "moMateTangent" => Self::Tangent,
            "moMateDistanceDim_c" => Self::Distance,
            "moMatePlanarAngleDim_c" => Self::Angle,
            "moMateSymmetric" => Self::Symmetric,
            "moMateWidth" => Self::Width,
            "moMateLock_c" => Self::Lock,
            "moMateSlot" => Self::Slot,
            "moMateLimitDistanceDim_c" => Self::LimitDistance,
            "moMateLimitPlanarAngleDim_c" => Self::LimitAngle,
            _ => return None,
        })
    }
}

/// A framed selection/occurrence-looking string inside one mate's source interval.
/// This is a candidate reference, not an established mate endpoint or topology ID.
#[derive(Clone, Debug, Serialize)]
pub struct MateReferenceCandidate {
    /// Exact saved reference and its CString frame offset in the decoded mate stream.
    pub reference: NativeString,
    /// Optional adjacent saved geometry descriptor, without interpreting its IDs.
    /// Present only for descriptor + word(1) + reference string framing.
    pub geometry_descriptor: Option<NativeString>,
}

/// A supported common mate header and the reference evidence in its source interval.
#[derive(Clone, Debug, Serialize)]
pub struct MateRecord {
    /// CString-framed mate name; not inferred from localized name prefixes.
    pub name: NativeString,
    /// Observed identifier word after the common header, not a cross-document ID.
    pub saved_identifier: u32,
    /// Raw tag word immediately preceding the name; its referenced class is unknown.
    pub name_prefix_tag: u16,
    /// Explicit native class declaration, if present at this record.
    /// None means a reused/unsupported class reference, not an absent mate type.
    pub class_name: Option<String>,
    /// Raw reused class tag preceding the feature-base tag, when no declaration exists.
    /// The external MFC object/class map is not decoded.
    pub class_reference: Option<u16>,
    /// Known family only when `class_name` explicitly establishes it.
    pub kind: Option<MateKind>,
    /// Start of the record's declaration/reference tag in decoded-stream bytes.
    pub source_offset: usize,
    /// End of this source interval, bounded by the next supported mate header or EOF.
    /// Not a validated complete serialized object boundary.
    pub evidence_end: usize,
    /// Saved occurrence/selection-looking strings, preserving order and duplicates.
    pub reference_candidates: Vec<MateReferenceCandidate>,
}

/// One distinct saved reference string and the mate records mentioning it.
/// This forms a bipartite evidence graph, not a solved component/joint graph.
#[derive(Clone, Debug, Serialize)]
pub struct MateReferenceNode {
    /// Exact saved string, without case folding, path rewriting or instance stripping.
    pub reference: String,
    /// Zero-based indices in this configuration stream's `records` collection.
    /// Each record occurs once even when it repeats the same saved string.
    pub record_indices: Vec<usize>,
}

/// Diagnostic local to a mate stream; unrelated document metadata remains available.
#[derive(Clone, Debug, Serialize)]
pub struct MateDiagnostic {
    /// Stable diagnostic identifier.
    pub code: String,
    /// Human-readable limitation or mismatch.
    pub message: String,
}

/// Partial mate evidence for exactly one native Config-N stream.
#[derive(Clone, Debug, Serialize)]
pub struct ConfigurationMates {
    /// Native configuration index from the exact source stream name.
    pub configuration_index: u32,
    /// Exact decoded source stream name.
    pub source_stream: String,
    /// Raw first header word; its object/class-map semantics remain unverified.
    pub header_word: Option<u32>,
    /// Saved short-count header, when supported. Extended counts remain unsupported.
    pub declared_count: Option<usize>,
    /// Supported common mate headers in source order.
    pub records: Vec<MateRecord>,
    /// Whether the recognized header count equals the saved count.
    /// This does not establish complete references, type resolution or active state.
    pub inventory_matches_declared_count: bool,
    /// Always false: complete entity/component ownership has not been decoded.
    pub entity_ownership_decoded: bool,
    /// Always false: distance/angle/limits, alignment and suppression remain unknown.
    pub parameters_decoded: bool,
    /// Unsupported framing and partial reference-graph limitations.
    pub diagnostics: Vec<MateDiagnostic>,
}
impl ConfigurationMates {
    /// Group identical reference candidates into a bipartite evidence graph.
    /// A width mate can mention four or more selections, so no pairwise joint or
    /// clique is fabricated. Stale names and unresolved entity paths remain intact.
    pub fn reference_graph(&self) -> Vec<MateReferenceNode> {
        let mut nodes: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for (index, record) in self.records.iter().enumerate() {
            for candidate in &record.reference_candidates {
                let records = nodes.entry(candidate.reference.value.clone()).or_default();
                if records.last() != Some(&index) {
                    records.push(index);
                }
            }
        }
        nodes
            .into_iter()
            .map(|(reference, record_indices)| MateReferenceNode {
                reference,
                record_indices,
            })
            .collect()
    }
}

/// Configuration-local mate inventories; absent streams never imply no constraints.
#[derive(Clone, Debug, Default, Serialize)]
pub struct MateData {
    /// Exact Config-N mate streams, including unsupported and saved-empty streams.
    pub configurations: Vec<ConfigurationMates>,
}
fn word(bytes: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(
        bytes.get(at..at.checked_add(2)?)?.try_into().ok()?,
    ))
}
fn dword(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        bytes.get(at..at.checked_add(4)?)?.try_into().ok()?,
    ))
}
fn strings(bytes: &[u8]) -> Vec<(NativeString, usize)> {
    let mut values = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        if let Some((value, end)) = native_string_at(bytes, at) {
            values.push((value, end));
            at = end;
        } else {
            at += 1;
        }
    }
    values
}
fn class_tag(tag: u16) -> bool {
    tag & 0x8000 != 0 && tag != 0xffff && tag != 0x8000
}
fn source_index(name: &str) -> Option<u32> {
    let index = name
        .strip_prefix("Contents/Config-")?
        .strip_suffix("-MatesList")?;
    if index.is_empty() || !index.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    index.parse().ok()
}
fn diagnostic(out: &mut ConfigurationMates, code: &str, message: &str) {
    out.diagnostics.push(MateDiagnostic {
        code: code.into(),
        message: message.into(),
    });
}
pub(crate) fn parse(streams: &BTreeMap<String, Vec<u8>>) -> MateData {
    MateData {
        configurations: streams
            .iter()
            .filter_map(|(name, bytes)| Some(parse_stream(bytes, name, source_index(name)?)))
            .collect(),
    }
}
fn parse_stream(bytes: &[u8], name: &str, index: u32) -> ConfigurationMates {
    let mut out = ConfigurationMates {
        configuration_index: index,
        source_stream: name.into(),
        header_word: dword(bytes, 0),
        declared_count: word(bytes, 4).filter(|n| *n != u16::MAX).map(usize::from),
        records: Vec::new(),
        inventory_matches_declared_count: false,
        entity_ownership_decoded: false,
        parameters_decoded: false,
        diagnostics: Vec::new(),
    };
    if out.header_word.is_none() || out.declared_count.is_none() {
        diagnostic(
            &mut out,
            "unsupported_mate_header",
            "Truncated header or unsupported extended mate count",
        );
        return out;
    }
    let texts = strings(bytes);
    let classes = mesh::class_intervals(bytes);
    let classes_by_start: BTreeMap<_, _> = classes.iter().map(|c| (c.content.start, c)).collect();
    for (value, end) in &texts {
        if value.value.is_empty() || value.offset < 8 {
            continue;
        }
        // Supported shared feature prefix: flag bytes, packed marker, identifier,
        // zero word, and a framed note. Parameter objects use identifier FFFFFFFF
        // and must not be counted as mates. Names themselves do not imply a type.
        let Some(prefix) = bytes.get(*end..end.saturating_add(16)) else {
            continue;
        };
        if prefix[0] != 0
            || ![0, 1].contains(&prefix[1])
            || prefix[2..7] != [0; 5]
            || ![0x40, 0xc0].contains(&prefix[7])
            || prefix[12..16] != [0; 4]
        {
            continue;
        }
        let identifier = dword(bytes, end + 8).expect("validated prefix");
        let tag = word(bytes, value.offset - 2).expect("checked offset");
        if identifier == 0
            || identifier == u32::MAX
            || !class_tag(tag)
            || native_string_at(bytes, end + 16).is_none()
        {
            continue;
        }
        let class = classes_by_start.get(&(value.offset - 2)).copied();
        if class.is_some_and(|class| !class.name.starts_with("moMate")) {
            continue;
        }
        let class_name = class.map(|class| class.name.clone());
        let reference = class
            .is_none()
            .then(|| word(bytes, value.offset - 4))
            .flatten()
            .filter(|t| class_tag(*t));
        if class_name.is_none() && reference.is_none() {
            continue;
        }
        let start = class
            .map(|class| class.class_offset)
            .unwrap_or(value.offset - 4);
        if start < 6 {
            continue;
        }
        out.records.push(MateRecord {
            name: value.clone(),
            saved_identifier: identifier,
            name_prefix_tag: tag,
            kind: class_name.as_deref().and_then(MateKind::from_class),
            class_name,
            class_reference: reference,
            source_offset: start,
            evidence_end: bytes.len(),
            reference_candidates: Vec::new(),
        });
    }
    for index in 0..out.records.len() {
        let end = out
            .records
            .get(index + 1)
            .map(|r| r.source_offset)
            .unwrap_or(bytes.len());
        let record = &mut out.records[index];
        record.evidence_end = end;
        let Some((_, name_end)) = native_string_at(bytes, record.name.offset) else {
            continue;
        };
        let Some((_, note_end)) = native_string_at(bytes, name_end + 16) else {
            continue;
        };
        let first = texts.partition_point(|(text, _)| text.offset < note_end);
        let last = texts.partition_point(|(text, _)| text.offset < end);
        for text_index in first..last {
            let (text, _) = &texts[text_index];
            if !text.value.contains('@') || text.value.contains('\\') || text.value.contains(':') {
                continue;
            }
            let geometry_descriptor = text_index.checked_sub(1).and_then(|i| {
                let (previous, previous_end) = &texts[i];
                (previous.offset >= note_end
                    && previous.value.starts_with("mo")
                    && previous.value.contains("IdRep")
                    && dword(bytes, *previous_end) == Some(1)
                    && previous_end.checked_add(4) == Some(text.offset))
                .then(|| previous.clone())
            });
            record.reference_candidates.push(MateReferenceCandidate {
                reference: text.clone(),
                geometry_descriptor,
            });
        }
    }
    out.inventory_matches_declared_count = out.declared_count == Some(out.records.len());
    if !out.inventory_matches_declared_count {
        diagnostic(
            &mut out,
            "mate_count_mismatch",
            "Recognized mate headers differ from the saved count; inventory is partial",
        );
    }
    // TODO(native-mates): decode the shared MFC class/object map, persistent
    // selection ownership, alignment/suppression and dimension/limit parameters.
    // Count agreement is not proof that reference candidates are active endpoints.
    diagnostic(
        &mut out,
        "partial_mate_relations",
        "Reference strings are candidates; reused class tags, topology ownership, active/suppressed state, parameters and solving remain unsupported",
    );
    out
}

#[cfg(test)]
mod tests;
