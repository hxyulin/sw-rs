<!-- SPDX-License-Identifier: MIT -->
<!-- Copyright (c) 2026 hxyulin <hxyulin@proton.me> -->
# Development and deployment

## Prerequisites

Use Rust stable (1.98.1 or newer for the viewer) with rustfmt, Clippy and
`wasm32-unknown-unknown`; Python 3 for
synthetic fixtures/local serving; Node 22+ and npm for docs/browser tests.
Prefer `mbx` for compiling commands when available; replace it with `cargo`
otherwise. Keep the root and viewer Cargo lockfiles. They are separate packages.
Install `wasm-bindgen-cli` matching the lockfile of the package being built.

The viewer uses published RXUI 0.1 crates. Its browser target enables wgpu's
`fragile-send-sync-non-atomic-wasm` feature for rxui-draw's upload-completion
callback. This build uses non-atomic WASM; an atomic/threaded build needs a
drawing backend that supports it.

## Checks

For native UI development, run the app directly. Its optional native host uses
RXUI's system theme and an embedded wgpu viewport:

```sh
cargo run --manifest-path viewer/Cargo.toml --features native --bin sw-viewer -- /path/to/model.SLDPRT
cargo test --manifest-path viewer/Cargo.toml --features native --locked
# GPU layout/rendering and interaction check, on a machine with a native adapter:
cargo test --manifest-path viewer/Cargo.toml --features native --bin sw-viewer --locked -- --ignored
```

```sh
cargo fmt --check
cargo fmt --manifest-path viewer/Cargo.toml --check
mbx test --all-features --locked
mbx clippy --all-targets --all-features --locked -- -D warnings
mbx test --manifest-path viewer/Cargo.toml --locked
mbx clippy --manifest-path viewer/Cargo.toml --all-targets --locked -- -D warnings
mbx clippy --manifest-path viewer/Cargo.toml --lib --target wasm32-unknown-unknown --locked -- -D warnings
```

For the core WASM boundary, build and run the Node smoke test as described in
[API usage](api.md#webassembly). For the viewer:

```sh
sh viewer/build.sh
npm ci --prefix viewer
cd viewer
npx playwright install chromium
npm test
```

The documented Playwright workflow runs real viewer tests in Chromium at 1x and
2x display scales, using software graphics. `SW_VIEWER_BROWSER_CHANNEL=chrome`
selects installed Chrome, as in CI. `SW_VIEWER_ARCHIVE=/absolute/private.zip`
enables optional private-CAD tests; no CAD is bundled or uploaded.

## Documentation site

The site uses pinned VitePress 1.6.4 with a patched Vite 6.4.5 override, with local search, sidebar navigation,
light/dark themes and checked internal Markdown links. Source pages in `docs/`
are also readable on GitHub. The viewer guide has one full source in `docs/viewer.md`.

```sh
npm ci
npm run docs:dev          # /sw-rs/docs/ on the development server
npm run docs:build        # docs/.vitepress/dist
npm run docs:preview
```

To assemble the exact Pages artifact (after building the viewer):

```sh
npm run site:build
npm run site:test        # viewer npm dependencies + Playwright browser required
python3 -m http.server 8098 --directory .site
```

Production paths are `/sw-rs/` for the existing app, `/sw-rs/docs/` for the book,
and `/sw-rs/api/sw_rs/` for rustdoc. For a local prefix-matching preview, serve a
parent directory containing the output as `sw-rs/`, or use the VitePress preview
for the book alone. Do not browse generated HTML through `file://`.

`site:build` builds the book, generates all-feature Rust reference with warnings
and broken intra-doc links denied, and copies only the viewer and generated docs
into `.site/`. It requires a completed `viewer/build.sh`; it never publishes
private research or arbitrary repository contents. Build outputs remain ignored.
`site:check` checks local links/fragments and required deployment assets.
`site:test` serves the assembled artifact under `/sw-rs/` and checks browser
navigation, local search, themes, mobile menus and Rust API pages. It reuses
the viewer's Playwright installation and `SW_VIEWER_BROWSER_CHANNEL` setting.

## CI and Pages

The Rust workflow independently checks the core, viewer and documentation on
pushes and pull requests. Both package formatting checks are explicit. Docs CI
builds rustdoc and VitePress; the viewer job compiles WASM and runs browser tests.
Documentation failures block publishing just as viewer failures do.

The Pages workflow builds the app/book/API into one artifact, checks links,
runs viewer and documentation browser tests and deploys through GitHub Actions. It preserves the
existing app URL and requires GitHub Pages source **GitHub Actions** in repository
settings. Build steps only require read access; the deploy job alone receives
`pages: write` and `id-token: write`.

To test the public viewer after deployment:

```sh
cd viewer
SW_VIEWER_BASE_URL=https://hxyulin.github.io/sw-rs/ npm test
```

Keep the trailing slash. No generated site files, test fixtures, CAD or per-file
audit reports should be committed. Update [coverage](parsing-coverage.md) with
its source sample/date when a new rule changes measured behavior. Preserve
attribution on adapted code and label provisional interpretations with the
remaining verification steps.
