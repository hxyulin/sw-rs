// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
//! Byte-oriented SolidWorks metadata reader with modern inline and legacy OLE loading.
//! No filesystem, host clock, native CAD runtime, or C++ dependency is required.
//!
//! ```
//! assert!(sw_rs::parse(b"not CAD", "part.SLDPRT").is_err());
//! ```
mod binary;
mod config;
mod container;
mod document;
mod features;
mod frame;
mod geometry;
mod inventory;
mod mates;
mod mesh;
mod native_reference;
mod object_archive;
mod ole;
mod presentation;
mod reader;
mod references;
mod surface;
mod visual;
mod xml;
mod zlb;
pub use binary::{BinaryReader, FramedString};
pub use container::{Container, Diagnostic, Limits, ParseError};
pub use document::{
    Component, Configuration, CutListItem, Document, MassProperties, MaterialAssignment,
    MaterialData, MaterialDensity, ReferenceReplacement, SavedAppearance, SavedAppearanceRecord,
    SavedMaterial, Sheet, SheetView,
};
pub use features::{FeatureCatalog, SavedFeature};
pub use frame::{CoordinateFrame, SOLIDWORKS_MODEL_FRAME};
pub use geometry::{
    GeometryDocument, GeometryModel, GeometrySource, Tessellation, geometry, geometry_with_limits,
};
pub use inventory::{
    ByteSpan, ClassDeclaration, ContainerRecord, ContainerRecordStatus, DocumentInventory,
    SourceFormat, StreamInspection, StreamInventory, XmlAttribute, XmlElement, XmlReference,
    XmlText,
};
pub use mates::{
    ConfigurationMates, MateData, MateDiagnostic, MateKind, MateRecord, MateReferenceCandidate,
    MateReferenceNode,
};
pub use native_reference::{
    NativeDocumentReference, NativeDocumentReferenceCandidate, NativeHandleClassEvidence,
    NativeReferenceStop,
};
pub use object_archive::{ArchiveContext, ArchiveEntry, ArchiveError, ArchiveObjectHeader};
pub use ole::{OleCompression, OleStreamSource};
pub use presentation::{
    AppearanceBinding, AppearanceDefinition, AppearanceLayer, AppearanceOrigin, AppearanceTarget,
    CachedPropertyResolution, PhysicalMaterialBinding, PhysicalMaterialDefinition,
    PresentationCoverage, PresentationData, PresentationScope, PresentationSource,
};
pub use reader::{DocumentReader, ReadOutput};
pub use references::{
    ReferenceGraph, ReferenceLink, ReferenceNode, ReferenceRecord, ReferenceRelation,
    ReferenceResolution,
};
pub use surface::{
    CachedSurfaceData, CachedSurfaceRecord, CachedSurfaceReferences, SavedSurfaceIdentity,
    SavedSurfaceReference, SurfaceIdentityResolution, SurfaceReferenceStatus,
};
pub use visual::{
    NativeAppearance, NativeAppearanceExtension, NativeShaderParameters, NativeString,
    NativeTextureBlock, NativeTextureMappingHypothesis, SavedDisplayState, UnresolvedVisualBinding,
    VisualData, VisualDiagnostic,
};
pub use zlb::{ZlbChunk, ZlbStream, decode_zlb};

/// Parse a modern inline or legacy OLE document using bounded default limits.
/// `filename` supplies the document kind; it is never opened or resolved.
pub fn parse(bytes: &[u8], filename: &str) -> Result<Document, ParseError> {
    parse_with_limits(bytes, filename, Limits::default())
}

/// Parse a document with caller-supplied allocation and record limits.
pub fn parse_with_limits(
    bytes: &[u8],
    filename: &str,
    limits: Limits,
) -> Result<Document, ParseError> {
    let container = Container::parse(bytes, limits)?;
    Ok(document::build(container, filename))
}

/// Interpret metadata from an already decoded container, consuming its streams.
/// This lets hosts retain embedded native documents without inflating twice.
pub fn metadata_from_container(container: Container, filename: &str) -> Document {
    document::build(container, filename)
}

/// Parse an uploaded Uint8Array into JSON. Errors become JavaScript exceptions.
/// Build with `wasm-pack build --target web --features wasm`.
#[cfg(feature = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn parse_json(bytes: &[u8], filename: &str) -> Result<String, wasm_bindgen::JsError> {
    let doc = parse(bytes, filename).map_err(|e| wasm_bindgen::JsError::new(&e.to_string()))?;
    serde_json::to_string(&doc).map_err(|e| wasm_bindgen::JsError::new(&e.to_string()))
}

/// Decode cached Part tessellations from a browser Uint8Array into versioned JSON.
/// This does not evaluate a B-Rep or validate saved cache freshness.
#[cfg(feature = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn geometry_json(bytes: &[u8], filename: &str) -> Result<String, wasm_bindgen::JsError> {
    let doc = geometry(bytes, filename).map_err(|e| wasm_bindgen::JsError::new(&e.to_string()))?;
    serde_json::to_string(&serde_json::json!({"schema":"sw-rs-geometry-v1", "geometry":doc}))
        .map_err(|e| wasm_bindgen::JsError::new(&e.to_string()))
}

/// Bounded archive imports and an in-memory, filesystem-independent CAD file set.
#[cfg(feature = "archive")]
pub mod archive;

/// Parse renderer-independent material/appearance relationships into versioned JSON.
/// Texture paths remain inert; this does not execute a saved shader or resolve UVs.
#[cfg(feature = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn presentation_json(bytes: &[u8], filename: &str) -> Result<String, wasm_bindgen::JsError> {
    let doc = parse(bytes, filename).map_err(|e| wasm_bindgen::JsError::new(&e.to_string()))?;
    serde_json::to_string(&doc.presentation())
        .map_err(|e| wasm_bindgen::JsError::new(&e.to_string()))
}

/// Inspect source records and opaque byte ranges as JSON without semantic decoding.
/// Recognized unsupported containers produce an inventory with an explicit error.
#[cfg(feature = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn inventory_json(bytes: &[u8], filename: &str) -> Result<String, wasm_bindgen::JsError> {
    let reader = DocumentReader::open(bytes, filename, Limits::default())
        .map_err(|e| wasm_bindgen::JsError::new(&e.to_string()))?;
    serde_json::to_string(reader.inventory())
        .map_err(|e| wasm_bindgen::JsError::new(&e.to_string()))
}

/// Inspect saved feature identities and scoped references without resolving topology.
#[cfg(feature = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn references_json(bytes: &[u8], filename: &str) -> Result<String, wasm_bindgen::JsError> {
    let reader = DocumentReader::open(bytes, filename, Limits::default())
        .map_err(|e| wasm_bindgen::JsError::new(&e.to_string()))?;
    let graph = reader
        .references()
        .map_err(|e| wasm_bindgen::JsError::new(&e.to_string()))?;
    serde_json::to_string(graph).map_err(|e| wasm_bindgen::JsError::new(&e.to_string()))
}

/// Compose same-source cached Part surface evidence with source-reference JSON.
/// Requires canonical cached geometry; native face/body ownership remains unresolved.
#[cfg(feature = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn geometry_references_json(
    bytes: &[u8],
    filename: &str,
) -> Result<String, wasm_bindgen::JsError> {
    let reader = DocumentReader::open(bytes, filename, Limits::default())
        .map_err(|e| wasm_bindgen::JsError::new(&e.to_string()))?;
    let graph = reader
        .references_with_geometry()
        .map_err(|e| wasm_bindgen::JsError::new(&e.to_string()))?;
    serde_json::to_string(&graph).map_err(|e| wasm_bindgen::JsError::new(&e.to_string()))
}
