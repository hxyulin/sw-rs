#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
# Copyright (c) 2026 hxyulin <hxyulin@proton.me>
"""Check the assembled Pages artifact, including local HTML links and fragments."""
from html.parser import HTMLParser
from pathlib import Path
from urllib.parse import unquote, urlsplit
import sys
import re

class Page(HTMLParser):
    def __init__(self, text):
        super().__init__(convert_charrefs=True)
        self.ids = set()
        self.links = []
        self.feed(text)

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        if 'id' in attrs:
            self.ids.add(attrs['id'])
        if tag == 'a' and 'name' in attrs:
            self.ids.add(attrs['name'])
        for attr in ('href', 'src'):
            if attrs.get(attr):
                self.links.append(attrs[attr])

root = Path(sys.argv[1] if len(sys.argv) > 1 else '.site').resolve()
required = ('index.html', 'main.js', 'worker.js', 'scene.js', 'pkg/sw_viewer.js',
            'pkg/sw_viewer_bg.wasm', 'demo.zip', 'docs/index.html', 'docs/format.html',
            'docs/architecture.html', 'docs/api.html', 'api/sw_rs/index.html', '.nojekyll')
errors = [f'Missing asset: {name}' for name in required if not (root / name).is_file()]
pages = {p: Page(p.read_text()) for p in root.rglob('*.html')}
checked = 0
for source, page in pages.items():
    for raw in page.links:
        link = urlsplit(raw)
        if link.scheme or link.netloc:
            if link.scheme in ('https', 'http') and link.netloc == 'hxyulin.github.io' and link.path.startswith('/sw-rs/'):
                path = link.path[len('/sw-rs/'):]
                target = root / unquote(path)
            else:
                continue
        elif link.path.startswith('/'):
            if not link.path.startswith('/sw-rs/'):
                errors.append(f'{source.relative_to(root)}: outside project prefix: {raw}')
                continue
            target = root / unquote(link.path[len('/sw-rs/'):])
        else:
            target = source.parent / unquote(link.path) if link.path else source
        target = target.resolve()
        if not target.is_relative_to(root):
            errors.append(f'{source.relative_to(root)}: escapes site: {raw}')
            continue
        if target.is_dir():
            target = target / 'index.html'
        checked += 1
        if not target.is_file():
            errors.append(f'{source.relative_to(root)}: missing {raw}')
        elif link.fragment and target in pages:
            ids = pages[target].ids
            fragment = link.fragment
            # rustdoc uses percent-encoded IDs literally, and JS selects source
            # line ranges via #START-END rather than a single HTML ID.
            valid = fragment in ids or unquote(fragment) in ids
            if not valid and target.is_relative_to(root / 'api/src'):
                match = re.fullmatch(r'(\d+)-(\d+)', fragment)
                valid = bool(match and all(value in ids for value in match.groups()))
            if not valid:
                errors.append(f'{source.relative_to(root)}: missing fragment {raw}')
if errors:
    raise SystemExit('\n'.join(errors))
print(f'Checked {len(pages)} HTML pages and {checked} local links/assets/fragments')
