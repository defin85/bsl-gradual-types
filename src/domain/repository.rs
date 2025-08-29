//! Type repository trait and implementations

use crate::domain::analysis::type_checker::TypeContext;
use crate::domain::search::RawTypeData;
use crate::domain::types::TypeResolution;
use anyhow::Result;
use std::sync::Arc;
use std::sync::RwLock;

/// Trait для репозитория типов
pub trait TypeRepository: Send + Sync {
    /// Получить все типы платформы
    fn get_platform_types(&self) -> Result<Vec<RawTypeData>>;

    /// Получить типы конфигурации
    fn get_configuration_types(&self, config_path: &str) -> Result<Vec<RawTypeData>>;

    /// Сохранить типы в репозиторий
    fn save_types(&self, types: Vec<RawTypeData>) -> Result<()>;

    /// Получить статистику репозитория
    fn get_stats(&self) -> RepositoryStats;
}

/// Статистика репозитория
#[derive(Debug, Clone, Default)]
pub struct RepositoryStats {
    pub total_types: usize,
    pub platform_types: usize,
    pub configuration_types: usize,
    pub user_defined_types: usize,
}

/// In-memory реализация репозитория для тестирования
pub struct InMemoryTypeRepository {
    platform_types: RwLock<Vec<RawTypeData>>,
    configuration_types: RwLock<Vec<RawTypeData>>,
}

impl InMemoryTypeRepository {
    pub fn new() -> Self {
        Self {
            platform_types: RwLock::new(Vec::new()),
            configuration_types: RwLock::new(Vec::new()),
        }
    }
}

impl TypeRepository for InMemoryTypeRepository {
    fn get_platform_types(&self) -> Result<Vec<RawTypeData>> {
        let types = self.platform_types.read().unwrap();
        Ok(types.clone())
    }

    fn get_configuration_types(&self, _config_path: &str) -> Result<Vec<RawTypeData>> {
        let types = self.configuration_types.read().unwrap();
        Ok(types.clone())
    }

    fn save_types(&self, types: Vec<RawTypeData>) -> Result<()> {
        // Разделяем типы по источникам
        let mut platform_types = self.platform_types.write().unwrap();
        let mut config_types = self.configuration_types.write().unwrap();

        for type_data in types {
            match &type_data.source {
                crate::data::TypeSource::Platform { .. } => {
                    platform_types.push(type_data);
                }
                crate::data::TypeSource::Configuration { .. } => {
                    config_types.push(type_data);
                }
                _ => {
                    // По умолчанию считаем платформенным
                    platform_types.push(type_data);
                }
            }
        }

        Ok(())
    }

    fn get_stats(&self) -> RepositoryStats {
        let platform_count = self.platform_types.read().unwrap().len();
        let config_count = self.configuration_types.read().unwrap().len();

        RepositoryStats {
            total_types: platform_count + config_count,
            platform_types: platform_count,
            configuration_types: config_count,
            user_defined_types: 0, // TODO: Добавить поддержку пользовательских типов
        }
    }
}

/// Сервис разрешения типов
pub struct TypeResolutionService {
    #[allow(dead_code)]
    repository: Arc<dyn TypeRepository>,
}

impl TypeResolutionService {
    pub fn new(repository: Arc<dyn TypeRepository>) -> Self {
        Self { repository }
    }

    /// Инициализировать сервис разрешения типов
    pub async fn initialize(&self) -> Result<()> {
        // Инициализируем PlatformTypeResolver через вызов singleton
        use crate::domain::resolvers::platform::PlatformTypeResolver;
        let _platform_resolver = PlatformTypeResolver::instance();

        // TODO: Здесь можно добавить синхронизацию данных между resolver и repository
        Ok(())
    }

    /// Получить все платформенные типы через репозиторий
    pub fn get_platform_types(&self) -> Result<Vec<crate::data::RawTypeData>> {
        self.repository.get_platform_types()
    }

    /// Разрешить выражение в типе
    pub async fn resolve_expression(
        &self,
        _expression: &str,
        _context: &TypeContext,
    ) -> Result<TypeResolution> {
        // TODO: Implement expression resolution
        Ok(TypeResolution::unknown())
    }

    /// Получить все платформенные глобальные типы (для CentralTypeSystem)
    pub fn get_all_platform_globals(&self) -> &std::collections::HashMap<String, TypeResolution> {
        use crate::domain::resolvers::platform::PlatformTypeResolver;
        let resolver = PlatformTypeResolver::instance();
        resolver.get_platform_globals()
    }

    /// Получить автодополнения для запроса (для WebTypeService)
    pub fn get_completions(
        &self,
        query: &str,
    ) -> Vec<crate::domain::CompletionItem> {
        use crate::domain::resolvers::platform::PlatformTypeResolver;
        let resolver = PlatformTypeResolver::instance();
        resolver.get_completions(query)
    }

    /// Асинхронное разрешение выражений (для CentralTypeSystem)
    pub async fn resolve_expression_async(&self, expression: &str) -> TypeResolution {
        use crate::domain::resolvers::platform::PlatformTypeResolver;
        // Используем immutable версию для работы с синглтоном
        let resolver = PlatformTypeResolver::instance();
        resolver.resolve_expression_immutable(expression)
    }

    /// Поиск типов по запросу (для WebTypeService)
    pub fn search_types(&self, query: &str) -> Vec<String> {
        let completions = self.get_completions(query);
        completions.into_iter().map(|c| c.label).collect()
    }

    /// Получить информацию о конкретном типе (для CentralTypeSystem)
    pub fn get_type_info(&self, type_name: &str) -> Option<crate::system::coordination::TypeInfo> {
        let platform_globals = self.get_all_platform_globals();
        let resolution = platform_globals.get(type_name)?;

        if !matches!(
            resolution.certainty,
            crate::domain::types::Certainty::Unknown
        ) {
            Some(crate::system::coordination::TypeInfo {
                name: type_name.to_string(),
                category: format!("{:?}", resolution.result),
                description: resolution.metadata.notes.first().cloned(),
                methods: Vec::new(), // TODO: реализовать через анализ metadata когда будет готово
                properties: Vec::new(), // TODO: реализовать через анализ metadata когда будет готово
                constructors: Vec::new(), // TODO: добавить когда будет готово
            })
        } else {
            None
        }
    }
}

/// Сервис проверки типов
pub struct TypeCheckerService {
    // TODO: Implement after migration complete
}

impl TypeCheckerService {
    pub fn new() -> Self {
        Self {}
    }

    /// Проверить совместимость присваивания типов
    pub fn is_assignment_compatible(&self, _from: &TypeResolution, _to: &TypeResolution) -> bool {
        // TODO: Implement proper type compatibility check
        true
    }
}
