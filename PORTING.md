<!-- SPDX-License-Identifier: MIT -->
<!-- Copyright (c) 2026 hxyulin <hxyulin@proton.me> -->
# Replacing rm-simulator's native SolidWorks tooling

Audit date: 2026-10-23. Initial sw-rs implementation: `93372ee`.
Simulator paths below are relative to the sibling `rm-simulator` checkout.
This records the initial audit and implementation roadmap. Subsequent progress:
metadata interchange, cached Part meshes and the simulator's `--sw-rs` CLI
integration are implemented. The 340-file consumed graph matches; all 243
previously prepared Part meshes match and four additional caches recover.
The small assembly also matches end to end, and real native/WASM geometry
agrees. See VALIDATION.md for the current measured scope. The remaining
Parasolid/drawing/BOM expansion and browser file-set/worker APIs remain future
milestones. Historical gaps described below refer to the initial `93372ee`.

## What the simulator actually uses

There are two external readers, both in the optional offline import workbench.
Neither is part of the ordinary simulator Cargo graph or live match runtime.

1. `scripts/extract-solidworks.py` invokes **openswx/swx_dump FILE**. Its Python
   adapter owns source hashes, bounded subprocess execution, directory inventory,
   Windows reference relocation, cycle-safe document traversal, configuration
   lookup, diagnostics and `rm-solidworks-metadata-v1` catalogue output.
2. `scripts/prepare-solidworks-project.py::read_geometry` invokes
   **sldkit-rs geometry FILE**. It requires `geometry.source.sha256`,
   `geometry.length_unit`, configuration identity/index, and
   `geometry.model.tessellations[*].vertices/triangles`. It converts declared
   millimetres to metres and reindexes exactly duplicated vertices by float bits.
3. That preparation script expands active Part occurrences, composes saved
   matrices, caches repeated meshes and failures, enforces configuration and size
   limits, and emits `rm-solidworks-project-v2`. It retains source hashes,
   occurrence identities, hidden/suppressed distinctions and prior review data.
4. The RXUI importer consumes the prepared JSON, not native CAD. It supplies
   grouping, collision fitting, explicit mass review and joint review. Native
   mates are not decoded by the existing pipeline. Recent candidate recovery
   uses saved relative poses; it is not a native mate solver.

Therefore, porting every openswx feature alone cannot replace the current
pipeline: **Part tessellation extraction is the immediate missing capability**.
The Python traversal/preparation logic can remain during the first replacement.
The renderer, physics and reviewed-recipe format do not need redesigning for it.
Do not introduce a sibling path/git dependency into simulator crates. Keep the
initial integration an offline CLI; use an independently versioned published
crate later if direct Rust consumption is wanted.

## Measured integration gaps in the initial port

The 340-file comparison in VALIDATION.md checks numeric and occurrence parity,
with deliberate JSON normalization. It did not establish drop-in schema parity.
A separate test ran the simulator's actual `extract()` function with each reader
on the small armor-module assembly and its referenced Parts:

- Both runs produced 3 documents, 3 configurations, 3 occurrences and 3 cached
  physical-property records, with zero failed documents.
- The openswx catalogue was 11,443 bytes. The sw-rs catalogue was 365,314 bytes
  under the same compact JSON serialization, about 32 times larger.
- openswx COM is an `{x,y,z}` object; sw-rs currently emits a three-element array.
  `integrations/scenario-viewer/src/rxui_viewer/importer.rs::part_mass` indexes
  `native_cache.center_of_gravity` by x/y/z in reviewed-cache mode. A raw sw-rs
  catalogue therefore fails that path's finite-inertial validation.
- The extractor excludes `preview_png_base64`, but not sw-rs's `preview_png`
  array. Our preview leaks into catalogue metadata. Preview exclusion or a
  compatibility DTO is required; merely replacing the binary path is inadequate.
- Additional differences include `document_type` versus `doc_type`,
  `parent_index` versus `derived_from_index`, missing cut lists/configuration
  previews/drawing references, and diagnostics retained in document metadata
  rather than promoted to the catalogue's diagnostic channel.
- `--swx-dump`, tool name `openswx/swx_dump`, `--openswx-revision`,
  `--sldkit`, `sldkit_binary_sha256`, and the cache wrapper's
  `upstream_declared_units: SI` are backend-specific provenance. They must become
  truthful reader identities/capabilities; sw-rs does not declare validated SI.
- Missing transforms are intentionally null in sw-rs. Preserve their unknown
  status and refuse physical placement; do not copy upstream's identity fallback.

Private evidence: `local-assets/extractor-audit.json` and the two extractor
catalogues. No simulator source was changed by this audit.

## Permissive geometry sources

openswx's pinned root and libopenparasolid are MIT. Its cached drawing-face
reader (`libopenswx/src/drawing_face_tessellation.cc`) reads
`uoTempFaceTessData_c` tables from drawing `Contents/VBLists`. That is useful
portable code, but it is **not evidence of coverage for the Part display meshes**
currently obtained through sldkit. Test Part carriers independently.

There is a stronger geometry starting point than rewriting everything:

- [sldkit's current license notice](https://github.com/monozukuri-ai/sldkit/blob/main/LICENSE)
  explicitly preserves MIT permissions for revision
  `c43b5ef752a0e58535b7f8f744f7d7c68e3c0e90` and earlier MIT-distributed material.
- That revision's [LICENSE](https://github.com/monozukuri-ai/sldkit/blob/c43b5ef752a0e58535b7f8f744f7d7c68e3c0e90/LICENSE)
  is MIT. Its Rust parser includes byte-based Part geometry decoding,
  configuration mapping and tessellation output, rather than just metadata.
- It vendors `cadmpeg-codec-sldprt` **0.5.3+sldkit.4**, with its own Apache-2.0
  LICENSE and PATCHES.md. The published `parasolid-core` 0.1.0 dependency is MIT.
  These are independent license grants, not permissions inferred from a current
  top-level project license.
- A source archive of that exact MIT revision was obtained under ignored
  `local-assets/sldkit-mit-audit/`. Its `sldkit-parser` passed
  `mbx check -p sldkit-parser --target wasm32-unknown-unknown --locked` unmodified.
  This is target type-checking, not linked browser execution or a memory test.
- The legacy CLI also built successfully. Through the simulator's existing
  `read_geometry` adapter, both armor-module Part definitions matched the
  currently evaluated CLI exactly after preparation: 2,166 vertices / 4,312
  triangles for the bracket, and 14,888 / 29,796 for the armor. Configuration
  mapping evidence also matched. This is a two-Part probe, not corpus parity.
  The full `prepare()` result also matched exactly for that assembly: three
  Part occurrences, two shared meshes, no diagnostics. This used an openswx
  catalogue for both runs to isolate the geometry backend; it does not remove
  the sw-rs COM/preview compatibility gaps above.
  Private reports: `local-assets/legacy-geometry-audit.json` and
  `local-assets/legacy-project-audit.json`.
- The unmodified dependency tree includes native zstd through cadmpeg-container
  as well as ZIP, CFB and web-time. A successful target check does not satisfy
  sw-rs's pure Rust/no-native-runtime design by itself. Prefer a narrow portable
  decoder extraction, or explicitly optional features, over importing that tree.
- All resolved license expressions in that MIT workspace offered permissive
  choices; no dependency was added to sw-rs by the investigation.

Use the exact MIT snapshot and separately attributed Apache source if adopting
code. Retain its original notices and patch provenance. Newer sldkit additions
offered only under PolyForm Noncommercial are not a permissive port source.
Do not assume the old reader reproduces later fixes across the corpus;
the two-Part comparison is promising but deliberately narrow. Local source hashes show the legacy `geometry.rs` differs from the
currently evaluated version even though several supporting modules are identical.

## Recommended implementation order and acceptance gates

### 1. Finish metadata interchange compatibility

Add a versioned CLI/schema boundary or an explicit openswx-compatible output
mode. Preserve the Rust API's unknown-state distinctions. Support the complete
metadata consumed or preserved by the existing extractor: COM object shape,
configuration parents, cut lists, previews, document kind and reference fields.
Keep preview bytes opt-in/out-of-band for normal catalogue extraction. Add
reader-neutral provenance and diagnostic handling in the Python adapter.

Gate: run the real extractor on all 340 files with both readers; compare source
hashes, resolved/ambiguous/unresolved edges, native config indices/names,
occurrence paths/order/flags, cached values and omission behavior. Test the
reviewed-cache import path explicitly. Add null-transform, malformed-record,
preview-output-limit and stale-source regressions. Preserve unvalidated units.

### 2. Add portable Part tessellation decoding

Evaluate the pinned MIT sldkit/Apache cadmpeg geometry path first. Keep decoding
from bytes independent of files, host clocks and Python. Support the observed
Part mesh carriers, including reuse of validated class descriptors and all
required decompression wrappers. Retain stream/record offsets, source
configuration ownership, native units, face/mesh identity and partial failures.
Never merge ambiguous configurations or fill missing geometry silently.

Expose a versioned geometry DTO/CLI with the values the preparation adapter
needs, then replace its backend-specific `--sldkit` dispatch and hash field.
An exact legacy-compatible CLI envelope is an alternative migration bridge.
No mass properties need to be calculated from these open per-face display meshes.

Gate: compare actual vertices and ordered triangle indices, not just mesh counts
or bounds, on all imported Part/configuration pairs. Compare both successes and
explicit failures. Keep the current exact reindexing, winding and signed-zero
policy. Exercise repeated meshes, ambiguous configs, source-index aliases,
unsupported units, truncation and aggregate limits. Validate placed small
assemblies against an independently checked STEP export; agreement between two
readers alone cannot validate coordinate conventions.

### 3. Replace the offline workflow end to end

Run catalogue extraction, project preparation, reimport/review merging, RXUI
loading, reviewed mass selection, collision fitting, joint review and recipe
export through sw-rs output. Repeated parts must keep their occurrence identities
and shared geometry. Unresolved active references must remain explicit, including
the user's existing omission policy. Preserve original CAD and source hashes.

Gate: a known small assembly produces equivalent cooked geometry and physical
recipes, and a large assembly respects existing bounds. Existing script/importer
regressions pass with a sw-rs backend. Only then remove the openswx CMake harness
and the sldkit CLI requirement from the current workflow. Run simulator checks
appropriate to the integration and `just verify` before any PR.

### 4. Make the same pipeline usable in the browser

Provide a caller-owned file-set/resolver interface using normalized virtual
paths, bytes and hashes; browser code cannot resolve saved Windows paths against
a host disk. Port reference resolution and bounded occurrence traversal as pure
functions, or retain them in a JS adapter with the same tested semantics. Reject
ambiguous basename matches and traversal outside the supplied file set.

Add Web Worker execution, progress/cancellation, selective stream decoding,
explicit memory budgets and typed mesh buffers. Avoid JSON number arrays and
base64 previews for large geometry transfers. Keep filesystem helpers in native
adapters. Test real linked WASM mesh decoding and directory/file-set imports in a
browser, including large inputs, worker cancellation and released buffers.

### 5. Expand the portable openswx library surface

Port the remaining reusable reader algorithms in bounded milestones:

- Container inspection: typed record/index inventories, duplicate history,
  exact unknown-region coverage, extraction diagnostics and stream inspection.
- Embedded Parasolid wrappers: multi-frame zlib/ZLB extraction, configuration
  tags, ghost partitions, transmit/partition/delta distinctions, safe extraction.
- Parasolid header/schema/catalogs, sequential records, typed references and
  owned model graph; retain unknown fields and explicit resource limits.
- Topology traversal, delta plans/application, analytic/NURBS/procedural geometry,
  evaluators, inversion, boundary/trim binding and failure semantics. Preserve the
  upstream supported subsets rather than claiming a complete Parasolid kernel.
- Drawing sheets/views, references, previews, cached faces/edges/annotations,
  appearance, hidden-line logic and SVG export, with their existing limitations.
- BOM expansion/path policies as caller-supplied resolution; filesystem and
  native CLI convenience functions remain adapters.
- Optional exact STEP export after binding: current upstream includes
  `libopenswstep`, which is **absent from our pinned snapshot**. Pin and audit a
  newer revision explicitly before including its supported subset/tests.

At the pinned revision, C++ source/header counts were 11,153 for libopenswx,
12,912 for libopenparasolid and 1,138 for libopenbom (25,203 total), plus 19,830
lines of libopenswx/libopenparasolid tests. Counts include comments and are a
scope indicator, not an effort estimate or proof of completeness. Port synthetic
upstream tests alongside each API; keep private CAD fixtures outside Git.

The asmbox HTTP service, authentication/database, deployment scripts and web UI
are application infrastructure, not requirements for replacing the simulator
reader or building a WASM library. A literal entire-repository rewrite includes
those too; it should be a separate decision from portable reader completeness.

## Recommendation

Build the complete portable reading/conversion stack in stages, beginning with
metadata compatibility and permissively sourced Part meshes. Reuse audited Rust
geometry code where it already exists, port the useful C++ reader algorithms,
and keep the simulator workflow working at each milestone. General mate solving,
feature-history regeneration and fully validated inertials remain separate work:
a complete openswx port does not automatically provide those capabilities.
