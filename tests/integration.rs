// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
//! Public API regressions, grouped by subsystem for Cargo test filters.

#[path = "integration/archive.rs"]
mod archive;
#[path = "integration/binary.rs"]
mod binary;
#[path = "integration/geometry.rs"]
mod geometry;
#[path = "integration/inventory.rs"]
mod inventory;
#[path = "integration/material.rs"]
mod material;
#[path = "integration/ole.rs"]
mod ole;
#[path = "integration/parser.rs"]
mod parser;
