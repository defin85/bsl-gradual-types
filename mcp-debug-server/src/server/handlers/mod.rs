//! Handler modules for MCP Debug Server
//!
//! Contains all tool handlers organized by functionality:
//! - `debug_handlers` - breakpoints, launch, eval, backtrace, events
//! - `session_handlers` - create, list, terminate sessions
//! - `step_handlers` - next, step_in, step_out, continue

pub mod debug_handlers;
pub mod session_handlers;
pub mod step_handlers;
