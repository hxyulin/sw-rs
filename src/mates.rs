// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
//! Partial configuration-local mate inventory; no constraint solving or topology inference.
use crate::{NativeString, mesh, visual::native_string_at};
use serde::Serialize;
use std::collections::BTreeMap;
mod state;
pub use state::{MateAlignment, MateAlignmentValue, MateSavedState, MateStateFlag};

/// Semantic family established by an explicit saved native class declaration,
/// directly or through a supported configuration-local class reference.
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
    /// Cam/follower tangency; contact profile and solution branch remain unknown.
    CamTangent,
    /// Saved distance dimension; named scalar prefixes may be available separately.
    Distance,
    /// Saved planar angle dimension; named scalar prefixes may be available separately.
    Angle,
    /// Symmetric positioning of selected entities.
    Symmetric,
    /// Width mate, which can reference more than two entities.
    Width,
    /// Locked relative placement, not proof that either component is grounded.
    Lock,
    /// Slot mate; free/centered/dimensioned mode remains unknown.
    Slot,
    /// Limited distance; configured bounds may be available for a supported prefix.
    LimitDistance,
    /// Limited planar angle; configured bounds may be available for a supported prefix.
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
            "moMateCamTangent" => Self::CamTangent,
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

/// Cached plane parameters without established endpoint ownership or selection order.
#[derive(Clone, Debug, Serialize)]
pub struct MatePlaneGeometryCandidate {
    /// A saved point on the plane, in metres in this mate's owning assembly frame.
    /// Tangential coordinates are not a unique plane origin or a joint pivot.
    pub point_metres: [f64; 3],
    /// Saved unit normal in the same owning assembly frame; not Part-local geometry.
    pub normal: [f64; 3],
    /// Explicit schema-1 moAbsoluteRefPlnData_c declaration in the decoded mate stream.
    pub declaration_offset: usize,
    /// First of eight saved f64 parameters: point, normal, then two checked zeros.
    /// This is a prefix offset, not a complete native object boundary.
    pub parameters_offset: usize,
}
impl MateReferenceCandidate {
    /// Interpret a single-row surface descriptor without resolving its owner.
    /// Multi-row descriptors return None; use `surface_references` to retain every row.
    pub fn surface_reference(&self) -> Option<crate::SavedSurfaceReference> {
        crate::SavedSurfaceReference::parse(self.geometry_descriptor.as_ref()?)
    }
    /// Interpret all rows in the adjacent descriptor, retaining duplicates and
    /// conflicting identities without choosing a mate endpoint or occurrence.
    pub fn surface_references(&self) -> Vec<crate::SavedSurfaceReference> {
        self.geometry_descriptor
            .as_ref()
            .map(crate::SavedSurfaceReference::parse_all)
            .unwrap_or_default()
    }
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
    /// The complete MFC object/class map remains undecoded; see resolution evidence.
    pub class_reference: Option<u16>,
    /// Known family from a direct declaration or supported local class-reference resolution.
    pub kind: Option<MateKind>,
    /// Provenance for a reused class resolved through a supported local profile.
    /// None does not imply a missing type; direct declarations use `class_name`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class_reference_resolution: Option<MateClassReferenceResolution>,
    /// Start of the record's declaration/reference tag in decoded-stream bytes.
    pub source_offset: usize,
    /// End of this source interval, bounded by the next supported mate header or EOF.
    /// Not a validated complete serialized object boundary.
    pub evidence_end: usize,
    /// Saved occurrence/selection-looking strings, preserving order and duplicates.
    pub reference_candidates: Vec<MateReferenceCandidate>,
    /// Explicit cached plane prefixes, retaining source order without assigning an
    /// endpoint index or pairing them with reference strings. Saved caches can be
    /// stale; absence means unsupported/unknown, not an absence of planar selections.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub cached_plane_geometry: Vec<MatePlaneGeometryCandidate>,
    /// Named numerical parameter prefixes inside this evidence interval.
    /// Unknown class references retain raw values without assigning physical units.
    /// These values are not automatically driving dimensions or travel bounds.
    pub parameter_candidates: Vec<MateParameterCandidate>,
    /// Configured numerical bounds in a supported distance/angle limit prefix.
    /// Absence means unsupported/unknown, never an unbounded joint. These values
    /// do not establish active state, a joint coordinate, or collision-free travel.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub configured_limits: Option<MateLimitBounds>,
    /// Supported saved suppression/alignment/rotation-lock fields, scoped to this
    /// configuration. These do not establish effective activation or motion freedom.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub saved_state: Option<MateSavedState>,
}

/// Physical unit established by an explicit supported native parameter class.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MateLimitUnit {
    /// Length parameter in SI metres.
    Metres,
    /// Planar-angle parameter in radians.
    Radians,
}

/// Configured distance/angle bounds from a supported explicit parameter and
/// its adjacent native tolerance object. Every scalar uses the declared `unit`.
#[derive(Clone, Debug, Serialize)]
pub struct MateLimitBounds {
    /// Explicit quantity unit; not inferred from names or neighboring mate types.
    pub unit: MateLimitUnit,
    /// Saved absolute minimum, computed from the dimension plus lower deviation.
    pub minimum: f64,
    /// Saved absolute maximum, computed from the dimension plus upper deviation.
    pub maximum: f64,
    /// Saved dimension used as the tolerance reference, not a solved joint coordinate.
    pub dimension_value: f64,
    /// Signed lower deviation stored by the source.
    pub lower_deviation: f64,
    /// Signed upper deviation stored by the source.
    pub upper_deviation: f64,
    /// Dimension scalar offset in the configuration's decoded source stream.
    pub dimension_value_offset: usize,
    /// Exact explicit moDimTolerance_c declaration offset in the same stream.
    pub tolerance_declaration_offset: usize,
    /// Lower-deviation f64 offset in the same stream.
    pub lower_deviation_offset: usize,
    /// Upper-deviation f64 offset in the same stream.
    pub upper_deviation_offset: usize,
}

/// A short class reference tied to an explicit declaration in the same stream.
/// This is a partial native profile, not a recovered general MFC archive dictionary.
#[derive(Clone, Debug, Serialize)]
pub struct MateClassReferenceResolution {
    /// Exact runtime-class name from the matched inline declaration.
    pub class_name: String,
    /// Archive-local index, not a global SolidWorks type tag.
    pub class_index: u32,
    /// Index source: leading u32 at zero, or this record's reused class tag.
    pub index_source_offset: usize,
    /// Matched explicit class declaration's decoded-stream offset.
    pub declaration_offset: usize,
    /// Offset of this record's short reused class tag in the same stream.
    pub reference_offset: usize,
    /// Configuration-local order proof, absent for a direct leading-index match.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub declaration_order_evidence: Option<usize>,
}

/// An explicit class in a complete inventory of one supported prefix family.
#[derive(Clone, Debug, Serialize)]
pub struct MateClassDeclaration {
    /// Exact runtime-class name; labels and scalar values do not establish it.
    pub class_name: String,
    /// Explicit declaration offset in this configuration's decoded mate stream.
    pub declaration_offset: usize,
}

/// Earliest observed use of one short class index in a supported prefix family.
#[derive(Clone, Debug, Serialize)]
pub struct MateClassIndexUse {
    /// Short archive-local class index, with the class-tag bit removed.
    pub class_index: u32,
    /// Earliest reused-tag offset; zero denotes the leading u32 anchor.
    pub source_offset: usize,
}

/// Constraints proving a unique ordered match without counting object-map slots.
#[derive(Clone, Debug, Serialize)]
pub struct MateClassOrderEvidence {
    /// `mates` or `parameters`; each has a separately checked common prefix.
    pub prefix_family: &'static str,
    /// First mate class index from the leading u32 at decoded offset zero.
    pub leading_class_index: u32,
    /// All supported declarations in source order, including unused classes.
    pub declarations: Vec<MateClassDeclaration>,
    /// Distinct indices in ascending order and their earliest use offsets.
    pub index_uses: Vec<MateClassIndexUse>,
}

/// A supported named native parameter prefix, not a complete dimension object.
#[derive(Clone, Debug, Serialize)]
pub struct MateParameterCandidate {
    /// Saved parameter name and its decoded-stream offset.
    pub name: NativeString,
    /// Explicit parameter class, when declared directly at this prefix.
    pub class_name: Option<String>,
    /// Raw reused class tag, retained even when a local proof resolves it.
    pub class_reference: Option<u16>,
    /// Declaration and order proof when a reused parameter class is unambiguous.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class_reference_resolution: Option<MateClassReferenceResolution>,
    /// Finite saved scalar. This is not a measured joint coordinate.
    pub value: f64,
    /// `metres` or `radians` for an explicit or proven reused parameter class.
    /// None means that the physical meaning and units remain unknown.
    pub unit: Option<&'static str>,
    /// Exact little-endian f64 offset in the configuration's decoded source stream.
    pub value_offset: usize,
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
    /// Raw leading word; anchors the first class index only in the supported profile.
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
    /// Always false: complete parameter and effective activation semantics remain unknown.
    /// Supported saved state fields are retained separately per record.
    pub parameters_decoded: bool,
    /// Always false: complete bounds and enable/active-state semantics remain unsupported.
    /// Supported configured numerical bounds are retained per record separately.
    pub limit_bounds_decoded: bool,
    /// Recognized records with a supported configured numerical bound prefix.
    pub records_with_configured_limits: usize,
    /// Shared proofs for uniquely matched reused mate/parameter class references.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub class_order_evidence: Vec<MateClassOrderEvidence>,
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
fn finite_double(bytes: &[u8], at: usize) -> Option<f64> {
    let value = f64::from_le_bytes(bytes.get(at..at.checked_add(8)?)?.try_into().ok()?);
    value.is_finite().then_some(value)
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
pub(crate) fn source_index(name: &str) -> Option<u32> {
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
        limit_bounds_decoded: false,
        records_with_configured_limits: 0,
        class_order_evidence: Vec::new(),
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
            class_reference_resolution: None,
            class_name,
            class_reference: reference,
            source_offset: start,
            evidence_end: bytes.len(),
            reference_candidates: Vec::new(),
            cached_plane_geometry: Vec::new(),
            parameter_candidates: Vec::new(),
            configured_limits: None,
            saved_state: None,
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
            if let Some(parameter) =
                parameter_candidate(bytes, &texts[text_index], note_end, end, &classes_by_start)
            {
                record.parameter_candidates.push(parameter);
            }
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
    resolve_first_class_references(&mut out);
    resolve_ordered_class_references(&mut out, bytes, &classes);
    if out.inventory_matches_declared_count {
        for record in &mut out.records {
            record.configured_limits = configured_limits(bytes, record, &classes);
            record.saved_state = state::parse(bytes, record);
            record.cached_plane_geometry = cached_planes(bytes, record, &classes, &texts);
        }
    }
    out.records_with_configured_limits = out
        .records
        .iter()
        .filter(|r| r.configured_limits.is_some())
        .count();
    if !out.inventory_matches_declared_count {
        diagnostic(
            &mut out,
            "mate_count_mismatch",
            "Recognized mate headers differ from the saved count; inventory is partial",
        );
    }
    // TODO(native-mates): decode the shared MFC class/object map, persistent
    // selection ownership and unsupported state/dimension/limit layouts.
    // Count agreement is not proof that reference candidates are active endpoints.
    diagnostic(
        &mut out,
        "partial_mate_relations",
        "Supported class, parameter, configured-bound and saved-state profiles retain source evidence. Full endpoint ownership, effective activation, travel limits and constraint solving remain unsupported.",
    );
    out
}

fn cached_planes(
    bytes: &[u8],
    record: &MateRecord,
    classes: &[mesh::ClassInterval],
    texts: &[(NativeString, usize)],
) -> Vec<MatePlaneGeometryCandidate> {
    classes
        .iter()
        .filter(|class| {
            class.name == "moAbsoluteRefPlnData_c"
                && class.class_offset >= record.source_offset
                && class.content.start < record.evidence_end
                // Class-like bytes inside a framed string are not declarations.
                && !texts.iter().any(|(text, end)| {
                    text.offset <= class.class_offset && class.class_offset < *end
                })
        })
        .filter_map(|class| {
            let at = class.content.start;
            let end = at.checked_add(64)?;
            if end > record.evidence_end || end > class.content.end {
                return None;
            }
            let point_metres = [
                finite_double(bytes, at)?,
                finite_double(bytes, at + 8)?,
                finite_double(bytes, at + 16)?,
            ];
            let normal = [
                finite_double(bytes, at + 24)?,
                finite_double(bytes, at + 32)?,
                finite_double(bytes, at + 40)?,
            ];
            // Native plane witnesses match the API's eight parameters in the
            // owning assembly frame. Reject nonplanar or unknown cache layouts.
            // The normal is observed as unit length; do not normalize corrupt data.
            const NORMAL_SQUARED_TOLERANCE: f64 = 1e-8;
            let squared_length: f64 = normal.iter().map(|n| n * n).sum();
            if (squared_length - 1.0).abs() > NORMAL_SQUARED_TOLERANCE
                || finite_double(bytes, at + 48)? != 0.0
                || finite_double(bytes, at + 56)? != 0.0
            {
                return None;
            }
            Some(MatePlaneGeometryCandidate {
                point_metres,
                normal,
                declaration_offset: class.class_offset,
                parameters_offset: at,
            })
        })
        .collect()
}

fn configured_limits(
    bytes: &[u8],
    record: &MateRecord,
    classes: &[mesh::ClassInterval],
) -> Option<MateLimitBounds> {
    let (class_name, unit_name, unit) = match record.kind? {
        MateKind::LimitDistance => ("moLengthParameter_c", "metres", MateLimitUnit::Metres),
        MateKind::LimitAngle => ("moAngleParameter_c", "radians", MateLimitUnit::Radians),
        _ => return None,
    };
    let [parameter] = record.parameter_candidates.as_slice() else {
        return None;
    };
    if parameter.class_name.as_deref() != Some(class_name) || parameter.unit != Some(unit_name) {
        return None;
    }
    let value_end = parameter.value_offset.checked_add(8)?;
    if *bytes.get(value_end)? != 0 {
        return None;
    }
    let declaration_offset = value_end.checked_add(1)?;
    let tolerance = classes
        .iter()
        .find(|c| c.class_offset == declaration_offset && c.name == "moDimTolerance_c")?;
    let payload = tolerance.content.start;
    let upper_offset = payload.checked_add(8)?;
    let lower_offset = payload.checked_add(16)?;
    let end = lower_offset.checked_add(8)?;
    if end > record.evidence_end
        || end > tolerance.content.end
        || dword(bytes, payload)? != 3
        || dword(bytes, payload.checked_add(4)?)? != 0
    {
        return None;
    }
    let upper = finite_double(bytes, upper_offset)?;
    let lower = finite_double(bytes, lower_offset)?;
    let minimum = parameter.value + lower;
    let maximum = parameter.value + upper;
    // This profile was validated with nonnegative intervals and a reference
    // value inside the configured interval. Other states remain unsupported.
    if lower > 0.
        || upper < 0.
        || !minimum.is_finite()
        || !maximum.is_finite()
        || minimum < 0.
        || minimum > maximum
    {
        return None;
    }
    Some(MateLimitBounds {
        unit,
        minimum,
        maximum,
        dimension_value: parameter.value,
        lower_deviation: lower,
        upper_deviation: upper,
        dimension_value_offset: parameter.value_offset,
        tolerance_declaration_offset: declaration_offset,
        lower_deviation_offset: lower_offset,
        upper_deviation_offset: upper_offset,
    })
}

fn resolve_first_class_references(out: &mut ConfigurationMates) {
    let Some(index) = leading_class_index(out) else {
        return;
    };
    let name = out.records[0]
        .class_name
        .clone()
        .expect("checked leading class");
    let tag = 0x8000 | index as u16;
    for record in &mut out.records {
        if record.class_name.is_none() && record.class_reference == Some(tag) {
            record.kind = MateKind::from_class(&name);
            record.class_reference_resolution = Some(MateClassReferenceResolution {
                class_name: name.clone(),
                class_index: index,
                index_source_offset: 0,
                declaration_offset: 6,
                reference_offset: record.source_offset,
                declaration_order_evidence: None,
            });
        }
    }
}

fn leading_class_index(out: &ConfigurationMates) -> Option<u32> {
    let first = out.records.first()?;
    let index = out
        .header_word
        .filter(|index| (1..0x7fff).contains(index))?;
    (out.inventory_matches_declared_count
        && first.source_offset == 6
        && first.class_name.is_some()
        && u32::from(first.name_prefix_tag & 0x7fff) < index)
        .then_some(index)
}

// MFC class indices increase with declaration order, regardless of intervening
// object registrations. A reused class must have been declared before its first
// use. Match only pairs common to EVERY order-preserving injection; never assign
// a slot by counting scanned declarations or assume an unused class was reused.
fn ordered_class_matches(proof: &MateClassOrderEvidence) -> Option<BTreeMap<u32, usize>> {
    let declarations = &proof.declarations;
    if declarations.is_empty()
        || declarations
            .windows(2)
            .any(|p| p[0].declaration_offset >= p[1].declaration_offset)
        || proof
            .index_uses
            .windows(2)
            .any(|p| p[0].class_index >= p[1].class_index)
        || proof
            .index_uses
            .iter()
            .any(|u| u.class_index < proof.leading_class_index)
    {
        return None;
    }
    let mut names = std::collections::BTreeSet::new();
    if declarations
        .iter()
        .any(|d| !names.insert(d.class_name.as_str()))
    {
        return None;
    }
    let mut latest = declarations.len();
    let mut matches = BTreeMap::new();
    for (earliest, usage) in proof.index_uses.iter().enumerate().rev() {
        let deadline = if usage.source_offset == 0 {
            // The root mate class is independently anchored by the leading u32.
            if usage.class_index != proof.leading_class_index || earliest != 0 {
                return None;
            }
            0
        } else {
            declarations
                .partition_point(|d| d.declaration_offset < usage.source_offset)
                .checked_sub(1)?
        };
        latest = latest.checked_sub(1)?.min(deadline);
        if latest < earliest {
            return None;
        }
        // Earliest choices are 0, 1, ...: every domain is a source-order prefix.
        // Backward latest choices retain room for all later distinct class tags.
        if latest == earliest {
            matches.insert(usage.class_index, earliest);
        }
    }
    Some(matches)
}

fn class_index_uses(
    references: impl Iterator<Item = (u16, usize)>,
    leading: u32,
) -> Vec<MateClassIndexUse> {
    let mut uses = BTreeMap::<u32, usize>::new();
    for (tag, offset) in references {
        let index = u32::from(tag & 0x7fff);
        // Earlier indices belong to inherited context; no local class is assigned.
        if index >= leading {
            uses.entry(index)
                .and_modify(|first| *first = (*first).min(offset))
                .or_insert(offset);
        }
    }
    uses.into_iter()
        .map(|(class_index, source_offset)| MateClassIndexUse {
            class_index,
            source_offset,
        })
        .collect()
}

fn resolve_ordered_class_references(
    out: &mut ConfigurationMates,
    bytes: &[u8],
    classes: &[mesh::ClassInterval],
) {
    let Some(leading) = leading_class_index(out) else {
        return;
    };
    let mut index_uses = class_index_uses(
        out.records
            .iter()
            .filter_map(|r| Some((r.class_reference?, r.source_offset))),
        leading,
    );
    if let Some(first) = index_uses.first_mut().filter(|u| u.class_index == leading) {
        first.source_offset = 0;
    } else {
        index_uses.insert(
            0,
            MateClassIndexUse {
                class_index: leading,
                source_offset: 0,
            },
        );
    }
    let proof = MateClassOrderEvidence {
        prefix_family: "mates",
        leading_class_index: leading,
        declarations: out
            .records
            .iter()
            .filter_map(|r| {
                Some(MateClassDeclaration {
                    class_name: r.class_name.clone()?,
                    declaration_offset: r.source_offset,
                })
            })
            .collect(),
        index_uses,
    };
    if let Some(matches) = ordered_class_matches(&proof) {
        let proof_index = out.class_order_evidence.len();
        let mut used = false;
        for record in &mut out.records {
            let Some(tag) = record
                .class_reference
                .filter(|_| record.class_reference_resolution.is_none())
            else {
                continue;
            };
            let Some(&declaration) = matches.get(&u32::from(tag & 0x7fff)) else {
                continue;
            };
            let declaration = &proof.declarations[declaration];
            record.kind = MateKind::from_class(&declaration.class_name);
            record.class_reference_resolution = Some(order_resolution(
                declaration,
                tag,
                record.source_offset,
                proof_index,
            ));
            used = true;
        }
        if used {
            out.class_order_evidence.push(proof);
        }
    }

    let Some(declarations) = parameter_class_declarations(bytes, classes) else {
        return;
    };
    let mut references = Vec::new();
    for record in &out.records {
        for parameter in &record.parameter_candidates {
            if let Some(tag) = parameter.class_reference {
                let Some(offset) = parameter.name.offset.checked_sub(4) else {
                    return;
                };
                if u32::from(tag & 0x7fff) == leading {
                    return;
                }
                references.push((tag, offset));
            }
        }
    }
    let proof = MateClassOrderEvidence {
        prefix_family: "parameters",
        leading_class_index: leading,
        declarations,
        index_uses: class_index_uses(references.into_iter(), leading),
    };
    let Some(matches) = ordered_class_matches(&proof) else {
        return;
    };
    if !class_bindings_consistent(out, &proof, &matches) {
        // Both prefix families share one map. A conflict invalidates the order
        // profile, while independently anchored leading-class matches survive.
        for record in &mut out.records {
            if record
                .class_reference_resolution
                .as_ref()
                .is_some_and(|r| r.declaration_order_evidence.is_some())
            {
                record.class_reference_resolution = None;
                record.kind = record.class_name.as_deref().and_then(MateKind::from_class);
            }
        }
        out.class_order_evidence.clear();
        diagnostic(
            out,
            "class_order_conflict",
            "Mate and parameter class-order assignments conflict; only leading-index reuse is retained",
        );
        return;
    }
    let proof_index = out.class_order_evidence.len();
    let mut used = false;
    for record in &mut out.records {
        for parameter in &mut record.parameter_candidates {
            let Some(tag) = parameter.class_reference else {
                continue;
            };
            let Some(&declaration) = matches.get(&u32::from(tag & 0x7fff)) else {
                continue;
            };
            let declaration = &proof.declarations[declaration];
            parameter.unit = parameter_unit(&declaration.class_name);
            parameter.class_reference_resolution = Some(order_resolution(
                declaration,
                tag,
                parameter.name.offset - 4,
                proof_index,
            ));
            used = true;
        }
    }
    if used {
        out.class_order_evidence.push(proof);
    }
}

fn class_bindings_consistent(
    out: &ConfigurationMates,
    proof: &MateClassOrderEvidence,
    matches: &BTreeMap<u32, usize>,
) -> bool {
    let mut bindings = BTreeMap::from([(proof.leading_class_index, 6)]);
    for resolution in out
        .records
        .iter()
        .filter_map(|r| r.class_reference_resolution.as_ref())
    {
        bindings.insert(resolution.class_index, resolution.declaration_offset);
    }
    for (index, declaration) in matches {
        let offset = proof.declarations[*declaration].declaration_offset;
        if bindings
            .insert(*index, offset)
            .is_some_and(|old| old != offset)
        {
            return false;
        }
    }
    let offsets: Vec<_> = bindings.into_values().collect();
    offsets.windows(2).all(|pair| pair[0] < pair[1])
}

fn order_resolution(
    declaration: &MateClassDeclaration,
    tag: u16,
    offset: usize,
    proof_index: usize,
) -> MateClassReferenceResolution {
    MateClassReferenceResolution {
        class_name: declaration.class_name.clone(),
        class_index: u32::from(tag & 0x7fff),
        index_source_offset: offset,
        declaration_offset: declaration.declaration_offset,
        reference_offset: offset,
        declaration_order_evidence: Some(proof_index),
    }
}

fn parameter_unit(class: &str) -> Option<&'static str> {
    match class {
        "moLengthParameter_c" => Some("metres"),
        "moAngleParameter_c" => Some("radians"),
        _ => None,
    }
}

fn parameter_class_declarations(
    bytes: &[u8],
    classes: &[mesh::ClassInterval],
) -> Option<Vec<MateClassDeclaration>> {
    let mut declarations = Vec::new();
    for class in classes {
        let name_at = class.content.start.checked_add(2)?;
        let framed = word(bytes, class.content.start)
            .filter(|tag| class_tag(*tag))
            .and_then(|_| native_string_at(bytes, name_at))
            .and_then(|(_, end)| parameter_value_offset(bytes, end));
        if parameter_unit(&class.name).is_some() {
            // Include declarations even when the saved scalar is nonfinite or
            // truncated. Omitting a class could falsely make another match unique.
            framed?;
            declarations.push(MateClassDeclaration {
                class_name: class.name.clone(),
                declaration_offset: class.class_offset,
            });
        } else if framed.is_some() {
            // Unknown classes with this same prefix make the inventory incomplete.
            return None;
        }
    }
    Some(declarations)
}

fn parameter_value_offset(bytes: &[u8], name_end: usize) -> Option<usize> {
    if bytes.get(name_end..name_end.checked_add(16)?)?
        != [0, 0, 0, 0, 0, 0, 0, 0x40, 255, 255, 255, 255, 0, 0, 0, 0]
    {
        return None;
    }
    let (_, note_end) = native_string_at(bytes, name_end.checked_add(16)?)?;
    if word(bytes, note_end)? != 0 {
        return None;
    }
    note_end.checked_add(2)
}

fn parameter_candidate(
    bytes: &[u8],
    (name, name_end): &(NativeString, usize),
    first: usize,
    end: usize,
    classes: &BTreeMap<usize, &mesh::ClassInterval>,
) -> Option<MateParameterCandidate> {
    let tag_at = name.offset.checked_sub(2)?;
    if name.value.is_empty() || tag_at < first || !class_tag(word(bytes, tag_at)?) {
        return None;
    }
    // Feature-derived parameter prefix: marker 2.0, sentinel identifier,
    // zero word, framed note, zero word, then the scalar. Do not scan arbitrary
    // doubles or infer a dimension from a localized feature/parameter name.
    let value_offset = parameter_value_offset(bytes, *name_end)?;
    let value_end = value_offset.checked_add(8)?;
    if value_end > end {
        return None;
    }
    let value = finite_double(bytes, value_offset)?;
    let class = classes.get(&tag_at).filter(|c| c.class_offset >= first);
    let unit = match class.map(|c| c.name.as_str()) {
        Some("moLengthParameter_c") => Some("metres"),
        Some("moAngleParameter_c") => Some("radians"),
        Some(_) => return None,
        None => None,
    };
    let class_reference = if class.is_none() {
        Some(word(bytes, tag_at.checked_sub(2)?)?).filter(|t| class_tag(*t))
    } else {
        None
    };
    if class.is_none() && class_reference.is_none() {
        return None;
    }
    Some(MateParameterCandidate {
        name: name.clone(),
        class_name: class.map(|c| c.name.clone()),
        class_reference,
        class_reference_resolution: None,
        value,
        unit,
        value_offset,
    })
}

#[cfg(test)]
mod tests;
