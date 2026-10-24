<!-- SPDX-License-Identifier: MIT -->
<!-- Copyright (c) 2026 hxyulin <hxyulin@proton.me> -->
# Rust and WebAssembly API

The core accepts bytes plus a filename hint and returns owned, serializable
records. It does not resolve host paths, fetch texture files, read a clock or
require a native CAD runtime. Full types and fields are in the
[generated Rust reference](https://hxyulin.github.io/sw-rs/api/sw_rs/).

## Choose an entry point

| API | Result and cost |
| --- | --- |
| `parse(bytes, filename)` | Metadata `Document`; validates and inflates the container |
| `geometry(bytes, filename)` | Cached Part `GeometryDocument`; independently validates and inflates |
| `DocumentReader::open(bytes, filename, limits)` | One validated/inflated container shared by requested projections |
| `Container::parse(bytes, limits)` | Selected named decoded streams and diagnostics |
| `metadata_from_container(container, filename)` | Consumes an already decoded container for metadata |
| `decode_zlb(bytes, limits)` | Standalone legacy wrapper decoder for an already extracted stream |

`parse_with_limits` and `geometry_with_limits` accept explicit resource budgets.
Use convenience functions for one projection; use the shared reader when requesting
several. `DocumentReader` is not a lazy decompressor: container inflation happens
at open, while metadata, geometry and other projections are computed on demand.

## Shared reader

```rust
use sw_rs::{DocumentReader, Limits};
let bytes = std::fs::read("part.SLDPRT")?;
let reader = DocumentReader::open(&bytes, "part.SLDPRT", Limits::default())?;
let metadata = reader.metadata()?;
println!("{} configurations", metadata.configurations.len());
let geometry = reader.geometry()?;
println!("{} mesh tables", geometry.model.tessellations.len());
let inventory = reader.inventory();
println!("source {}", reader.source_sha256());
let _ = inventory;
let output = reader.finish()?; // transfers requested results and decoded buffers
let _ = output;
```

`metadata()`, `geometry()`, `features()`, `references()` and `surface_references()`
cache their requested results. `stream(name)` borrows original decoded bytes;
`inspect_stream(name)` explicitly creates a structural inspection. `finish()`
moves caches into `ReadOutput`; it does not compute unrequested projections.
The reader borrows source bytes and uses single-owner `OnceCell` caches; it is
not a concurrent shared parser service. See [reader inventory](reader-inventory.md)
for ownership and measurements.

## Evidence and presentation

`document.presentation()` projects saved physical and visual records into a
renderer-independent model. It does not select an approximation shader.
Geometry adds cache-local properties; [presentation](presentation.md) explains
the difference between material assignment and effective appearance.

`reader.references()` returns the base source graph without geometry decoding.
`reader.references_with_geometry()` composes an owned graph with optional cached
surface evidence. It requires supported canonical Part geometry and returns an
error for an unsupported cache. Source hashes and configuration scope guard graph
composition. `surface_references()` preserves duplicate, opaque and conflicting
tuples; matches to saved features remain candidates. See [reference rules](references.md).

## Features

| Cargo feature | Additional capability |
| --- | --- |
| default (none) | Metadata, inventories, cached Part geometry and evidence APIs |
| `archive` | In-memory ZIP/TAR/TAR.GZ import and virtual file resolution |
| `wasm` | `wasm-bindgen` JSON functions; no archive support implied |

The viewer is a separate Cargo package with its own lockfile, RXUI application,
worker protocol and rendering adapter. It is not linked into the core library.

## Limits and errors

Default `Limits`: 256 MiB input, 64 MiB compressed per stream, 128 MiB decoded
per stream, 512 MiB total decoded output including duplicates, and 100,000 marker
candidates. `Limits::unlimited()` removes those policy caps for caller-owned
local imports; payload sizes, checksums, indices and finite values are still
validated. The viewer opts into unlimited policy caps. Available RAM, WASM
address space and GPU memory still constrain it.

Limits bound decoding rather than whole-process memory; XML, JSON and host copies
add overhead. Core parsing is synchronous; use a worker for large browser inputs.
A hard parse failure returns `ParseError`. Invalid individual container records
produce diagnostics and are not selected. Unsupported containers can still be
represented by `DocumentReader::inventory()`; successful `open()` alone does not
mean `metadata()` or `geometry()` will succeed.

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


All exports take `(Uint8Array, filename)` and return a JSON string or throw a
JavaScript error on failure:

| Export | Projection |
| --- | --- |
| `parse_json` | Saved document metadata |
| `geometry_json` | Versioned cached Part geometry envelope |
| `presentation_json` | Metadata-level material/appearance model |
| `inventory_json` | Source inventory, including opaque unsupported containers |
| `references_json` | Base features and reference graph |
| `geometry_references_json` | Graph composed with canonical cached Part surface evidence |

These convenience exports create a reader per call; they do not share a persistent
JavaScript document handle. The viewer's worker owns a separate caching integration.
Preview PNG bytes are retained in Rust rather than expanded into metadata JSON.
Do not assume raw mass tokens are physics-ready or that serialized local indices
remain stable after editing/reloading a source document.
