//! Data Layer: Adapters
//!
//! Адаптеры для преобразования между форматами данных разных слоев

pub mod converters;

// Re-exports для удобства использования
pub use converters::{
    convert_discovered_metadata_to_raw, convert_resolutions_to_raw, convert_syntax_helper_to_raw,
};
