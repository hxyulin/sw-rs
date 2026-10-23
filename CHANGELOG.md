<!-- SPDX-License-Identifier: MIT -->
<!-- Copyright (c) 2026 hxyulin <hxyulin@proton.me> -->
# Changelog

## Unreleased

- Declare the assumed native SolidWorks model XYZ frame with explicit provenance;
  default the viewer to Y-up, add standard views/source-axis triad and a Z-up
  camera override without changing geometry or assembly transforms.

- Preserve saved shader/texture references and optical attributes; add saved
  material appearance and physical-family styles without redecoding geometry.
- Approximate known saved metal/plastic/fiber styles with procedural textures
  and valid diffuse/specularity/emission coefficients; keep unknown data explicit.

- Choose an assembly when opening archives with multiple assemblies, reopen the
  chooser, and browse an expandable occurrence hierarchy with component status.
- Expose saved physical-material definitions and exact configuration references,
  including source paths and raw density tokens, and show them in the inspector.
- Render assigned aluminum and carbon fiber with procedural brushed-metal and
  weave presets, generic materials with saved swatches, and provide diagnostic colors.
- Distinguish missing display carriers, absent recognized faces and unreadable
  face records; add a private-corpus stream-inventory example for further decoding.

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
