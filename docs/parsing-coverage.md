<!-- SPDX-License-Identifier: MIT -->
<!-- Copyright (c) 2026 hxyulin <hxyulin@proton.me> -->
# Measured parser coverage

## OLE follow-up, 2026-10-25

The reader now loads validated legacy OLE streams. Container loading, partial
metadata and saved-mesh recovery are measured separately:

- Public corpus: all 28 unique OLE files load, with all 736 extracted stream
  lengths/hashes and physical ranges matching an independent Python OLE/zlib
  reader. The 25 controlled SW2011 Parts all yield cached meshes, with table counts
  matching the author's recorded SolidWorks face counts. Three older Parts still
  have absent or unsupported cached geometry.
- Regression check: all 344 previously supported public modern source entries
  retain identical metadata and inventories, geometry success/failure, and saved
  table/triangle counts. These entries include three duplicate source hashes.
- Design-library follow-up: 92/93 OLE documents load, exposing 2,875 streams,
  including 485 validated ZLB wrappers. Cached meshes recover for 86/87 legacy
  external Parts. One Part fails strict CFB validation because its directory
  red-black tree has adjacent red nodes; no permissive repair is attempted.

Public files were pinned to `blussyya/sldprt-research-dump` revision
`94478c839d4f4885f811aae8e45d70076476a079` and the four demo fixtures in
`blussyya/sldprt-format-research` revision
`7a28c8bbc2d5372ab45e22f8d6f37ff8ed12ea2b`. The latter are byte-identical copies
of corpus files and are not counted twice. The controlled source includes
[native exports and a SolidWorks build log](https://github.com/blussyya/sldprt-research-dump/tree/94478c839d4f4885f811aae8e45d70076476a079/test%20files%20new).
No public or private source CAD is bundled. Per-file reports and comparison scripts
remain ignored under `local-assets/public-corpus-2026-10-25/` and
`local-assets/ole-investigation/`.

Native face-count agreement does not establish face identity, full geometric
equivalence or B-Rep support. Legacy binary property sets remain opaque. Missing
and ambiguous modern caches, native topology and mate endpoints remain open.

## Prior modern-container census

Historical snapshot: 2026-10-24, before OLE loading, EGA mechanisms plus the design library. Counts refer to
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
| Legacy compression helper | 490 extracted ZLB streams, 401 members, 45,323,897 decoded bytes | Complete wrapper decoding; document loading was not included in this snapshot |
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
