<!-- SPDX-License-Identifier: MIT -->
<!-- Copyright (c) 2026 hxyulin <hxyulin@proton.me> -->
# Initial port validation — 2026-10-23

Compared this initial Rust implementation against openswx revision
`a838a83e267fadbf8f01fd75a288b2ae517c9133`, using the existing private EGA corpus
from the rm-simulator import experiments. The corpus contains 295 Parts and
45 Assemblies. Source CAD and per-document outputs remain outside Git.

The reproducible `scripts/compare-corpus.py` run found:

- 340/340 native documents parsed.
- 1,418 configuration names, in the same order as upstream.
- 916 component records matched, including source paths, child configurations,
  suppression, visibility, BOM flags, references and saved transform tokens.
- 617 cached mass records matched exactly after normalizing COM JSON shape.
- Native version and global/configuration property maps matched.
- Zero differences in these compared fields. All 916 transforms in this corpus
  were present; synthetic tests cover missing and malformed transforms.

This establishes parity for those fields on that corpus. It does not establish
correct physical units, transform composition, complete format support, geometry
recovery, or behavior on other SolidWorks versions. Cut lists, per-configuration
previews, parent indices and drawing semantics were not compared by the corpus
runner. Parent-index decoding has a synthetic version-gated regression test.

The local full report (including per-document SHA-256) is ignored at
`local-assets/corpus-comparison.json`. The executable used by that run is
`local-assets/sw-dump-tested`; it is a snapshot of the release build so mbx
managed-target rotation cannot invalidate a running corpus probe.

Native regression coverage includes valid/empty streams, duplicate selection,
CRC failure and recovery, truncation, missing deflate terminators, unsupported
containers, malformed inputs, expansion/aggregate/record limits, Unicode XML,
configuration property override precedence, binary configuration names and
parent IDs, cached tokens and absent/malformed transforms. A library doctest
covers invalid input.

The library compiles to `wasm32-unknown-unknown` with `--features wasm`. Matching
wasm-bindgen 0.2.129 generated Node bindings; `scripts/wasm-smoke.cjs` exercised
actual WASM execution from a Uint8Array, Unicode metadata returned through JSON,
and an invalid-input exception. This is a JavaScript/WASM smoke test, not browser
UI or browser memory/performance validation. Web-target bindings are generated
separately using `wasm-bindgen --target web`.

Resolved direct and transitive dependency license expressions were inspected
using `cargo metadata --all-features --locked`: all offer permissive terms
(MIT/Apache-2.0, 0BSD, Zlib, Unlicense, and Unicode-3.0 as applicable).
No upstream CAD SDK, native zlib, or noncommercial parser dependency is linked.
