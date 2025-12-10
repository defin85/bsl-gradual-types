//! Type validation rules based on Balyuk & Popova (2021) research
//!
//! This module implements static type-checking validation inspired by:
//! "Static type-checking for programs developed on the platform 1C:Enterprise"
//! https://ceur-ws.org/Vol-2984/paper13.pdf
//!
//! Three main categories of errors detected:
//! 1. Incorrect parameter passing to methods
//! 2. Access to non-existent properties of objects
//! 3. Treating simple types as collections

mod error_kinds;
mod error_formatting;
mod type_validator;
mod tests;

// Re-export public API
pub use error_kinds::TypeErrorKind;
pub use type_validator::TypeValidator;
