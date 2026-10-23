#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
# Copyright (c) 2026 hxyulin <hxyulin@proton.me>
"""Compare metadata with a caller-supplied openswx CLI; never copy source CAD."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("source", type=Path)
parser.add_argument("--upstream", required=True, type=Path)
parser.add_argument("--rust", type=Path, default=Path("target/debug/sw-dump"))
parser.add_argument("--output", required=True, type=Path)
args = parser.parse_args()
files = sorted(p for p in args.source.rglob("*") if p.suffix.lower() in (".sldprt", ".sldasm"))
if not files:
    parser.error("no native documents found")
summary = dict(files=len(files), parsed=0, configurations=0, components=0, mass_records=0,
               compared_mass_records=0, compared_components=0, unknown_transforms=0, differences=[], sources=[])
for path in files:
    summary["sources"].append(dict(path=str(path.relative_to(args.source)), sha256=hashlib.sha256(path.read_bytes()).hexdigest()))
    try:
        def run(binary):
            result = subprocess.run([str(binary.resolve()), str(path.resolve())], capture_output=True, text=True, timeout=120, check=True)
            return json.loads(result.stdout)
        rust, upstream = run(args.rust), run(args.upstream)
        summary["parsed"] += 1
        rc, uc = rust["configurations"], upstream.get("configurations", [])
        summary["configurations"] += len(rc)
        def diff(label):
            summary["differences"].append(dict(path=str(path.relative_to(args.source)), field=label))
        if rust["version"] != upstream["version"]:
            diff("version")
        if rust["global_properties"] != upstream.get("global_properties", {}):
            diff("global_properties")
        if [c["name"] for c in rc] != [c["name"] for c in uc]:
            diff("configuration_names")
        for r, u in zip(rc, uc):
            if r.get("derived_from_index") != u.get("derived_from_index"):
                diff(f"{r['index']}:derived_from_index")
            if r.get("cut_list", []) != u.get("cut_list", []):
                diff(f"{r['index']}:cut_list")
            if r["properties"] != u.get("properties", {}):
                diff(f"{r['index']}:properties")
            if r["mass_properties"] is not None:
                summary["mass_records"] += 1
            expected_mass = u.get("mass_properties")
            if r["mass_properties"] != expected_mass:
                diff(f"{r['index']}:mass_properties")
            elif r["mass_properties"] is not None:
                summary["compared_mass_records"] += 1
            components, expected = r["components"], u.get("components", [])
            summary["components"] += len(components)
            if len(components) != len(expected):
                diff(f"{r['index']}:component_count")
            for a,b in zip(components, expected):
                # Missing native transforms intentionally remain None, whereas
                # openswx substitutes identity. Compare only observed transforms.
                if a["transform"] is None:
                    summary["unknown_transforms"] += 1
                    b = {**b, "transform": None}
                if a != b:
                    diff(f"{r['index']}:component:{a['name']}")
                else:
                    summary["compared_components"] += 1
    except (OSError, subprocess.SubprocessError, ValueError, KeyError) as error:
        summary["differences"].append(dict(path=str(path), error=str(error)))
args.output.parent.mkdir(parents=True, exist_ok=True)
args.output.write_text(json.dumps(summary, indent=2, ensure_ascii=False))
print(json.dumps({**{k:v for k,v in summary.items() if k not in ("sources", "differences")}, "difference_count": len(summary["differences"]), "first_differences": summary["differences"][:5]}, indent=2, ensure_ascii=False))
raise SystemExit(bool(summary["differences"]))
