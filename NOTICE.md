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

No code from sldkit (which includes PolyForm Noncommercial terms), sldprt2xt
(AGPL), a proprietary SolidWorks SDK, or a proprietary Parasolid SDK is included.
The source CAD used locally for comparison is not part of this repository and
is not relicensed by this project's license.

SolidWorks is a trademark of its respective owner. This independent project is
not affiliated with or endorsed by the trademark owner.
