//! Загрузчики источников данных (упрощённая структура)

pub mod category_hierarchy_parser;
pub mod config_parser_discovery;
pub mod config_parser_guided_discovery;
pub mod config_parser_quick_xml;
pub mod config_parser_xml;
// TEMPORARY: Disabled during architecture simplification
// pub mod facet_cache;
pub mod platform_types_repository;
pub mod syntax_helper_parser;
