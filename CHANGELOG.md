<!-- SPDX-License-Identifier: MIT -->
<!-- Copyright (c) 2026 hxyulin <hxyulin@proton.me> -->
# Changelog

## Unreleased

- Initial MIT-licensed Rust port of openswx's modern container and metadata
  readers, including native JSON inspection and optional WebAssembly bindings.
- Bound decoded allocations and record counts; validate sizes/checksums and
  preserve unknown component transforms and unvalidated physical cache tokens.
