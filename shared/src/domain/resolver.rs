//! Application Layer: Type Resolver Service

use std::sync::Arc;
use anyhow::Result;
use crate::domain::repository::{CompletionItem, TypeRepository};
use crate::domain::types::{TypeResolution, RawTypeData};

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
    ) -> Result<TypeResolution> {
        // TODO: Implement expression resolution
        Ok(TypeResolution::unknown())
    }

    /// Получить все платформенные глобальные типы (для SystemCoordinator)
    pub fn get_all_platform_globals(&self) -> std::collections::HashMap<String, TypeResolution> {
        // TODO: Re-implement after TypeSource is moved to shared
        std::collections::HashMap::new()
    }

    /// Получить автодополнения для запроса (для WebTypeService)
    pub fn get_completions(&self, query: &str) -> Vec<CompletionItem> {
        let all_types = self.get_all_platform_globals();
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
    fn determine_completion_kind(&self, resolution: &TypeResolution) -> crate::domain::repository::CompletionKind {
        use crate::domain::types::{ConcreteType, ResolutionResult};

        match &resolution.result {
            ResolutionResult::Concrete(ConcreteType::Platform(_)) => crate::domain::repository::CompletionKind::Global,
            ResolutionResult::Concrete(ConcreteType::Configuration(config)) => match config.kind {
                crate::domain::types::MetadataKind::Catalog => crate::domain::repository::CompletionKind::Catalog,
                crate::domain::types::MetadataKind::Document => crate::domain::repository::CompletionKind::Document,
                crate::domain::types::MetadataKind::Enum => crate::domain::repository::CompletionKind::Enum,
                _ => crate::domain::repository::CompletionKind::Global,
            },
            _ => crate::domain::repository::CompletionKind::Global,
        }
    }

    /// Асинхронное разрешение выражений (для SystemCoordinator)
    pub async fn resolve_expression_async(&self, expression: &str) -> TypeResolution {
        let all_types = self.get_all_platform_globals();

        if let Some(resolution) = all_types.get(expression) {
            return resolution.clone();
        }

        if let Some((base, member)) = self.parse_member_access(expression) {
            return self.resolve_member_access(&base, &member);
        }
        
        TypeResolution::unknown()
    }

    /// Парсинг доступа к членам вида \"Base.Member\"
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

        let (kind, prefix) = match base {
            "Справочники" | "Catalogs" => (MetadataKind::Catalog, "Справочники"),
            "Документы" | "Documents" => (MetadataKind::Document, "Документы"),
            "Перечисления" | "Enums" => (MetadataKind::Enum, "Перечисления"),
            "РегистрыСведений" | "InformationRegisters" => {
                (MetadataKind::Register, "РегистрыСведений")
            }
            _ => {
                let mut resolution = TypeResolution::unknown();
                resolution
                    .metadata
                    .notes
                    .push(format!("Unknown base type: {}", base));
                return resolution;
            }
        };

        TypeResolution {
            certainty: Certainty::Inferred(0.8),
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