// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
//! Decode-once source reader with on-demand metadata, geometry and structural inspection.
use crate::{
    Container, ContainerRecord, Document, DocumentInventory, GeometryDocument, Limits, ParseError,
    StreamInspection,
};
use std::cell::OnceCell;

/// Results requested from a document reader, transferred without copying mesh buffers.
/// Decoded streams remain available for embedded-file extraction or further decoders.
#[derive(Debug)]
pub struct ReadOutput {
    /// Verified first-valid decoded streams and container diagnostics.
    pub container: Container,
    /// Cached metadata, if requested before finishing.
    pub metadata: Option<Document>,
    /// Cached geometry success/failure, if requested before finishing.
    pub geometry: Option<Result<GeometryDocument, ParseError>>,
    /// Source inventory, if requested before finishing.
    pub inventory: Option<DocumentInventory>,
    /// Requested feature catalog, transferred without rebuilding.
    pub features: Option<crate::FeatureCatalog>,
    /// Requested source-reference graph; candidate links remain unverified.
    pub references: Option<crate::ReferenceGraph>,
}

/// Borrow source bytes and decompress/validate their container exactly once.
/// Metadata and geometry are cached only when requested; stream inspection is explicit.
/// This is an eager container reader, not lazy decompression. Source bytes are borrowed,
/// decoded buffers are owned, and all accessors return borrowed views where possible.
///
/// Unsupported/corrupt container errors remain available through `inventory()`;
/// metadata/geometry/stream access returns the original error. File-size violations
/// fail `open` before hashing or decoding. No paths or external resources are opened.
///
/// ```
/// let reader = sw_rs::DocumentReader::open(b"unknown", "part.SLDPRT", sw_rs::Limits::default())?;
/// assert!(reader.inventory().container_error.is_some());
/// assert!(reader.metadata().is_err());
/// # Ok::<(), sw_rs::ParseError>(())
/// ```
#[derive(Debug)]
pub struct DocumentReader<'a> {
    bytes: &'a [u8],
    filename: String,
    source_hash: String,
    records: Vec<ContainerRecord>,
    container: Result<Container, ParseError>,
    metadata: OnceCell<Document>,
    geometry: OnceCell<Result<GeometryDocument, ParseError>>,
    inventory: OnceCell<DocumentInventory>,
    features: OnceCell<crate::FeatureCatalog>,
    references: OnceCell<crate::ReferenceGraph>,
}
impl<'a> DocumentReader<'a> {
    /// Validate/decompress once, retaining trace records and container errors for inventory.
    pub fn open(bytes: &'a [u8], filename: &str, limits: Limits) -> Result<Self, ParseError> {
        if bytes.len() > limits.max_file_bytes {
            return Err(ParseError("input size limit exceeded".into()));
        }
        let mut records = Vec::new();
        let container = Container::parse_traced(bytes, limits, Some(&mut records));
        Ok(Self {
            bytes,
            filename: filename.into(),
            source_hash: crate::inventory::hash(bytes),
            records,
            container,
            metadata: OnceCell::new(),
            geometry: OnceCell::new(),
            inventory: OnceCell::new(),
            features: OnceCell::new(),
            references: OnceCell::new(),
        })
    }
    /// Borrow original encoded source bytes; no copy or decompression occurs.
    pub fn source_bytes(&self) -> &'a [u8] {
        self.bytes
    }
    /// SHA-256 of the original source bytes, computed once when opening the reader.
    pub fn source_sha256(&self) -> &str {
        &self.source_hash
    }
    /// Borrow the single verified decoded container, or return its original error.
    pub fn container(&self) -> Result<&Container, ParseError> {
        self.container.as_ref().map_err(Clone::clone)
    }
    /// Borrow one selected decoded stream without copying or reparsing.
    pub fn stream(&self, name: &str) -> Result<Option<&[u8]>, ParseError> {
        Ok(self.container()?.streams.get(name).map(Vec::as_slice))
    }
    /// Parse metadata once from the retained decoded streams, then return the cached view.
    pub fn metadata(&self) -> Result<&Document, ParseError> {
        let container = self.container()?;
        Ok(self
            .metadata
            .get_or_init(|| crate::document::build_borrowed(container, &self.filename)))
    }
    /// Decode cached Part geometry once from the same streams and metadata.
    /// Errors are cached too. No input re-inflation, carrier clone or mesh clone occurs.
    pub fn geometry(&self) -> Result<&GeometryDocument, ParseError> {
        let container = self.container()?;
        let doc = self.metadata()?;
        self.geometry
            .get_or_init(|| crate::geometry::from_decoded(container, doc, self.source_hash.clone()))
            .as_ref()
            .map_err(Clone::clone)
    }
    /// Inspect source records and accounting without invoking semantic or geometry decoders.
    /// Unsupported file types retain an all-opaque inventory and explicit container error.
    pub fn inventory(&self) -> &DocumentInventory {
        self.inventory.get_or_init(|| {
            crate::inventory::inventory(
                self.bytes,
                &self.source_hash,
                &self.records,
                &self.container,
            )
        })
    }
    /// Structurally inspect a selected decoded stream on demand.
    /// Produces XML records or binary declaration/string candidates; does not resolve
    /// native object ownership. Results are owned and intentionally not cached here.
    pub fn inspect_stream(&self, name: &str) -> Result<Option<StreamInspection>, ParseError> {
        Ok(self
            .stream(name)?
            .map(|bytes| crate::inventory::inspect(name, bytes)))
    }
    /// Decode saved feature-tree identities once, retaining duplicates and raw tokens.
    /// Tree order is not assumed to be regeneration order; no features are evaluated.
    pub fn features(&self) -> Result<&crate::FeatureCatalog, ParseError> {
        let container = self.container()?;
        Ok(self.features.get_or_init(|| {
            let inspection = container.inspect_stream("swXmlContents/KeyWords");
            crate::FeatureCatalog::parse(inspection.as_ref())
        }))
    }
    /// Connect decoded source records with scoped, typed reference evidence once.
    /// XML links can resolve syntactically; native ID matches remain candidates.
    /// Does not decode geometry or scan arbitrary binary strings for ID matches.
    pub fn references(&self) -> Result<&crate::ReferenceGraph, ParseError> {
        let container = self.container()?;
        let features = self.features()?;
        let metadata = self.metadata()?;
        Ok(self.references.get_or_init(|| {
            let xml = ["swXmlContents/Features", "swXmlContents/COMPINSTANCETREE"]
                .iter()
                .filter_map(|name| container.inspect_stream(name))
                .collect::<Vec<_>>();
            crate::ReferenceGraph::build(&self.source_hash, features, metadata, &xml)
        }))
    }
    /// Transfer retained streams and requested cached results without copying their buffers.
    /// Source-file errors remain errors; unused projections remain None.
    pub fn finish(self) -> Result<ReadOutput, ParseError> {
        Ok(ReadOutput {
            container: self.container?,
            metadata: self.metadata.into_inner(),
            geometry: self.geometry.into_inner(),
            inventory: self.inventory.into_inner(),
            features: self.features.into_inner(),
            references: self.references.into_inner(),
        })
    }
}
