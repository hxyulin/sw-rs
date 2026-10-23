<!-- SPDX-License-Identifier: MIT -->
<!-- Copyright (c) 2026 hxyulin <hxyulin@proton.me> -->
# Changelog

## Unreleased

- Initially select the largest shallow assembly, so archives with a full model
  and many small subassemblies open the main model first.

- Follow explicit copy/export reference replacement chains and resolve embedded
  imported/virtual native documents within their owners, including Unicode streams.
- Remove viewer traversal, occurrence, triangle, vertex and descriptor-attempt
  policy caps; preserve cycle and corrupt-data checks, and keep bounded library
  container defaults available to callers.
- Build wireframe indices lazily over shared vertex buffers to reduce large-model
  memory use; surface GPU upload failures instead of silently showing an empty view.

- Label assemblies with missing references as partial while rendering available
  siblings; report skipped reference counts and original paths.
- Add a parts-list search field with case-insensitive filename/path matching,
  Unicode input, clear/no-results states and a sidebar `/` focus shortcut.

- Remove fixed viewer import byte and archive-entry caps while preserving
  integrity, path and mesh-complexity validation; browser memory still applies.
- Show local file-read bytes/percentage, ZIP extraction entry progress and
  indeterminate decompression/mesh decoding, with cancellation and error cleanup.

- Build, browser-test and deploy the viewer to GitHub Pages from main, including
  relative project-site paths and bundled license notices.

- Add an RXUI web archive viewer with worker-side native CAD decoding, assembly
  placement, metadata/configuration inspection, orbit/pan/zoom and wireframe.
- Add optional bounded ZIP/TAR/TAR.GZ imports and virtual reference resolution;
  preserve explicit ambiguity, unknown transforms and unsupported geometry.

- Add a pure Rust cached Part mesh decoder adapted from Apache-2.0 cadmpeg at
  the audited MIT sldkit snapshot, without native/IR dependencies.
- Add versioned metadata/geometry CLI and WASM outputs, object-shaped COM,
  cut lists, drawing references and preview-free JSON for simulator ingestion.
- Preserve legacy mesh order and expose source hashes, normals and offsets;
  reject ambiguous ownership and malformed descriptor tables.

- Initial MIT-licensed Rust port of openswx's modern container and metadata
  readers, including native JSON inspection and optional WebAssembly bindings.
- Bound decoded allocations and record counts; validate sizes/checksums and
  preserve unknown component transforms and unvalidated physical cache tokens.
