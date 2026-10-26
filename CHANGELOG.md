<!-- SPDX-License-Identifier: MIT -->
<!-- Copyright (c) 2026 hxyulin <hxyulin@proton.me> -->
# Changelog

## Unreleased

- Make `ArchiveObjectHeader` pointer state read-only. Replace direct access to
  `object_index`, `class_index` and `is_new` with methods of the same names.
  Headers come from `ArchiveContext::read_object`; their JSON fields stay unchanged.

- Add an optional native RXUI workbench using the default system theme, with a
  file picker, document search, virtual occurrence tree, property inspector and
  depth-tested wgpu viewport. Support background parsing, saved configurations,
  hidden occurrences, standard views, orbit, pan and zoom without a browser.

- Move the viewer to the published RXUI 0.1 and rxui-draw crates, replacing
  pinned Git dependencies. Update ZIP to 9.1 and refresh both Cargo lockfiles.
  The viewer now requires Rust 1.98.1 or newer.

- Support external-reference byte 5 and prefix word 0x0100, retaining both raw.
  Add the three-string 17-byte scalar profile and direct-array 28-/32-byte
  extensions. Complete model-header reports now uniquely decode all 358 streams
  in the pinned public modern corpus without version-based profile selection.

- Expose cached model-header reports through `DocumentReader::model_headers()`
  and `sw-dump model-headers`. Select only a unique whole-stream decode, retain
  failed/ambiguous alternatives and transfer selected definitions without cloning.
  Keep ordinary metadata/geometry parsing independent of the new inspection.

- Decode complete leading-word model headers with explicit 32-, 64- and 68-byte
  scalar extensions. Check supported word values and profile combinations;
  preserve raw data and require matching profiles for root backreferences.

- Decode complete direct-array model headers, including their following saved
  external reference, checked empty trailing list and optional scalar block.
  Retain observed external-object leading bytes 2 and 3 without assigning an API
  document type. Validate complete-header boundaries on original public streams.

- Read native model-header prefixes sequentially through both schema-zero string
  arrays, saved log/stamp records and the external-object list. Preserve exact
  source fields and map registrations; keep parent tails and the configuration
  archive handoff unresolved. Add explicit direct-array and leading-word profiles.

- Decode complete edge references with explicit two-word and four-word scalar
  trailers. Preserve exact sibling boundaries and typed root backreferences;
  reject unsupported edge header flags and mismatched profiles.

- Add complete face-reference reads for explicit three-word and five-word trailer
  profiles, including sequential sibling reads and typed root backreferences.
  Add a three-string external-object profile with a 12-byte tail to preserve the
  following FR fields in controlled SW2022 Part samples. Keep profile selection
  explicit; other reference header/trailer variants remain unresolved.

- Decode schema-one end-face/sketch `3Int`, thin-offset and recursive linear-pattern
  surface payloads in the sequential reference reader. Retain raw trailing words
  and typed backreferences without inferring topology ownership.

- Add sequential native reference-subtree profiles with typed object/handle
  backreferences, source spans, raw external-object scalar blocks and rollback on
  failure. Keep face/edge arrays explicitly incomplete until their parent trailers
  are decoded. Inspect edge prefixes and matching resolved-feature carriers for
  additional saved-handle candidates without guessing archive slots or owners.

- Retain first unsupported native-prefix fields and configuration-backed handle
  candidates in the opt-in reference graph. Include both source intervals and
  unresolved class tags; reject ambiguous or cross-configuration context. Keep
  class-map continuity and mate/appearance ownership unresolved.

- Expose saved document handles from supported explicit native face-reference
  prefixes in the opt-in reference graph. Retain raw header words, entity class,
  checked prefix spans and exact CString offsets; keep file/occurrence/topology
  ownership unresolved and stop at unknown reused classes.

- Decode every top-level row in grouped mate/cache surface descriptors, retaining
  exact text spans, order and duplicates. Block feature candidates when later rows
  disagree or remain invalid; keep nested expressions opaque and mate endpoints
  unresolved. Add plural mate-reference access without selecting the first row.

- Load legacy OLE/CFB documents with strict sector/directory validation and bounded
  raw/ZLB stream extraction. Retain fragmented source ranges, original paths,
  hashes and ZLB member spans in inventory; reject normalized-name collisions.
  Existing metadata and saved-mesh decoders now accept compatible legacy streams
  on native hosts and WASM. Legacy binary property sets remain opaque.

- Add a searchable GitHub Pages documentation book and generated Rust API reference, shorten the README, preserve the existing viewer URL, and fix viewer formatting in CI.

- Decode the observed legacy chunked ZLB wrapper with exact size/checksum/member
  validation and source spans; add native-corpus audit tooling without enabling
  unsupported OLE document parsing.

- Decode saved surface-reference tuples in mate descriptors and opt-in cache
  inspections, preserving signed tail bits, opaque fields, invalid tokens and
  conflicting duplicates without inventing native face/body ownership.
- Add same-source cache-to-feature reference candidates, CLI/core WASM inspection
  and an expanded corpus audit; keep normal geometry loading free of the new scan.

- Add checked archive-local class/object maps, including class-only declarations,
  backreferences, cycles, explicit external mappings and wide indices; validate
  configuration record classes and reused slots during sequential metadata parsing.
- Document binary reference rules and a corpus-backed mate-context hypothesis;
  add a reproducible archive-map audit without promoting inferred mate families.

- Add opt-in saved feature catalogs and a shared source-reference graph with
  typed stream-local XML links, native identity candidates, duplicate preservation
  and explicit unresolved topology; expose reader, CLI and WASM access.

- Add a source inventory inspector with verified record spans/hashes, duplicates,
  rejected/index records, opaque byte accounting and on-demand paged XML/binary views.
- Support source-aware UTF-8/UTF-16 XML inspection, wrapped KeyWords payloads,
  namespaces, text and ambiguous syntactic references without invented native ownership.
- Add a decode-once document reader with cached metadata/geometry and borrowed
  stream views; use it in the viewer to avoid a second Part decompression pass.
- Share checked transactional binary/string readers and retain existing convenience
  APIs; add CLI/WASM inventory access and reproducible census/performance examples.

- Inventory saved per-configuration mate headers, explicit native families and
  selection-reference evidence; expose a bipartite reference graph, CLI audit,
  web metadata and inspector counts without changing saved placements.
- Retain unresolved mate class/object references and explicit unknown ownership,
  parameter and solver support, with source offsets and verification TODOs.

- Preserve additional raw native shader parameters and expose opt-in provisional
  texture-mapping candidates with source offsets and controlled-save TODOs;
  retain unknown units, projection modes, dimensions and ownership explicitly.

- Decode supported native visual-property prefixes, exact scoped part defaults,
  display-state keys and conditional raw shader/texture/mapping records.
- Preserve unresolved feature/body/component identities with byte provenance;
  export cached face/body property links keyed by carrier and descriptor offset.
- Resolve supported cache layers conservatively, retaining conflicts and unknown
  native ownership instead of guessing surface appearances.

- Add a renderer-independent presentation model separating physical materials,
  visual definitions, default relationships and scoped appearance bindings;
  expose source catalogs per document with explicit unknown decoding coverage.
- Centralize exact physical-material resolution and keep viewer approximations
  separate from native appearance ownership.

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
