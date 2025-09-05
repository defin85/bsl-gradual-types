//! Общие компоненты интерфейса

#[cfg(feature = "web-ui")]
pub mod header;
#[cfg(feature = "web-ui")]
pub mod search_bar;
#[cfg(feature = "web-ui")]
pub mod metric_card;

#[cfg(feature = "web-ui")]
pub use header::*;
#[cfg(feature = "web-ui")]
pub use search_bar::*;
#[cfg(feature = "web-ui")]
pub use metric_card::*;
