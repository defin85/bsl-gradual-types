//! The core analysis engine, independent of any specific adapter (backend, CLI, etc.).

use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tracing::{info, warn};

use crate::domain::repository::{InMemoryTypeRepository, TypeRepository};
use crate::domain::resolver::TypeResolver;
use crate::domain::types::TypeResolution;
// TEMPORARY: Эти импорты будут удалены в Phase 3 рефакторинга
// AnalysisEngine не должен создавать Infrastructure компоненты
// TODO Phase 3: Переместить инициализацию в SystemCoordinator (backend)
use crate::loaders::{
    SyntaxHelperParser,
    convert_syntax_helper_to_raw
};

// HACK: Временно используем stub версию SyntaxHelperParser до Phase 3
// В будущем эта логика переедет в backend/system/system_coordinator.rs

/// A simplified, self-contained analysis result for the CLI.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CliAnalysisResult {
    pub file_path: String,
    pub type_resolutions: Vec<(String, TypeResolution)>,
    pub analysis_duration_ms: u128,
}

/// The core analysis engine.
/// It orchestrates parsing and type resolution.
pub struct AnalysisEngine {
    resolver: Arc<TypeResolver>,
    repository: Arc<dyn TypeRepository>,
}

impl AnalysisEngine {
    /// Creates a new analysis engine with explicit initialization.
    pub async fn new_with_init(
        syntax_helper_path: Option<&Path>,
        _config_path: Option<&Path>,
    ) -> Result<Self> {
        info!("🚀 AnalysisEngine: инициализация Domain Layer...");

        let mut syntax_parser = SyntaxHelperParser::new();

        // Загружаем синтаксис-помощник если путь указан
        if let Some(syntax_path) = syntax_helper_path {
            info!("📂 Загружаем синтаксис-помощник: {}", syntax_path.display());

            match syntax_parser.parse_syntax_helper(syntax_path) {
                Ok(()) => {
                    info!("✅ Парсинг синтаксис-помощника завершен успешно");
                }
                Err(e) => {
                    warn!("⚠️ Ошибка парсинга синтаксис-помощника: {}", e);
                    info!("📦 Будем использовать базовые типы платформы 1С...");
                }
            }
        }

        // Создаем repository и resolver
        let repository = Arc::new(InMemoryTypeRepository::new());

        // Загружаем данные в репозиторий
        let database = syntax_parser.export_database();
        if !database.nodes.is_empty() {
            let platform_raw_data = convert_syntax_helper_to_raw(&database);
            repository.load_types(platform_raw_data)?;

            let stats = repository.get_stats();
            info!("📊 Загружено {} типов из синтаксис-помощника", stats.total_types);
        } else {
            // Загружаем базовые типы
            Self::load_fallback_types(&repository)?;
        }

        let resolver = Arc::new(TypeResolver::new(repository.clone()));

        info!("✅ AnalysisEngine: Domain Layer готов!");
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

    /// Загрузка базовых типов как fallback
    fn load_fallback_types(repository: &Arc<InMemoryTypeRepository>) -> Result<()> {
        use crate::domain::types::{RawTypeData, RawDataSource};

        info!("📦 Загружаем базовые типы платформы 1С...");

        let basic_types = vec![
            RawTypeData {
                name: "Строка".to_string(),
                english_name: "String".to_string(),
                description: "Базовый тип Строка".to_string(),
                category: "Примитивные типы".to_string(),
                source: RawDataSource::Platform,
                methods: Vec::new(),
                properties: Vec::new(),
                facets: Vec::new(),
                kind: None,
                attributes: Vec::new(),
                tabular_sections: Vec::new(),
            },
            RawTypeData {
                name: "Число".to_string(),
                english_name: "Number".to_string(),
                description: "Базовый тип Число".to_string(),
                category: "Примитивные типы".to_string(),
                source: RawDataSource::Platform,
                methods: Vec::new(),
                properties: Vec::new(),
                facets: Vec::new(),
                kind: None,
                attributes: Vec::new(),
                tabular_sections: Vec::new(),
            },
            RawTypeData {
                name: "Булево".to_string(),
                english_name: "Boolean".to_string(),
                description: "Базовый тип Булево".to_string(),
                category: "Примитивные типы".to_string(),
                source: RawDataSource::Platform,
                methods: Vec::new(),
                properties: Vec::new(),
                facets: Vec::new(),
                kind: None,
                attributes: Vec::new(),
                tabular_sections: Vec::new(),
            },
            RawTypeData {
                name: "Дата".to_string(),
                english_name: "Date".to_string(),
                description: "Базовый тип Дата".to_string(),
                category: "Примитивные типы".to_string(),
                source: RawDataSource::Platform,
                methods: Vec::new(),
                properties: Vec::new(),
                facets: Vec::new(),
                kind: None,
                attributes: Vec::new(),
                tabular_sections: Vec::new(),
            },
            RawTypeData {
                name: "Неопределено".to_string(),
                english_name: "Undefined".to_string(),
                description: "Базовый тип Неопределено".to_string(),
                category: "Примитивные типы".to_string(),
                source: RawDataSource::Platform,
                methods: Vec::new(),
                properties: Vec::new(),
                facets: Vec::new(),
                kind: None,
                attributes: Vec::new(),
                tabular_sections: Vec::new(),
            },
        ];

        repository.load_types(basic_types)?;
        info!("✅ Загружено {} базовых типов", 5);
        Ok(())
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