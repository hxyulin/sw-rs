<!-- SPDX-License-Identifier: MIT -->
<!-- Copyright (c) 2026 hxyulin <hxyulin@proton.me> -->
# Architecture

sw-rs separates container recovery, saved-record interpretation and application
rendering. Every layer preserves uncertainty instead of manufacturing missing
relationships. The core has no filesystem, networking, host clock, native CAD
runtime or simulator dependency; its CLI owns native file access.

## Data flow

```text
caller-owned bytes + filename hint
  └─ Container: validate/inflate named streams once
       ├─ Document: metadata, configurations, materials, visual and mate records
       │    └─ PresentationData: renderer-independent physical/visual evidence
       ├─ GeometryDocument: validated cached Part tables + cache-local properties
       ├─ Inventory: source/record/decoded spans, hashes and structural inspection
       └─ FeatureCatalog / ReferenceGraph / CachedSurfaceData: opt-in evidence

Archive FileSet → viewer ArchiveProject → occurrence traversal → Scene
                                                       ├─ hierarchy + diagnostics
                                                       ├─ presentation catalogs
                                                       └─ mesh buffers + approximate material batches
```

`DocumentReader` retains one decoded container and caches requested projections.
Standalone `parse`/`geometry` APIs remain useful when only one projection is
needed. Its [ownership contract](reader-inventory.md) describes source borrowing,
retained memory and transfer through `finish()`.

## Core modules

| Module | Responsibility |
| --- | --- |
| `container.rs`, `zlb.rs` | Modern inline framing and standalone legacy compression |
| `binary.rs`, `object_archive.rs` | Checked cursor, framed strings, transactional class/object maps |
| `config.rs`, `xml.rs`, `document.rs` | Supported configuration/native XML records into metadata |
| `features.rs`, `references.rs`, `surface.rs` | Saved identity evidence and scoped graph composition |
| `mesh.rs`, `geometry.rs` | Supported DisplayLists cache profile and unit conversion |
| `material.rs`, `visual.rs`, `presentation.rs` | Physical definitions, native visual evidence and typed presentation |
| `inventory.rs`, `reader.rs` | Provenance/opaque accounting and shared-reader integration |
| `archive.rs` (feature) | In-memory archive validation, path normalization and file-set resolution |
| `lib.rs`, `bin/sw-dump.rs` | Public Rust/WASM boundary and filesystem CLI adapter |

Modules are private where possible, with public types reexported from `sw_rs`.
The `archive` feature exposes its own module. Geometry is Part-only; generic
assembly composition is not silently hidden inside the core geometry function.

## Viewer application

The separate `viewer/` package pins RXUI/Astrelis and has its own lockfile.
`project.rs` resolves uploaded documents, follows saved rename mappings and
embedded ownership, traverses occurrences and collects missing/unsupported nodes.
`appearance.rs` turns explicit saved evidence into approximate rendering presets.
`web.rs` exposes the worker-facing scene/model boundary.

In `viewer/web/`, the worker owns synchronous Rust parsing and caches. `main.js`
coordinates imports, chooser/search/tree/source-inspector DOM overlays and RXUI
selection. `scene.js` owns camera controls and WebGL rendering. Vertex buffers
and per-material draw batches are separate; switching appearance styles redraws
without decoding the mesh again. Cancellation terminates the worker and stale
results cannot replace a newer import. The native HTML overlays supplement the
canvas UI's incomplete accessibility tree.

## Identity boundaries

- Archive class/object slots are serialization-context indices, not CAD IDs.
- XML file/model IDs resolve only inside their stream-local namespace.
- Feature source IDs belong to a source document; history/current topology matter.
- Surface source/local/tail tuples retain the full ordered suffix.
- Cache table identity is `(stream, descriptor offset)`, not a B-Rep face ID.
- Occurrences are distinct instances, even when they share one document mesh.

Graph composition checks source SHA and configuration scope. `Candidate`,
`Ambiguous` and `Unresolved` are substantive results; consumers must not treat
them as `Resolved`. No guessed relationship drives physical simulation or
native appearance ownership.

## Extension rules

New decoders should use checked reads, declare a supported profile, retain source
locators and raw unknown fields, and stop when payload boundaries are unknown.
Add synthetic truncation/ambiguity regressions and private corpus comparisons.
Keep experimental interpretations opt-in with explicit verification TODOs.
A new container adapter must preserve duplicate/rejection/opaque accounting and
budgets, rather than returning streams while discarding their provenance.
