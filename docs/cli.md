<!-- SPDX-License-Identifier: MIT -->
<!-- Copyright (c) 2026 hxyulin <hxyulin@proton.me> -->
# Command-line reference

`sw-dump` reads one native file and writes JSON to stdout. Install/build from
this checkout; the package is not published. The CLI uses bounded default
`Limits` and has no flag to remove them. Archive composition lives in the viewer
and audit examples, not this command.

```sh
mbx run --release --locked --bin sw-dump -- [MODE] FILE [OPTION]
```

| Invocation after `--` | Output |
| --- | --- |
| `FILE` or `metadata FILE` | Saved metadata (`sw-rs-metadata-v1`) |
| `geometry FILE.SLDPRT` | `sw-rs-geometry-v1` envelope |
| `mates FILE.SLDASM` | `sw-rs-mates-v1` header inventory |
| `inventory FILE` | Source inventory and no selected stream inspection |
| `inventory FILE STREAM_NAME` | Inventory plus structural XML/binary inspection |
| `references FILE` | Source-reference graph, without geometry |
| `references FILE.SLDPRT --geometry` | Graph plus canonical cached surface evidence |
| `--version` | Package version |

```sh
mbx run --release --locked --bin sw-dump -- inventory part.SLDPRT swXmlContents/MATERIALTREE
mbx run --release --locked --bin sw-dump -- references part.SLDPRT --geometry
```

Geometry failure writes a JSON envelope with `geometry: null` and a diagnostic,
then exits unsuccessfully. Other hard failures exit unsuccessfully through the
Rust error result; scripts must check the exit status rather than relying on a
uniform error envelope. Inventory can describe a recognized unsupported container
without successfully interpreting its contents.

## Research examples

Run with `mbx run --release --locked --example NAME -- ARGS`. Add
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
mbx run --release --locked --manifest-path viewer/Cargo.toml --example inspect_project -- archive.zip
```

Keep original CAD and per-file audit JSON in ignored `local-assets/`; public
counts and interpretations belong in the format research pages.
