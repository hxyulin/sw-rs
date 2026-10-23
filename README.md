<!-- SPDX-License-Identifier: MIT -->
<!-- Copyright (c) 2026 hxyulin <hxyulin@proton.me> -->
# sw-rs

[Open SW Explorer](https://hxyulin.github.io/sw-rs/) · [Viewer guide](viewer/README.md)

A pure Rust, byte-oriented reader for modern SolidWorks files, with optional
WebAssembly bindings. The name is provisional. This is an initial metadata port
of [openswx](https://github.com/schwitters/openswx) at
`a838a83e267fadbf8f01fd75a288b2ae517c9133`, with MIT metadata code and Apache-2.0 mesh routines.

The library accepts bytes and a filename hint. It has no filesystem access,
networking, C++ FFI, SolidWorks installation, or simulator dependency. Native
file access lives in the `sw-dump` CLI. All resolved dependencies offer permissive
licenses; upstream copyright and permission notices are retained in
[LICENSE-openswx](LICENSE-openswx). The narrow mesh adapter retains its
[Apache attribution](NOTICE-cadmpeg.md) and [license](LICENSE-Apache-2.0).
The package's combined license expression is `MIT AND Apache-2.0`. No newer
noncommercial sldkit code or source CAD is included.

## Current scope

- Modern (2015+) inline chunk streams: per-file name rotation, pure Rust raw
  deflate, decoded-size and CRC32 checks, first-valid duplicate selection.
- Native version, global/configuration properties, Unicode configuration names,
  saved parent indices, cached mass-property tokens, document PNG previews.
- Assembly occurrence names, external paths, referenced configurations,
  suppression/visibility/BOM flags, and optional saved 16-value transforms.
- Saved weldment cut lists and basic drawing-sheet/view/reference metadata.
- Saved physical-material definitions, explicit configuration assignments and
  density tokens from `MATERIALTREE`, with source paths and unresolved references.
- Cached Part DisplayLists meshes, normals, source offsets and preserved winding,
  with single-configuration ownership and validated decoding.
- Raw decoded streams and diagnostics for later format work.

This is **not a complete openswx port** or a CAD kernel. OLE2 and ZIP/OPC return
unsupported-format errors. Drawing containers can be inspected, but drawing
rendering, Parasolid B-Rep, STEP export, mates and BOM file resolution are not
implemented. Cached Part meshes are supported only by the profile below. Unknown
container record families remain diagnostics. Malformed XML is reported.

Mass, COM, volume and inertia retain source numeric tokens: units, frame, signs
and cache freshness are unverified. Do not feed these directly to physics.
Transforms use openswx's column-major interpretation; agreement with upstream
is not independent placement validation. Absent/malformed transforms are `None`
(JSON `null`), rather than fabricated identity matrices.

`Document::coordinate_frame` and `GeometryDocument::coordinate_frame` explicitly
declare the assumed SolidWorks model XYZ convention: right-handed, native
document origin, Front plane XY, Front-view up +Y and toward-observer +Z.
This follows [SolidWorks' plane/direction definitions](https://help.solidworks.com/2021/english/SolidWorks/cworks/c_specifying_directions.htm).
It is labelled `assumed_default_solidworks_convention`; named coordinate-system
features and saved camera orientations are not decoded or automatically applied.
Model coordinates are never recentered or reordered by this declaration. Custom
reference frames do not replace the document's global model basis. Drawings
expose no 3D frame. Cached mesh output stays in millimetres; the viewer composes
saved component poses in metres and converts mesh units exactly once.

`Document::materials` retains material definitions and configuration references.
`Configuration::material` selects only a unique exact saved reference;
`definition_index` resolves a unique definition within that document. Definitions
alone do not prove assignment, and no parent-configuration or body assignment is
inferred. Density retains its raw token and explicitly saved units, if any.
Each definition also retains saved shader records, texture references and optical
attributes with their source paths. These are metadata; external paths are never
opened. Duplicate or unknown appearance records remain available for inspection.
Physical material labels and display appearances are separate. The viewer maps
explicit aluminum/carbon-fiber assignments to its own procedural brushed-metal
and woven-composite presets; other assignments use a generic saved-swatch preset.
These are approximate visualizations, not recovered SolidWorks shaders or UVs.
Unassigned parts retain diagnostic colors, and a toolbar control switches the
whole scene back to diagnostic colors.
The **Appearance style** selector defaults to approximate saved material
appearances and can switch to physical material-family presets. Saved PhotoWorks
identities take precedence over generic realtime shader hints. This does not
recover final face/body overrides from binary visual states.

## Renderer-independent presentation model

`Document::presentation()` projects decoded records into `sw-rs-presentation-v1`:
physical definitions/bindings and visual definitions/bindings are separate. Each
physical definition may reference a material-default appearance. This relationship
is not an applied face/body/component override. Source stream/element paths and
unresolved references are retained; indices are local to one document snapshot.
Recompute the projection after mutating a document's public source records.
`physical_material_for_configuration(name)` validates exact, unique configuration
and definition references before returning physical evidence.

Appearance targets distinguish document, component occurrence, body, feature and
face identities. Configuration and display-state scopes are separate; unknown
scope never means “all states”. Applied appearance bindings remain empty until
ownership decoding is implemented. Explicit coverage flags distinguish unknown
bindings, display states and texture mappings from known absence. Native entity
identifiers must not be replaced with tessellation triangle indices.

The viewer exports `presentations` keyed by document path, including embedded
paths. Hierarchy occurrences refer to that path and document-local
`physical_material_index`/`material_default_appearance_index` when resolved.
Repeated occurrences share a catalog. Existing `materials`, `material_index`
and `batches` remain the viewer's approximate rendering adapter, separate from
source definitions; they are not native appearance ownership records.
The WASM `presentation_json(bytes, filename)` entry point exposes the same core
projection. The model includes no WebGL, RXUI, GPU resources or renderer presets.

Next decoding work is independent visual definitions, saved display states,
face/body/occurrence ownership and texture projection. A renderer can then resolve
those bindings and translate visual evidence into its own shading parameters.
This change does not recover those records or reproduce SolidWorks shaders.

## Native use

Use `mbx` for compiling commands when installed; otherwise substitute `cargo`.

```sh
mbx run --release --bin sw-dump -- /path/to/model.SLDASM
mbx run --release --bin sw-dump -- geometry /path/to/part.SLDPRT
mbx test --all-features --locked
mbx clippy --all-targets --all-features --locked -- -D warnings
cargo fmt --check
```

```rust,no_run
let bytes = std::fs::read("model.SLDASM")?;
let document = sw_rs::parse(&bytes, "model.SLDASM")?;
for config in document.configurations {
    println!("{}: {} occurrences", config.name, config.components.len());
}
# Ok::<(), Box<dyn std::error::Error>>(())
```

`parse_with_limits` and `Container::parse` accept `Limits`. Defaults allow 256 MiB
of input, 64 MiB per compressed stream, 128 MiB per decoded stream, 512 MiB
cumulative decoded output (including duplicates), and 100,000 marker candidates.
Local callers can use `Limits::unlimited()`; the web viewer does so. Defaults bound decoding, not total
process memory: XML trees, JSON serialization, and host copies add overhead.
Choose smaller limits for constrained hosts. Use a Web Worker for large files;
parsing is synchronous and does not promise fast processing of arbitrary input.

## WebAssembly

With the `wasm32-unknown-unknown` target and matching `wasm-bindgen` CLI installed:

```sh
mbx build --release --lib --target wasm32-unknown-unknown --features wasm --locked
wasm-bindgen --target web --out-dir pkg/web target/wasm32-unknown-unknown/release/sw_rs.wasm
```

```js
import init, { parse_json } from './pkg/web/sw_rs.js';
await init();
// file is a browser File from a picker or drop event.
const document = JSON.parse(parse_json(new Uint8Array(await file.arrayBuffer()), file.name));
console.log(document.configurations);
```

`parse_json` accepts `Uint8Array`, returns `sw-rs-metadata-v1` JSON, and throws
on a hard parse error. COM serializes as `{x,y,z}`; previews stay in the native
Rust API rather than expanding JSON. `geometry_json(bytes, filename)` returns a
`sw-rs-geometry-v1` envelope from the same byte-based decoder as the CLI. `wasm-pack build --target web --features wasm` is an alternative
packaging command. To exercise the WASM boundary in Node:

```sh
mbx build --lib --target wasm32-unknown-unknown --features wasm --locked
wasm-bindgen --target nodejs --out-dir pkg/node target/wasm32-unknown-unknown/debug/sw_rs.wasm
node scripts/wasm-smoke.cjs
```

## Validation and next work

See [VALIDATION.md](VALIDATION.md) for the tested source and measured comparison.
The optional corpus runner accepts privately held native files and a separately
built upstream CLI; no corpus is redistributed:

```sh
mbx build --release --locked
python3 scripts/compare-corpus.py /path/to/native-files \
  --upstream /path/to/swx_dump --rust target/release/sw-dump \
  --output local-assets/corpus-comparison.json
```

[PORTING.md](PORTING.md) traces the actual rm-simulator consumers, measured JSON
compatibility gaps, permissive geometry sources (including a pinned earlier MIT
sldkit revision), and acceptance gates for replacing both external readers.
Metadata interchange and Part tessellations now cover the current simulator
workflow on the evaluated corpus. Complete portable Parasolid/drawing support
remains in later milestones.
General mate solving and feature-history regeneration require separate designs
and validation.

## Cached Part mesh profile

`geometry(bytes, filename)` reads one modern Part DisplayLists carrier for one
canonical native configuration. It validates compact/extended face headers,
six descriptor channels, finite source float coordinates and strip winding,
including validated class reuse. Output coordinates are millimetres; source
stream/table offsets and normals are retained. Tables follow the old CAD IR's
lexicographic table-key order so prepared project signatures remain stable.
Unsupported carriers, multi-configuration ownership and missing meshes return
errors. There are no fixed mesh-count, table-attempt or triangle policy caps;
descriptor lengths, finite coordinates and index ranges are checked against the
actual payload. The simulator adapter independently enforces 250,000 unique
vertices per Part; that external adapter does not limit this viewer.

The profile recovers saved display caches only: they may be partial or stale and
have no inferred B-Rep ownership or mass. It does not evaluate features or certify
source-frame alignment. Both metadata and geometry work without native libraries.
Optional integration comparison (no source CAD is redistributed):

```sh
python3 scripts/compare-geometry.py /path/to/native-files \
  --preparer /path/to/rm-simulator/scripts/prepare-solidworks-project.py \
  --upstream /path/to/sldkit-rs --rust target/release/sw-dump \
  --output local-assets/geometry-comparison.json
```

## Web viewer

[SW Explorer](viewer/README.md) is a separate RXUI/WASM app for ZIP, TAR and
TAR.GZ native CAD archives, with a document/configuration inspector and an
orbitable 3D viewport. An assembly chooser and expandable occurrence hierarchy
expose nested components, including missing and unsupported references. The
inspector reports saved material assignments for the opened configuration.
Parsing runs in a cancelable worker; uploads stay local.

```sh
sh viewer/build.sh
python3 -m http.server 8097 --bind 127.0.0.1 --directory viewer/web
```

Open http://127.0.0.1:8097. The `archive` Cargo feature separately exposes bounded
archive loading and virtual reference resolution without a viewer dependency.
