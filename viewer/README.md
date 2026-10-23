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

Open or drop an archive. If it contains multiple SLDASM documents, a native modal
lets you search full paths and choose which assembly to open. **Choose assembly…**
reopens it. Escape or **Browse individual documents** dismisses the chooser;
select any document in the RXUI sidebar to continue. A single assembly or individual
native document opens directly. The existing document search also keeps Parts
and drawings independently usable.

The **Component occurrences** panel shows the selected configuration's actual
assembly instances, including repeated subassemblies and embedded documents,
rather than grouping archive folders. The root starts expanded; nested assemblies
start collapsed. Expand with the row button or Right arrow; Left collapses or
moves to the parent, Up/Down and Home/End move between visible rows. Enter or a
row's **Open** button opens its resolved document and saved configuration, including
embedded Parts absent from the archive document list. Missing, ambiguous,
suppressed, hidden, cyclic and unsupported occurrences remain labelled in the tree.
Unresolved references never offer an Open action. Suppressed occurrences stay
excluded from rendering. **Show hidden Parts** controls saved hidden flags.
Repeated occurrences retain distinct source poses while sharing decoded geometry.
Missing references are allowed: available sibling parts still render, while the
header and inspector label the assembly as partial and list omitted references.
Missing files are never replaced with invented geometry.

Use **Search parts** above the document list to filter filenames or full archive
paths, ignoring case. Unicode names are supported. Press **/** with the sidebar
focused to jump to search. Filtering keeps the current scene; selecting a result
opens it. **Clear** restores the list, and opening a new archive resets the search.

Drag in the viewport to orbit; Shift-drag or right-drag pans. Wheel zooms, and
**Fit view**, double-click or F fits the model. **Wireframe** changes the drawing
mode. The sidebar supports Tab/Shift-Tab navigation and Enter/Space activation;
the viewport accepts F after focus. The header has an ordinary browser status
region and Cancel / clear action. A loading panel shows file-read bytes and
percentage, ZIP extraction entry progress, and an indeterminate bar for TAR
extraction and mesh decoding (where no reliable total is known). Cancelling or
finishing an import dismisses the panel. The chooser, search and occurrence tree
use accessible native HTML overlays. The remaining canvas UI does not yet publish
a full DOM accessibility tree or support touch-specific navigation.

The inspector exposes saved properties, raw cached mass tokens and diagnostics.
All coordinates preserve the source assembly frame, with metre/millimetre
conversion matching the audited display carrier. **Material colors** uses
approximate viewer presets based on explicit saved material assignments; **Diagnostic colors** uses the path palette. These presets do not recover
original CAD appearances. The inspector reports explicit saved material
assignments for the opened configuration and preserves raw density tokens with
unverified units labelled; material names do not establish recovered appearance.

Aluminum uses an approximate metallic/roughness shader with procedural satin
brushing; carbon fiber uses a filtered woven-bundle pattern. Other assigned
materials use their saved RGB swatch, when available, with a generic surface
preset. Patterns use dominant-plane projection of placed world coordinates in
metres, so they stay fixed while orbiting; original UVs, machining direction and
fiber orientation are not recovered. Unassigned Parts keep diagnostic colors.
Presets and draw ranges are carried separately from geometry, and repeated
occurrences with the same saved preset share its record. No SolidWorks shader
library, external texture download or proprietary material asset is required.
Unknown transforms, missing references,
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

The viewer has no fixed upload, native-document, expanded-byte or archive-entry
count cap. The Rust library keeps bounded defaults for other callers and exposes
explicit overrides. Browser RAM, WASM address space and GPU capacity still bound
what can be opened; local processing does not make memory unlimited. Ignored
non-CAD members still undergo integrity checks. Absolute paths, parent traversal, drive/alternate-stream syntax and
case-insensitive duplicates are rejected. Files are never extracted to disk.
External CAD references resolve inside the uploaded file set only, preferring
owner-relative paths and then unique longest Windows path suffixes. Ambiguous
basenames are never selected arbitrarily. Saved `ReplaceRef` old-to-new path
chains are followed before resolution, so copy/export renames do not require
fuzzy filename matching. Embedded `ImportedComp` and `VirtualComp` native
documents are resolved within their owning assembly, including Unicode names.
Embedded caches are not mixed into the archive’s global filename search.

The viewer has no fixed traversal-depth, visit, occurrence, marker-count,
mesh-table, vertex-count or triangle policy caps. Cyclic references, malformed
descriptors, invalid indices and non-finite transforms/coordinates remain
explicit diagnostics. Geometry-cache eviction avoids retaining every previously
selected mesh; it does not truncate the scene. Wireframe indices are built only
when requested and share the shaded mesh’s vertex buffer.
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
and failed-import recovery, assembly choice/reopening, occurrence expansion and
keyboard navigation, embedded-document opening and saved material labels. Native
tests cover archive integrity, bounds,
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

CI uses the GitHub runner’s installed Chrome (`SW_VIEWER_BROWSER_CHANNEL=chrome`);
local tests default to Playwright’s bundled Chromium.

For native assembly diagnostics without a browser or GPU:

```sh
mbx run --manifest-path viewer/Cargo.toml --release --example inspect_project -- archive.zip
# Optionally append a path substring to inspect selected assemblies only.
```

The JSONL report includes rendered triangles, occurrences, unresolved references,
parser diagnostics and saved rename mappings. Keep reports and source CAD local.
Some native files have no supported saved display tessellation; allowing missing
references cannot reconstruct their geometry. The viewer does not regenerate
B-Rep geometry or load external files from saved Windows paths.
