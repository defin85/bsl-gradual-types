//! Adapters for external data sources (compat re-export)

// Для совместимости: реэкспортируем загрузчики из плоской структуры `data::loaders`
pub use crate::data::loaders::category_hierarchy_parser;
pub use crate::data::loaders::config_parser_discovery;
pub use crate::data::loaders::config_parser_guided_discovery;
pub use crate::data::loaders::config_parser_quick_xml;
pub use crate::data::loaders::config_parser_xml;
pub use crate::data::loaders::facet_cache;
pub use crate::data::loaders::platform_types_v2;
pub use crate::data::loaders::syntax_helper_parser;
