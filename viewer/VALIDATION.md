<!-- SPDX-License-Identifier: MIT -->
<!-- Copyright (c) 2026 hxyulin <hxyulin@proton.me> -->
# Web viewer validation — 2026-10-23

The viewer pins RXUI `14ec844a76dacf88231d0260bbe0f0713fa7f195`, which pins
Astrelis `4e360d5a3ad5dbe8bff11baf3eaa512a9241db21`. No sibling paths, modified
framework checkouts or native runner are used. The separate viewer Cargo lock
and the core lock are retained. Builds use mbx and the installed stable toolchain.

The root suite passed 12 parser tests, five archive tests and two doctests.
Archive cases include ZIP/TAR/TAR.GZ byte preservation, Unicode paths, ignored
files, CRC damage in ignored members, duplicates, unsafe paths, links, limits,
truncation and deterministic virtual reference resolution.

Four native viewer tests passed: metre/frame conversion and composition,
shared decoding across repeated/hidden occurrences, unknown transforms/cycles,
and visit-budget accounting even when no geometry succeeds. Native and WASM
Clippy passed with warnings denied; no dependency-specific lint waiver is used.

The linked WASM/RXUI app ran under Chromium through Playwright 1.64.1, with its
software WebGPU/WebGL adapter. Ten cases passed at each device pixel ratio, 1
and 2 (20 total):

- Startup and an actual RXUI canvas click loading the generated synthetic demo.
- ZIP, TAR and TAR.GZ imports, each resolving a four-occurrence assembly,
  selecting a Part, changing visible pixels by orbit and wireframe, and clearing.
- Bad archive recovery followed by successful direct native Part import.
- Actual RXUI Open action invoking the browser's file chooser.
- Cancellation invalidating a pending file read, followed by a successful import.
- Keyboard focus/Enter activation invoking the same file chooser.
- Optional privately held native archive import, worker parsing, CJK font loading
  and nonempty geometry, with no uncaught page errors.
- Resize to 960 × 700, then correctly aligned RXUI clicks and Part selection.

The optional private case used the existing small armor assembly and its two
Part definitions: three rendered occurrences, 38,420 triangles, zero scene
diagnostics. Saved source poses and metadata are displayed without assigning
physical validity. Original CAD and browser screenshots remain ignored local
artifacts. The ordinary public suite runs 18 cases and skips the two private
cases unless SW_VIEWER_ARCHIVE is supplied.

The generated demo has four occurrences, three shared mesh definitions and 48
triangles. It is a synthetic source-format fixture, not a SolidWorks-authored
model or an independent geometry oracle. The real-file result supplements it;
it does not establish complete CAD or cross-browser support.

T3's collaborative preview navigated to the app, but automation inspection failed
twice with `Preview automation snapshot timed out after 15000ms.` Inspection and
browser tests therefore used the repository's Playwright path. The local server
serves viewer/web only. Generated WASM, fixtures, fonts copied for serving, browser
artifacts, private source files, caches and node_modules are excluded from Git.

Observed and fixed during validation: canvas intrinsic sizing could expand its
grid track and misalign clicks; the host now constrains the grid track. Input
and test snapshots prepare dirty RXUI descriptions before using geometry. The
custom host honors RXUI keyboard default-prevention before applying Tab/activation
defaults. Worker identity and file-read generations prevent stale results from
reviving cancelled imports. Source CJK fonts are self-hosted with OFL notices.

Not certified by this run: Safari/Firefox, touch/pen navigation, a full DOM
accessibility bridge, GPU device-loss recovery, or arbitrary large browser heaps.
Compressed XZ/Zstandard TAR remains unsupported; ZIP, raw TAR and gzip TAR are
the implemented initial profile. The core's existing native cache limitations
remain explicit in the inspector.
