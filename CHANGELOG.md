<!-- SPDX-License-Identifier: MIT -->
<!-- Copyright (c) 2026 hxyulin <hxyulin@proton.me> -->
# Changelog

## Unreleased

- Add a pure Rust cached Part mesh decoder adapted from Apache-2.0 cadmpeg at
  the audited MIT sldkit snapshot, without native/IR dependencies.
- Add versioned metadata/geometry CLI and WASM outputs, object-shaped COM,
  cut lists, drawing references and preview-free JSON for simulator ingestion.
- Preserve legacy mesh order and expose source hashes, normals and offsets;
  reject ambiguous ownership and bound descriptor decoding work.

- Initial MIT-licensed Rust port of openswx's modern container and metadata
  readers, including native JSON inspection and optional WebAssembly bindings.
- Bound decoded allocations and record counts; validate sizes/checksums and
  preserve unknown component transforms and unvalidated physical cache tokens.
