#!/bin/sh
# SPDX-License-Identifier: MIT
# Copyright (c) 2026 hxyulin <hxyulin@proton.me>
# Run from the Cargo package whose lockfile supplies the bindings version.
set -eu
bindings_version=$(sed -n '/name = "wasm-bindgen"/{n;s/version = "\(.*\)"/\1/p;}' Cargo.lock)
if [ -z "$bindings_version" ]; then
  echo 'wasm-bindgen version missing from Cargo.lock' >&2
  exit 1
fi
if [ "$(wasm-bindgen --version 2>/dev/null || true)" != "wasm-bindgen $bindings_version" ]; then
  cargo install wasm-bindgen-cli --version "$bindings_version" --locked --force
fi
