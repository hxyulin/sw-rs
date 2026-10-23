<!-- SPDX-License-Identifier: MIT -->
<!-- Copyright (c) 2026 hxyulin <hxyulin@proton.me> -->
# sw-rs

A pure Rust, byte-oriented reader for modern SolidWorks files, with optional
WebAssembly bindings. The name is provisional. This is an initial metadata port
of [openswx](https://github.com/schwitters/openswx) at
`a838a83e267fadbf8f01fd75a288b2ae517c9133`, under MIT.

The library accepts bytes and a filename hint. It has no filesystem access,
networking, C++ FFI, SolidWorks installation, or simulator dependency. Native
file access lives in the `sw-dump` CLI. All resolved dependencies offer permissive
licenses; upstream copyright and permission notices are retained in
[LICENSE-openswx](LICENSE-openswx). No sldkit code or source CAD is included.

## Current scope

- Modern (2015+) inline chunk streams: per-file name rotation, pure Rust raw
  deflate, decoded-size and CRC32 checks, first-valid duplicate selection.
- Native version, global/configuration properties, Unicode configuration names,
  saved parent indices, cached mass-property tokens, document PNG previews.
- Assembly occurrence names, external paths, referenced configurations,
  suppression/visibility/BOM flags, and optional saved 16-value transforms.
- Raw decoded streams and diagnostics for later format work.

This is **not a complete openswx port** or a CAD kernel. OLE2 and ZIP/OPC return
unsupported-format errors. Drawing containers can be inspected, but drawing
semantics, cut lists, per-configuration previews, geometry/tessellation, Parasolid
B-Rep, STEP export, mates and BOM file resolution are not implemented. Unknown
container record families remain diagnostics. Malformed XML is reported.

Mass, COM, volume and inertia retain source numeric tokens: units, frame, signs
and cache freshness are unverified. Do not feed these directly to physics.
Transforms use openswx's column-major interpretation; agreement with upstream
is not independent placement validation. Absent/malformed transforms are `None`
(JSON `null`), rather than fabricated identity matrices.

## Native use

Use `mbx` for compiling commands when installed; otherwise substitute `cargo`.

```sh
mbx run --release --bin sw-dump -- /path/to/model.SLDASM
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
of input, 128 MiB per decoded stream, 512 MiB cumulative decoded output (including
duplicates), and 100,000 marker candidates. These bound decoding, not total
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

`parse_json` accepts `Uint8Array`, returns a JSON string, and throws on a hard
parse error. `wasm-pack build --target web --features wasm` is an alternative
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
Metadata interchange compatibility and Part tessellations are the next steps;
complete portable Parasolid/drawing support can follow in tested milestones.
General mate solving and feature-history regeneration require separate designs
and validation.
