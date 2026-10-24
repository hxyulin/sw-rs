<!-- SPDX-License-Identifier: MIT -->
<!-- Copyright (c) 2026 hxyulin <hxyulin@proton.me> -->
# Getting started

## Browser

Open [SW Explorer](https://hxyulin.github.io/sw-rs/) and select a native
`.SLDPRT`, `.SLDASM` or `.SLDDRW`, or a ZIP, raw TAR or TAR.GZ archive.
If multiple assemblies are present, choose one in the assembly dialog.
Parsing runs locally in a worker. See the [viewer guide](viewer.md) for
hierarchy navigation, materials, source inspection and import limitations.

## Native CLI

Install Rust stable, clone the repository, then:

```sh
git clone https://github.com/hxyulin/sw-rs.git
cd sw-rs
cargo run --release --locked --bin sw-dump -- model.SLDASM
cargo run --release --locked --bin sw-dump -- geometry part.SLDPRT
```

If installed, use `mbx` in place of `cargo` for compiling commands to share the
compiler cache. The [CLI reference](cli.md) lists every mode.

## Rust dependency

The crate is currently unpublished. In a sibling checkout, use a path dependency:

```toml
[dependencies]
sw-rs = { path = "../sw-rs" }
```

```rust
let bytes = std::fs::read("part.SLDPRT")?;
let document = sw_rs::parse(&bytes, "part.SLDPRT")?;
println!("{} configurations", document.configurations.len());
```

The filename is only a type hint; the library never opens it. For metadata,
geometry and inventories from the same file, prefer [DocumentReader](api.md).
Default [resource limits](api.md#limits-and-errors) apply unless explicitly overridden.

## Build the viewer

Install the WASM target and a `wasm-bindgen-cli` version matching **viewer/Cargo.lock**:

```sh
rustup target add wasm32-unknown-unknown
# Read the locked wasm-bindgen version; do not choose an unrelated CLI version.
cargo install wasm-bindgen-cli --version "$(sed -n '/name = "wasm-bindgen"/{n;s/version = "\(.*\)"/\1/p;}' viewer/Cargo.lock)" --locked
sh viewer/build.sh
python3 -m http.server 8097 --bind 127.0.0.1 --directory viewer/web
```

Open <http://127.0.0.1:8097/>. Use HTTP or HTTPS: browser workers and modules
cannot load reliably through `file://`. See [development](development.md) for
browser prerequisites, tests and the documentation site.
