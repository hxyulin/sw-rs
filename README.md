<!-- SPDX-License-Identifier: MIT -->
<!-- Copyright (c) 2026 hxyulin <hxyulin@proton.me> -->
# sw-rs

Read SolidWorks files in Rust, on native hosts and in WebAssembly.

[Documentation](https://hxyulin.github.io/sw-rs/docs/) · [SW Explorer](https://hxyulin.github.io/sw-rs/) · [Rust API](https://hxyulin.github.io/sw-rs/api/sw_rs/) · [Changelog](CHANGELOG.md)

Extract saved configurations, assembly occurrences, properties, materials and
cached Part meshes without a SolidWorks installation or native CAD library.
SW Explorer opens individual files and ZIP/TAR/TAR.GZ projects locally in your
browser, with an assembly hierarchy, source inspector and procedural materials.

| Read | Current scope |
| --- | --- |
| Metadata | Configurations, component paths/poses, properties, cut lists, drawing references |
| Geometry | Supported saved Part tessellations; assemblies composed from saved placements |
| Presentation | Physical materials, raw appearance records and partial scoped/cache bindings |
| Research | Source inventories, saved feature IDs, mate headers and surface-reference evidence |

This is a partial reader for modern inline-stream and legacy OLE files.
OLE loading validates sector chains and compressed streams; legacy binary
property sets, Parasolid topology, feature regeneration, complete appearance
ownership and mate solving remain unsupported. See the
[measured coverage](https://hxyulin.github.io/sw-rs/docs/parsing-coverage.html).

## Quickstart

Try [SW Explorer](https://hxyulin.github.io/sw-rs/) without installing anything.
For native JSON inspection, clone this repository and run:

```sh
cargo run --release --locked --bin sw-dump -- model.SLDASM
cargo run --release --locked --bin sw-dump -- geometry part.SLDPRT
cargo run --release --locked --bin sw-dump -- inventory part.SLDPRT
```

The byte-oriented Rust API has no filesystem or network dependency:

```rust,no_run
let bytes = std::fs::read("model.SLDASM")?;
let document = sw_rs::parse(&bytes, "model.SLDASM")?;
for configuration in &document.configurations {
    println!("{}: {} occurrences", configuration.name, configuration.components.len());
}
# Ok::<(), Box<dyn std::error::Error>>(())
```

The crate is not published yet. See [getting started](docs/getting-started.md)
for local dependencies, [API usage](docs/api.md) for shared-reader and WASM
examples, and [development](docs/development.md) for builds and checks.

## Documentation and contributing

The [documentation site](https://hxyulin.github.io/sw-rs/docs/) covers the
[library architecture](docs/architecture.md), [save-format research](docs/format.md),
[viewer](docs/viewer.md) and remaining decoding work. The Markdown sources live
in `docs/`; generated Rust reference is built from source comments.

See [CONTRIBUTING.md](CONTRIBUTING.md) for setup and validation. Keep private
CAD and research dumps outside Git; use synthetic fixtures for regressions.

## License

The combined library is **MIT AND Apache-2.0**: selected MIT
[openswx code](LICENSE-openswx) and attributed Apache-2.0 mesh/reference
adaptations. Retain both sets of notices; see [NOTICE.md](NOTICE.md) and
[NOTICE-cadmpeg.md](NOTICE-cadmpeg.md). Viewer fonts have separate SIL OFL terms.
No SolidWorks shader library, proprietary textures or source CAD is bundled.
