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
