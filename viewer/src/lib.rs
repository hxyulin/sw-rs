// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
//! Archive-backed SolidWorks viewer model and RXUI browser application.
pub mod appearance;
#[cfg(any(target_arch = "wasm32", test))]
mod material_info;
pub mod project;
#[cfg(target_arch = "wasm32")]
mod web;
#[cfg(target_arch = "wasm32")]
pub use web::WebApp;
