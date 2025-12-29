//! MCP Debug Server
//!
//! AI-assisted interactive debugging through DAP (Debug Adapter Protocol).
//!
//! This crate provides a Model Context Protocol (MCP) server that allows AI assistants
//! like Claude to interactively debug programs using standard debuggers (CodeLLDB, GDB, etc.)
//! through the Debug Adapter Protocol.
//!
//! ## Features
//!
//! - **12 MCP Tools**: create_session, set_breakpoint, launch, step, eval, backtrace, etc.
//! - **3 MCP Resources**: session info, state, breakpoints
//! - **Full Async Architecture**: EventRouter + EventProcessor for real-time event handling
//! - **Thread-safe**: Concurrent debug sessions via `Arc<RwLock<HashMap>>`
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use mcp_debug_server::session::SessionManager;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let manager = SessionManager::new();
//!
//!     // Create debug session
//!     let session_id = manager
//!         .create_session("./target/debug/my_app".to_string(), "codelldb".to_string())
//!         .await?;
//!
//!     // Set breakpoint, launch, debug...
//!     // See examples/simple_debug.rs for full workflow
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────┐
//! │ MCP Client  │ (Claude Code)
//! └──────┬──────┘
//!        │ stdio/JSON-RPC
//!        ▼
//! ┌─────────────┐
//! │ MCP Server  │ (12 tools, 3 resources)
//! └──────┬──────┘
//!        │
//!        ▼
//! ┌─────────────┐
//! │ SessionMgr  │ (Arc<RwLock<HashMap>>)
//! └──────┬──────┘
//!        │
//!        ▼
//! ┌─────────────┐
//! │ DapClient   │ (EventRouter + EventProcessor)
//! └──────┬──────┘
//!        │ DAP Protocol
//!        ▼
//! ┌─────────────┐
//! │ CodeLLDB    │ (or GDB, debugpy, etc.)
//! └─────────────┘
//! ```
//!
//! ## Modules
//!
//! - [`session`]: Session management and state tracking
//! - [`dap`]: DAP client and event handling
//! - [`server`]: MCP tools and resources
//! - [`types`]: Shared types (SessionId, errors)
//! - [`config`]: Adapter configuration

pub mod config;
pub mod dap;
pub mod server;
pub mod session;
pub mod types;
