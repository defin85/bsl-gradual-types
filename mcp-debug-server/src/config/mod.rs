//! Configuration module for DAP adapters and MCP server settings

pub mod adapters;

// Re-export commonly used functions
pub use adapters::{find_codelldb, resolve_adapter};
