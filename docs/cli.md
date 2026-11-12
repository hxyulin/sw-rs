<!-- SPDX-License-Identifier: MIT -->
<!-- Copyright (c) 2026 hxyulin <hxyulin@proton.me> -->
# Command-line reference

`sw-dump` reads one native file and writes JSON to stdout, except for the raw
`stream` mode. Install/build from
this checkout; the package is not published. The CLI uses bounded default
`Limits` and has no flag to remove them. Archive composition lives in the viewer
and audit examples, not this command.

```sh
cargo run --release --locked --bin sw-dump -- [MODE] FILE [OPTION]
```

| Invocation after `--` | Output |
| --- | --- |
| `FILE` or `metadata FILE` | Saved metadata (`sw-rs-metadata-v1`) |
| `geometry FILE.SLDPRT` | `sw-rs-geometry-v1` envelope |
| `geometry-status FILE.SLDPRT` | Display-cache availability, face counts and configuration ambiguity |
| `mates FILE.SLDASM` | `sw-rs-mates-v1` header inventory |
| `assembly-analysis FILE.SLDASM` | Configuration-local occurrences, mate evidence and source-reference graph |
| `stream FILE STREAM_NAME` | Exact decoded stream bytes on stdout, with no JSON wrapper |
| `inventory FILE` | Source inventory and no selected stream inspection |
| `inventory FILE STREAM_NAME` | Inventory plus structural XML/binary inspection |
| `model-headers FILE` | Complete local model-header report (`sw-rs-model-headers-v1`) |
| `references FILE` | Source-reference graph, without geometry |
| `references FILE.SLDPRT --geometry` | Graph plus canonical cached surface evidence |
| `--version` | Package version |

```sh
cargo run --release --locked --bin sw-dump -- inventory part.SLDPRT swXmlContents/MATERIALTREE
cargo run --release --locked --bin sw-dump -- references part.SLDPRT --geometry
cargo run --release --locked --bin sw-dump -- assembly-analysis assembly.SLDASM > assembly.json
cargo run --release --locked --bin sw-dump -- stream assembly.SLDASM Contents/Config-0-MatesList > mates.bin
```

## Assembly analysis

`assembly-analysis` writes `sw-rs-assembly-analysis-v1`. It retains source SHA-256,
native version, configuration indices and names, exact occurrence identities,
saved local transforms, suppression, visibility, BOM exclusion and explicit
rigid/flexible flags. Components are direct saved occurrences; this command does
not recursively open external files or compose world transforms.

Each configuration includes its mate inventory and a bipartite graph linking exact
saved reference strings to mate record indices. Native configuration indices remain
present even when their names are unavailable. The source-reference graph and explicit path
replacement records retain ambiguous and unresolved identities. Matching strings
do not establish physical mate endpoints or a solved component graph.

For reused mate tags, `class_reference_resolution` provides the exact declaration
and leading-index offsets when the supported local anchor establishes a type.
Other tags remain unknown. This resolves a subset of types, not native endpoints
or parameters. See the [leading class-index profile](mates.md#leading-class-index-profile).

Mate `parameter_candidates` retain supported named scalar prefixes and byte
offsets. Explicit length/angle parameter declarations establish metres/radians;
reused unknown classes have a null unit. A scalar in a limit mate is not identified
as its lower or upper bound. See [numerical mate evidence](mates.md#numerical-parameter-prefixes).

The coverage object reports grounding, limit bounds, mate ownership and solving
as unsupported. Fixed state is unknown, even when a transform is identity or
mates appear to constrain a component. Saved flexible flags remain separate from
grounding. Missing values never become zero bounds or unconstrained motion.

`stream` exports the reader's decoded bytes with the same limits and integrity
checks as other modes. Its offsets match inventory inspections and mate evidence;
it does not export the encoded container record. An absent stream fails without
writing payload bytes. Redirect stdout to a file for binary streams.

Geometry failure writes a JSON envelope with `geometry: null` and a diagnostic,
then exits unsuccessfully. Other hard failures exit unsuccessfully through the
Rust error result; scripts must check the exit status rather than relying on a
uniform error envelope. Inventory can describe a recognized unsupported container
without successfully interpreting its contents.

## Display-cache diagnosis

```sh
cargo run --release --locked --bin sw-dump -- geometry-status part.SLDPRT
```

This opt-in Part inspection writes `sw-rs-geometry-cache-inspection-v1`. It
distinguishes an absent DisplayLists carrier, multiple ambiguous carriers, no
supported face records, unsupported/invalid face records, and supported faces
whose configuration assignment is ambiguous. These are successful analytical
results; corrupt containers and non-Part inputs still fail.

Each carrier retains its decoded stream name, size, face-marker presence and
supported descriptor-table offsets with vertex/triangle counts. All saved
configuration indices and names remain visible. Multiple configurations leave
`assigned_configuration_index` null even when face decoding succeeds. Only the
existing single-configuration profile supplies an index. This does not select
the first configuration or allow those faces to be attached to an assembly.

The inspection decodes face tables to count them, then releases mesh buffers.
It does not decode configuration-to-cache ownership, prove cache completeness
or freshness, or establish B-Rep body ownership. It has no fallback that invents
meshes from bounding boxes or silently attaches another configuration's geometry.

## Complete model-header inspection

```sh
cargo run --release --locked --bin sw-dump -- model-headers model.SLDASM
```

This opt-in mode reports each exact `Contents/Config-N-ModelHeader` stream. It
selects a layout only when one supported attempt reads a complete non-null header
and consumes the entire stream. All six layout attempts remain in the output.
Unsupported and ambiguous streams have no selected `decoding`; they remain in the
report with their field failures or unread-byte intervals. These per-stream results
do not make the command fail. Invalid containers still return a hard error.

A selected decoding includes the explicit layout, root pointer, archive-local slot
ledger and typed definitions with source spans. Raw strings, scalar words and
bytes retain their saved values. Header-local success does not establish a
configuration map handoff, active dependencies, mate endpoints or appearance
ownership. No numeric filename/version heuristic selects the layout. The public modern
corpus report uniquely selects all 358 model-header streams across 341 unique
source files. Unsupported or ambiguous layouts in other files remain reported.

## Research examples

Run with `cargo run --release --locked --example NAME -- ARGS`. Add
`--features archive` before `--example` for ZIP corpus examples.

| Example | Purpose |
| --- | --- |
| `audit_references ARCHIVE --geometry` | Feature/graph and cached surface census |
| `audit_archive_rules ARCHIVE` | Configuration archive-map prefix audit |
| `audit_zlb EXTRACTED_STREAM_OR_DIRECTORY` | Legacy multi-member wrapper verification |
| `inspect_mates ARCHIVE` | Mate header and selection-string evidence |
| `inspect_appearances ARCHIVE --cached --hypotheses` | Raw appearance data and opt-in mapping hypotheses |
| `investigate_geometry ARCHIVE` | Saved cache availability and failure categories |

For native scene composition without a GPU:

```sh
cargo run --release --locked --manifest-path viewer/Cargo.toml --example inspect_project -- archive.zip
```

Keep original CAD and per-file audit JSON in ignored `local-assets/`; public
counts and interpretations belong in the format research pages.
