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
mod xml;
pub use container::{Container, Diagnostic, Limits, ParseError};
pub use document::{Component, Configuration, Document, MassProperties};

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

/// Parse an uploaded Uint8Array into JSON. Errors become JavaScript exceptions.
/// Build with `wasm-pack build --target web --features wasm`.
#[cfg(feature = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn parse_json(bytes: &[u8], filename: &str) -> Result<String, wasm_bindgen::JsError> {
    let doc = parse(bytes, filename).map_err(|e| wasm_bindgen::JsError::new(&e.to_string()))?;
    serde_json::to_string(&doc).map_err(|e| wasm_bindgen::JsError::new(&e.to_string()))
}
