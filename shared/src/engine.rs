//! The core analysis engine, independent of any specific adapter (backend, CLI, etc.).
//!
//! Phase 3: Simplified - no Infrastructure dependencies
//! Infrastructure initialization moved to SystemCoordinator (backend)

use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tracing::{info, warn};

use crate::domain::repository::{InMemoryTypeRepository, TypeRepository};
use crate::domain::resolver::TypeResolver;
use crate::domain::types::TypeResolution;

/// A simplified, self-contained analysis result for the CLI.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CliAnalysisResult {
    pub file_path: String,
    pub type_resolutions: Vec<(String, TypeResolution)>,
    pub analysis_duration_ms: u128,
}

/// The core analysis engine.
/// It orchestrates parsing and type resolution.
///
/// Phase 3: Simplified - no Infrastructure dependencies, receives pre-initialized components
pub struct AnalysisEngine {
    resolver: Arc<TypeResolver>,
    repository: Arc<dyn TypeRepository>,
}

impl AnalysisEngine {
    /// Creates a new analysis engine with pre-initialized components (Phase 3)
    ///
    /// No I/O operations, no Infrastructure - pure Domain orchestration
    pub fn new(resolver: Arc<TypeResolver>, repository: Arc<dyn TypeRepository>) -> Self {
        info!("✨ AnalysisEngine: создан с готовыми компонентами (Phase 3)");
        Self {
            resolver,
            repository,
        }
    }

    /// DEPRECATED: Old initialization method with Infrastructure dependencies
    ///
    /// This method will be removed in future versions. Use `new()` instead.
    /// Infrastructure initialization is now in SystemCoordinator (backend).
    #[deprecated(
        since = "0.4.3",
        note = "Use AnalysisEngine::new() with pre-initialized components. Infrastructure moved to SystemCoordinator."
    )]
    #[allow(dead_code)]
    pub async fn new_with_init(
        _syntax_helper_path: Option<&Path>,
        _config_path: Option<&Path>,
    ) -> Result<Self> {
        warn!("⚠️ AnalysisEngine::new_with_init() is deprecated!");
        warn!("⚠️ Infrastructure initialization moved to SystemCoordinator");
        warn!("⚠️ Use AnalysisEngine::new() with pre-initialized components");

        // Stub implementation for backward compatibility
        let repository = Arc::new(InMemoryTypeRepository::new());
        let resolver = Arc::new(TypeResolver::new(repository.clone()));

        Ok(Self {
            resolver,
            repository,
        })
    }


    /// Получить resolver для TypeSystemService
    pub fn get_resolver(&self) -> Arc<TypeResolver> {
        self.resolver.clone()
    }

    /// Получить repository для TypeInferenceService
    pub fn get_repository(&self) -> Arc<dyn TypeRepository> {
        self.repository.clone()
    }

    /// Analyzes a single BSL file.
    pub async fn analyze_file<P: AsRef<Path>>(&self, path: P) -> Result<CliAnalysisResult> {
        let start_time = std::time::Instant::now();
        let path_str = path.as_ref().display().to_string();

        let _content = std::fs::read_to_string(path.as_ref())
            .map_err(|e| anyhow::anyhow!("Failed to read file {}: {}", path_str, e))?;

        // MOCK IMPLEMENTATION
        let mut resolutions_map = HashMap::new();
        resolutions_map.insert(
            "ПеременнаяА".to_string(),
            self.resolver.resolve_expression_sync("Строка"),
        );
        resolutions_map.insert(
            "ПеременнаяБ".to_string(),
            self.resolver.resolve_expression_sync("Справочники.Контрагенты"),
        );
        let resolutions: Vec<(String, TypeResolution)> = resolutions_map.into_iter().collect();

        let result = CliAnalysisResult {
            file_path: path_str,
            type_resolutions: resolutions,
            analysis_duration_ms: start_time.elapsed().as_millis(),
        };

        Ok(result)
    }
}