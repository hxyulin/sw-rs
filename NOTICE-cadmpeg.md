<!-- SPDX-License-Identifier: Apache-2.0 -->
# cadmpeg tessellation adaptation

`src/mesh.rs` adapts the descriptor-table, class-reuse and strip decoding routines
from `vendor/cadmpeg-codec-sldprt/src/tessellation.rs` and header offsets in
`src/layout.rs` at sldkit MIT revision
`c43b5ef752a0e58535b7f8f744f7d7c68e3c0e90`.

Original project: cadmpeg, https://github.com/cadmpeg/cadmpeg
Original registry version: 0.5.3, upstream revision
`dbb308e52ab05c911b26963a02fbb294eefab3ce`.
The source is Apache-2.0, modified by sldkit as 0.5.3+sldkit.4.
The upstream SPDX notice and modification notice are retained in the adapted file.
See LICENSE-Apache-2.0 for the complete license and
https://github.com/monozukuri-ai/sldkit/blob/c43b5ef752a0e58535b7f8f744f7d7c68e3c0e90/vendor/cadmpeg-codec-sldprt/PATCHES.md
for original patch provenance. No source CAD or newer noncommercial sldkit code
is included. Changes: remove CAD IR/native dependencies, expose array meshes with
source byte ranges, add resource checks, and omit topology/ownership inference.
The adapted module retains Apache-2.0 terms; MIT applies to the existing port
under its retained notices, rather than relicensing Apache material as MIT.

## Native appearance framing

`src/visual.rs` adapts visual-property prefix decoding, part-default class chains,
feature source/timestamp prefixes, and cache-property precedence from Apache-2.0
cadmpeg revision `4df21312c62cc1207a4c2274e29b4118fd232ec6`, specifically
`crates/cadmpeg-codec-sldprt/src/appearance.rs`,
`src/appearance/visual_states.rs`, and `src/layout.rs`.

Changes: remove CAD IR, B-Rep and native-runtime dependencies; preserve variable
UTF-16 strings, optical bits and partial raw texture/mapping blocks; expose
configuration/display-state scopes, decoded byte provenance, unresolved native
entity identities and cache-local table bindings. Typed body-property slots are
validated against local files; declaration intervals are not treated as a complete
MFC reference graph. Native topology ownership is not inferred. Synthetic tests
are authored here. No source CAD, original shader implementations or texture
assets are distributed. The adapted module retains Apache-2.0 terms.

The checked UTF-16 string cursor was extracted from `src/visual.rs` into
`src/binary.rs` for shared use by record decoders. Changes: transactional reads,
checked borrowed slices, public primitive helpers, and direct surrogate-aware
Unicode decoding without an intermediate UTF-16 allocation. It retains
Apache-2.0 terms and the appearance-framing provenance above.

## Saved feature identities

`src/features.rs` adapts the `Keywords` feature-record interpretation from
`crates/cadmpeg-codec-sldprt/src/history/mod.rs` and `src/records.rs` at the same
Apache-2.0 revision `4df21312c62cc1207a4c2274e29b4118fd232ec6`.
Changes: use existing source-aware structural XML inspection; retain duplicate
IDs, original tags/attributes, reserved tokens and source byte spans; omit
operation evaluation, inferred suppression, IR projection and topology binding.
Saved XML preorder is not asserted to be regeneration order. The adapted module
retains Apache-2.0 terms; new reference-graph infrastructure is MIT.

## Saved surface-reference tokens

`src/surface.rs` adapts persistent surface-reference string interpretation and
duplicate-consensus rules from `crates/cadmpeg-codec-sldprt/src/tessellation.rs`
at Apache-2.0 revision `4df21312c62cc1207a4c2274e29b4118fd232ec6`.
Changes: preserve raw framed text/offsets, class names, invalid required fields,
opaque tail tokens and terminal-comma evidence; support checked extended CString
lengths; keep complete tuples and partial feature references separate; expose an
opt-in source-scoped inspection without CAD IR or native topology dependencies.
Native B-Rep face/body ownership is not inferred. The adapted module retains
Apache-2.0 terms. New fixtures and reader/graph integration retain their own notices.
