//! Validators submodule for semantic validation
//!
//! Contains type validation and call validation logic.

mod call_validator;
mod type_validator;

pub use call_validator::{validate_global_function_call_context, validate_method_call_context};
#[allow(unused_imports)]
pub use call_validator::validation_result_to_diagnostic;
pub use type_validator::validation_result_v2_to_diagnostic;

