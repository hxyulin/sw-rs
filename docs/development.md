<!-- SPDX-License-Identifier: MIT -->
<!-- Copyright (c) 2026 hxyulin <hxyulin@proton.me> -->
# Development and deployment

## Prerequisites

Use Rust stable (1.98.1 or newer for the viewer) with rustfmt, Clippy and
`wasm32-unknown-unknown`; Python 3 for
synthetic fixtures/local serving; Node 22+ and npm for docs/browser tests.
The commands below use `cargo`. Where `mbx` is installed, run Cargo commands
through `mbx` instead. Keep the root and viewer Cargo lockfiles. They are separate packages.
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
cargo test --all-features --locked
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --manifest-path viewer/Cargo.toml --locked
cargo clippy --manifest-path viewer/Cargo.toml --all-targets --locked -- -D warnings
cargo clippy --manifest-path viewer/Cargo.toml --lib --target wasm32-unknown-unknown --locked -- -D warnings
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

### Run a subset

The core integration suite uses one binary with subsystem modules: `archive`,
`binary`, `geometry`, `inventory`, `material`, `ole` and `parser`. Cargo filters
match these module paths as well as unit-test module paths:

```sh
cargo test --all-features --locked --test integration parser::
cargo test --all-features --locked material::
cargo test --all-features --locked --lib native_reference::subtree::
cargo test --manifest-path viewer/Cargo.toml --locked project::traversal_tests::
cargo test --all-features --locked -- --list
```

Use `--test integration` to select public API tests, `--lib` for unit tests, or
`--doc` for documentation examples. Archive tests require `--features archive`
or `--all-features`. The former individual `--test parser`, `--test ole`, etc.
targets are now modules in `--test integration`.

Both packages offer an optimized test profile. It inherits the test profile,
keeps debug assertions and overflow checks, and uses optimization level 2:

```sh
cargo test --profile test-fast --all-features --locked
cargo test --profile test-fast --manifest-path viewer/Cargo.toml --locked
```

The default remains unoptimized for shorter edit/build cycles. The optimized
profile has its own build artifacts and an initial compilation cost. Use it
for repeated runs of large synthetic fixtures.

Browser tests are grouped into `appearance`, `camera`, `imports`, `inventory`,
`navigation` and `private` spec files and describe blocks:

```sh
npm --prefix viewer test -- imports --project chromium
npm --prefix viewer test -- --grep 'navigation'
npm --prefix viewer test -- --project chromium-hidpi
npm --prefix viewer test -- --list
SW_VIEWER_FULL_MATRIX=1 npm --prefix viewer test
```

The default matrix runs every case at 1x display scale and the `@hidpi` rendering
and interaction cases at 2x, using software graphics. This covers camera views,
canvas clicks, the file chooser, keyboard focus, resize, Unicode search,
occurrence-tree interaction and appearance switching at both scales. Archive
formats and metadata checks run at 1x. `SW_VIEWER_FULL_MATRIX=1` restores every
case at both scales. The default uses two browser workers. Override it with
`npm --prefix viewer test -- --workers=1` on constrained machines. Four workers
were slower in local measurements.

`SW_VIEWER_BROWSER_CHANNEL=chrome` selects installed Chrome, as in CI.
`SW_VIEWER_ARCHIVE=/absolute/private.zip` enables the optional private-CAD test;
no CAD is bundled or uploaded. Add `SW_VIEWER_FULL_MATRIX=1` to run it at both
scales.

## Test runtime

Local measurements on 2026-10-27 used the existing release WASM build and warm
Rust build artifacts. Browser numbers include runner startup and teardown.
These are individual runs except where noted, not CI performance guarantees.

| Suite | Configuration | Time |
| --- | --- | --- |
| Browser, 66 cases | Original full matrix, one worker | 65.8 s |
| Browser, 42 cases | Selected 2x coverage, one worker | 37.0 s |
| Browser, 42 cases | Selected 2x coverage, two workers (two runs) | 21.1–22.2 s |
| Browser, 66 cases | Full matrix, two workers | 31.8 s |
| Browser, 42 cases | Selected 2x coverage, four workers | 42.3 s |
| Core integration, 65 tests | Default profile, execution only | 1.89 s |
| Core integration, 65 tests | `test-fast`, execution only | 0.10 s |
| Viewer model, 18 tests | Default profile, execution only | 0.81 s |
| Viewer model, 18 tests | `test-fast`, execution only | 0.05 s |

The first `test-fast` builds took 13.8 s for the core and 10.9 s for the viewer
with mixed cache hits. Keep that compilation cost in mind when choosing a
profile for one run.

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

CI caches Cargo registry sources and build outputs separately for each job.
Build cache keys include the lockfiles and commit, with restore prefixes for
previous commits and dependency sets. This refreshes the cache after source
changes while allowing Cargo to rebuild anything invalidated by a compiler or
lockfile change. npm caches package downloads for both browser and docs jobs.

The core, viewer and Pages jobs share a cached `wasm-bindgen` executable.
`scripts/install-wasm-bindgen.sh` checks its version against the current
package's lockfile and installs it only when missing or mismatched. Superseded
Rust workflow runs are cancelled for the same ref. Pages deployments stay queued.

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
