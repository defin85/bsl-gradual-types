//! Leptos компоненты для BSL Type System

#[cfg(feature = "web-ui")]
pub mod app;
#[cfg(feature = "web-ui")]
pub mod dashboard;
#[cfg(feature = "web-ui")]
pub mod type_cards;
#[cfg(feature = "web-ui")]
pub mod type_table;
#[cfg(feature = "web-ui")]
pub mod type_graph;
#[cfg(feature = "web-ui")]
pub mod api;
#[cfg(feature = "web-ui")]
pub mod common;

#[cfg(feature = "web-ui")]
pub use app::App;
#[cfg(feature = "web-ui")]
pub use dashboard::*;
#[cfg(feature = "web-ui")]
pub use type_cards::*;
#[cfg(feature = "web-ui")]
pub use type_table::*;
#[cfg(feature = "web-ui")]
pub use type_graph::*;
