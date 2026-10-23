<!-- SPDX-License-Identifier: MIT -->
<!-- Copyright (c) 2026 hxyulin <hxyulin@proton.me> -->
# Attribution and port provenance

This repository ports selected portions of **openswx** from C++20 to Rust:
https://github.com/schwitters/openswx/tree/a838a83e267fadbf8f01fd75a288b2ae517c9133

Copyright (c) 2026 openswx contributors. MIT License.
The complete upstream notice is retained verbatim in `LICENSE-openswx` and must
accompany distributions containing this port, together with this project's
`LICENSE`. Adapted modules also retain upstream attribution in their headers.

Source mapping:

- `src/container.rs`: `libopenswx/src/internal/modern_parser.cc`,
  `rol_codec.h`, and the raw-deflate contract in `decompressor.cc`.
- `src/config.rs`: `libopenswx/src/internal/cmgrhdr_reader.h`.
- `src/document.rs`: `libopenswx/src/document.cc` and saved record contracts.
- `src/xml.rs`: `libopenswx/src/internal/xml_readers.cc`.

Rust-specific adaptations include checked slice access, resource budgets, strict
finite numeric parsing, explicit unknown transforms, and optional wasm-bindgen
bindings. This port does not claim full upstream feature parity.

The mesh decoder adapts a narrow Apache-2.0 cadmpeg implementation from the
older MIT-licensed sldkit snapshot; see `NOTICE-cadmpeg.md` and
`LICENSE-Apache-2.0` for its independent terms and modification provenance.
No newer PolyForm Noncommercial sldkit code, AGPL sldprt2xt code, proprietary
SolidWorks SDK, or proprietary Parasolid SDK is included. The combined package
requires the retained MIT and Apache-2.0 notices (`MIT AND Apache-2.0`).
The source CAD used locally for comparison is not part of this repository and
is not relicensed by this project's license.

SolidWorks is a trademark of its respective owner. This independent project is
not affiliated with or endorsed by the trademark owner.

The optional viewer pins RXUI and Astrelis under their MIT OR Apache-2.0 terms.
The application-owned canvas host follows their documented browser integration
APIs; neither native window services nor private browser probes are dependencies.
Embedded Source Sans 3 and the renamed Noto Sans SC-derived CJK subset retain
separate SIL OFL 1.1 terms and original copyright notices under `viewer/assets/`.
They are font assets, not relicensed source code. See `viewer/assets/README.md`
for pinned sources, checksums and the reproducible CJK subset process.
