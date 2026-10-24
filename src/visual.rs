// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
// Appearance framing adapted from cadmpeg 4df21312c62cc1207a4c2274e29b4118fd232ec6.
// Modified: standalone source records, variable strings, partial coverage, no CAD IR.
// See NOTICE-cadmpeg.md and LICENSE-Apache-2.0.
//! Supported native visual-property prefixes and explicit saved cache relationships.
//! Class declarations delimit candidate intervals, not a complete MFC object graph.
use crate::{
    AppearanceBinding, AppearanceDefinition, AppearanceLayer, AppearanceOrigin, AppearanceTarget,
    Configuration, PresentationScope, PresentationSource, mesh,
};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

/// Exact framed binary string and its decoded-stream byte location.
#[derive(Clone, Debug, Serialize)]
pub struct NativeString {
    /// Exact UTF-16 string, including blank values and original path separators.
    pub value: String,
    /// Byte offset of its CString frame in the source stream.
    pub offset: usize,
}

/// Supported binary visual-property prefix, independent of physical material.
#[derive(Clone, Debug, Serialize)]
pub struct NativeAppearance {
    /// Exact primary packed-color word, including inheritance/sentinel values.
    pub packed_color: u32,
    /// Low-to-high RGB bytes only when the high byte is zero. Other saved words
    /// remain uninterpreted; no color profile or alpha/inheritance rule is assumed.
    pub color_rgb: Option<[u8; 3]>,
    /// Uninterpreted saved property flags.
    pub flags: u32,
    /// Secondary packed color, preserved without guessing its role.
    pub secondary_color: u32,
    /// Saved legacy image reference; never opened or fetched.
    pub texture_reference: NativeString,
    /// Saved appearance/material label; may be blank and does not assign physical material.
    pub material_name: NativeString,
    /// Six legacy optical doubles when all are finite; no PBR conversion is applied.
    /// Observed ordering agrees with ambient/diffuse/specular/shininess/transparency/emission.
    pub optical_values: Option<[f64; 6]>,
    /// Exact hexadecimal bits of the six doubles, including invalid/nonfinite values.
    /// Strings avoid loss of 64-bit precision in browser JSON consumers.
    pub optical_bits: [String; 6],
    /// End of the validated prefix in decoded-stream bytes.
    pub prefix_end: usize,
    /// Additional supported shader/reference fields; None means unsupported or truncated.
    pub extension: Option<NativeAppearanceExtension>,
}

/// Raw legacy texture-block fields; projection and physical units remain unverified.
#[derive(Clone, Debug, Serialize)]
pub struct NativeTextureBlock {
    /// Thirteen saved finite double values, retained in source order.
    pub values: [f64; 13],
    /// Exact reference framed inside this block, including blank values.
    pub reference: NativeString,
    /// Two additional finite doubles, without guessed scale/offset semantics.
    pub additional_values: [f64; 2],
    /// Six trailing selector words, retained without invented flag names.
    pub selectors: [u32; 6],
}

/// Source-backed fields in supported visual-property extensions.
/// Mapping numbers remain raw: their axes, units and projection semantics are unverified.
#[derive(Clone, Debug, Serialize)]
pub struct NativeAppearanceExtension {
    /// Conditional legacy texture block; distinct from the later mapping fields.
    pub legacy_texture_block: Option<NativeTextureBlock>,
    /// Saved shader parameters preceding the resource and mapping records.
    pub shader_parameters: NativeShaderParameters,
    /// Exact realtime shader identifier, not a shader implementation.
    pub shader_identifier: NativeString,
    /// Exact saved appearance-library reference, typically a p2m path.
    pub appearance_reference: NativeString,
    /// Exact saved surface-finish reference, sometimes a DDS path.
    pub surface_finish_reference: NativeString,
    /// Offset of the two stored mapping-sized scalars in decoded-stream bytes.
    pub mapping_offset: usize,
    /// Two raw mapping-sized scalars; no physical-unit interpretation is asserted.
    pub mapping_scalars: [f32; 2],
    /// Three raw mapping flag words; numeric values are not converted to an enum.
    pub mapping_flags: [u32; 3],
    /// Fourteen raw mapping values; not an established UV transform.
    pub mapping_values: [f32; 14],
    /// End of the decoded extension prefix; trailing fields remain unsupported.
    pub prefix_end: usize,
}

/// Saved shader fields whose binary framing is supported but semantics are partial.
/// These are source values, never converted to renderer/PBR coefficients.
#[derive(Clone, Debug, Serialize)]
pub struct NativeShaderParameters {
    /// Exact framed label preceding shader selection, including blank values.
    pub label: NativeString,
    /// Four saved selector words, including the unknown third selector.
    pub selectors: [u32; 4],
    /// Byte offset of the discriminator after the shader identifier.
    pub offset: usize,
    /// Raw shader discriminator byte.
    pub discriminator: u8,
    /// Raw packed word; no color or inheritance interpretation is established.
    pub packed_word: u32,
    /// Ten finite floats, preserved in order without asserting a matrix layout.
    pub values: [f32; 10],
    /// Raw word separating the two float groups.
    pub mode: u32,
    /// Five finite parameter values without guessed renderer meanings.
    pub parameters: [f32; 5],
    /// Three trailing raw flag words.
    pub flags: [u32; 3],
    /// Raw packed word following the appearance-library reference.
    pub appearance_word: u32,
    /// Two finite values preceding the surface-finish reference.
    pub finish_values: [f32; 2],
}

/// Opt-in interpretation of a raw mapping block, not verified native mapping.
///
/// The field grouping is inferred from recurring orthogonal vectors and size-like
/// values in the EGA/design-library corpus and the public IRenderMaterial API.
/// It does not establish projection type, units, handedness or entity ownership.
#[derive(Clone, Debug, Serialize)]
pub struct NativeTextureMappingHypothesis {
    /// Always `provisional`; consumers must not mistake this for decoded coverage.
    pub status: &'static str,
    /// Offset of the fourteen-float block in its definition's decoded source stream.
    pub source_offset: usize,
    /// Candidate U direction from raw values 0..3, without normalization.
    pub u_direction_candidate: [f32; 3],
    /// Candidate V direction from raw values 3..6, without normalization.
    pub v_direction_candidate: [f32; 3],
    /// Candidate origin from raw values 6..9; length unit and frame are unknown.
    pub origin_candidate: [f32; 3],
    /// Candidate repeat width/height from raw values 12..14, only if both positive.
    /// Units remain unknown; negative/default sentinels are never clamped.
    pub repeat_size_candidate: Option<[f32; 2]>,
    /// Raw values 9..12; no angle/scale meaning is asserted.
    pub remaining_values: [f32; 3],
    /// Required controlled-save comparisons before treating the candidates as decoded.
    pub verification_required: [&'static str; 3],
}

impl NativeAppearanceExtension {
    /// Offer a provisional grouping only for finite, approximately orthonormal
    /// first/second vectors. This checks plausibility, not semantic correctness.
    /// Raw fields and `texture_mappings_decoded` remain authoritative/unchanged.
    pub fn texture_mapping_hypothesis(&self) -> Option<NativeTextureMappingHypothesis> {
        let values = self.mapping_values;
        if !values.iter().all(|v| v.is_finite()) {
            return None;
        }
        let u = [values[0], values[1], values[2]];
        let v = [values[3], values[4], values[5]];
        let dot = |a: [f32; 3], b: [f32; 3]| a.into_iter().zip(b).map(|(a, b)| a * b).sum::<f32>();
        if (dot(u, u) - 1.).abs() > 1e-4 || (dot(v, v) - 1.).abs() > 1e-4 || dot(u, v).abs() > 1e-4
        {
            return None;
        }
        // TODO(native-mapping): controlled saves must verify U/V order and signs,
        // origin frame/units, and independent width/height changes. Corpus defaults
        // alone cannot distinguish these candidates from another field ordering.
        Some(NativeTextureMappingHypothesis {
            status: "provisional",
            source_offset: self.mapping_offset.checked_add(20)?,
            u_direction_candidate: u,
            v_direction_candidate: v,
            origin_candidate: [values[6], values[7], values[8]],
            repeat_size_candidate: (values[12] > 0. && values[13] > 0.)
                .then_some([values[12], values[13]]),
            remaining_values: [values[9], values[10], values[11]],
            verification_required: [
                "change U and V directions independently; verify order, signs and frame",
                "translate mapping origin; verify component/model frame and length units",
                "change width and height independently; verify units and sentinel meaning",
            ],
        })
    }
}

/// Saved state key and its independently resolved configuration and display name.
#[derive(Clone, Debug, Serialize)]
pub struct SavedDisplayState {
    /// Exact combined saved key, including brackets and underscores.
    pub key: String,
    /// Unique exact configuration prefix, or Unknown when not uniquely matched.
    pub configuration: PresentationScope,
    /// Saved suffix after a uniquely matched configuration prefix, otherwise Unknown.
    pub display_state: PresentationScope,
    /// Framed key provenance.
    pub source: PresentationSource,
}

/// Visual identity evidence without an established native entity or scope link.
#[derive(Clone, Debug, Serialize)]
pub struct UnresolvedVisualBinding {
    /// Owning serialized class, not an inferred part/face assignment.
    pub owner_class: String,
    /// Source identity words, such as a feature source ID and timestamp.
    pub identity: Vec<u32>,
    /// Definition index in this collection, adjusted by presentation projection.
    pub appearance_index: Option<usize>,
    /// Reason the source record cannot be promoted to an applied native binding.
    pub reason: String,
    /// Source record provenance.
    pub source: PresentationSource,
}

/// Unsupported or malformed visual data, without failing unrelated geometry.
#[derive(Clone, Debug, Serialize)]
pub struct VisualDiagnostic {
    /// Stable reason code, distinct from a human-readable message.
    pub code: String,
    /// Specific unverified layout or ownership limitation.
    pub message: String,
    /// Decoded source stream and byte location.
    pub source: PresentationSource,
}

/// Decoded visual evidence with explicit partial coverage; empty is not known absence.
#[derive(Clone, Debug, Default, Serialize)]
pub struct VisualData {
    /// Binary definitions, including definitions whose applied ownership is unresolved.
    pub definitions: Vec<AppearanceDefinition>,
    /// Supported explicit document or saved-cache bindings only.
    pub bindings: Vec<AppearanceBinding>,
    /// Saved framed configuration/display-state keys, preserving unknown matches.
    pub display_states: Vec<SavedDisplayState>,
    /// Source identities that cannot yet be linked to native topology/history.
    pub unresolved_bindings: Vec<UnresolvedVisualBinding>,
    /// Unsupported records and incomplete graph/texture-mapping coverage.
    pub diagnostics: Vec<VisualDiagnostic>,
}

fn source(stream: &str, offset: usize) -> PresentationSource {
    PresentationSource {
        stream: stream.into(),
        path: format!("@byte[{offset}]"),
    }
}
fn u32_at(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        bytes.get(at..at.checked_add(4)?)?.try_into().ok()?,
    ))
}
struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}
impl Reader<'_> {
    fn take(&mut self, len: usize) -> Option<&[u8]> {
        let end = self.at.checked_add(len)?;
        let bytes = self.bytes.get(self.at..end)?;
        self.at = end;
        Some(bytes)
    }
    fn u32(&mut self) -> Option<u32> {
        Some(u32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }
    fn words<const N: usize>(&mut self) -> Option<[u32; N]> {
        let mut values = [0; N];
        for value in &mut values {
            *value = self.u32()?;
        }
        Some(values)
    }
    fn doubles<const N: usize>(&mut self) -> Option<[f64; N]> {
        let mut values = [0.; N];
        for value in &mut values {
            *value = f64::from_le_bytes(self.take(8)?.try_into().ok()?);
        }
        values.iter().all(|v| v.is_finite()).then_some(values)
    }
    fn floats<const N: usize>(&mut self) -> Option<[f32; N]> {
        let values = self.words::<N>()?.map(f32::from_bits);
        values.iter().all(|v| v.is_finite()).then_some(values)
    }
    fn string(&mut self) -> Option<NativeString> {
        let offset = self.at;
        if self.take(3)? != [0xff, 0xfe, 0xff] {
            return None;
        }
        let first = self.take(1)?[0];
        let count = match first {
            254 => u16::from_le_bytes(self.take(2)?.try_into().ok()?) as usize,
            255 => usize::try_from(self.u32()?).ok()?,
            count => count as usize,
        };
        let units = self
            .take(count.checked_mul(2)?)?
            .as_chunks::<2>()
            .0
            .iter()
            .map(|b| u16::from_le_bytes(*b))
            .collect::<Vec<_>>();
        Some(NativeString {
            value: String::from_utf16(&units).ok()?,
            offset,
        })
    }
}

pub(crate) fn native_string_at(bytes: &[u8], at: usize) -> Option<(NativeString, usize)> {
    let mut reader = Reader { bytes, at };
    let value = reader.string()?;
    Some((value, reader.at))
}

fn extension(reader: &mut Reader<'_>) -> Option<NativeAppearanceExtension> {
    let legacy_texture_block = match reader.u32()? {
        0 => {
            if reader.words::<5>()? != [1, 0, 1, 0, 1] {
                return None;
            }
            None
        }
        1 => {
            let values = reader.doubles::<13>()?;
            let reference = reader.string()?;
            if reader.take(2)? != [0, 0] || reader.u32()? != 0 {
                return None;
            }
            let additional_values = reader.doubles::<2>()?;
            let selectors = reader.words::<6>()?;
            if selectors != [0, 1, 1, 0, 1, 1] {
                return None;
            }
            Some(NativeTextureBlock {
                values,
                reference,
                additional_values,
                selectors,
            })
        }
        _ => return None,
    };
    let label = reader.string()?;
    let selectors = reader.words::<4>()?;
    if selectors[0] != 1 || selectors[1] != 1 || selectors[3] != 0 {
        return None;
    }
    let shader_identifier = reader.string()?;
    let offset = reader.at;
    let discriminator = reader.take(1)?[0];
    let packed_word = reader.u32()?;
    let values = reader.floats::<10>()?;
    let mode = reader.u32()?;
    let parameters = reader.floats::<5>()?;
    let flags = reader.words::<3>()?;
    let appearance_reference = reader.string()?;
    let appearance_word = reader.u32()?;
    let finish_values = reader.floats::<2>()?;
    let surface_finish_reference = reader.string()?;
    let mapping_offset = reader.at;
    let mapping_scalars = reader.floats::<2>()?;
    let mapping_flags = reader.words::<3>()?;
    let mapping_values = reader.floats::<14>()?;
    reader.string()?;
    Some(NativeAppearanceExtension {
        legacy_texture_block,
        shader_parameters: NativeShaderParameters {
            label,
            selectors,
            offset,
            discriminator,
            packed_word,
            values,
            mode,
            parameters,
            flags,
            appearance_word,
            finish_values,
        },
        shader_identifier,
        appearance_reference,
        surface_finish_reference,
        mapping_offset,
        mapping_scalars,
        mapping_flags,
        mapping_values,
        prefix_end: reader.at,
    })
}

fn definition(bytes: &[u8], at: usize, stream: &str) -> Option<AppearanceDefinition> {
    let mut reader = Reader { bytes, at };
    let packed = reader.u32()?;
    let flags = reader.u32()?;
    let secondary_color = reader.u32()?;
    let texture_reference = reader.string()?;
    let material_name = reader.string()?;
    let mut optical_bits = [0; 6];
    for value in &mut optical_bits {
        *value = u64::from_le_bytes(reader.take(8)?.try_into().ok()?);
    }
    let optical = optical_bits.map(f64::from_bits);
    let optical_values = optical.iter().all(|v| v.is_finite()).then_some(optical);
    let [r, g, b, _] = packed.to_le_bytes();
    let native = NativeAppearance {
        packed_color: packed,
        color_rgb: (packed >> 24 == 0).then_some([r, g, b]),
        flags,
        secondary_color,
        texture_reference,
        material_name,
        optical_values,
        optical_bits: optical_bits.map(|bits| format!("{bits:016X}")),
        prefix_end: reader.at,
        extension: extension(&mut reader),
    };
    Some(AppearanceDefinition {
        origin: AppearanceOrigin::SavedAppearance,
        swatch_rgb: (packed >> 24 == 0).then(|| format!("{r:02X}{g:02X}{b:02X}")),
        saved: None,
        native: Some(native),
        source: source(stream, at),
    })
}

fn diagnostic(out: &mut VisualData, stream: &str, at: usize, code: &str, message: &str) {
    out.diagnostics.push(VisualDiagnostic {
        code: code.into(),
        message: message.into(),
        source: source(stream, at),
    });
}

fn state_key(
    key: NativeString,
    stream: &str,
    configurations: &[Configuration],
) -> SavedDisplayState {
    let candidates = configurations
        .iter()
        .filter_map(|configuration| {
            if configuration.name.is_empty() {
                return None;
            }
            let names = [
                format!("{}_", configuration.name),
                format!("<{}>_", configuration.name),
            ];
            let matches: BTreeSet<_> = names
                .iter()
                .filter_map(|prefix| key.value.strip_prefix(prefix))
                .filter(|name| !name.is_empty())
                .collect();
            (matches.len() == 1).then(|| {
                (
                    configuration.name.clone(),
                    (*matches.first().unwrap()).to_owned(),
                )
            })
        })
        .collect::<Vec<_>>();
    let (configuration, display_state) = match candidates.as_slice() {
        [(configuration, display)] => (
            PresentationScope::Named {
                name: configuration.clone(),
            },
            PresentationScope::Named {
                name: display.clone(),
            },
        ),
        _ => (PresentationScope::Unknown, PresentationScope::Unknown),
    };
    SavedDisplayState {
        key: key.value,
        configuration,
        display_state,
        source: source(stream, key.offset),
    }
}

/// Decode scoped prefix evidence; unsupported MFC graph branches stay unresolved.
pub(crate) fn parse(
    streams: &BTreeMap<String, Vec<u8>>,
    configurations: &[Configuration],
    kind: &str,
) -> VisualData {
    let mut out = VisualData::default();
    for (stream, bytes) in streams
        .iter()
        .filter(|(name, _)| name.as_str() == "ThirdPtyStore/VisualStates")
    {
        let classes = mesh::class_intervals(bytes);
        let state_start = out.display_states.len();
        for class in classes
            .iter()
            .filter(|class| class.name == "visStateKeyData_c")
        {
            // Exact framed strings in the key interval, not arbitrary path/name scans.
            let mut at = class.content.start;
            while at < class.content.end {
                if bytes.get(at..at.saturating_add(3)) == Some(&[0xff, 0xfe, 0xff]) {
                    let mut reader = Reader {
                        bytes: &bytes[..class.content.end],
                        at,
                    };
                    if let Some(key) = reader.string() {
                        at = reader.at;
                        if !key.value.is_empty() {
                            out.display_states
                                .push(state_key(key, stream, configurations));
                        }
                        continue;
                    }
                }
                at += 1;
            }
        }
        let supported_root = matches!(
            u32_at(bytes, 0),
            Some(12000 | 14000 | 16000 | 17000 | 18000)
        ) && u32_at(bytes, 4) == Some(0)
            && classes
                .first()
                .is_some_and(|c| c.name == "visStateKeyData_c" && c.class_offset == 8);
        let state = match &out.display_states[state_start..] {
            [state] if matches!(state.configuration, PresentationScope::Named { .. }) => {
                Some(state.clone())
            }
            _ => None,
        };
        for (index, class) in classes
            .iter()
            .enumerate()
            .filter(|(_, c)| c.name == "moVisualProperties_c")
        {
            let Some(value) = definition(&bytes[..class.content.end], class.content.start, stream)
            else {
                diagnostic(
                    &mut out,
                    stream,
                    class.content.start,
                    "unsupported_visual_property_prefix",
                    "Visual-property declaration has an unsupported or truncated prefix",
                );
                continue;
            };
            let appearance_index = out.definitions.len();
            out.definitions.push(value);
            let previous = classes[..index]
                .iter()
                .rev()
                .take(4)
                .map(|c| c.name.as_str())
                .collect::<Vec<_>>();
            let part_chain = previous.starts_with(&[
                "visStateVisualPropertiesAttribute_c",
                "moUnitComponent_c",
                "moCompHolder_c",
            ]) || previous.starts_with(&[
                "visStateVisualStateAttribute_c",
                "visStateVisualPropertiesAttribute_c",
                "moUnitComponent_c",
                "moCompHolder_c",
            ]) || previous.starts_with(&[
                "moCompHolder_c",
                "visStateVisualStateAttribute_c",
                "visStateVisualPropertiesAttribute_c",
                "moUnitComponent_c",
            ]);
            let attribute = classes[..index]
                .iter()
                .rev()
                .find(|c| c.name == "visStateVisualPropertiesAttribute_c");
            let valid_attribute = attribute.is_some_and(|c| {
                matches!(
                    &bytes[c.content.start..c.content.end],
                    [1, 0, 0, 0] | [1, 0, 0, 0, 0, 0]
                )
            });
            if kind == "part"
                && supported_root
                && part_chain
                && valid_attribute
                && let Some(state) = &state
            {
                out.bindings.push(AppearanceBinding {
                    layer: AppearanceLayer::Document,
                    target: AppearanceTarget::Document,
                    configuration: state.configuration.clone(),
                    display_state: state.display_state.clone(),
                    appearance_index: Some(appearance_index),
                    reference: format!("visual-property@{}", class.content.start),
                    source: source(stream, attribute.unwrap().content.start),
                });
            } else {
                out.unresolved_bindings.push(UnresolvedVisualBinding {
                    owner_class: class.name.clone(),
                    identity: vec![],
                    appearance_index: Some(appearance_index),
                    reason:
                        "No unique supported document-owner and configuration/display-state chain"
                            .into(),
                    source: source(stream, class.content.start),
                });
            }
        }
        // This supported feature prefix is source identity evidence only. Matching
        // it to current native feature history/B-Rep ownership is a separate step.
        for class in classes.iter().filter(|c| c.name == "moCompFeature_c") {
            let part = &bytes[..class.content.end];
            for at in class.content.start..class.content.end.saturating_sub(35) {
                if !matches!(u32_at(part, at), Some(17000 | 18000))
                    || [12, 16, 20].map(|offset| u32_at(part, at + offset))
                        != [Some(1), Some(1), Some(2)]
                    || part.get(at + 24..at + 32) != Some(&[7, 128, 1, 0, 0, 0, 9, 128])
                {
                    continue;
                }
                let (Some(id), Some(timestamp)) = (u32_at(part, at + 4), u32_at(part, at + 8))
                else {
                    continue;
                };
                if [id, timestamp].iter().any(|v| *v == 0 || *v == u32::MAX) {
                    continue;
                }
                let Some(value) = definition(part, at + 32, stream) else {
                    continue;
                };
                let appearance_index = out.definitions.len();
                out.definitions.push(value);
                out.unresolved_bindings.push(UnresolvedVisualBinding { owner_class: class.name.clone(), identity: vec![id,timestamp], appearance_index: Some(appearance_index), reason: "Feature source/timestamp decoded; current feature-history and persistent surface ownership are not decoded".into(), source: source(stream,at) });
            }
        }
        for class in classes.iter().filter(|c| {
            matches!(
                c.name.as_str(),
                "moCompFace_c"
                    | "moCompSolidBody_c"
                    | "moCompSurfaceBody_c"
                    | "moContextChainComponent_c"
            )
        }) {
            let bytes = &bytes[..class.content.end];
            let at = class.content.start;
            let identity = if class.name == "moCompSolidBody_c" {
                let tag = bytes
                    .get(at..at + 2)
                    .map(|b| u16::from_le_bytes([b[0], b[1]]));
                match (tag, u32_at(bytes, at + 9), u32_at(bytes, at + 13)) {
                    (Some(tag), Some(id), Some(copy))
                        if tag > 0x8000
                            && tag != u16::MAX
                            && id != 0
                            && id != u32::MAX
                            && id == copy
                            && bytes.get(at + 2..at + 9) == Some(&[0; 7]) =>
                    {
                        vec![id]
                    }
                    _ => vec![],
                }
            } else {
                vec![]
            };
            out.unresolved_bindings.push(UnresolvedVisualBinding { owner_class: class.name.clone(), identity, appearance_index:None,
                reason:"Native component/body/face reference graph not decoded; appearance ownership not inferred from nearby records".into(), source:source(stream,at) });
        }
        diagnostic(
            &mut out,
            stream,
            0,
            "partial_visual_graph",
            "Class-reuse graph, component/face/body ownership and texture projection semantics remain incomplete; decoded defaults are not complete effective surface appearances",
        );
    }
    out
}

/// Cache-local assignments from validated face-property slots and body property groups.
pub(crate) fn cached(
    bytes: &[u8],
    stream: &str,
    faces: &[mesh::DisplayFace],
    configurations: &[Configuration],
) -> VisualData {
    let classes = mesh::class_intervals(bytes);
    let mut out = VisualData::default();
    let configuration = match configurations {
        [configuration] => PresentationScope::Named {
            name: configuration.name.clone(),
        },
        _ => PresentationScope::Unknown,
    };
    let mut definition_indices = BTreeMap::new();
    let mut bind = |value: &AppearanceDefinition, face: &mesh::DisplayFace, layer| {
        let provenance = value.source.clone();
        let appearance_index = *definition_indices
            .entry(provenance.path.clone())
            .or_insert_with(|| {
                let index = out.definitions.len();
                out.definitions.push(value.clone());
                index
            });
        out.bindings.push(AppearanceBinding {
            layer,
            target: AppearanceTarget::Tessellation {
                source_stream: stream.into(),
                source_offset: face.table.start,
            },
            configuration: configuration.clone(),
            display_state: PresentationScope::Unknown,
            appearance_index: Some(appearance_index),
            reference: format!("display-property:{}", provenance.path),
            source: provenance,
        });
    };
    for face in faces {
        let Some(class) = classes.iter().find(|c| {
            c.name == "uoTempFaceTessData_c"
                && c.content.start <= face.table.start
                && face.table.end <= c.content.end
        }) else {
            continue;
        };
        for value in inline(
            bytes,
            stream,
            face.metadata.start,
            face.metadata.end.min(class.content.end),
        ) {
            bind(&value, face, AppearanceLayer::CachedFace);
        }
    }
    for (index, class) in classes
        .iter()
        .enumerate()
        .filter(|(_, c)| c.name == "uoBodyPropInfo_c")
    {
        let previous = classes[..index]
            .iter()
            .rev()
            .find(|c| c.name == "uoBodyPropInfo_c")
            .map_or(0, |c| c.content.end);
        // Decode only the body's own property slot. The declaration interval can
        // include reused face/body objects; scanning that entire interval would
        // incorrectly assign a later object's color to this body group.
        if let Some(value) = body_property(&bytes[..class.content.end], stream, class.content.start)
        {
            for face in faces
                .iter()
                .filter(|f| previous <= f.table.start && f.table.end <= class.class_offset)
            {
                bind(&value, face, AppearanceLayer::CachedBody);
            }
        }
    }
    out
}
fn body_property(bytes: &[u8], stream: &str, at: usize) -> Option<AppearanceDefinition> {
    if bytes.get(at..at.checked_add(2)?) == Some(&[0x33, 0x80]) {
        return definition(bytes, at + 2, stream);
    }
    // Observed body-property header: null slot, state selectors, two booleans,
    // reserved word, body label, reserved word, present marker, sentinel, object.
    if bytes.get(at..at.checked_add(2)?) != Some(&[0, 0])
        || u32_at(bytes, at + 2) != Some(1)
        || u32_at(bytes, at + 6) != Some(6)
        || !matches!(u32_at(bytes, at + 10), Some(0 | 1))
        || !matches!(u32_at(bytes, at + 14), Some(0 | 1))
        || u32_at(bytes, at + 18) != Some(0)
    {
        return None;
    }
    let mut reader = Reader { bytes, at: at + 22 };
    reader.string()?;
    if reader.words::<3>()? != [0, 1, u32::MAX] {
        return None;
    }
    let tag = u16::from_le_bytes(reader.take(2)?.try_into().ok()?);
    if tag <= 0x8000 || tag == 0xffff {
        return None;
    }
    definition(bytes, reader.at, stream)
}

fn inline(bytes: &[u8], stream: &str, start: usize, end: usize) -> Vec<AppearanceDefinition> {
    let Some(part) = bytes.get(start..end) else {
        return vec![];
    };
    let mut values = Vec::new();
    let mut relative = 0;
    while relative + 2 <= part.len() {
        if part[relative..relative + 2] == [0x33, 0x80]
            && let Some(value) = definition(&bytes[..end], start + relative + 2, stream)
        {
            let native = value.native.as_ref().expect("binary definition");
            let next = native
                .extension
                .as_ref()
                .map_or(native.prefix_end, |extension| extension.prefix_end);
            relative = next - start;
            values.push(value);
            continue;
        }
        relative += 1;
    }
    values
}

#[cfg(test)]
mod tests;
