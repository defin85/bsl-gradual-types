//! Индексация BSL модулей конфигурации (CommonModule/ObjectModule/ManagerModule/RecordSetModule)
//!
//! Цель: извлечь экспортные процедуры/функции из модулей конфигурации и
//! зарегистрировать их в SignatureIndex как `SignatureSource::Configuration`.
//!
//! На первом этапе извлекается только имя, список параметров и признак `Экспорт`.
//! Типы параметров и возвращаемых значений добавляются отдельными этапами (см. roadmap).

mod ast_fallback;
mod compare;
mod directives;
mod indexing;
mod inference;
mod metrics;
mod parsing;
mod return_inference;
mod single_pass;
#[cfg(test)]
mod tests;
mod types;
mod utils;

pub use compare::{
    compare_module_parsing_from_file, compare_module_parsing_from_file_with_progress,
    compare_module_parsing_from_file_with_progress_mode,
    single_pass_module_stats_from_file_with_progress_mode,
};
pub use indexing::{
    collect_module_paths, index_configuration_bsl_modules,
    index_configuration_bsl_modules_by_paths, index_configuration_bsl_modules_with_progress,
    index_configuration_bsl_modules_with_progress_parallel,
};
pub use types::{
    IndexedConfigSignatures, ModuleIndexProgress, ModuleIndexResult, ModuleParseComparison,
    ModuleParseStats, ModuleSignatureSnapshot, SinglePassMode,
};

pub(crate) use indexing::index_configuration_bsl_modules_with_progress_parallel_cached;
pub(crate) use types::ParsedModuleData;
