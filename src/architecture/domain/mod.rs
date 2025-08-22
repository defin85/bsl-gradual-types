//! Domain Layer - бизнес-логика идеальной архитектуры
//!
//! Центральная бизнес-логика для разрешения типов BSL
//! Принципы: Single Responsibility, правильные абстракции, честная неопределённость

use crate::core::types::PrimitiveType;
use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::adapters::config_parser_guided_discovery::ConfigurationGuidedParser;
use crate::architecture::data::stats::RepositoryStats;
use crate::architecture::data::{RawTypeData, TypeRepository, TypeSource};
use crate::core::types::{Certainty, ConcreteType, FacetKind, ResolutionResult, TypeResolution};
use crate::parser::common::Parser;
use crate::parser::tree_sitter_adapter::TreeSitterAdapter;

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
        expression: &str,
        context: &TypeContext,
        repository: &dyn TypeRepository,
    ) -> Result<TypeResolution>;

    /// Получить автодополнение для префикса
    async fn get_completions(
        &self,
        prefix: &str,
        context: &TypeContext,
        repository: &dyn TypeRepository,
    ) -> Result<Vec<CompletionItem>>;
}

/// Результат автодополнения
#[derive(Debug, Clone)]
pub struct CompletionItem {
    pub label: String,
    pub detail: Option<String>,
    pub documentation: Option<String>,
    pub kind: CompletionKind,
    pub insert_text: String,
}

/// Тип автодополнения
#[derive(Debug, Clone, PartialEq)]
pub enum CompletionKind {
    Variable,
    Function,
    Method,
    Property,
    Type,
    Keyword,
    Snippet,
}

impl TypeResolutionService {
    /// Создать новый сервис разрешения типов
    pub fn new(repository: Arc<dyn TypeRepository>) -> Self {
        let mut resolvers: Vec<Box<dyn TypeResolverAny>> = Vec::new();

        // Добавляем стандартные резолверы
        resolvers.push(Box::new(PlatformTypeResolver::new()));
        resolvers.push(Box::new(ConfigurationTypeResolver::new()));
        resolvers.push(Box::new(BslCodeResolver::new()));
        resolvers.push(Box::new(BuiltinTypeResolver::new()));
        resolvers.push(Box::new(ExpressionResolver::new()));

        Self {
            repository,
            resolvers,
            cache: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(RwLock::new(ResolutionMetrics::default())),
        }
    }

    /// Разрешить тип выражения (основной API)
    pub async fn resolve_expression(
        &self,
        expression: &str,
        context: &TypeContext,
    ) -> TypeResolution {
        let start_time = std::time::Instant::now();

        // Проверяем кеш
        let cache_key = format!("{}:{:?}", expression, context.current_facet);
        if let Some(cached) = self.get_from_cache(&cache_key).await {
            self.increment_cache_hit().await;
            return cached.resolution;
        }

        self.increment_cache_miss().await;

        // Пытаемся разрешить через резолверы
        for resolver in &self.resolvers {
            if resolver.can_resolve(expression) {
                match resolver
                    .resolve(expression, context, self.repository.as_ref())
                    .await
                {
                    Ok(resolution) => {
                        // Кешируем успешное разрешение
                        self.cache_resolution(&cache_key, &resolution).await;
                        self.record_resolution_time(start_time.elapsed()).await;
                        self.increment_successful_resolution().await;
                        return resolution;
                    }
                    Err(e) => {
                        warn!("⚠️ Resolver failed for '{}': {}", expression, e);
                        continue;
                    }
                }
            }
        }

        // Если никто не смог разрешить - честно возвращаем Unknown
        self.increment_failed_resolution().await;
        TypeResolution::unknown()
    }

    /// Получить автодополнение для префикса
    pub async fn get_completions(
        &self,
        prefix: &str,
        context: &TypeContext,
    ) -> Vec<CompletionItem> {
        let mut all_completions = Vec::new();

        // Собираем автодополнение от всех резолверов
        for resolver in &self.resolvers {
            if let Ok(completions) = resolver
                .get_completions(prefix, context, self.repository.as_ref())
                .await
            {
                all_completions.extend(completions);
            }
        }

        // Убираем дубликаты и сортируем
        all_completions.sort_by(|a, b| a.label.cmp(&b.label));
        all_completions.dedup_by(|a, b| a.label == b.label);

        all_completions
    }

    /// Поиск типов по запросу
    pub async fn search_types(&self, query: &str) -> Result<Vec<TypeSearchResult>> {
        // Ищем в репозитории
        let raw_types = self.repository.search_types(query).await?;

        // Конвертируем в результаты поиска с релевантностью
        let mut results = Vec::new();
        for raw_type in raw_types {
            let relevance = self.calculate_relevance(&raw_type.russian_name, query);
            results.push(TypeSearchResult {
                raw_data: raw_type.clone(),
                relevance_score: relevance,
                match_highlights: self.find_match_highlights(&raw_type.russian_name, query),
            });
        }

        // Сортируем по релевантности
        results.sort_by(|a, b| {
            b.relevance_score
                .partial_cmp(&a.relevance_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(results)
    }

    /// Получить метрики работы сервиса
    pub async fn get_metrics(&self) -> ResolutionMetrics {
        self.metrics.read().await.clone()
    }

    /// Получить статистику репозитория
    pub async fn get_stats(&self) -> RepositoryStats {
        self.repository.get_stats()
    }

    /// ПУБЛИЧНЫЙ МЕТОД: Получить все типы из репозитория
    pub async fn get_all_types(&self) -> Result<Vec<TypeSearchResult>> {
        let raw_types = self.repository.load_all_types().await?;

        let mut results = Vec::new();
        for raw_type in raw_types {
            results.push(TypeSearchResult {
                raw_data: raw_type,
                relevance_score: 1.0,
                match_highlights: Vec::new(),
            });
        }

        Ok(results)
    }

    // === ПРИВАТНЫЕ МЕТОДЫ ===

    async fn get_from_cache(&self, key: &str) -> Option<CachedTypeResolution> {
        let cache = self.cache.read().await;
        if let Some(cached) = cache.get(key) {
            // Проверяем TTL (например, 1 час)
            if cached.created_at.elapsed().as_secs() < 3600 {
                return Some(cached.clone());
            }
        }
        None
    }

    async fn cache_resolution(&self, key: &str, resolution: &TypeResolution) {
        let mut cache = self.cache.write().await;
        cache.insert(
            key.to_string(),
            CachedTypeResolution {
                resolution: resolution.clone(),
                created_at: std::time::Instant::now(),
                access_count: 1,
                last_accessed: std::time::Instant::now(),
            },
        );
    }

    async fn increment_cache_hit(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.cache_hits += 1;
    }

    async fn increment_cache_miss(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.cache_misses += 1;
    }

    async fn increment_successful_resolution(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.successful_resolutions += 1;
        metrics.total_resolutions += 1;
    }

    async fn increment_failed_resolution(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.failed_resolutions += 1;
        metrics.total_resolutions += 1;
    }

    async fn record_resolution_time(&self, duration: std::time::Duration) {
        let mut metrics = self.metrics.write().await;
        let time_ms = duration.as_millis() as f64;

        // Простое скользящее среднее
        if metrics.total_resolutions > 0 {
            metrics.average_resolution_time_ms = (metrics.average_resolution_time_ms
                * (metrics.total_resolutions - 1) as f64
                + time_ms)
                / metrics.total_resolutions as f64;
        } else {
            metrics.average_resolution_time_ms = time_ms;
        }
    }

    fn calculate_relevance(&self, type_name: &str, query: &str) -> f32 {
        let type_lower = type_name.to_lowercase();
        let query_lower = query.to_lowercase();

        // Точное совпадение
        if type_lower == query_lower {
            return 1.0;
        }

        // Начинается с запроса
        if type_lower.starts_with(&query_lower) {
            return 0.8;
        }

        // Содержит запрос
        if type_lower.contains(&query_lower) {
            return 0.6;
        }

        // Похожесть (простая реализация)
        let similarity = self.simple_similarity(&type_lower, &query_lower);
        similarity * 0.4
    }

    fn simple_similarity(&self, a: &str, b: &str) -> f32 {
        let max_len = a.len().max(b.len());
        if max_len == 0 {
            return 1.0;
        }

        let common_chars = a
            .chars()
            .zip(b.chars())
            .take_while(|(ch_a, ch_b)| ch_a == ch_b)
            .count();

        common_chars as f32 / max_len as f32
    }

    fn find_match_highlights(&self, text: &str, query: &str) -> Vec<TextSpan> {
        let mut highlights = Vec::new();
        let text_lower = text.to_lowercase();
        let query_lower = query.to_lowercase();

        if let Some(start) = text_lower.find(&query_lower) {
            highlights.push(TextSpan {
                start,
                end: start + query.len(),
                text: query.to_string(),
            });
        }

        highlights
    }
}

/// Результат поиска типов
#[derive(Debug, Clone)]
pub struct TypeSearchResult {
    pub raw_data: RawTypeData,
    pub relevance_score: f32,
    pub match_highlights: Vec<TextSpan>,
}

/// Выделенный фрагмент текста
#[derive(Debug, Clone)]
pub struct TextSpan {
    pub start: usize,
    pub end: usize,
    pub text: String,
}

// === СПЕЦИАЛИЗИРОВАННЫЕ РЕЗОЛВЕРЫ ===

/// Резолвер платформенных типов (Массив, ТаблицаЗначений)
pub struct PlatformTypeResolver {
    platform_types_cache: Arc<RwLock<HashMap<String, TypeResolution>>>,
}

impl PlatformTypeResolver {
    pub fn new() -> Self {
        Self {
            platform_types_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Инициализировать кеш из репозитория
    pub async fn initialize_cache(&self, repository: &dyn TypeRepository) -> Result<()> {
        println!("🔧 Инициализация PlatformTypeResolver кеша...");

        let platform_types = repository
            .load_types_filtered(&super::data::TypeFilter {
                source: Some(TypeSource::Platform {
                    version: "8.3".to_string(),
                }),
                ..Default::default()
            })
            .await?;

        let mut cache = self.platform_types_cache.write().await;
        for raw_type in platform_types {
            let resolution = self.convert_raw_to_resolution(&raw_type)?;
            cache.insert(raw_type.russian_name.clone(), resolution.clone());

            // Добавляем английское имя
            if !raw_type.english_name.is_empty() {
                cache.insert(raw_type.english_name.clone(), resolution);
            }
        }

        println!("✅ PlatformTypeResolver кеш готов: {} типов", cache.len());
        Ok(())
    }

    fn convert_raw_to_resolution(&self, raw_type: &RawTypeData) -> Result<TypeResolution> {
        Ok(TypeResolution {
            certainty: Certainty::Known, // Платформенные типы всегда известны
            result: ResolutionResult::Concrete(ConcreteType::Platform(
                crate::core::types::PlatformType {
                    name: raw_type.russian_name.clone(),
                    methods: raw_type
                        .methods
                        .iter()
                        .map(|m| crate::core::types::Method {
                            name: m.name.clone(),
                            is_function: m.is_function,
                            parameters: m
                                .parameters
                                .iter()
                                .map(|p| crate::core::types::Parameter {
                                    name: p.name.clone(),
                                    type_: Some(p.type_name.clone()),
                                    optional: p.is_optional,
                                    by_value: p.is_by_value,
                                })
                                .collect(),
                            return_type: m.return_type.clone(),
                        })
                        .collect(),
                    properties: raw_type
                        .properties
                        .iter()
                        .map(|p| crate::core::types::Property {
                            name: p.name.clone(),
                            type_: p.type_name.clone(),
                            readonly: p.is_readonly,
                        })
                        .collect(),
                },
            )),
            source: crate::core::types::ResolutionSource::Static,
            metadata: crate::core::types::ResolutionMetadata::default(),
            active_facet: None,
            available_facets: raw_type.available_facets.iter().map(|f| f.kind).collect(),
        })
    }
}

#[async_trait]
impl TypeResolver for PlatformTypeResolver {
    fn can_resolve(&self, expression: &str) -> bool {
        // Платформенные типы: Массив, ТаблицаЗначений, Структура, etc.
        let platform_patterns = [
            "Массив",
            "Array",
            "ТаблицаЗначений",
            "ValueTable",
            "Структура",
            "Structure",
            "Соответствие",
            "Map",
            "СписокЗначений",
            "ValueList",
            "ДеревоЗначений",
            "ValueTree",
        ];

        platform_patterns
            .iter()
            .any(|pattern| expression.contains(pattern))
    }

    async fn resolve(
        &self,
        expression: &str,
        _context: &TypeContext,
        _repository: &dyn TypeRepository,
    ) -> Result<TypeResolution> {
        let cache = self.platform_types_cache.read().await;

        // Простое разрешение по имени (можно расширить)
        let parts: Vec<&str> = expression.split('.').collect();
        let base_type = parts[0];

        if let Some(resolution) = cache.get(base_type) {
            Ok(resolution.clone())
        } else {
            Ok(TypeResolution::unknown())
        }
    }

    async fn get_completions(
        &self,
        prefix: &str,
        _context: &TypeContext,
        _repository: &dyn TypeRepository,
    ) -> Result<Vec<CompletionItem>> {
        let cache = self.platform_types_cache.read().await;
        let mut completions = Vec::new();

        for (name, _resolution) in cache.iter() {
            if name.to_lowercase().starts_with(&prefix.to_lowercase()) {
                completions.push(CompletionItem {
                    label: name.clone(),
                    detail: Some("Платформенный тип".to_string()),
                    documentation: Some(format!("Платформенный тип {}", name)),
                    kind: CompletionKind::Type,
                    insert_text: name.clone(),
                });
            }
        }

        Ok(completions)
    }
}

/// Резолвер конфигурационных типов (Справочники.Контрагенты)
pub struct ConfigurationTypeResolver {
    guided_parser: Arc<RwLock<Option<ConfigurationGuidedParser>>>,
}

impl ConfigurationTypeResolver {
    pub fn new() -> Self {
        Self {
            guided_parser: Arc::new(RwLock::new(None)),
        }
    }

    /// Инициализировать с конфигурацией
    pub async fn initialize_with_config(&self, config_path: &str) -> Result<()> {
        println!("🔧 Инициализация ConfigurationTypeResolver...");

        let mut parser = ConfigurationGuidedParser::new(config_path);
        let _config_types = parser.parse_with_configuration_guide()?;

        *self.guided_parser.write().await = Some(parser);

        println!("✅ ConfigurationTypeResolver готов");
        Ok(())
    }
}

#[async_trait]
impl TypeResolver for ConfigurationTypeResolver {
    fn can_resolve(&self, expression: &str) -> bool {
        // Конфигурационные типы: Справочники.*, Документы.*, etc.
        expression.contains("Справочники.")
            || expression.contains("Документы.")
            || expression.contains("Перечисления.")
            || expression.contains("РегистрыСведений.")
    }

    async fn resolve(
        &self,
        expression: &str,
        _context: &TypeContext,
        _repository: &dyn TypeRepository,
    ) -> Result<TypeResolution> {
        // TODO: Реализовать разрешение конфигурационных типов
        Ok(TypeResolution {
            certainty: Certainty::Inferred(0.8), // Не 100% уверены без полной конфигурации
            result: ResolutionResult::Dynamic,
            source: crate::core::types::ResolutionSource::Inferred,
            metadata: crate::core::types::ResolutionMetadata::default(),
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
        expression: &str,
        _context: &TypeContext,
        _repository: &dyn TypeRepository,
    ) -> Result<TypeResolution> {
        let parser_opt = self.parser.read().await;

        if let Some(parser) = parser_opt.as_ref() {
            // TODO: Использовать tree-sitter для анализа выражения
            // Пока возвращаем базовое разрешение
            Ok(TypeResolution {
                certainty: Certainty::Inferred(0.5),
                result: ResolutionResult::Dynamic,
                source: crate::core::types::ResolutionSource::Inferred,
                metadata: crate::core::types::ResolutionMetadata::default(),
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
            crate::core::types::PrimitiveType::String
        } else if expression.contains("Число") || expression.contains("Number") {
            crate::core::types::PrimitiveType::Number
        } else if expression.contains("Булево") || expression.contains("Boolean") {
            crate::core::types::PrimitiveType::Boolean
        } else if expression.contains("Дата") || expression.contains("Date") {
            crate::core::types::PrimitiveType::Date
        } else {
            return Ok(TypeResolution::unknown());
        };

        Ok(TypeResolution {
            certainty: Certainty::Known,
            result: ResolutionResult::Concrete(ConcreteType::Primitive(primitive_type)),
            source: crate::core::types::ResolutionSource::Static,
            metadata: crate::core::types::ResolutionMetadata::default(),
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
    use crate::architecture::data::{InMemoryTypeRepository, ParseMetadata, TypeSource};

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
            available_facets: vec![crate::core::types::Facet {
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
