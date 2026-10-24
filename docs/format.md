<!-- SPDX-License-Identifier: MIT -->
<!-- Copyright (c) 2026 hxyulin <hxyulin@proton.me> -->
# SolidWorks save-format guide

This is a record of the supported profiles and research in sw-rs, not an official
SolidWorks file specification. A file can contain readable metadata while its
geometry or entity relationships remain unsupported. Native version words,
serialization schemas and document configuration indices are different values.

## Layers of a saved document

| Layer | What we know | Further reading |
| --- | --- | --- |
| Container | Modern named compressed inline streams; legacy OLE recognized but not loaded | [Containers](containers.md) |
| Payload structure | XML trees, binary scalars and framed strings, partial MFC serialization | [Metadata](metadata.md), [archive maps](archive-reference-rules.md) |
| Document records | Properties, configurations, occurrences, references, cut lists and drawing metadata | [Metadata](metadata.md) |
| Display cache | Saved Part triangles, normals and cache-local property slots | [Geometry](geometry.md) |
| Materials/visuals | Physical definitions and assignments; partial visual states and property records | [Presentation](presentation.md), [appearance research](appearance-research.md) |
| Identity | XML IDs, saved feature IDs, surface tuples and unresolved native ownership | [Reference graph](references.md), [surface rules](surface-reference-rules.md) |
| Relations | Saved mate headers and selection strings, not decoded constraints | [Mates](mates.md) |
| Inventory | Source hashes, record validation, duplicates, opaque spans and structural candidates | [Reader/inventory](reader-inventory.md) |

## Container recovery is only the first step

Modern documents repeat named inline records. The reader validates names, sizes,
raw DEFLATE boundaries and CRCs and keeps the first valid stream of each name.
Recovered streams include XML metadata, configuration headers, display data,
visual states and native body/history payloads. Their names do not prove that we
understand their contents. Tail/index records are not compressed inline payloads.

Legacy files use Microsoft Compound File Binary/OLE structured storage, with some
logical streams wrapped in `__ZLB` compression. We can validate that wrapper for
already extracted streams; runtime OLE enumeration and provenance remain pending.
A ZIP archive uploaded to the viewer is an outer collection of native documents,
not a newly supported ZIP/OPC native SolidWorks container.

## Identities are contextual

SolidWorks must reconnect saved data, but not every saved integer is a globally
stable face ID. Archive maps depend on sequential serialization. Surface tuples
include feature source, local surface and ordered tail fields. Current topology,
feature history/timestamps, configuration, display state and component occurrence
can all affect the final association. Public API persistent-reference contracts
do not specify their native file encoding.

The parser therefore retains raw records and context while exposing only verified
links as resolved. Matching a source ID or finding a class-name string is evidence,
not proof that a whole native object graph has been decoded.

## Evidence levels

1. **Documented framing:** public Microsoft MFC/CFB contracts establish generic
   serialization behavior, not every SolidWorks payload schema.
2. **Implemented profile:** checked parsing plus authored tests and native probes
   establish the supported record layout.
3. **Corpus observation:** counts, repeated values and associations describe the
   tested sample, not all SolidWorks versions or future saves.
4. **Hypothesis:** plausible interpretations remain provisional with explicit
   TODOs; they do not enable coverage flags or native face assignments.

The linked research pages retain pinned prior-art sources, exact counts,
reproduction commands and remaining validation work. Private models, per-file
offsets/hashes and extraction artifacts stay ignored; no proprietary textures or
shader implementation are distributed.

## What remains unknown

OLE document integration, full native object payload boundaries, feature history
and Parasolid topology, current body/face ownership, complete display-state and
occurrence appearance overrides, verified texture projection, mate parameters
and endpoints, custom coordinate systems and drawing rendering remain open.
Controlled one-change SolidWorks saves are especially useful for appearance
ownership and texture mapping; corpus-only hypotheses are labeled accordingly.
See the [coverage snapshot](parsing-coverage.md) for separate measured layers.
