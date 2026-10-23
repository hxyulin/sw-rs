#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
# Copyright (c) 2026 hxyulin <hxyulin@proton.me>
"""Compare prepared Part meshes through rm-simulator's caller-supplied adapter."""
import argparse
from concurrent.futures import ThreadPoolExecutor
import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("source", type=Path)
parser.add_argument("--preparer", type=Path, required=True, help="rm-simulator/scripts/prepare-solidworks-project.py")
parser.add_argument("--upstream", type=Path, required=True, help="existing sldkit-rs geometry CLI")
parser.add_argument("--rust", type=Path, default=Path("target/release/sw-dump"))
parser.add_argument("--output", type=Path, required=True)
args = parser.parse_args()
spec = importlib.util.spec_from_file_location("preparer", args.preparer)
adapter = importlib.util.module_from_spec(spec)
spec.loader.exec_module(adapter)
files = sorted(p for p in args.source.rglob("*") if p.suffix.lower() == ".sldprt")
if not files:
    parser.error("no native Parts found")

def invoke(binary, path, geometry=True):
    command = [str(binary.resolve())] + (["geometry"] if geometry else []) + [str(path.resolve())]
    with tempfile.TemporaryFile() as output, tempfile.TemporaryFile() as errors:
        result = subprocess.run(command, stdout=output, stderr=errors, timeout=120)
        if output.tell() > 128 * 1024 * 1024:
            raise ValueError("reader JSON output limit")
        output.seek(0)
        decoded = json.load(output)
    if result.returncode:
        raise ValueError(",".join(str(d.get("code", "unknown")) for d in decoded.get("diagnostics", [])[:3]))
    return decoded

def compare(path):
    row = dict(source=str(path.relative_to(args.source)), sha256=hashlib.sha256(path.read_bytes()).hexdigest())
    metadata = invoke(args.rust, path, geometry=False)
    configurations = metadata["configurations"]
    outputs = {}
    for label, binary in (("old", args.upstream), ("new", args.rust)):
        try:
            decoded = invoke(binary, path)
            geometry = decoded.get("geometry") or {}
            if geometry.get("source", {}).get("sha256") != row["sha256"]:
                raise ValueError("geometry source hash mismatch")
            outputs[label] = geometry
        except (OSError, ValueError, subprocess.SubprocessError) as error:
            outputs[label] = str(error)
    outcomes = []
    for config in configurations:
        item = dict(configuration=config["name"], source_index=config["index"])
        for label in ("old", "new"):
            try:
                if isinstance(outputs[label], str):
                    raise ValueError(outputs[label])
                vertices, triangles, mapping = adapter.decode_geometry(
                    outputs[label], config["name"], config["index"], len(configurations))
                digest = hashlib.sha256(json.dumps([vertices, triangles], separators=(",", ":")).encode()).hexdigest()
                item[label] = dict(vertices=len(vertices), triangles=len(triangles), sha256=digest, mapping=mapping)
            except (KeyError, TypeError, ValueError) as error:
                item[label] = dict(error=str(error))
        outcomes.append(item)
    row["outcomes"] = outcomes
    return row

with ThreadPoolExecutor(max_workers=4) as pool:
    rows = list(pool.map(compare, files))
requests = [o for row in rows for o in row["outcomes"]]
regressions = [dict(source=row["source"], **o) for row in rows for o in row["outcomes"]
               if "sha256" in o["old"] and o["old"]["sha256"] != o["new"].get("sha256")]
summary = dict(files=len(rows), configuration_requests=len(requests),
               old_success=sum("sha256" in o["old"] for o in requests),
               new_success=sum("sha256" in o["new"] for o in requests),
               additional_recoveries=sum("error" in o["old"] and "sha256" in o["new"] for o in requests),
               regressions=regressions)
args.output.parent.mkdir(parents=True, exist_ok=True)
args.output.write_text(json.dumps(dict(summary=summary, rows=rows), indent=2, ensure_ascii=False))
print(json.dumps(summary, indent=2, ensure_ascii=False))
raise SystemExit(bool(regressions))
