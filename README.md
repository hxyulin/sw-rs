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
- Partial configuration-local mate inventories, explicit native families and saved
  reference-string evidence graphs, with unresolved class/entity references.
- Saved physical-material definitions, explicit configuration assignments and
  density tokens from `MATERIALTREE`, with source paths and unresolved references.
- Cached Part DisplayLists meshes, normals, source offsets and preserved winding,
  with single-configuration ownership and validated decoding.
- Source-aware record/stream inventories with hashes, byte spans, duplicates,
  rejected/index records and explicit opaque ranges.
- On-demand XML/UTF-16 structural inspection and binary class/string candidates.
- Raw decoded streams and diagnostics for later format work.

This is **not a complete openswx port** or a CAD kernel. OLE2 and ZIP/OPC return
unsupported-format errors. Drawing containers can be inspected, but drawing
rendering, Parasolid B-Rep, STEP export, mate solving and BOM file resolution are not
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

Appearance targets distinguish document, component occurrence, body, feature,
face and **cached tessellation** identities. Cached tables use their carrier name
and exact descriptor byte offset; they never impersonate native B-Rep faces.
Configuration and display-state scopes distinguish unknown, universal and named
states. Binary source locations use `@byte[N]` within the decoded stream.

`Document::visuals` decodes supported `ThirdPtyStore/VisualStates` profiles:
framed UTF-16 state keys; packed color words; variable-length texture/material
strings; six legacy optical values (and exact hexadecimal bits); supported shader
identifiers and appearance/surface-finish references. Conditional texture blocks
and mapping numbers are retained raw. Their axes, projection types and physical
units are not established, so `texture_mappings_decoded` remains false.
`shader_parameters` additionally preserves supported raw shader groups.
`NativeAppearanceExtension::texture_mapping_hypothesis()` offers an opt-in,
explicitly provisional grouping of plausible directions/origin/repeat dimensions;
negative/default dimensions remain unresolved. It does not produce a verified UV
transform. See [appearance research and verification TODOs](docs/appearance-research.md).

Verified part-default class chains can produce document-level bindings scoped to
one uniquely matched configuration/display-state key. Ambiguous prefixes,
multiple keys, unsupported versions, standalone definitions and assembly context
chains do not become whole-document bindings. Feature source IDs/timestamps and
some duplicated body identity words are preserved as unresolved evidence until
current history/topology associations can be verified. This is a partial profile,
not a general MFC object-graph decoder or complete display-state inventory.

`GeometryDocument::presentation` adds face-local and body-default property slots
from the validated display cache without changing mesh order or geometry.
`cached_property(stream, offset)` selects only the stored cache property: a
face-local property outranks a cached body default, and conflicting/unresolved
higher-layer links block fallback. `document_appearance_for_state(config, state)`
looks up only a document default. Neither method claims the complete effective
native appearance when component/feature/face ownership is still unknown.

The viewer exports `presentations` keyed by document path, including embedded
paths. Hierarchy occurrences refer to that path and document-local
`physical_material_index`/`material_default_appearance_index` when resolved.
Repeated occurrences share a catalog. Existing `materials`, `material_index`
and `batches` remain the viewer's approximate rendering adapter; recovered native
links do not silently replace that adapter while higher-level ownership remains
incomplete. The WASM `presentation_json(bytes, filename)` entry point exposes the
metadata projection; geometry JSON additionally exposes cache-local bindings.

Use `mbx run --release --features archive --example inspect_appearances -- ARCHIVE --cached` to audit locally, with per-file source hashes and explicit unsupported
containers/geometry. Add `--hypotheses` for raw mapping records and provisional
interpretations. Original CAD and reports stay outside Git. Legacy OLE
containers remain unsupported by the runtime.

The next verification needs controlled SolidWorks saves: an unchanged baseline,
one part/body/feature/face override at a time, an override on just one of two
repeated assembly occurrences, two configurations and two display states, and
one texture-mapping parameter changed per save. Those comparisons are needed to
verify the remaining reference graph and projection semantics.

## Mate relationships

`Document::mates` inventories supported `Contents/Config-N-MatesList` streams,
retaining exact mate names, source offsets, explicit class families and unresolved
reused class tags. Saved selection/occurrence strings remain reference candidates;
`ConfigurationMates::reference_graph()` groups their mentions without inventing
component pairs, topology ownership or joints. The web inspector reports saved
mate header counts. Alignment, suppression, dimension/limit parameters and solving
remain unsupported. See [mate relations and decoding TODOs](docs/mates.md).

```sh
mbx run --locked --bin sw-dump -- mates assembly.SLDASM
mbx run --release --locked --features archive --example inspect_mates -- ARCHIVE.zip
```

## Document inventory and shared reader

Saved feature IDs and cross-record evidence are available through the opt-in
`DocumentReader::features()` / `references()` APIs and `sw-dump references FILE`.
These distinguish resolved XML relationships from provisional native ID matches;
they do not yet decode the complete persistent entity graph. See
[saved identities and reference decoding](docs/references.md).

The [binary reference investigation](docs/archive-reference-rules.md) documents
confirmed MFC rules, native corpus checks and unresolved mate-context hypotheses.
`ArchiveContext` provides checked class/object map operations for sequential
schema-specific decoders; the configuration-name reader now validates these maps.

**Inspect source…** in the web viewer opens a filterable stream inventory with
source hashes, validation results, opaque ranges and paged XML/binary inspection.
Legacy OLE and other unsupported native containers retain an opaque inventory.
Structural candidates and available decoders are distinguished from semantic
coverage; this is not a claim that every native object is understood.

For multiple projections, `DocumentReader::open(bytes, filename, limits)` validates
and decompresses once, then caches requested `metadata()` and `geometry()` results.
`stream(name)` borrows decoded bytes; `inventory()` and `inspect_stream(name)`
provide source/structural views. `finish()` transfers owned results and buffers.
Existing `parse`/`geometry` convenience APIs remain available. A shared checked
`BinaryReader` provides transactional primitive/UTF-16 reads for future decoders.
See [reader interface, inventory contract and measurements](docs/reader-inventory.md).

```sh
mbx run --locked --bin sw-dump -- inventory part.SLDPRT
mbx run --locked --bin sw-dump -- inventory part.SLDPRT swXmlContents/MATERIALTREE
```

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
