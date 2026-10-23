<!-- SPDX-License-Identifier: MIT -->
<!-- Copyright (c) 2026 hxyulin <hxyulin@proton.me> -->
# SW Explorer — RXUI web viewer

Live app: **https://hxyulin.github.io/sw-rs/**

Source: https://github.com/hxyulin/sw-rs

A local-first browser viewer for native SolidWorks documents in ZIP, TAR and
TAR.GZ/TGZ archives, or an individual SLDPRT/SLDASM/SLDDRW file. RXUI draws the
browser and inspector through WebGPU. An app-owned WebGL 2 viewport renders
Rust-decoded meshes; there is no Bevy, winit, server-side parser or CAD SDK.

Uploaded bytes stay in a module worker in this browser. The static server serves
only the app, its WASM, fonts and optional synthetic demo. It receives no uploaded
archive bytes. Parsing is cancelable by terminating the worker; stale results
and pending file reads cannot replace a newer import. Geometry transfers as
Float32Array buffers, rather than JSON coordinate arrays.

## Build and run

Prerequisites: stable Rust with `wasm32-unknown-unknown`, Python 3, and a
`wasm-bindgen` CLI matching the version in `Cargo.lock` (currently 0.2.129).
Compilation uses mbx when available, otherwise Cargo. No npm build is required.

```sh
# From sw-rs:
sh viewer/build.sh
python3 -m http.server 8097 --bind 127.0.0.1 --directory viewer/web
```

Open <http://127.0.0.1:8097> in a browser supporting WebGPU and WebGL 2.
WebGPU requires a secure context: localhost or HTTPS. Opening index.html directly
through file:// does not load browser modules/workers correctly. Deploy the
contents of `viewer/web/` as static files, preserving its relative paths, and
serve .wasm with application/wasm. No cross-origin isolation headers are needed.

The generated demo is a synthetic native-container fixture with an assembly and
three Part definitions, not a SolidWorks-authored model. Its source generator is
`scripts/make-fixtures.py`. No privately held CAD is bundled.

## Use

Open or drop an archive. The shallowest assembly is initially selected; choose
another document or saved configuration in the RXUI sidebar. Suppressed
occurrences stay excluded. **Show hidden Parts** controls saved hidden flags.
Repeated occurrences retain distinct source poses while sharing decoded geometry.

Drag in the viewport to orbit; Shift-drag or right-drag pans. Wheel zooms, and
**Fit view**, double-click or F fits the model. **Wireframe** changes the drawing
mode. The sidebar supports Tab/Shift-Tab navigation and Enter/Space activation;
the viewport accepts F after focus. The header has an ordinary browser status
region and Cancel / clear action. The canvas UI does not yet publish a full DOM
accessibility tree or support touch-specific navigation.

The inspector exposes saved properties, raw cached mass tokens and diagnostics.
All coordinates preserve the source assembly frame, with metre/millimetre
conversion matching the audited display carrier. Colors are diagnostic palette
colors, not recovered CAD appearances. Unknown transforms, missing references,
ambiguous paths/configurations, cycles and unsupported meshes are reported.
An incomplete assembly remains labelled with its issue count; opening a drawing
shows metadata without inventing a rendered drawing. The view does not validate
inertials, regenerate features, solve mates, edit CAD or export physical models.

## Limits and archive policy

ZIP supports stored and deflated members. Raw TAR and gzip-compressed TAR are
supported. XZ/Zstandard TAR, encrypted ZIPs, links and special members are not
supported in this version. Errors are explicit. Source names remain Unicode;
a bundled, renamed Noto-derived CJK font loads on demand for Chinese paths.
Font provenance and SIL OFL notices are in `assets/README.md` and alongside fonts.

Defaults: 128 MiB upload, 64 MiB per member, 256 MiB cumulative member bytes,
4,096 archive entries. Ignored non-CAD files count toward limits and integrity
checks. Absolute paths, parent traversal, drive/alternate-stream syntax and
case-insensitive duplicates are rejected. Files are never extracted to disk.
External CAD references resolve inside the uploaded file set only, preferring
owner-relative paths and then unique longest Windows path suffixes. Ambiguous
basenames are never selected arbitrarily.

The visual traversal is bounded to 48 assembly levels, 4,096 visits (including
failed nodes), 1,024 rendered occurrences and 500,000 output triangles. Bounded
geometry-cache eviction avoids retaining every previously selected mesh.
Parser output and GPU copies add memory beyond raw archive limits; these are not
promises about peak browser heap use. CAD cache/format limitations from the core
reader still apply, including single-configuration Part display ownership.

## Tests

```sh
mbx test --all-features --locked                  # from sw-rs root
mbx test --manifest-path viewer/Cargo.toml --locked
cd viewer
npm ci
npx playwright install chromium                  # once, if not already installed
npm test
```

The browser suite uses software WebGPU in Chromium and exercises actual RXUI
canvas clicks, file chooser activation, ZIP/TAR/TAR.GZ assembly composition,
Part selection, visible orbit/wireframe changes, keyboard activation, cancellation
and failed-import recovery. Native tests cover archive integrity, bounds,
traversal, source transforms, hidden occurrences and failure accounting.

Optional private-CAD validation runs only when explicitly supplied locally:

```sh
SW_VIEWER_ARCHIVE=/absolute/path/to/private.zip npm test
```

Test artifacts and generated fixtures remain gitignored. No archive contents are
sent to a service. Rebuild before browser tests after Rust changes. Worker
cancellation, font loading, high-DPI and error tests do not establish cross-browser
or mobile certification; Chromium is the validated target. Graphics device or
context loss currently requires reloading the viewer.

## GitHub Pages

`.github/workflows/pages.yml` builds and smoke-tests the viewer on pushes to
`main`, then deploys only `viewer/web/` through GitHub's Pages Actions artifact.
The app, workers, WASM, demo, fonts and license notices use relative paths, so the
same bundle works at the `/sw-rs/` project-site prefix. Generated deployment
files remain outside Git. GitHub Pages supplies HTTPS for WebGPU's secure context.

To check an already running deployment with the same public browser suite:

```sh
SW_VIEWER_BASE_URL=https://hxyulin.github.io/sw-rs/ npm test
```

Keep the trailing slash. The external-URL setting disables Playwright's local
server startup and resolves navigation relative to that project prefix.
