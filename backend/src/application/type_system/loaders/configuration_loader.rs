//! Configuration Loader - loading types from 1C configuration
//!
//! Functions for discovering and loading configuration metadata types.

use anyhow::Result;
use tracing::info;

use bsl_shared::engine::AnalysisEngine;

use crate::data::loaders::config_metadata_parser::ConfigurationDiscovery;
use crate::data::loaders::progress::ProgressUpdate;

/// Load types from 1C configuration (MILESTONE 2.17)
///
/// Correct way to load configuration types through Application Layer
/// (instead of LSP directly accessing TypeRepository).
///
/// # Arguments
/// * `analysis_engine` - AnalysisEngine for accessing TypeRepository
/// * `config_path` - Path to configuration folder (containing Configuration.xml)
///
/// # Returns
/// Result<usize> - Number of successfully loaded types
///
/// # Errors
/// - If config_path doesn't exist or is invalid
/// - If Configuration.xml is missing
/// - If metadata parsing fails
/// - If loading types into TypeRepository fails
///
/// # Note
/// This function ensures proper layer separation:
/// - LSP Server (Presentation) -> TypeSystemService (Application) -> TypeRepository (Domain)
/// - Instead of: LSP Server -> TypeRepository (bypassing Application Layer)
pub fn load_configuration_types(
    analysis_engine: &AnalysisEngine,
    config_path: &std::path::Path,
) -> Result<usize> {
    info!(
        "Loading configuration types from: {}",
        config_path.display()
    );

    // Path validation (protection from path traversal)
    if !config_path.exists() {
        anyhow::bail!(
            "Configuration path does not exist: {}",
            config_path.display()
        );
    }

    if !config_path.is_dir() {
        anyhow::bail!("Configuration path is not a directory");
    }

    let config_xml = config_path.join("Configuration.xml");
    if !config_xml.exists() {
        anyhow::bail!("Configuration.xml not found in directory");
    }

    // Path canonicalization (protection from ../..)
    let canonical_path = config_path
        .canonicalize()
        .map_err(|e| anyhow::anyhow!("Invalid configuration path: {}", e))?;

    // Discover metadata with terminal progress bar
    // show_progress = true for CLI (built-in indicatif ProgressBar in discovery.rs)
    // Callback not needed - progress shown directly in terminal
    let discovery = ConfigurationDiscovery::new(canonical_path.clone(), true);
    let metadata = discovery
        .discover_all_metadata(None::<fn(ProgressUpdate)>)
        .map_err(|e| anyhow::anyhow!("Failed to discover metadata: {}", e))?;

    info!("Discovered {} metadata objects", metadata.len());

    // Load types in batches for performance
    const BATCH_SIZE: usize = 100;
    let mut loaded = 0;
    let mut batch = Vec::with_capacity(BATCH_SIZE);

    for obj in metadata.iter() {
        let raw_type = obj.to_raw_type_data(None);
        batch.push(raw_type);

        if batch.len() >= BATCH_SIZE {
            analysis_engine.get_repository().load_types(batch.clone())?;
            loaded += batch.len();
            batch.clear();
        }
    }

    // Load remainder
    if !batch.is_empty() {
        analysis_engine.get_repository().load_types(batch.clone())?;
        loaded += batch.len();
    }

    info!(
        "Successfully loaded {} configuration types from {}",
        loaded,
        canonical_path.display()
    );

    Ok(loaded)
}

/// Get module paths for configuration type (Milestone 3.14)
///
/// Used for Go To Definition navigation to ObjectModule.bsl, ManagerModule.bsl, etc.
///
/// # Arguments
/// * `analysis_engine` - AnalysisEngine for accessing TypeRepository
/// * `type_name` - Configuration type name (e.g., "Справочники.Партнеры")
///
/// # Returns
/// * `Some(ModulePaths)` - Module paths if type is configuration-based
/// * `None` - If type is platform or not found in repository
pub fn get_module_paths_for_type(
    analysis_engine: &AnalysisEngine,
    type_name: &str,
) -> Option<bsl_shared::domain::type_definition_location::ModulePaths> {
    // Get raw type data from repository
    let repository = analysis_engine.get_repository();
    repository
        .find_type(type_name)
        .and_then(|raw| raw.module_paths.clone())
}

/// Pre-resolve types for commonly used signatures (Milestone 3.15)
///
/// Called after loading platform types to speed up first hover/completion.
/// Fills lazy cache in MethodSignature for types from common_types list.
///
/// # Arguments
/// * `analysis_engine` - AnalysisEngine for type resolution
///
/// # Example
/// ```text
/// prewarm_signature_cache(analysis_engine);
/// ```
pub fn prewarm_signature_cache(analysis_engine: &AnalysisEngine) {
    let resolver = analysis_engine.get_resolver();
    let repository = analysis_engine.get_repository();
    let signature_index = repository.get_signature_index_clone();

    // Commonly used types for pre-warm
    let common_types = [
        "Массив",
        "ТаблицаЗначений",
        "СтрокаТаблицыЗначений",
        "Соответствие",
        "Структура",
        "СписокЗначений",
        "Строка",
        "Число",
        "Дата",
        "Булево",
        // Faceted types
        "СправочникМенеджер",
        "СправочникОбъект",
        "СправочникСсылка",
        "ДокументМенеджер",
        "ДокументОбъект",
        "ДокументСсылка",
    ];

    let mut warmed_count = 0;

    for type_name in &common_types {
        // Get methods for type
        let methods = signature_index.get_type_methods(type_name);
        for method in methods {
            // Trigger lazy resolution via closure
            let _ = method.get_resolved_return_type(|t| resolver.resolve_expression_sync(t));
            warmed_count += 1;
        }
    }

    info!(
        "Signature cache pre-warmed: {} method signatures for {} common types",
        warmed_count,
        common_types.len()
    );
}
