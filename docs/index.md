---
# SPDX-License-Identifier: MIT
# Copyright (c) 2026 hxyulin <hxyulin@proton.me>
layout: home
hero:
  name: sw-rs
  text: Read the saved model.
  tagline: A Rust and WebAssembly reader for SolidWorks metadata, cached geometry and source evidence.
  actions:
    - theme: brand
      text: Get started
      link: /getting-started
    - theme: alt
      text: Explore the save format
      link: /format
    - theme: alt
      text: Open SW Explorer
      link: https://hxyulin.github.io/sw-rs/
features:
  - title: Portable reader
    details: Bytes in, typed Rust records out. No CAD installation, C++ runtime, filesystem or network needed by the core.
  - title: Inspect before inferring
    details: Source hashes, byte spans, raw tokens and unresolved identities distinguish recovered records from verified relationships.
  - title: Local browser viewer
    details: Open native files or archives, choose an assembly and inspect its hierarchy, saved materials and available meshes.
---

## Read by task

- **Use the library:** [quickstart](getting-started.md), [Rust and WASM API](api.md), [CLI](cli.md), [generated Rust reference](https://hxyulin.github.io/sw-rs/api/sw_rs/).
- **Understand the format:** [format overview](format.md), [containers](containers.md), [metadata](metadata.md), [geometry](geometry.md), [materials and presentation](presentation.md).
- **Follow the investigation:** [measured coverage](parsing-coverage.md), [archive maps](archive-reference-rules.md), [identities](references.md), [surface tuples](surface-reference-rules.md), [mates](mates.md), [appearance hypotheses](appearance-research.md), [legacy compression](legacy-zlb-rules.md).
- **Contribute:** [architecture](architecture.md), [development and deployment](development.md), [validation](validation.md), [porting history](porting.md), [licensing](licensing.md).

This is an independent partial reader. Recovered streams are not complete semantic
understanding; cached triangles are not regenerated CAD; candidate IDs are not
resolved topology. Coverage and hypotheses are documented separately.
