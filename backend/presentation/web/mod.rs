//! Web UI presentation layer

#[cfg(feature = "web-ui")]
pub mod components;

// CSR entrypoint for WASM (mounted by Trunk)
#[cfg(all(feature = "web-ui", target_arch = "wasm32"))]
pub mod client;


