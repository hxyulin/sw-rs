<!-- SPDX-License-Identifier: MIT -->
<!-- Copyright (c) 2026 hxyulin <hxyulin@proton.me> -->
# Contributing

Start with the [development guide](docs/development.md) for prerequisites, native
and WASM checks, browser tests, documentation builds and Pages deployment.
[Architecture](docs/architecture.md) describes module ownership; the
[format guide](docs/format.md) distinguishes implemented rules from hypotheses.

Use checked byte reads, preserve unknown records and source provenance, and avoid
new dependencies unless the change needs them. Add meaningful synthetic tests for
new parser rules, including malformed and ambiguous input. Keep original CAD,
extracted streams and private research reports in ignored `local-assets/`.

Before committing, run formatting for **both** Cargo packages, their tests and
strict Clippy. Build documentation and run `npm run site:check` against an assembled
site when changing navigation, published paths or deployment. Rebuild WASM before
browser tests after Rust changes. See the development guide for exact commands.

Record user-visible changes in `CHANGELOG.md`. Use `type(scope): summary` commit
messages and retain original licenses/provenance notices. Document source-based
observations separately from unverified layout interpretations; no matching ID,
class-name scan or approximate shader should be described as complete native
ownership or rendering support.
