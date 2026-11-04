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

The browser can render supplied PNG, JPEG, BMP and WebP textures for supported
cache-table or unambiguous document appearances. Saved mode also applies decoded
cache or uniquely scoped document RGB without requiring a physical material
assignment. Supported complete native archives also recover owner/state links and
project feature/body colors onto saved caches, with uncertain references kept
diagnostic. Put the image files in the same archive as the CAD. Placement uses a labelled approximate Part-local projection; SolidWorks UVs
and face-level texture assignments remain unsupported. Missing or ambiguous images
keep the existing shading. Desktop bitmap rendering is not implemented.
