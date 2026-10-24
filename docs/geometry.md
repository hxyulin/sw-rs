<!-- SPDX-License-Identifier: MIT -->
<!-- Copyright (c) 2026 hxyulin <hxyulin@proton.me> -->
# Cached geometry and coordinates


`geometry(bytes, filename)` reads one modern Part DisplayLists carrier for one
canonical native configuration. It validates compact/extended face headers,
six descriptor channels, finite source float coordinates and strip winding,
including validated class reuse. Output coordinates are millimetres; source
stream/table offsets and normals are retained. Tables follow the old CAD IR's
lexicographic table-key order so prepared project signatures remain stable.
Unsupported carriers, multi-configuration ownership and missing meshes return
errors. There are no fixed mesh-count, table-attempt or triangle policy caps;
descriptor lengths, finite coordinates and index ranges are checked against the
actual payload. The simulator adapter independently enforces 250,000 unique
vertices per Part; that external adapter does not limit this viewer.

The profile recovers saved display caches only: they may be partial or stale and
have no inferred B-Rep ownership or mass. It does not evaluate features or certify
source-frame alignment. Both metadata and geometry work without native libraries.
Optional integration comparison (no source CAD is redistributed):

```sh
python3 scripts/compare-geometry.py /path/to/native-files \
  --preparer /path/to/rm-simulator/scripts/prepare-solidworks-project.py \
  --upstream /path/to/sldkit-rs --rust target/release/sw-dump \
  --output local-assets/geometry-comparison.json
```


## Coordinate frame and units

`Document::coordinate_frame` and `GeometryDocument::coordinate_frame` explicitly
declare the assumed SolidWorks model XYZ convention: right-handed, native
document origin, Front plane XY, Front-view up +Y and toward-observer +Z.
This follows [SolidWorks' plane/direction definitions](https://help.solidworks.com/2021/english/SolidWorks/cworks/c_specifying_directions.htm).
It is labelled `assumed_default_solidworks_convention`; named coordinate-system
features and saved camera orientations are not decoded or automatically applied.
Model coordinates are never recentered or reordered by this declaration. Custom
reference frames do not replace the document's global model basis. Drawings
expose no 3D frame. Cached mesh output stays in millimetres; the viewer composes
saved component poses in metres and converts mesh units exactly once.


## Missing geometry

The native EGA audit found mirror Parts with environment/lighting-only display
caches and retained `StockBodies`; embedded Parts retain `LocalBodies` without
supported cached face tables. Those streams contain evidence of Parasolid data,
but this project does not decode that topology or tessellate it. Reference
resolution, relaxed policy caps and matching filenames cannot manufacture the
missing triangles. See [coverage](parsing-coverage.md) for exact failure categories.

Assembly composition is currently viewer application code, not `geometry()`.
It uses saved occurrence placements, preserves repeated instances and records
missing/hidden/suppressed/unsupported nodes. Allowing missing references renders
available siblings; a nonempty scene is not proof of a complete assembly.
