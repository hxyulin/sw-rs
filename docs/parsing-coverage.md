<!-- SPDX-License-Identifier: MIT -->
<!-- Copyright (c) 2026 hxyulin <hxyulin@proton.me> -->
# Measured parser coverage

Snapshot: 2026-10-24, EGA mechanisms plus the design library. Counts refer to
source entries, not unique designs. Eight library lock files are excluded.
No overall semantic parsing percentage is claimed: recovered bytes, recognized
records, resolved relationships and renderable geometry are different measures.

| Layer | Current measurement | Practical meaning |
| --- | --- | --- |
| Native container | 487/580 documents: EGA 340/340; library 147/240 | Modern named streams validate/decompress; 93 legacy OLE documents remain opaque |
| Stream inventory | 27,955 selected decoded streams in the matched inventory | Streams can be inspected; this is not complete interpretation of their binary objects |
| Configurations | 2,512 header records across 487 modern documents | Names, native indices, parent evidence and version-gated fields; map reuse is checked |
| Saved feature tree | 23,390 entries; 22,932 valid numeric IDs summed per document | Names, raw types, attributes, IDs and XML containment; no feature regeneration |
| XML relationships | 11,496 typed stream-local reference links | Unique file/model ID references; no inferred native topology |
| Part geometry | EGA 248/295; library 79/222 real external Part entries | Supported saved triangle caches for one canonical configuration |
| Assembly rendering | EGA 34/45 have some supported triangles at the initial viewer selection | Nonempty rendering is not complete assembly recovery |
| Mates | 2,675 headers; 149 with an explicitly identified family | Native count/header evidence; no resolved endpoints, parameters or solving |
| Mate surface descriptors | 4,983 strings: 4,217 complete, 568 opaque, 198 invalid | Typed saved ID tuples, not resolved mate participants |
| Cached surface references | 433,680 strings across 218,500 tables | Opt-in metadata interpretation; duplicate/conflicting tuples preserved |
| Cache-to-feature links | 215,030 unique source-ID matches | Candidate saved-feature links; current face/body ownership still unverified |
| Legacy compression helper | 490 extracted ZLB streams, 401 members, 45,323,897 decoded bytes | Complete wrapper decoding; OLE document loading is still pending |
| Materials and appearance | Definitions, configuration references, raw shader/texture/optical evidence and partial scoped/cache assignments | Physical and visual evidence stay separate; effective per-face/native rendering remains incomplete |
| Drawings | Basic sheet/view/reference metadata, tested separately with fixtures | No native drawing renderer; none are in this two-archive native census |

The EGA Part failures remain 23 without recognized supported faces and 24 with
ambiguous cache configuration ownership. Library failures comprise 87 legacy
OLE Parts, 55 modern configuration-ambiguous Parts and one modern Part without
recognized faces. Embedded/virtual Parts are outside these external-Part ratios;
the prior EGA survey found no supported cached faces in its 372 embedded entries.

The assembly census uses the initial viewer configuration with hidden geometry excluded.
The refreshed main EGA assembly remains at 3,380,059 triangles, 615 rendered
occurrences, one missing external reference and 1,169 visited hierarchy nodes.
Unsupported geometry and hidden/suppressed nodes still prevent treating this
as a complete recovered CAD model. The new rules do not add rendered geometry.

Exact B-Rep topology, sketch/feature evaluation, complete display-state/appearance
ownership and mate solving remain unsupported. Saved placements and procedural
appearance approximations continue to drive the viewer.

Reproduction and evidence:

- [Archive map rules](archive-reference-rules.md): documented framing and native prefix checks.
- [Surface reference rules](surface-reference-rules.md): tuple grammar, suffix collisions, source matching and opt-in APIs.
- [Legacy ZLB rules](legacy-zlb-rules.md): full multi-member decompression and the remaining OLE boundary.
- `audit_references ARCHIVE --geometry` and `audit_zlb DIRECTORY` examples report counts/hashes.
- `viewer/examples/inspect_project.rs` refreshes assembly renderability without a GPU.
- Private census, native probes and validation logs remain ignored in `local-assets/surface-investigation/`.
