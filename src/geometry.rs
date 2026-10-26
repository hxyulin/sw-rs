// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
//! Byte-based cached Part geometry adapter.
use crate::{Container, Limits, ParseError, document, mesh};
use serde::Serialize;

/// Native identity of the original input document.
#[derive(Debug, Serialize)]
pub struct GeometrySource {
    /// SHA-256 of original bytes, independent of filenames or mesh conversion.
    pub sha256: String,
}
/// Cached display triangles, with exact strip winding and decoded-source offsets.
#[derive(Debug, Serialize)]
pub struct Tessellation {
    /// Stable source stream name; no body ownership is inferred.
    pub source_stream: String,
    /// Descriptor table offset in the decoded source stream, in bytes.
    pub source_offset: usize,
    /// Adjacent decoded metadata window for optional reference inspection.
    /// Not a native object boundary; omitted from ordinary geometry JSON.
    #[serde(skip_serializing)]
    pub metadata_span: crate::ByteSpan,
    /// Cached vertex positions in millimetres, converted from source float metres.
    pub vertices: Vec<[f64; 3]>,
    /// Ordered zero-based vertex indices; strip winding is preserved.
    pub triangles: Vec<[u32; 3]>,
    /// Source strip lengths, whose sum equals vertex count.
    pub strip_lengths: Vec<u32>,
    /// Source unitless normal vectors; empty if absent in the saved table.
    pub normals: Vec<[f64; 3]>,
}
/// Saved mesh data without invented B-Rep bodies or physical mass.
#[derive(Debug, Serialize)]
pub struct GeometryModel {
    /// Validated descriptor tables in legacy lexicographic table-key order.
    pub tessellations: Vec<Tessellation>,
}
/// Partial cached Part geometry for exactly one source configuration.
#[derive(Debug, Serialize)]
pub struct GeometryDocument {
    /// Original input identity.
    pub source: GeometrySource,
    /// Output coordinate unit, currently `millimeter` for this carrier profile.
    pub length_unit: &'static str,
    /// Native model XYZ interpretation, explicitly based on the default convention.
    pub coordinate_frame: crate::CoordinateFrame,
    /// Source configurations; this profile refuses ambiguous multi-config ownership.
    pub configurations: Vec<crate::Configuration>,
    /// Saved display geometry, not a freshly evaluated feature history.
    pub model: GeometryModel,
    /// Source presentation plus cache-local links keyed by stream and descriptor offset.
    /// These cache identities do not establish native B-Rep face/body ownership.
    pub presentation: crate::PresentationData,
    /// Explicit cache and interpretation limitations.
    pub limitations: Vec<&'static str>,
}
/// Decode a modern or legacy Part's saved DisplayLists using a pure Rust profile.
///
/// The initial profile requires one canonical native configuration and one
/// DisplayLists carrier. Missing or ambiguous geometry is an error, never an
/// empty successful result. Descriptor lengths and indices are validated against
/// the actual payload; mesh counts have no policy cap. Original bytes are hashed.
///
/// ```
/// assert!(sw_rs::geometry(b"not CAD", "part.SLDPRT").is_err());
/// ```
pub fn geometry(bytes: &[u8], filename: &str) -> Result<GeometryDocument, ParseError> {
    geometry_with_limits(bytes, filename, Limits::default())
}
/// Decode saved Part geometry with caller-supplied container byte/record limits.
/// Descriptor lengths, finite coordinates and triangle indices remain validated.
pub fn geometry_with_limits(
    bytes: &[u8],
    filename: &str,
    limits: Limits,
) -> Result<GeometryDocument, ParseError> {
    if !filename.to_ascii_lowercase().ends_with(".sldprt") {
        return Err(ParseError(
            "cached geometry profile accepts Parts only".into(),
        ));
    }
    let mut container = Container::parse(bytes, limits)?;
    let (name, data) = display_carrier(container.streams.iter_mut())?;
    let name = name.clone();
    // The consuming convenience API can release unrelated decoded streams before
    // mesh allocation. Shared readers retain them intentionally for further access.
    let data = std::mem::take(data);
    let doc = document::build(container, filename);
    from_display(&data, name, &doc, crate::inventory::hash(bytes))
}

pub(crate) fn from_decoded(
    container: &Container,
    doc: &crate::Document,
    source_hash: String,
) -> Result<GeometryDocument, ParseError> {
    if doc.document_type != "part" {
        return Err(ParseError(
            "cached geometry profile accepts Parts only".into(),
        ));
    }

    let (name, data) = display_carrier(container.streams.iter())?;
    from_display(data, name.clone(), doc, source_hash)
}
fn display_carrier<'a, T>(
    streams: impl Iterator<Item = (&'a String, T)>,
) -> Result<(&'a String, T), ParseError> {
    let mut carriers = streams.filter(|(name, _)| name.rsplit('/').next() == Some("DisplayLists"));
    let first = carriers
        .next()
        .ok_or_else(|| ParseError("Part has no DisplayLists carrier".into()))?;
    let count = 1 + carriers.count();
    if count != 1 {
        return Err(ParseError(format!(
            "ambiguous Part DisplayLists carriers: found {count}"
        )));
    }
    Ok(first)
}
fn from_display(
    data: &[u8],
    name: String,
    doc: &crate::Document,
    source_hash: String,
) -> Result<GeometryDocument, ParseError> {
    let faces = mesh::section_display_faces(data);
    let mut presentation = doc.presentation();
    let cached = crate::visual::cached(data, &name, &faces, &doc.configurations);
    presentation.append_visuals(&cached);
    presentation.coverage.cached_appearance_bindings_decoded = true;
    let mut meshes: Vec<_> = faces
        .into_iter()
        .map(|face| (face.table.start, face.mesh, face.metadata))
        .enumerate()
        .collect();
    // Legacy CAD IR sorts its generated mesh IDs lexicographically. Preserve that
    // order so exact project/review signatures survive a backend replacement.
    meshes.sort_by_cached_key(|(index, _)| index.to_string());
    if meshes.is_empty() {
        // A saved environment/lighting-only DisplayLists is not malformed mesh
        // data. Keep it distinct from a known face class whose tables fail the
        // supported grammar; neither case licenses inventing B-Rep geometry.
        let reason = if mesh::has_face_tessellation_marker(data) {
            "face-class markers exist but their headers or descriptor tables are unsupported or invalid"
        } else {
            "DisplayLists has no uoTempFaceTessData_c records; no cached faces in the supported profile"
        };
        return Err(ParseError(format!(
            "Part has no supported cached display tessellation: {reason}"
        )));
    }
    if doc.configurations.len() != 1 {
        return Err(ParseError(
            "display tessellation configuration ownership is ambiguous".into(),
        ));
    }
    let mut tessellations = Vec::new();
    for (_, (offset, m, metadata)) in meshes {
        tessellations.push(Tessellation {
            source_stream: name.clone(),
            source_offset: offset,
            metadata_span: crate::ByteSpan {
                offset: metadata.start,
                length: metadata.end.saturating_sub(metadata.start),
            },
            vertices: m.vertices,
            triangles: m.triangles,
            strip_lengths: m.strip_lengths,
            normals: m.normals,
        });
    }
    Ok(GeometryDocument {
        source: GeometrySource {
            sha256: source_hash,
        },
        length_unit: "millimeter",
        coordinate_frame: crate::SOLIDWORKS_MODEL_FRAME,
        configurations: doc.configurations.clone(),
        model: GeometryModel { tessellations },
        presentation,
        limitations: vec![
            "saved_display_cache_may_be_partial_or_stale",
            "no_BRep_body_or_face_ownership",
            "single_native_configuration_only",
            "display_cache_alignment_not_independently_validated",
        ],
    })
}
