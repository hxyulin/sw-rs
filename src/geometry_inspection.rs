// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
//! Display-cache availability without assigning multi-configuration meshes.
use crate::{Container, Document, mesh};
use serde::Serialize;

/// Eligibility under the current cached Part geometry profile, not a B-Rep result.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GeometryCacheStatus {
    /// One carrier, supported faces and one saved configuration.
    SupportedSingleConfiguration,
    /// No stream whose final path component is DisplayLists.
    CarrierAbsent,
    /// More than one DisplayLists carrier; none is selected.
    AmbiguousCarriers,
    /// Carrier has no recognized face marker; other geometry may still exist.
    NoSupportedFaceRecords,
    /// Face markers exist, but no descriptor table satisfies the supported grammar.
    UnsupportedOrInvalidFaceRecords,
    /// Supported faces exist, but their saved configuration cannot be assigned.
    AmbiguousConfiguration,
}

/// One saved native configuration, without inferring an active configuration.
#[derive(Clone, Debug, Serialize)]
pub struct GeometryCacheConfiguration {
    /// Native Config-N index, not vector position.
    pub index: u32,
    /// Exact saved configuration name; empty when unavailable.
    pub name: String,
}

/// Summary of one supported descriptor table; vertices are intentionally omitted.
#[derive(Clone, Debug, Serialize)]
pub struct CachedFaceSummary {
    /// Descriptor-table byte offset in the enclosing decoded stream.
    pub source_offset: usize,
    /// Number of positions decoded under the current grammar.
    pub vertices: usize,
    /// Number of triangles decoded under the current grammar.
    pub triangles: usize,
}

/// A retained DisplayLists carrier; presence alone does not mean usable geometry.
#[derive(Clone, Debug, Serialize)]
pub struct DisplayCacheInspection {
    /// Exact decoded stream name; all face offsets are relative to it.
    pub source_stream: String,
    /// Size of the verified decoded carrier.
    pub decoded_bytes: usize,
    /// Whether the recognized face-class name occurs, without asserting valid framing.
    pub face_class_marker_present: bool,
    /// Supported descriptor tables, without body or configuration ownership.
    pub faces: Vec<CachedFaceSummary>,
}

/// On-demand Part cache diagnosis, including successful decoding with unknown scope.
#[derive(Clone, Debug, Serialize)]
pub struct GeometryCacheInspection {
    /// Versioned analytical contract.
    pub schema: &'static str,
    /// SHA-256 of the complete original native document.
    pub source_sha256: String,
    /// Eligibility under the existing geometry decoder's profile.
    pub status: GeometryCacheStatus,
    /// Every saved configuration; no selection by name or stream position.
    pub configurations: Vec<GeometryCacheConfiguration>,
    /// Every DisplayLists carrier; ambiguous alternatives remain separate.
    pub carriers: Vec<DisplayCacheInspection>,
    /// Config-N index only when the single-configuration geometry profile is supported.
    /// None never means configuration zero or an absent model.
    pub assigned_configuration_index: Option<u32>,
    /// Always false: counts do not establish complete or current display geometry.
    pub cache_completeness_verified: bool,
    /// Always false: B-Rep body ownership is not established by descriptor tables.
    pub body_ownership_decoded: bool,
}

pub(crate) fn inspect(
    container: &Container,
    doc: &Document,
    hash: &str,
) -> GeometryCacheInspection {
    let carriers: Vec<_> = container
        .streams
        .iter()
        .filter(|(name, _)| name.rsplit('/').next() == Some("DisplayLists"))
        .map(|(name, bytes)| {
            let faces = mesh::section_display_faces(bytes)
                .into_iter()
                .map(|face| CachedFaceSummary {
                    source_offset: face.table.start,
                    vertices: face.mesh.vertices.len(),
                    triangles: face.mesh.triangles.len(),
                })
                .collect();
            DisplayCacheInspection {
                source_stream: name.clone(),
                decoded_bytes: bytes.len(),
                face_class_marker_present: mesh::has_face_tessellation_marker(bytes),
                faces,
            }
        })
        .collect();
    let status = match carriers.as_slice() {
        [] => GeometryCacheStatus::CarrierAbsent,
        [carrier] if carrier.faces.is_empty() => {
            if carrier.face_class_marker_present {
                GeometryCacheStatus::UnsupportedOrInvalidFaceRecords
            } else {
                GeometryCacheStatus::NoSupportedFaceRecords
            }
        }
        [_] if doc.configurations.len() != 1 => GeometryCacheStatus::AmbiguousConfiguration,
        [_] => GeometryCacheStatus::SupportedSingleConfiguration,
        _ => GeometryCacheStatus::AmbiguousCarriers,
    };
    GeometryCacheInspection {
        schema: "sw-rs-geometry-cache-inspection-v1",
        source_sha256: hash.into(),
        status,
        configurations: doc
            .configurations
            .iter()
            .map(|c| GeometryCacheConfiguration {
                index: c.index,
                name: c.name.clone(),
            })
            .collect(),
        carriers,
        assigned_configuration_index: (status == GeometryCacheStatus::SupportedSingleConfiguration)
            .then(|| doc.configurations[0].index),
        cache_completeness_verified: false,
        body_ownership_decoded: false,
    }
}
