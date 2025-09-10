//! Type repository trait and implementations

use crate::domain::types::{TypeResolution, RawTypeData};
use anyhow::Result;
use std::sync::Arc;
use std::sync::RwLock;

/// Элемент автодополнения (совместимый с LSP)
#[derive(Debug, Clone, serde::Serialize)]
pub struct CompletionItem {
    pub label: String,
    pub kind: CompletionKind,
    pub detail: Option<String>,
    pub documentation: Option<String>,
    // Дополнительные поля для LSP
    pub insert_text: Option<String>,
    pub filter_text: Option<String>,
    pub sort_text: Option<String>,
}

impl CompletionItem {
    /// Создать элемент автодополнения с базовыми полями
    pub fn new(label: String, kind: CompletionKind) -> Self {
        Self {
            insert_text: Some(label.clone()),
            filter_text: Some(label.clone()),
            sort_text: Some(label.clone()),
            label,
            kind,
            detail: None,
            documentation: None,
        }
    }

    /// Создать элемент с дополнительными полями  
    pub fn with_details(
        label: String,
        kind: CompletionKind,
        detail: Option<String>,
        documentation: Option<String>,
    ) -> Self {
        Self {
            insert_text: Some(label.clone()),
            filter_text: Some(label.clone()),
            sort_text: Some(label.clone()),
            label,
            kind,
            detail,
            documentation,
        }
    }
}

/// Тип элемента автодополнения
#[derive(Debug, Clone, Copy, serde::Serialize)]
pub enum CompletionKind {
    Text,
    Method,
    Function,
    Constructor,
    Field,
    Variable,
    Class,
    Interface,
    Module,
    Property,
    Unit,
    Value,
    Enum,
    Keyword,
    Snippet,
    Color,
    File,
    Reference,
    Folder,
    EnumMember,
    Constant,
    Struct,
    Type,
    Event,
    Operator,
    TypeParameter,
    // Дополнительные варианты для BSL
    Global,
    Catalog,
    Document,
}

impl From<CompletionKind> for u8 {
    fn from(kind: CompletionKind) -> Self {
        match kind {
            CompletionKind::Text => 1,
            CompletionKind::Method => 2,
            CompletionKind::Function | CompletionKind::Global => 3,
            CompletionKind::Constructor => 4,
            CompletionKind::Field => 5,
            CompletionKind::Variable => 6,
            CompletionKind::Class | CompletionKind::Catalog | CompletionKind::Document => 7,
            CompletionKind::Interface => 8,
            CompletionKind::Module => 9,
            CompletionKind::Property => 10,
            CompletionKind::Unit => 11,
            CompletionKind::Value => 12,
            CompletionKind::Enum => 13,
            CompletionKind::Keyword => 14,
            CompletionKind::Snippet => 15,
            CompletionKind::Color => 16,
            CompletionKind::File => 17,
            CompletionKind::Reference => 18,
            CompletionKind::Folder => 19,
            CompletionKind::EnumMember => 20,
            CompletionKind::Constant => 21,
            CompletionKind::Struct => 22,
            CompletionKind::Type => 26,
            CompletionKind::Event => 23,
            CompletionKind::Operator => 24,
            CompletionKind::TypeParameter => 25,
        }
    }
}

/// Trait для репозитория типов
pub trait TypeRepository: Send + Sync {
    /// Получить все типы платформы
    fn get_platform_types(&self) -> Result<Vec<RawTypeData>>;

    /// Получить типы конфигурации
    fn get_configuration_types(&self, config_path: &str) -> Result<Vec<RawTypeData>>;

    /// Получить все типы (платформенные + конфигурационные)
    fn get_all_types(&self) -> Vec<RawTypeData>;

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

    fn get_all_types(&self) -> Vec<RawTypeData> {
        let mut all_types = Vec::new();

        // Добавляем платформенные типы
        if let Ok(platform_types) = self.get_platform_types() {
            all_types.extend(platform_types);
        }

        // Добавляем конфигурационные типы
        if let Ok(config_types) = self.get_configuration_types("") {
            all_types.extend(config_types);
        }

        all_types
    }

    fn save_types(&self, _types: Vec<RawTypeData>) -> Result<()> {
        // TODO: Re-implement after TypeSource is moved to shared
        /*
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
        */
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
pub struct TypeResolver {
    #[allow(dead_code)]
    repository: Arc<dyn TypeRepository>,
}

impl TypeResolver {
    pub fn new(repository: Arc<dyn TypeRepository>) -> Self {
        Self { repository }
    }

    /// Инициализировать сервис разрешения типов  
    pub async fn initialize(&self) -> Result<()> {
        // ✅ Убираем инициализацию через синглтон
        // Теперь всё работает через repository pattern

        println!("TypeResolver initialized with repository-only pattern");
        Ok(())
    }

    /// Получить все платформенные типы через репозиторий
    pub fn get_platform_types(&self) -> Result<Vec<RawTypeData>> {
        // TODO: Re-implement after TypeSource is moved to shared
        Ok(vec![])
    }

    /// Разрешить выражение в типе
    pub async fn resolve_expression(
        &self,
        _expression: &str,
        // TODO: Re-enable after TypeContext is moved back
        // _context: &TypeContext,
    ) -> Result<TypeResolution> {
        // TODO: Implement expression resolution
        Ok(TypeResolution::unknown())
    }

    /// Получить все платформенные глобальные типы (для SystemCoordinator)
    pub fn get_all_platform_globals(&self) -> std::collections::HashMap<String, TypeResolution> {
        // TODO: Re-implement after TypeSource is moved to shared
        /*
        // ✅ ИСПОЛЬЗУЕМ repository вместо синглтона
        let all_raw_types = self.repository.get_all_types();

        // Конвертируем RawTypeData в TypeResolution и фильтруем только платформенные
        let mut platform_globals = std::collections::HashMap::new();
        for raw_type in all_raw_types {
            if matches!(raw_type.source, crate::data::TypeSource::Platform { .. }) {
                let resolution = self.convert_raw_data_to_type_resolution(&raw_type);
                platform_globals.insert(raw_type.name.clone(), resolution);
            }
        }
        */
        std::collections::HashMap::new()
    }

    /// Получить автодополнения для запроса (для WebTypeService)
    pub fn get_completions(&self, query: &str) -> Vec<CompletionItem> {
        // ✅ ИСПОЛЬЗУЕМ repository вместо синглтона
        let all_types = self.get_all_platform_globals();

        // Простая фильтрация по запросу
        let mut completions = Vec::new();

        for (name, resolution) in &all_types {
            if name.to_lowercase().contains(&query.to_lowercase()) {
                let item = CompletionItem::with_details(
                    name.clone(),
                    self.determine_completion_kind(&resolution),
                    Some(format!("{:?}", resolution.result)),
                    resolution.metadata.notes.first().cloned(),
                );

                completions.push(item);
            }
        }

        completions
    }

    /// Определить тип автодополнения на основе TypeResolution
    fn determine_completion_kind(&self, resolution: &TypeResolution) -> CompletionKind {
        use crate::domain::types::{ConcreteType, ResolutionResult};

        match &resolution.result {
            ResolutionResult::Concrete(ConcreteType::Platform(_)) => CompletionKind::Global,
            ResolutionResult::Concrete(ConcreteType::Configuration(config)) => match config.kind {
                crate::domain::types::MetadataKind::Catalog => CompletionKind::Catalog,
                crate::domain::types::MetadataKind::Document => CompletionKind::Document,
                crate::domain::types::MetadataKind::Enum => CompletionKind::Enum,
                _ => CompletionKind::Global,
            },
            _ => CompletionKind::Global,
        }
    }

    /// Асинхронное разрешение выражений (для SystemCoordinator)
    pub async fn resolve_expression_async(&self, expression: &str) -> TypeResolution {
        // ✅ ИСПОЛЬЗУЕМ repository вместо синглтона
        // Пока простая реализация - ищем точное совпадение по имени
        let all_types = self.get_all_platform_globals();

        // Проверяем прямое совпадение
        if let Some(resolution) = all_types.get(expression) {
            return resolution.clone();
        }

        // Проверяем доступ к членам (например "Справочники.Контрагенты")
        if let Some((base, member)) = self.parse_member_access(expression) {
            return self.resolve_member_access(&base, &member);
        }

        // Если ничего не найдено, возвращаем неизвестный тип
        TypeResolution::unknown()
    }

    /// Парсинг доступа к членам вида "Base.Member"
    fn parse_member_access(&self, expression: &str) -> Option<(String, String)> {
        if let Some(dot_pos) = expression.find('.') {
            let base = expression[..dot_pos].to_string();
            let member = expression[dot_pos + 1..].to_string();
            if !base.is_empty() && !member.is_empty() {
                return Some((base, member));
            }
        }
        None
    }

    /// Разрешение доступа к членам конфигурации
    fn resolve_member_access(&self, base: &str, member: &str) -> TypeResolution {
        use crate::domain::types::{
            Certainty, ConcreteType, ConfigurationType, MetadataKind, ResolutionMetadata,
            ResolutionResult, ResolutionSource,
        };

        // Определяем тип метаданных
        let (kind, prefix) = match base {
            "Справочники" | "Catalogs" => (MetadataKind::Catalog, "Справочники"),
            "Документы" | "Documents" => (MetadataKind::Document, "Документы"),
            "Перечисления" | "Enums" => (MetadataKind::Enum, "Перечисления"),
            "РегистрыСведений" | "InformationRegisters" => {
                (MetadataKind::Register, "РегистрыСведений")
            }
            _ => {
                // Неизвестный базовый тип
                let mut resolution = TypeResolution::unknown();
                resolution
                    .metadata
                    .notes
                    .push(format!("Unknown base type: {}", base));
                return resolution;
            }
        };

        // Создаем синтетический конфигурационный тип
        TypeResolution {
            certainty: Certainty::Inferred(0.8), // Не полная уверенность без реальной конфигурации
            result: ResolutionResult::Concrete(ConcreteType::Configuration(ConfigurationType {
                kind,
                name: member.to_string(),
                attributes: vec![],
                tabular_sections: vec![],
            })),
            source: ResolutionSource::Inferred,
            metadata: ResolutionMetadata {
                file: Some(format!("{}:{}", prefix, member)),
                line: None,
                column: None,
                notes: vec![format!(
                    "Inferred {} type: {}.{}",
                    match kind {
                        MetadataKind::Catalog => "catalog",
                        MetadataKind::Document => "document",
                        MetadataKind::Enum => "enum",
                        MetadataKind::Register => "information register",
                        _ => "configuration object",
                    },
                    base,
                    member
                )],
            },
            active_facet: Some(crate::domain::types::FacetKind::Manager),
            available_facets: vec![
                crate::domain::types::FacetKind::Manager,
                crate::domain::types::FacetKind::Object,
                crate::domain::types::FacetKind::Reference,
            ],
        }
    }

    /// Поиск типов по запросу (для WebTypeService)
    pub fn search_types(&self, query: &str) -> Vec<String> {
        let completions = self.get_completions(query);
        completions.into_iter().map(|c| c.label).collect()
    }

    /// Получить базу данных синтакс-помощника для построения иерархий
    pub fn get_syntax_helper_database(
        &self,
    ) -> Option<()> {  // TODO: Return proper type after moving to shared
        // ✅ ИСПОЛЬЗУЕМ repository через SystemCoordinator вместо устаревшего CentralTypeSystem
        // В текущем контексте возвращаем None - база данных строится
        // из репозитория по мере необходимости

        // TODO: Реализовать создание SyntaxHelperDatabase из repository данных
        // Пока возвращаем None как признак того, что нужно строить базу
        // из get_all_platform_globals()
        None
    }

    /// Конвертировать RawTypeData в TypeResolution
    #[allow(dead_code)]
    fn convert_raw_data_to_type_resolution(&self, _raw_type: &RawTypeData) -> TypeResolution {
        // TODO: Re-implement after TypeSource is moved to shared
        TypeResolution::unknown()
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
