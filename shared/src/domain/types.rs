//! BSL Type System
//!
//! Most of this module is implemented in `bsl-types` and re-exported from `bsl-shared`
//! to keep historical import paths stable (`bsl_shared::domain::types::*`).

// Core type-system structs/enums.
pub use bsl_types::types::*;

// Diagnostics are still hosted in `bsl-shared` because they depend on IR spans.
mod diagnostics;
pub use diagnostics::*;
