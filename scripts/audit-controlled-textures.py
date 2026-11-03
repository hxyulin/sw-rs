#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
# Copyright (c) 2026 hxyulin <hxyulin@proton.me>
"""Compare native texture parameters to reopened API evidence in a local corpus.

Run inspect_appearances with --hypotheses first, then pass CORPUS.zip REPORT.jsonl.
The corpus and per-file report remain private. This script prints only counts.
"""
import hashlib
import json
import math
import pathlib
import sys
import zipfile


def close(actual, expected):
    return math.isclose(actual, expected, rel_tol=2e-6, abs_tol=1e-8)


def main():
    archive, report = sys.argv[1:]
    with zipfile.ZipFile(archive) as corpus:
        manifest = json.loads(corpus.read("manifest.json").decode("utf-8-sig"))
        for entry in manifest["files"]:
            data = corpus.read(entry["path"])
            assert len(data) == entry["bytes"], entry["path"]
            assert hashlib.sha256(data).hexdigest() == entry["sha256"], entry["path"]
        events = [json.loads(line) for line in corpus.read("texture/evidence.jsonl")
                  .decode("utf-8-sig").splitlines() if line.strip()]
    reopened = {e["fixture"]: e for e in events
                if e.get("operation") == "mapping" and e.get("stage") == "reopened"}
    rows = [json.loads(line) for line in pathlib.Path(report).read_text().splitlines()]
    counts = {"payload_hashes": len(manifest["files"]), "texture_files": 0,
              "center_checks": 0, "size_checks": 0, "rotation_checks": 0,
              "projection_coupled": 0}
    for row in rows:
        if not row["path"].startswith("texture/"):
            continue
        name = pathlib.PurePosixPath(row["path"]).stem
        api = reopened[name]
        assert row["extensions"] == 1, name
        records = row["mapping_hypotheses"]
        assert len(records) == 1, name
        parsed = records[0]["bitmap_texture_parameters"]
        assert parsed is not None and records[0]["hypothesis"] is None, name
        assert all(close(a, b) for a, b in zip(parsed["center_metres"], api["center"])), name
        counts["center_checks"] += 1
        if api["mappingType"] == 4:
            assert all(close(1 / a, b) for a, b in zip(
                parsed["reciprocal_scale_per_metre"], [api["width"], api["height"]])), name
            counts["size_checks"] += 1
        else:
            counts["projection_coupled"] += 1
        # Circular comparison preserves stored wraparound and the getter's radians.
        rotation_sum = math.radians(parsed["stored_rotation_degrees"]) + api["rotation"]
        assert close(math.sin(rotation_sum), 0) and close(math.cos(rotation_sum), 1), name
        counts["rotation_checks"] += 1
        counts["texture_files"] += 1
    assert counts["texture_files"] == len(reopened) == 34, counts
    print(json.dumps(counts, sort_keys=True))


if __name__ == "__main__":
    main()
