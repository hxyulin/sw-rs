#!/bin/sh
# SPDX-License-Identifier: MIT
# Copyright (c) 2026 hxyulin <hxyulin@proton.me>
set -eu
cd "$(dirname "$0")"
if command -v mbx >/dev/null 2>&1; then
  mbx build --target wasm32-unknown-unknown --release --locked
else
  echo 'mbx unavailable; compiling without the shared cache' >&2
  cargo build --target wasm32-unknown-unknown --release --locked
fi
python3 scripts/make-fixtures.py
mkdir -p web/fonts web/licenses
cp assets/SourceSans3-OFL.md assets/NotoSansSC-OFL.txt assets/README.md web/licenses/
cp ../LICENSE ../LICENSE-Apache-2.0 ../LICENSE-openswx ../NOTICE.md ../NOTICE-cadmpeg.md web/licenses/
touch web/.nojekyll
cp assets/SWViewerCJK.ttf assets/NotoSansSC-OFL.txt web/fonts/
wasm-bindgen --target web --out-dir web/pkg target/wasm32-unknown-unknown/release/sw_viewer.wasm
