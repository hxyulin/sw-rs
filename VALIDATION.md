<!-- SPDX-License-Identifier: MIT -->
<!-- Copyright (c) 2026 hxyulin <hxyulin@proton.me> -->
# Initial port validation — 2026-10-23

## Web archive investigation — 2026-10-24

The locally supplied EGA mechanisms ZIP contains 340 native documents (295 Parts
and 45 Assemblies). Its SHA-256 is
`9b688805963fe892e12332d3ff02991c4fc9261c56d04d93d8e2a61c4c092ddc`.
The archive, decoded streams, per-document reports and screenshots stay in
gitignored `local-assets/`; the public viewer contains synthetic fixtures only.

At `3578b39`, 34 of the 45 first-configuration assembly scenes contain geometry.
The main assembly renders 3,380,059 triangles across 615 supported occurrences,
with one unresolved source reference. This establishes partial display recovery,
not completeness: unsupported constituent Parts still leave gaps. Saved rename
maps and owner-scoped imported/virtual documents account for the improvement
from the initial 18 renderable assemblies. Reflected orthogonal transforms are
accepted; a filename beginning with `Mirror` is not itself a rejection reason.

Reproduce the native report with:

```sh
mbx run --manifest-path viewer/Cargo.toml --release --example inspect_project -- \
  /path/to/archive.zip > local-assets/assemblies.jsonl
```

The browser suite exercises synthetic data at device-pixel ratios 1 and 2;
`SW_VIEWER_ARCHIVE=/path/to/archive.zip npm --prefix viewer test` adds an optional
local real-CAD check. Browser tests do not establish geometry completeness,
material correctness, cross-browser certification or physical-model validity.

The material audit found `MATERIALTREE` in 274 of the 295 external Parts, with
214 saved definitions and 1,109 uniquely resolved configuration references.
Explicit assignments include aluminum-classified 6061/6061-T6/7075-O and
carbon-fiber-classified VMA/Thornel Mat VMA. Density tokens include `2700.000000`
and `2000.000000`; their XML does not explicitly establish units. Definitions
without an exact configuration reference remain unassigned. Body-level
ownership and final face appearances are not decoded. Private evidence is
`local-assets/material-investigation/report.json` and `audit.json`, including
source hashes. Synthetic tests cover Unicode encodings, exact references,
duplicate/ambiguous records, invalid density tokens and malformed XML.

A separate geometry inventory found 248 supported external Part entries, 23
without recognized cached face records, and 24 with ambiguous configuration
ownership. The 23 unsupported Mirror Parts have 7,640-byte environment/lighting
display streams and retained `StockBodies`. All 372 inspected embedded Part
entries lack recognized cached faces, including the 116 distinct embedded paths
reached in the prior assembly report; those retain `LocalBodies`. Inflated body
samples identify Parasolid binary transmit data. No failure in this inventory
had a recognized face marker with an unreadable table. Geometry output is
unchanged by the more specific diagnostics; there is no claimed rendering gain.
Regenerated display caches or a separately validated body decoder are needed for
these files. Counts include duplicate bytes and embedded documents outside the
selected visible configuration; deduplicated totals and source hashes remain in
`local-assets/geometry-investigation-summary.json`.

The hierarchy/material viewer passes 48 public browser tests at device-pixel
ratios 1 and 2, with two opt-in private-archive cases skipped in that suite.
The full real ZIP was additionally loaded locally: the chooser lists all 45
assemblies, the main scene has 1,317 occurrence records and unchanged geometry,
and a carbon-fiber Part opens with its exact VMA assignment and procedural weave.
Material/diagnostic switching changes rendered pixels and restores the original
mode. Native tests cover exact configuration material selection, preset
classification and complete nonoverlapping draw ranges. These procedural
appearances use world-space projection and artistic optical parameters; they
are not recovered source face appearances, UVs or fiber directions.

## Original metadata comparison

Compared this initial Rust implementation against openswx revision
`a838a83e267fadbf8f01fd75a288b2ae517c9133`, using the existing private EGA corpus
from the rm-simulator import experiments. The corpus contains 295 Parts and
45 Assemblies. Source CAD and per-document outputs remain outside Git.

The reproducible `scripts/compare-corpus.py` run found:

- 340/340 native documents parsed.
- 1,418 configuration names, in the same order as upstream.
- 916 component records matched, including source paths, child configurations,
  suppression, visibility, BOM flags, references and saved transform tokens.
- 617 cached mass records matched exactly after normalizing COM JSON shape.
- Native version and global/configuration property maps matched.
- Zero differences in these compared fields. All 916 transforms in this corpus
  were present; synthetic tests cover missing and malformed transforms.

This establishes parity for those fields on that corpus. It does not establish
correct physical units, transform composition, complete format support, geometry
recovery, or behavior on other SolidWorks versions. Cut lists, per-configuration
previews, parent indices and drawing semantics were not compared by the corpus
runner. Parent-index decoding has a synthetic version-gated regression test.

The local full report (including per-document SHA-256) is ignored at
`local-assets/corpus-comparison.json`. The executable used by that run is
`local-assets/sw-dump-tested`; it is a snapshot of the release build so mbx
managed-target rotation cannot invalidate a running corpus probe.

Native regression coverage includes valid/empty streams, duplicate selection,
CRC failure and recovery, truncation, missing deflate terminators, unsupported
containers, malformed inputs, expansion/aggregate/record limits, Unicode XML,
configuration property override precedence, binary configuration names and
parent IDs, cached tokens and absent/malformed transforms. A library doctest
covers invalid input.

The library compiles to `wasm32-unknown-unknown` with `--features wasm`. Matching
wasm-bindgen 0.2.129 generated Node bindings; `scripts/wasm-smoke.cjs` exercised
actual WASM execution from a Uint8Array, Unicode metadata returned through JSON,
and an invalid-input exception. This is a JavaScript/WASM smoke test, not browser
UI or browser memory/performance validation. Web-target bindings are generated
separately using `wasm-bindgen --target web`.

Resolved direct and transitive dependency license expressions were inspected
using `cargo metadata --all-features --locked`: all offer permissive terms
(MIT/Apache-2.0, 0BSD, Zlib, Unlicense, and Unicode-3.0 as applicable).
No upstream CAD SDK, native zlib, or noncommercial parser dependency is linked.

## Simulator replacement validation

The metadata JSON now uses the reviewed-cache COM object shape, optional native
parent field name, cut lists and basic drawing metadata. Preview byte arrays are
omitted from normal JSON, and the native CLI adds versioned `metadata`/`geometry`
commands. The simulator accepts both with explicit `--sw-rs` backend selection,
truthful reader/hash provenance, schema checks and propagated diagnostics.

A fresh 340-file comparison checks the previous fields plus parent indices and
cut lists: zero differences. Running the real extractor on the complete corpus
preserves the consumed graph exactly against the existing openswx catalogue:
all document identities/source hashes, configurations, resolved/unresolved edges,
component source records and saved physical values. Extra diagnostic messages
and intentionally unverified unit/presence labels are not falsely normalized
into upstream declarations. Private reports: `metadata-porting-corpus.json`,
`catalogue-graph-comparison.json` under `local-assets/`.

Across 295 Parts and 1,373 source configuration requests, the geometry adapter
recovers 247 meshes versus the old reader's 243. All 243 previously accepted
meshes match exactly after the simulator's bit-preserving reindexing, including
ordered indices and coordinates. Four additional Parts have validated display
caches while the old reader fails `geometry.byte_ledger_invalid`; these recoveries
do not claim B-Rep validity. All other requests remain unsupported. The final
resource-budget pass preserves all 247 successes with no changed meshes.
Private reports: `geometry-corpus.json`, `final-geometry-budget-check.json`.
`scripts/compare-geometry.py` reproduces the integration comparison using a
caller-supplied simulator adapter and existing CLI.

Running both real scripts with `--sw-rs` on the small armor module yields three
occurrences and two shared meshes. Parts and mesh definitions match the old
workflow exactly; the project has no diagnostics. Cached COM is an object, and
source-frame/mass review remains unapproved. Private comparison:
`local-assets/armor-sw-rs-comparison.json`. Review-mode mass tests verify that
object-shaped COM produces finite inertials and the former array shape fails.

The actual WASM module now decodes synthetic display meshes and a real native
Part: native/WASM geometry JSON matches exactly. This includes source SHA-256,
Unicode configuration metadata, normals, source offsets, coordinates and indices.
The test runs through wasm-bindgen and Node; it is not browser UI, worker,
file-set resolution or large-browser-memory validation. Reproduce with:

```sh
node scripts/wasm-smoke.cjs /path/to/part.SLDPRT /path/to/sw-dump
```

No sldkit or cadmpeg dependency was imported wholesale. The narrow Apache mesh
routines are adapted into `src/mesh.rs` with their license/provenance retained;
SHA-256 is the only new direct dependency. Source CAD remains outside Git.

A larger Sentry preparation produced 540 occurrences and 133 shared meshes
(598,387 unique vertices, 1,206,113 placed vertices), within the existing bounds.
Its 277 diagnostics retain unresolved references and unsupported source meshes;
it remains unreviewed and blocked from physical export. Private evidence:
`local-assets/large-project-check.json`. Native reader regression tests, Clippy,
WASM execution, 21 Python adapter tests and 10 RXUI importer tests passed.
