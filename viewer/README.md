<!-- SPDX-License-Identifier: MIT -->
<!-- Copyright (c) 2026 hxyulin <hxyulin@proton.me> -->
# SW Explorer

A local RXUI/WebAssembly viewer for SolidWorks files and ZIP/TAR/TAR.GZ projects.

[Open the app](https://hxyulin.github.io/sw-rs/) · [Viewer guide](../docs/viewer.md) · [Build and test](../docs/development.md)

```sh
# From the repository root, with Rust/WASM and matching wasm-bindgen installed:
sh viewer/build.sh
python3 -m http.server 8097 --bind 127.0.0.1 --directory viewer/web
```

Open <http://127.0.0.1:8097/>. Uploads stay in the browser. The
[full guide](../docs/viewer.md) covers assembly selection, hierarchy, search,
materials, coordinate controls, source inspection and known limitations.
