<!-- SPDX-License-Identifier: MIT -->
<!-- Copyright (c) 2026 hxyulin <hxyulin@proton.me> -->
# SW Explorer

A local RXUI viewer for SolidWorks files and ZIP/TAR/TAR.GZ projects, with
native and browser hosts.

Run the native workbench with the default system theme:

```sh
cargo run --manifest-path viewer/Cargo.toml --features native --bin sw-viewer
# Optionally append: -- /path/to/model.SLDPRT
```

It has a native file picker, document search, occurrence tree, model viewport and
property inspector. Drag to orbit, Shift-drag or right-drag to pan, and scroll to
zoom. Native development needs no browser, Node or WebAssembly build.

[Open the app](https://hxyulin.github.io/sw-rs/) · [Viewer guide](../docs/viewer.md) · [Build and test](../docs/development.md)

```sh
# From the repository root, with Rust/WASM and matching wasm-bindgen installed:
sh viewer/build.sh
python3 -m http.server 8097 --bind 127.0.0.1 --directory viewer/web
```

Open <http://127.0.0.1:8097/>. Uploads stay in the browser. The
[full guide](../docs/viewer.md) covers assembly selection, hierarchy, search,
materials, coordinate controls, source inspection and known limitations.
