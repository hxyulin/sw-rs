// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
//! Byte-oriented SolidWorks 2015+ metadata reader, ported from MIT openswx.
//! No filesystem, host clock, native CAD runtime, or C++ dependency is required.
//!
//! ```
//! assert!(sw_rs::parse(b"not CAD", "part.SLDPRT").is_err());
//! ```
mod config;
mod container;
mod document;
mod frame;
mod geometry;
mod mesh;
mod xml;
pub use container::{Container, Diagnostic, Limits, ParseError};
pub use document::{
    Component, Configuration, CutListItem, Document, MassProperties, MaterialAssignment,
    MaterialData, MaterialDensity, ReferenceReplacement, SavedAppearance, SavedAppearanceRecord,
    SavedMaterial, Sheet, SheetView,
};
pub use frame::{CoordinateFrame, SOLIDWORKS_MODEL_FRAME};
pub use geometry::{
    GeometryDocument, GeometryModel, GeometrySource, Tessellation, geometry, geometry_with_limits,
};

/// Parse a modern document using bounded default resource limits.
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
