#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
# Copyright (c) 2026 hxyulin <hxyulin@proton.me>
"""Assemble the public viewer, book and Rust reference into one Pages artifact."""
from pathlib import Path
import os
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[1]
os.chdir(ROOT)
viewer = ROOT / 'viewer/web'
for required in ('index.html', 'pkg/sw_viewer.js', 'pkg/sw_viewer_bg.wasm', 'demo.zip',
                 'fonts/SWViewerCJK.ttf', 'licenses/NOTICE.md'):
    if not (viewer / required).is_file():
        raise SystemExit(f'Missing viewer/{required}; run sh viewer/build.sh first')
subprocess.run(['npm', 'run', 'docs:build'], check=True)
builder = 'mbx' if shutil.which('mbx') else 'cargo'
if builder == 'cargo':
    print('mbx unavailable; compiling documentation without the shared cache', flush=True)
env = dict(os.environ)
env['RUSTDOCFLAGS'] = env.get('RUSTDOCFLAGS', '') + ' -D warnings -D rustdoc::broken_intra_doc_links'
subprocess.run([builder, 'doc', '--lib', '--all-features', '--no-deps', '--locked'], check=True, env=env)
# mbx preserves the checkout target path (including a managed target symlink).
# Honor an explicit caller target directory as Cargo does.
target = Path(os.environ.get('CARGO_TARGET_DIR', 'target'))
site = ROOT / '.site'
if site.exists():
    shutil.rmtree(site)
shutil.copytree(viewer, site, ignore=shutil.ignore_patterns('.DS_Store', '*.map'))
shutil.copytree(ROOT / 'docs/.vitepress/dist', site / 'docs')
shutil.copytree(target / 'doc', site / 'api')
# rustdoc's help/settings toolbar points to a top-level index that Cargo does
# not create for a single documented crate.
(site / 'api/index.html').write_text('''<!doctype html><html lang="en"><head>
<meta charset="utf-8"><meta http-equiv="refresh" content="0;url=sw_rs/index.html">
<title>sw-rs Rust API</title></head><body><a href="sw_rs/index.html">sw-rs Rust API</a></body></html>''')
licenses = site / 'docs/licenses'
licenses.mkdir(exist_ok=True)
shutil.copyfile(ROOT / 'node_modules/vitepress/LICENSE', licenses / 'VitePress.txt')
(site / '.nojekyll').touch()
subprocess.run(['python3', 'scripts/check-site.py', str(site)], check=True)
print(f'Pages artifact ready: {site}', flush=True)
