<!-- SPDX-License-Identifier: MIT -->
<!-- Copyright (c) 2026 hxyulin <hxyulin@proton.me> -->
# SW Explorer

## Native workbench

```sh
cargo run --manifest-path viewer/Cargo.toml --features native --bin sw-viewer
# Open a file directly:
cargo run --manifest-path viewer/Cargo.toml --features native --bin sw-viewer -- /path/to/model.SLDPRT
```

The native host uses RXUI's default system theme, typography, controls, text input
and virtual occurrence tree. Documents sit on the left, a depth-tested wgpu model
viewport in the center, and saved properties and diagnostics on the right.
Opening and parsing run on background workers. The file picker accepts individual
native documents and ZIP/TAR/TAR.GZ projects. Documents and configurations can be
changed without reopening the project. Tree selection shows occurrence details;
it does not yet highlight geometry.

Drag to orbit, Shift-drag or right-drag to pan, and scroll to zoom. Front, Top,
Right, Isometric and Fit control the camera. Show hidden includes saved hidden
occurrences. Material colors uses saved preset swatches with simple lighting;
Diagnostic colors uses the path palette. This native viewport does not yet have
the browser's procedural material shaders, wireframe or source-inspection dialog.
The native workbench needs Rust 1.98.1 or newer and a graphics adapter, with no
Node, browser or WebAssembly build. The browser host remains available below.

## Browser host

Live app: **https://hxyulin.github.io/sw-rs/**

Source: https://github.com/hxyulin/sw-rs

A local-first browser viewer for native SolidWorks documents in ZIP, TAR and
TAR.GZ/TGZ archives, or an individual SLDPRT/SLDASM/SLDDRW file. RXUI draws the
browser and inspector through WebGPU using the published RXUI 0.1 and rxui-draw
crates. An app-owned WebGL 2 viewport renders
Rust-decoded meshes; there is no Bevy, winit, server-side parser or CAD SDK.

Uploaded bytes stay in a module worker in this browser. The static server serves
only the app, its WASM, fonts and optional synthetic demo. It receives no uploaded
archive bytes. Parsing is cancelable by terminating the worker; stale results
and pending file reads cannot replace a newer import. Geometry transfers as
Float32Array buffers, rather than JSON coordinate arrays.

## Build and run

Prerequisites: Rust 1.98.1 or newer with `wasm32-unknown-unknown`, Python 3, and a
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

Click an occurrence row, move through the tree with the arrow keys, or click
geometry in the viewport to select that instance. Selection highlights its
rendered subtree in cyan, including in wireframe. A viewport drag still orbits
or pans; clicking empty space clears selection. Repeated instances of the same
Part remain separate selections.

The component inspector shows the source document, configuration, status,
triangle count, saved material when available, assembly position in millimetres
and the saved world transform. **Focus** fits the selected subtree without
changing the orbit direction. **Isolate** shows only that subtree. **Hide** /
**Show** changes the selected subtree's visibility. **Restore all** clears viewer
visibility overrides, including after selection is cleared. These controls use
already loaded geometry and leave the opened assembly unchanged. Excluded or
unavailable occurrences can be inspected but have no geometry controls. Native
saved hidden/suppressed flags still determine which geometry was loaded.
Selection and visibility overrides reset when a scene/configuration is reloaded.

**Saved assembly mates** lists evidence for the opened document's exact native
configuration. Expand a record to see its class evidence, source offset and saved
reference strings. Unknown reused mate types stay unknown. This list is scoped
to the opened assembly, not filtered by the selected component: entity ownership,
parameters and active state are not resolved. Mate editing and constrained motion
are not implemented. See [mate decoding](mates.md) for the current parser contract.

Occurrence geometry ranges and saved transforms come from the shared Rust scene
model. Browser selection, picking and visibility controls use WebGL; the native
host does not yet expose these interactions. Browser picking checks visible
triangles on the CPU and may take longer on large scenes.

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

The camera defaults to **Y · SolidWorks default**: native model XYZ is right-handed,
the Front plane is XY, and +Z points toward a Front-view observer with +Y up.
**Front**, **Top**, **Right** and **Isometric** select standard orientations and
restore the Y-up camera convention. The colored **MODEL XYZ** triad shows source
axes (X red, Y green, Z blue). For assemblies authored with vertical Z, choose
**Z · authored Z-up** under **Orbit up**. Fit keeps that up-axis choice.

These controls change the camera, pan basis and lighting up direction only;
they do not rotate/recenter source vertices, alter occurrence transforms or infer
physical gravity. The inspector labels the model-frame convention as assumed.
Custom coordinate-system features and saved view orientations are not decoded.
Named reference frames are not applied silently to model coordinates.

The inspector exposes saved properties, raw cached mass tokens and diagnostics.
All coordinates preserve the source assembly frame, with metre/millimetre
conversion matching the audited display carrier. **Material colors** uses supported
saved visual colors and approximate material presets; **Diagnostic colors** uses
the path palette. Shading remains approximate. The inspector reports explicit saved material
assignments for the opened configuration and preserves raw density tokens with
unverified units labelled; material names do not establish recovered appearance.

Aluminum uses an approximate metallic/roughness shader with procedural satin
brushing; carbon fiber uses a filtered woven-bundle pattern. Other assigned
materials use their saved RGB swatch, when available, with a generic surface
preset. Patterns use dominant-plane projection of placed world coordinates in
metres, so they stay fixed while orbiting; original UVs, machining direction and
fiber orientation are not recovered. Parts without a supported saved visual color
or a physical-material preset keep diagnostic colors.
Presets and draw ranges are carried separately from geometry, and repeated
occurrences with the same saved preset share its record. No SolidWorks shader
library, external texture download or proprietary material asset is required.

**Appearance style** defaults to **Saved material appearance**. Supported exact
cache-table RGB takes precedence over a uniquely scoped document RGB, then the
physical material's saved shader or preset. Saved visual colors work without a
physical material assignment and use a generic opaque surface. Conflicting,
unresolved and sentinel cache records block document-color fallback. Multiple
display states are not guessed. This does not recover native face/body/occurrence
ownership, optical response or transparent rendering.

For the supported complete native visual archive, saved configuration indices
select state IDs and their owner/property links. Feature colors require a unique
same-document source match. Face colors use the selected face's complete reference
path. Checked saved body groups and carrier-local shared style references support
body colors, including copied bodies without a new visual-state property. Shared
style links supply optical settings only when every supported anchor agrees. They
do not infer bitmap ownership or UVs. Null links cannot inherit a historical color, and
conflicting links remain unresolved. This reconstructs the saved display cache;
current B-Rep ownership and face timestamps remain unverified. Partial archive
walks never resume at scanned markers or establish complete ownership.

The physical material fallback uses saved shader identities, swatches and valid
diffuse/specularity/emission coefficients. Known
styles include satin aluminum, polished/matte metals, plastic, glass-fiber and
carbon-fiber patterns. PhotoWorks shader names/paths take precedence over generic
realtime hints. **Material family presets** instead uses the physical material
classification; a carbon-fiber material can have a saved plastic appearance.
Changing styles preserves the scene/configuration and needs no mesh decoding.

The browser also renders supplied PNG, JPEG, BMP and WebP images for the supported
bitmap profile and the supported `carbonfiberepoxy` selector profiles, selected by an exact cache-table binding or an unambiguous
document-level appearance and display state. Include the image alongside the CAD documents in the uploaded
archive. Saved Windows references resolve against virtual archive paths; missing
or ambiguous image references keep the existing shading and report an issue.
No host filesystem or external URL is opened. Corrupt or over-16-megapixel images
fall back to shading. Desktop bitmap rendering is not implemented.

Bitmap placement uses approximate Part-local planar projection, with arc-length
coordinates for checked axis-aligned cylindrical carbon cache faces,
with decoded scale, center and stored rotation for the controlled bitmap profile.
Carbon fibre reuses those scalar locations as an unverified placement approximation.
It requires the referenced image in the archive; `.p2m` files are not executed, and
its bump, reflection and directional shader behavior is not reproduced. Exact supported cache-table
bitmap bindings take precedence over a document default. Placement follows the
Part through assembly transforms. It is explicitly labelled as approximate; native projection
directions and UVs are not recovered. Other cached face/body properties and unresolved cache links block
this document-level fallback. Multiple display states are not guessed. The
projection-coupled bitmap profile remains unsupported. **Material family presets**
and **Diagnostic colors** disable bitmap sampling.

The inspector shows saved shader and texture reference names. Original `.p2m`
shader programs and DDS resources are not loaded. Transparent rendering, full
shader parameters and native face/body/display-state appearance overrides remain
unsupported. Raw optical tokens are
retained in parser output even when the viewport does not implement them.
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
Font provenance and SIL OFL notices are in `viewer/assets/README.md` and alongside fonts.

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
`main`, then assembles the viewer, documentation and Rust API into one Pages artifact through GitHub's Pages Actions artifact.
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

The exported scene `presentations` catalogs also include supported native visual
prefixes, scoped document defaults and cache-local property links. Coverage flags
and unresolved identities distinguish these from complete native face/body or
component ownership. The current rendered presets remain approximations while
those ownership and texture-projection rules await controlled SolidWorks saves.

The inspector reports recognized/saved mate-header counts for the selected native
configuration when a MatesList stream exists. Scene metadata includes the partial
mate inventory and reference candidates. These records do not drive placement or
animation; reused class/object references, ownership and parameters remain
unresolved. See [mate relations](mates.md).

Use **Inspect source…** beside the standard view controls to browse the selected
document's record/stream inventory. Filter stream names, inspect XML or binary
candidates and use Previous/Next records for large reports. Unsupported native
containers retain an opaque inventory. The worker retains one inspected source
and one stream report until closing the dialog; inspection does not request a
scene or change placement. See [the inventory/reader contract](reader-inventory.md).


Supported native saved appearances now carry their own draw-batch surface response.
The adapter recognizes the exact `defaultplastic`, `blacklowglossplastic`,
`lightgreyhighglossplastic`, `satinfinishaluminum` and `carbonfiberepoxy` identifiers
for discriminator 2/3. Saved finish roughness and reflectivity agree with reopened
material queries on the checked profiles. The viewer uses them as approximate GGX
roughness and normal-incidence reflectance; this is not the SolidWorks shader.
Legacy diffuse, specular and emission values supply approximate multipliers.
Only the explicit aluminum shader selects a metal response. Unsupported shaders,
invalid coefficients and transparent profiles keep the color-only fallback.
Equal colors with different optical settings remain separate draw batches.
Bitmap sampling retains the selected surface response instead of resetting it.
Lighting, tone mapping, shadows, edge display and native bump response still differ
from SolidWorks, so base-color agreement does not establish visual equivalence.


Curved carbon cache faces that fit an axis-aligned circular cylinder use continuous
arc-length coordinates. The fit checks every vertex against the circle and each
triangle normal against the cylinder axis. Planar faces and rejected fits keep
the planar fallback. A full cylinder retains one wrap seam; open rounded edges
do not acquire seams when triangle normals cross a dominant-plane boundary.
This geometry-based approximation does not recover native surface UV origins or
ensure continuity between separate cache faces. The source bitmap and repeat
sizes remain unchanged. Supported GPUs also use up to 8× anisotropic filtering
for oblique bitmap detail, with trilinear filtering as the capability fallback.
