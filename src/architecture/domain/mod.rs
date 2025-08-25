//! Domain Layer - бизнес-логика идеальной архитектуры
//!
//! Центральная бизнес-логика для разрешения типов BSL
//! Принципы: Single Responsibility, правильные абстракции, честная неопределённость

use crate::domain::types::PrimitiveType;
use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::data::loaders::config_parser_guided_discovery::ConfigurationGuidedParser;
use crate::unified::data::stats::RepositoryStats;
use crate::unified::data::{RawTypeData, TypeRepository, TypeSource};
use crate::domain::types::{Certainty, ConcreteType, FacetKind, ResolutionResult, TypeResolution};
use crate::parsing::bsl::tree_sitter_adapter::TreeSitterAdapter;

/// Центральный сервис разрешения типов
///
/// Единая точка бизнес-логики для всех операций с типами BSL
pub struct TypeResolutionService {
    /// Репозиторий данных (Data Layer)
    repository: Arc<dyn TypeRepository>,

    /// Резолверы для разных типов выражений
    // Храним резолверы как TypeResolverAny для возможности downcast в тестах
    resolvers: Vec<Box<dyn TypeResolverAny>>,

    /// Кеш разрешений для производительности
    cache: Arc<RwLock<HashMap<String, CachedTypeResolution>>>,

    /// Статистика работы сервиса
    metrics: Arc<RwLock<ResolutionMetrics>>,
}

/// Кешированное разрешение типа
#[derive(Debug, Clone)]
pub struct CachedTypeResolution {
    pub resolution: TypeResolution,
    pub created_at: std::time::Instant,
    pub access_count: u64,
    pub last_accessed: std::time::Instant,
}

/// Метрики работы сервиса разрешения
#[derive(Debug, Clone, Default)]
pub struct ResolutionMetrics {
    pub total_resolutions: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub average_resolution_time_ms: f64,
    pub successful_resolutions: u64,
    pub failed_resolutions: u64,
}

/// Контекст для разрешения типов
#[derive(Debug, Clone)]
pub struct TypeContext {
    pub file_path: Option<String>,
    pub line: Option<u32>,
    pub column: Option<u32>,
    pub local_variables: HashMap<String, TypeResolution>,
    pub current_function: Option<String>,
    pub current_facet: Option<FacetKind>,
}

/// Абстракция резолвера типов
#[async_trait]
pub trait TypeResolver: Send + Sync {
    /// Может ли резолвер обработать данное выражение
    fn can_resolve(&self, expression: &str) -> bool;

    /// Разрешить тип выражения
    async fn resolve(
        &self,
        _expression: &str,
        _context: &TypeContext,
        _repository: &dyn TypeRepository,
    ) -> Result<TypeResolution> {
        // TODO: Реализовать разрешение конфигурационных типов
        Ok(TypeResolution {
           certainty: Certainty::Inferred(0.8), // Не 100% уверены без полной конфигурации
           result: ResolutionResult::Dynamic,
           source: crate::domain::types::ResolutionSource::Inferred,
           metadata: crate::domain::types::ResolutionMetadata::default(),
           active_facet: Some(FacetKind::Manager),
           available_facets: vec![FacetKind::Manager, FacetKind::Object, FacetKind::Reference],
       })
    }

    async fn get_completions(
        &self,
        _prefix: &str,
        _context: &TypeContext,
        _repository: &dyn TypeRepository,
    ) -> Result<Vec<CompletionItem>> {
        // TODO: Реализовать автодополнение конфигурационных типов
        Ok(Vec::new())
    }
}

/// Резолвер BSL кода (с tree-sitter парсером!)
pub struct BslCodeResolver {
    parser: Arc<RwLock<Option<TreeSitterAdapter>>>,
}

impl BslCodeResolver {
    pub fn new() -> Self {
        Self {
            parser: Arc::new(RwLock::new(None)),
        }
    }

    /// Инициализировать tree-sitter парсер
    pub async fn initialize_parser(&self) -> Result<()> {
        info!("🔧 Инициализация BslCodeResolver с tree-sitter...");

        match TreeSitterAdapter::new() {
            Ok(adapter) => {
                *self.parser.write().await = Some(adapter);
                info!("✅ BslCodeResolver готов с tree-sitter-bsl");
                Ok(())
            }
            Err(e) => {
                warn!("⚠️ Tree-sitter недоступен: {}", e);
                Err(e)
            }
        }
    }
}

#[async_trait]
impl TypeResolver for BslCodeResolver {
    fn can_resolve(&self, expression: &str) -> bool {
        // BSL выражения: переменные, вызовы функций
        expression.chars().any(|c| c.is_alphabetic())
            && !expression.contains("Справочники.")
            && !expression.contains("Документы.")
    }

    async fn resolve(
        &self,
        _expression: &str,
        _context: &TypeContext,
        _repository: &dyn TypeRepository,
    ) -> Result<TypeResolution> {
        let parser_opt = self.parser.read().await;

        if let Some(_parser) = parser_opt.as_ref() {
            // TODO: Использовать tree-sitter для анализа выражения
            // Пока возвращаем базовое разрешение
            Ok(TypeResolution {
                certainty: Certainty::Inferred(0.5),
                result: ResolutionResult::Dynamic,
            source: crate::domain::types::ResolutionSource::Inferred,
            metadata: crate::domain::types::ResolutionMetadata::default(),
                active_facet: None,
                available_facets: Vec::new(),
            })
        } else {
            Ok(TypeResolution::unknown())
        }
    }

    async fn get_completions(
        &self,
        _prefix: &str,
        _context: &TypeContext,
        _repository: &dyn TypeRepository,
    ) -> Result<Vec<CompletionItem>> {
        // TODO: Реализовать автодополнение на основе BSL парсинга
        Ok(Vec::new())
    }
}

/// Резолвер встроенных типов (Строка, Число, Булево)
pub struct BuiltinTypeResolver;

impl BuiltinTypeResolver {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl TypeResolver for BuiltinTypeResolver {
    fn can_resolve(&self, expression: &str) -> bool {
        let builtins = [
            "Строка",
            "String",
            "Число",
            "Number",
            "Булево",
            "Boolean",
            "Дата",
            "Date",
        ];
        builtins.iter().any(|builtin| expression.contains(builtin))
    }

    async fn resolve(
        &self,
        expression: &str,
        _context: &TypeContext,
        _repository: &dyn TypeRepository,
    ) -> Result<TypeResolution> {
        let primitive_type = if expression.contains("Строка") || expression.contains("String")
        {
            crate::domain::types::PrimitiveType::String
        } else if expression.contains("Число") || expression.contains("Number") {
            crate::domain::types::PrimitiveType::Number
        } else if expression.contains("Булево") || expression.contains("Boolean") {
            crate::domain::types::PrimitiveType::Boolean
        } else if expression.contains("Дата") || expression.contains("Date") {
            crate::domain::types::PrimitiveType::Date
        } else {
            return Ok(TypeResolution::unknown());
        };

        Ok(TypeResolution {
            certainty: Certainty::Known,
            result: ResolutionResult::Concrete(ConcreteType::Primitive(primitive_type)),
            source: crate::domain::types::ResolutionSource::Static,
            metadata: crate::domain::types::ResolutionMetadata::default(),
            active_facet: None,
            available_facets: Vec::new(),
        })
    }

    async fn get_completions(
        &self,
        prefix: &str,
        _context: &TypeContext,
        _repository: &dyn TypeRepository,
    ) -> Result<Vec<CompletionItem>> {
        let builtins = [
            ("Строка", "String"),
            ("Число", "Number"),
            ("Булево", "Boolean"),
            ("Дата", "Date"),
        ];

        let mut completions = Vec::new();
        for (ru, en) in &builtins {
            if ru.to_lowercase().starts_with(&prefix.to_lowercase())
                || en.to_lowercase().starts_with(&prefix.to_lowercase())
            {
                completions.push(CompletionItem {
                    label: ru.to_string(),
                    detail: Some("Встроенный тип".to_string()),
                    documentation: Some(format!("Примитивный тип {}", ru)),
                    kind: CompletionKind::Type,
                    insert_text: ru.to_string(),
                });
            }
        }

        Ok(completions)
    }
}

/// Резолвер сложных выражений (объект.метод().свойство)
pub struct ExpressionResolver;

impl ExpressionResolver {
    pub fn new() -> Self {
        Self
    }

    fn clean_segment(seg: &str) -> (String, bool) {
        // Убираем пробелы и хвостовые скобки для вызовов методов
        let s = seg.trim();
        let is_call = s.ends_with(')');
        // Отрезаем часть после '('
        let name = if let Some(idx) = s.find('(') {
            &s[..idx]
        } else {
            s
        };
        (name.trim().to_string(), is_call)
    }

    fn primitive_from_name(name: &str) -> Option<ConcreteType> {
        let n = name.trim();
        if n.eq_ignore_ascii_case("Строка") || n.eq_ignore_ascii_case("String") {
            return Some(ConcreteType::Primitive(PrimitiveType::String));
        }
        if n.eq_ignore_ascii_case("Число") || n.eq_ignore_ascii_case("Number") {
            return Some(ConcreteType::Primitive(PrimitiveType::Number));
        }
        if n.eq_ignore_ascii_case("Булево") || n.eq_ignore_ascii_case("Boolean") {
            return Some(ConcreteType::Primitive(PrimitiveType::Boolean));
        }
        if n.eq_ignore_ascii_case("Дата") || n.eq_ignore_ascii_case("Date") {
            return Some(ConcreteType::Primitive(PrimitiveType::Date));
        }
        None
    }

    async fn resolve_type_by_name(
        &self,
        name: &str,
        repository: &dyn TypeRepository,
    ) -> Option<TypeResolution> {
        if let Some(ct) = Self::primitive_from_name(name) {
            return Some(TypeResolution::known(ct));
        }
        if name.is_empty() {
            return None;
        }
        let candidates = repository.search_types(name).await.ok()?;
        for raw in candidates {
            if raw.russian_name == name || raw.english_name == name {
                let mut res = TypeResolution::from_raw_data(&raw);
                // Отмечаем как выведенный, т.к. получен из контекста выражения
                res.certainty = Certainty::Inferred(0.8);
                return Some(res);
            }
        }
        None
    }
}

#[async_trait]
impl TypeResolver for ExpressionResolver {
    fn can_resolve(&self, expression: &str) -> bool {
        // Сложные выражения с точками и скобками ИЛИ простая точечная навигация
        expression.contains('.')
    }

    async fn resolve(
        &self,
        expression: &str,
        _context: &TypeContext,
        repository: &dyn TypeRepository,
    ) -> Result<TypeResolution> {
        // Базовый разбор точечных выражений: Base.Segment1.Segment2...
        let mut parts = expression
            .split('.')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty());
        let base = match parts.next() {
            Some(b) => b,
            None => return Ok(TypeResolution::unknown()),
        };

        // Разрешаем базовый тип
        let mut current = match self.resolve_type_by_name(base, repository).await {
            Some(res) => res,
            None => return Ok(TypeResolution::unknown()),
        };

        // Навигация по свойствам/методам
        for seg in parts {
            let (name, is_call) = Self::clean_segment(seg);
            // Получаем описание текущего типа в виде RawTypeData
            let raw = current.to_raw_data();

            // Пытаемся найти метод
            let method_opt = raw
                .methods
                .iter()
                .find(|m| m.name.to_lowercase() == name.to_lowercase());
            if let Some(m) = method_opt {
                // Метод. Пытаемся вывести тип результата
                if let Some(rt) = &m.return_type {
                    if let Some(next) = self.resolve_type_by_name(rt, repository).await {
                        current = TypeResolution {
                            certainty: Certainty::Inferred(0.7),
                            ..next
                        };
                        continue;
                    }
                }
                // Нет информации о типе возвращаемого значения
                current = TypeResolution::unknown();
                break;
            }

            // Если не метод (или не найден), пробуем как свойство
            let prop_opt = raw
                .properties
                .iter()
                .find(|p| p.name.to_lowercase() == name.to_lowercase());
            if let Some(p) = prop_opt {
                if let Some(next) = self.resolve_type_by_name(&p.type_name, repository).await {
                    current = TypeResolution {
                        certainty: Certainty::Inferred(0.9),
                        ..next
                    };
                    continue;
                } else if let Some(ct) = Self::primitive_from_name(&p.type_name) {
                    current = TypeResolution::known(ct);
                    continue;
                }
                current = TypeResolution::unknown();
                break;
            }

            // Если сегмент выглядит как вызов, но метод не найден — считаем неизвестным
            if is_call {
                current = TypeResolution::unknown();
                break;
            }

            // Не нашли ни метода, ни свойства
            current = TypeResolution::unknown();
            break;
        }

        Ok(current)
    }

    async fn get_completions(
        &self,
        prefix: &str,
        _context: &TypeContext,
        repository: &dyn TypeRepository,
    ) -> Result<Vec<CompletionItem>> {
        let mut out = Vec::new();
        // Если нет точки — предлагаем базовые типы по первому сегменту
        if !prefix.contains('.') {
            let first = prefix.trim();
            if first.is_empty() {
                return Ok(Vec::new());
            }
            for raw in repository.search_types(first).await.unwrap_or_default() {
                if raw
                    .russian_name
                    .to_lowercase()
                    .starts_with(&first.to_lowercase())
                    || raw
                        .english_name
                        .to_lowercase()
                        .starts_with(&first.to_lowercase())
                {
                    out.push(CompletionItem {
                        label: raw.russian_name.clone(),
                        detail: Some("Тип".to_string()),
                        documentation: Some(raw.documentation.clone()),
                        kind: CompletionKind::Type,
                        insert_text: raw.russian_name,
                    });
                }
            }
            return Ok(out);
        }

        // Иначе пытаемся предложить члены типа после последней точки
        let mut segs: Vec<&str> = prefix.split('.').collect();
        let last = segs.pop().unwrap_or("").trim();
        let base_expr = segs.join(".");

        // Разрешаем тип до последней точки
        let resolved = self
            .resolve(&base_expr, _context, repository)
            .await
            .unwrap_or(TypeResolution::unknown());
        if matches!(resolved.certainty, Certainty::Unknown) {
            return Ok(Vec::new());
        }
        let raw = resolved.to_raw_data();
        let last_lower = last.to_lowercase();

        // Предлагаем методы
        for m in &raw.methods {
            if last.is_empty() || m.name.to_lowercase().starts_with(&last_lower) {
                out.push(CompletionItem {
                    label: m.name.clone(),
                    detail: Some("Метод".to_string()),
                    documentation: Some(m.documentation.clone()),
                    kind: CompletionKind::Method,
                    insert_text: format!("{}()", m.name),
                });
            }
        }
        // Предлагаем свойства
        for p in &raw.properties {
            if last.is_empty() || p.name.to_lowercase().starts_with(&last_lower) {
                out.push(CompletionItem {
                    label: p.name.clone(),
                    detail: Some("Свойство".to_string()),
                    documentation: Some(p.description.clone()),
                    kind: CompletionKind::Property,
                    insert_text: p.name.clone(),
                });
            }
        }

        // Сортируем и убираем дубликаты
        out.sort_by(|a, b| a.label.cmp(&b.label));
        out.dedup_by(|a, b| a.label == b.label);
        Ok(out)
    }
}

// === TYPE CHECKER SERVICE (минимальный) ===

/// Минимальный сервис проверки совместимости типов (Domain)
pub struct TypeCheckerService;

impl TypeCheckerService {
    pub fn new() -> Self {
        Self
    }

    /// Базовая проверка присваивания: конкретные типы должны совпадать по дисриминатору
    pub fn is_assignment_compatible(&self, from: &TypeResolution, to: &TypeResolution) -> bool {
        match (&from.result, &to.result) {
            (ResolutionResult::Concrete(cf), ResolutionResult::Concrete(ct)) => {
                std::mem::discriminant(cf) == std::mem::discriminant(ct)
            }
            // Разрешаем неизвестные/динамические на данном этапе
            _ => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unified::data::{InMemoryTypeRepository, ParseMetadata, TypeSource};

    #[tokio::test]
    async fn test_type_resolution_service() {
        // Создаём репозиторий с тестовыми данными
        let repo = Arc::new(InMemoryTypeRepository::new());

        let test_types = vec![RawTypeData {
            id: "array".to_string(),
            russian_name: "Массив".to_string(),
            english_name: "Array".to_string(),
            source: TypeSource::Platform {
                version: "8.3".to_string(),
            },
            category_path: vec!["Коллекции".to_string()],
            methods: vec![],
            properties: vec![],
            documentation: "Коллекция упорядоченных значений".to_string(),
            examples: vec![],
            available_facets: vec![crate::domain::types::Facet {
                kind: FacetKind::Object,
                methods: vec![],
                properties: vec![],
            }],
            parse_metadata: ParseMetadata {
                file_path: "test.xml".to_string(),
                line: 0,
                column: 0,
            },
        }];

        repo.save_types(test_types).await.unwrap();

        // Создаём сервис
        let service = TypeResolutionService::new(repo);

        // Инициализируем резолверы
        if let Some(platform_resolver) = service
            .resolvers
            .iter()
            .find_map(|r| r.as_any().downcast_ref::<PlatformTypeResolver>())
        {
            platform_resolver
                .initialize_cache(service.repository.as_ref())
                .await
                .unwrap();
        }

        // Тестируем разрешение типов
        let context = TypeContext {
            file_path: None,
            line: None,
            column: None,
            local_variables: HashMap::new(),
            current_function: None,
            current_facet: None,
        };

        let resolution = service.resolve_expression("Массив", &context).await;
        assert_eq!(resolution.certainty, Certainty::Known);

        // Тестируем автодополнение
        let completions = service.get_completions("Масс", &context).await;
        assert!(!completions.is_empty());

        // Тестируем поиск
        let search_results = service.search_types("массив").await.unwrap();
        assert_eq!(search_results.len(), 1);
    }
}

// Хак для тестирования downcast
trait TypeResolverAny: TypeResolver {
    fn as_any(&self) -> &dyn std::any::Any;
}

impl TypeResolverAny for PlatformTypeResolver {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl TypeResolverAny for ConfigurationTypeResolver {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl TypeResolverAny for BslCodeResolver {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl TypeResolverAny for BuiltinTypeResolver {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl TypeResolverAny for ExpressionResolver {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Элемент автодополнения доменного слоя
#[derive(Debug, Clone)]
pub struct CompletionItem {
    pub label: String,
    pub detail: Option<String>,
    pub documentation: Option<String>,
    pub kind: CompletionKind,
    pub insert_text: String,
}

/// Вид элемента автодополнения доменного слоя
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CompletionKind {
    // Общие виды
    Variable,
    Function,
    Method,
    Property,
    Type,
    Keyword,
    Snippet,
    // Платформенные/конфигурационные виды
    Global,
    Catalog,
    Document,
    Enum,
    GlobalFunction,
}

impl From<crate::core::platform_resolver::CompletionItem> for CompletionItem {
    fn from(src: crate::core::platform_resolver::CompletionItem) -> Self {
        let kind = match src.kind {
            crate::core::platform_resolver::CompletionKind::Global => CompletionKind::Global,
            crate::core::platform_resolver::CompletionKind::Catalog => CompletionKind::Catalog,
            crate::core::platform_resolver::CompletionKind::Document => CompletionKind::Document,
            crate::core::platform_resolver::CompletionKind::Enum => CompletionKind::Enum,
            crate::core::platform_resolver::CompletionKind::Method => CompletionKind::Method,
            crate::core::platform_resolver::CompletionKind::Property => CompletionKind::Property,
            crate::core::platform_resolver::CompletionKind::GlobalFunction => {
                CompletionKind::GlobalFunction
            }
            crate::core::platform_resolver::CompletionKind::Variable => CompletionKind::Variable,
            crate::core::platform_resolver::CompletionKind::Function => CompletionKind::Function,
        };

        CompletionItem {
            label: src.label.clone(),
            detail: src.detail,
            documentation: src.documentation,
            kind,
            // По умолчанию вставляем метку
            insert_text: src.label,
        }
    }
}

impl From<crate::core::resolution::Completion> for CompletionItem {
    fn from(src: crate::core::resolution::Completion) -> Self {
        let kind = match src.kind {
            crate::core::resolution::CompletionKind::Type => CompletionKind::Type,
            crate::core::resolution::CompletionKind::Method => CompletionKind::Method,
            crate::core::resolution::CompletionKind::Property => CompletionKind::Property,
            crate::core::resolution::CompletionKind::Function => CompletionKind::Function,
        };
        CompletionItem {
            label: src.label.clone(),
            detail: src.detail,
            documentation: None,
            kind,
            insert_text: src.label,
        }
    }
}
