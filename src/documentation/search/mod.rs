//! Система поиска и индексации документации

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

// TODO: Update imports after documentation system refactoring
// use super::core::hierarchy::{AvailabilityContext, DocumentationSourceType};
// use super::core::providers::DocumentationProvider;
use crate::domain::types::FacetKind;

pub mod fuzzy;
// Импорты провайдеров через re-exports

/// Система поиска и индексации документации
pub struct DocumentationSearchEngine {
    /// Полнотекстовый индекс
    fulltext_index: Arc<RwLock<FullTextIndex>>,

    /// Индексы по категориям
    pub category_indexes: Arc<RwLock<HashMap<String, CategoryIndex>>>,

    /// Индексы по фасетам
    facet_indexes: Arc<RwLock<HashMap<FacetKind, FacetIndex>>>,

    /// Кеш популярных запросов
    query_cache: Arc<RwLock<HashMap<String, CachedSearchResult>>>,

    /// Статистика поиска
    search_statistics: Arc<RwLock<SearchStatistics>>,

    /// Fuzzy matcher для нечеткого поиска
    fuzzy_matcher: Arc<RwLock<fuzzy::FuzzyMatcher>>,
}

/// Расширенный запрос поиска
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedSearchQuery {
    /// Текст запроса
    pub query: String,

    /// Фильтры
    pub filters: SearchFilters,

    /// Сортировка
    pub sort: SearchSort,

    /// Пагинация
    pub pagination: SearchPagination,

    /// Дополнительные опции
    pub options: SearchOptions,
}

/// Фильтры поиска
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchFilters {
    /// По источникам данных
    pub source_types: Vec<DocumentationSourceType>,

    /// По категориям
    pub categories: Vec<String>,

    /// По фасетам
    pub facets: Vec<FacetKind>,

    /// По доступности
    pub availability: Vec<AvailabilityContext>,

    /// По версии платформы
    pub version_range: Option<VersionRange>,

    /// Включить методы в результаты
    pub include_methods: bool,

    /// Включить свойства в результаты
    pub include_properties: bool,

    /// Включить примеры кода
    pub include_examples: bool,
}

/// Диапазон версий
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionRange {
    /// Минимальная версия
    pub min_version: String,

    /// Максимальная версия
    pub max_version: Option<String>,
}

/// Сортировка результатов
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchSort {
    /// Поле для сортировки
    pub field: SortField,

    /// Направление сортировки
    pub direction: SortDirection,

    /// Вторичная сортировка
    pub secondary: Option<Box<SearchSort>>,
}

/// Поле сортировки
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SortField {
    /// По релевантности (score)
    Relevance,

    /// По названию
    Name,

    /// По категории
    Category,

    /// По популярности
    Popularity,

    /// По дате создания
    CreationDate,

    /// По количеству методов
    MethodsCount,
}

/// Направление сортировки
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SortDirection {
    Ascending,
    Descending,
}

/// Пагинация
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchPagination {
    /// Размер страницы
    pub page_size: usize,

    /// Номер страницы (начиная с 0)
    pub page_number: usize,

    /// Максимальное количество результатов
    pub max_results: Option<usize>,
}

/// Опции поиска
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchOptions {
    /// Нечеткий поиск (fuzzy)
    pub fuzzy_search: bool,

    /// Поиск по синонимам
    pub include_synonyms: bool,

    /// Поиск в примерах кода
    pub search_in_examples: bool,

    /// Минимальный score результата
    pub min_score: f64,

    /// Подсветка найденных терминов
    pub highlight_matches: bool,
}

/// Результаты поиска
#[derive(Debug, Clone, Serialize)]
pub struct SearchResults {
    /// Найденные элементы
    pub items: Vec<SearchResultItem>,

    /// Общее количество найденного
    pub total_count: usize,

    /// Фасеты для фильтрации
    pub facets: Vec<SearchFacet>,

    /// Время поиска (мс)
    pub search_time_ms: u64,

    /// Предложения исправлений
    pub suggestions: Vec<String>,

    /// Связанные запросы
    pub related_queries: Vec<String>,

    /// Информация о пагинации
    pub pagination_info: PaginationInfo,
}

/// Элемент результата поиска
#[derive(Debug, Clone, Serialize)]
pub struct SearchResultItem {
    /// ID типа
    pub type_id: String,

    /// Название для отображения
    pub display_name: String,

    /// Описание
    pub description: String,

    /// Категория
    pub category: String,

    /// Тип источника
    pub source_type: DocumentationSourceType,

    /// Score релевантности
    pub relevance_score: f64,

    /// Выделенные фрагменты
    pub highlights: Vec<HighlightFragment>,

    /// Путь в иерархии
    pub breadcrumb: Vec<String>,
}

/// Выделенный фрагмент
#[derive(Debug, Clone, Serialize)]
pub struct HighlightFragment {
    /// Поле где найдено совпадение
    pub field: String,

    /// Текст с выделением
    pub highlighted_text: String,
}

/// Фасет для фильтрации
#[derive(Debug, Clone, Serialize)]
pub struct SearchFacet {
    /// Название фасета
    pub name: String,

    /// Значения фасета с количествами
    pub values: Vec<FacetValue>,
}

/// Значение фасета
#[derive(Debug, Clone, Serialize)]
pub struct FacetValue {
    /// Значение
    pub value: String,

    /// Количество элементов
    pub count: usize,

    /// Выбрано ли в текущем запросе
    pub selected: bool,
}

/// Информация о пагинации
#[derive(Debug, Clone, Serialize)]
pub struct PaginationInfo {
    /// Текущая страница
    pub current_page: usize,

    /// Всего страниц
    pub total_pages: usize,

    /// Есть ли следующая страница
    pub has_next: bool,

    /// Есть ли предыдущая страница
    pub has_previous: bool,

    /// Размер страницы
    pub page_size: usize,
}

/// Полнотекстовый индекс
#[derive(Debug, Default)]
pub struct FullTextIndex {
    /// Индекс слов → документы
    word_index: HashMap<String, Vec<IndexedDocument>>,

    /// Индекс документов
    document_index: HashMap<String, DocumentIndexEntry>,

    /// Настройки индексации
    indexing_config: IndexingConfig,
}

/// Индексированный документ
#[derive(Debug, Clone)]
pub struct IndexedDocument {
    /// ID документа
    pub document_id: String,

    /// Вес слова в документе
    pub weight: f32,

    /// Позиции слова
    pub positions: Vec<usize>,
}

/// Запись в индексе документа
#[derive(Debug, Clone)]
pub struct DocumentIndexEntry {
    /// ID документа
    pub document_id: String,

    /// Заголовок документа
    pub title: String,

    /// Полный текст
    pub content: String,

    /// Метаданные
    pub metadata: DocumentMetadata,
}

/// Метаданные документа для индексации
#[derive(Debug, Clone)]
pub struct DocumentMetadata {
    /// Тип документа
    pub document_type: String,

    /// Категория
    pub category: String,

    /// Теги
    pub tags: Vec<String>,

    /// Дата создания
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// Популярность (количество обращений)
    pub popularity_score: f64,
}

/// Конфигурация индексации
#[derive(Debug, Clone)]
pub struct IndexingConfig {
    /// Минимальная длина слова для индексации
    pub min_word_length: usize,

    /// Максимальная длина слова
    pub max_word_length: usize,

    /// Стоп-слова (исключаемые из индекса)
    pub stop_words: Vec<String>,

    /// Учитывать регистр
    pub case_sensitive: bool,

    /// Индексировать примеры кода
    pub index_code_examples: bool,
}

/// Индекс по категориям
#[derive(Debug, Default)]
pub struct CategoryIndex {
    /// Категория → типы
    pub category_to_types: HashMap<String, Vec<String>>,

    /// Тип → категория
    pub type_to_category: HashMap<String, String>,
}

/// Индекс по фасетам
#[derive(Debug, Default)]
pub struct FacetIndex {
    /// Фасет → типы
    facet_to_types: HashMap<FacetKind, Vec<String>>,

    /// Тип → фасеты
    type_to_facets: HashMap<String, Vec<FacetKind>>,
}

/// Кешированный результат поиска
#[derive(Debug, Clone)]
struct CachedSearchResult {
    /// Результаты
    results: SearchResults,

    /// Время создания
    created_at: chrono::DateTime<chrono::Utc>,

    /// Время истечения
    expires_at: chrono::DateTime<chrono::Utc>,

    /// Хеш запроса
    query_hash: String,
}

/// Статистика поиска
#[derive(Debug, Clone, Serialize)]
pub struct SearchStatistics {
    /// Размер полнотекстового индекса
    pub fulltext_index_size: usize,

    /// Всего индексированных документов
    pub total_indexed_documents: usize,

    /// Всего выполнено запросов
    pub total_queries: usize,

    /// Среднее время поиска (мс)
    pub average_search_time_ms: f64,

    /// Популярные запросы
    pub popular_queries: Vec<PopularQuery>,

    /// Использование памяти индексами (MB)
    pub index_memory_mb: f64,

    /// Статистика по типам запросов
    pub query_type_stats: HashMap<String, usize>,
}

/// Популярный запрос
#[derive(Debug, Clone, Serialize)]
pub struct PopularQuery {
    /// Текст запроса
    pub query: String,

    /// Количество выполнений
    pub execution_count: usize,

    /// Средний score результатов
    pub average_score: f64,
}

impl DocumentationSearchEngine {
    /// Создать новую систему поиска
    pub fn new() -> Self {
        Self {
            fulltext_index: Arc::new(RwLock::new(FullTextIndex::default())),
            category_indexes: Arc::new(RwLock::new(HashMap::new())),
            facet_indexes: Arc::new(RwLock::new(HashMap::new())),
            query_cache: Arc::new(RwLock::new(HashMap::new())),
            search_statistics: Arc::new(RwLock::new(SearchStatistics::default())),
            fuzzy_matcher: Arc::new(RwLock::new(fuzzy::FuzzyMatcher::default_for_bsl())),
        }
    }

    /// Построить индексы из провайдеров
    pub async fn build_indexes(
        &self,
        platform_provider: &crate::documentation::PlatformDocumentationProvider,
        _configuration_provider: &crate::documentation::ConfigurationDocumentationProvider,
    ) -> Result<()> {
        println!("🏗️ Начинаем построение индексов поиска...");

        // Получаем все типы из платформенного провайдера
        let platform_types = platform_provider.get_all_types().await?;
        println!(
            "📊 Получено {} платформенных типов для индексации",
            platform_types.len()
        );

        // Строим полнотекстовый индекс
        self.build_fulltext_index(&platform_types).await?;
        println!("✅ Полнотекстовый индекс построен");

        // Строим индексы по категориям
        self.build_category_indexes(&platform_types).await?;
        println!("✅ Индексы по категориям построены");

        // Строим индексы по фасетам
        self.build_facet_indexes(&platform_types).await?;
        println!("✅ Индексы по фасетам построены");

        println!("🎉 Все индексы успешно построены!");
        Ok(())
    }

    /// Выполнить поиск
    pub async fn search(&self, query: AdvancedSearchQuery) -> Result<SearchResults> {
        let start_time = std::time::Instant::now();

        println!("🔍 Выполняем поиск: '{}'", query.query);

        // Полнотекстовый поиск
        let mut result_documents = self.perform_fulltext_search(&query).await?;

        // Применяем фильтры
        result_documents = self.apply_filters(result_documents, &query.filters).await?;

        // Сортируем результаты
        result_documents = self.sort_results(result_documents, &query.sort).await?;

        // Применяем пагинацию
        let total_count = result_documents.len();
        let (paginated_results, pagination_info) =
            self.apply_pagination(result_documents, &query.pagination);

        // Конвертируем в SearchResultItem
        let search_items = self
            .convert_to_search_results(&paginated_results, &query)
            .await?;

        // Строим фасеты
        let facets = self.build_search_facets(&query).await?;

        let search_time_ms = start_time.elapsed().as_millis() as u64;

        // Обновляем статистику
        self.update_search_statistics(search_time_ms).await;

        println!(
            "✅ Поиск завершен: найдено {} результатов за {}ms",
            total_count, search_time_ms
        );

        Ok(SearchResults {
            items: search_items,
            total_count,
            facets,
            search_time_ms,
            suggestions: self.generate_suggestions(&query.query).await?,
            related_queries: self.generate_related_queries(&query.query).await?,
            pagination_info,
        })
    }

    /// Получить статистику поиска
    pub async fn get_statistics(&self) -> Result<SearchStatistics> {
        Ok(self.search_statistics.read().await.clone())
    }

    /// Получить предложения для автодополнения
    pub async fn get_suggestions(&self, partial_query: &str) -> Result<Vec<String>> {
        if partial_query.len() < 2 {
            return Ok(Vec::new());
        }

        let fulltext_index = self.fulltext_index.read().await;
        let mut suggestions = Vec::new();

        // Поиск в индексе слов
        for word in fulltext_index.word_index.keys() {
            if word
                .to_lowercase()
                .starts_with(&partial_query.to_lowercase())
            {
                suggestions.push(word.clone());
            }
        }

        // Поиск в заголовках документов
        for doc_entry in fulltext_index.document_index.values() {
            if doc_entry
                .title
                .to_lowercase()
                .contains(&partial_query.to_lowercase())
            {
                suggestions.push(doc_entry.title.clone());
            }
        }

        // Убираем дубликаты и сортируем
        suggestions.sort();
        suggestions.dedup();

        // Ограничиваем количество предложений
        Ok(suggestions.into_iter().take(10).collect())
    }

    /// Получить популярные запросы
    pub async fn get_popular_queries(&self, limit: usize) -> Result<Vec<PopularQuery>> {
        let stats = self.search_statistics.read().await;
        Ok(stats.popular_queries.iter().take(limit).cloned().collect())
    }

    // Приватные методы поиска

    /// Выполнить полнотекстовый поиск с fuzzy matching
    async fn perform_fulltext_search(&self, query: &AdvancedSearchQuery) -> Result<Vec<String>> {
        let fulltext_index = self.fulltext_index.read().await;
        let query_words = self.tokenize_text(&query.query);
        let mut document_scores: HashMap<String, f64> = HashMap::new();

        // Сначала точный поиск
        for word in &query_words {
            let normalized_word = word.to_lowercase();

            if let Some(indexed_docs) = fulltext_index.word_index.get(&normalized_word) {
                for indexed_doc in indexed_docs {
                    let score = document_scores
                        .entry(indexed_doc.document_id.clone())
                        .or_insert(0.0);
                    *score += indexed_doc.weight as f64;
                }
            }
        }

        // Если включен fuzzy поиск и мало результатов, выполняем fuzzy matching
        if query.options.fuzzy_search && document_scores.len() < 10 {
            let fuzzy_results = self
                .perform_fuzzy_search(&query_words, &fulltext_index)
                .await;

            for (doc_id, score) in fuzzy_results {
                let existing_score = document_scores.entry(doc_id).or_insert(0.0);
                *existing_score += score * 0.7; // Fuzzy результаты имеют меньший вес
            }
        }

        // Сортируем по релевантности
        let mut results: Vec<(String, f64)> = document_scores.into_iter().collect();
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        Ok(results.into_iter().map(|(doc_id, _score)| doc_id).collect())
    }

    /// Выполнить fuzzy поиск
    async fn perform_fuzzy_search(
        &self,
        query_words: &[String],
        fulltext_index: &FullTextIndex,
    ) -> HashMap<String, f64> {
        let mut fuzzy_matcher = self.fuzzy_matcher.write().await;
        let mut document_scores: HashMap<String, f64> = HashMap::new();

        // Получаем все слова из индекса
        let all_words: Vec<String> = fulltext_index.word_index.keys().cloned().collect();

        for query_word in query_words {
            // Находим fuzzy совпадения для каждого слова запроса
            let fuzzy_matches = fuzzy_matcher.find_matches(query_word, &all_words);

            for fuzzy_match in fuzzy_matches.iter().take(5) {
                // Берем топ-5 fuzzy совпадений
                if let Some(indexed_docs) = fulltext_index.word_index.get(&fuzzy_match.term) {
                    for indexed_doc in indexed_docs {
                        let score = document_scores
                            .entry(indexed_doc.document_id.clone())
                            .or_insert(0.0);
                        *score += (indexed_doc.weight as f64) * fuzzy_match.similarity;
                    }
                }
            }
        }

        document_scores
    }

    /// Применить фильтры к результатам
    async fn apply_filters(
        &self,
        mut documents: Vec<String>,
        filters: &SearchFilters,
    ) -> Result<Vec<String>> {
        if filters.categories.is_empty() && filters.facets.is_empty() {
            return Ok(documents);
        }

        // Фильтрация по категориям
        if !filters.categories.is_empty() {
            let category_indexes = self.category_indexes.read().await;
            documents.retain(|doc_id| {
                category_indexes.values().any(|index| {
                    index
                        .type_to_category
                        .get(doc_id)
                        .map(|category| {
                            filters
                                .categories
                                .iter()
                                .any(|filter_cat| category.contains(filter_cat))
                        })
                        .unwrap_or(false)
                })
            });
        }

        // Фильтрация по фасетам
        if !filters.facets.is_empty() {
            let facet_indexes = self.facet_indexes.read().await;
            documents.retain(|doc_id| {
                filters.facets.iter().any(|facet| {
                    facet_indexes
                        .get(facet)
                        .map(|index| {
                            index
                                .type_to_facets
                                .get(doc_id)
                                .map(|facets| facets.contains(facet))
                                .unwrap_or(false)
                        })
                        .unwrap_or(false)
                })
            });
        }

        Ok(documents)
    }

    /// Сортировать результаты
    async fn sort_results(&self, documents: Vec<String>, sort: &SearchSort) -> Result<Vec<String>> {
        // Пока простая сортировка по алфавиту
        let mut documents = documents;
        match sort.field {
            SortField::Name => {
                documents.sort();
                if matches!(sort.direction, SortDirection::Descending) {
                    documents.reverse();
                }
            }
            _ => {
                // Для других типов сортировки пока оставляем как есть
            }
        }
        Ok(documents)
    }

    /// Применить пагинацию
    fn apply_pagination(
        &self,
        documents: Vec<String>,
        pagination: &SearchPagination,
    ) -> (Vec<String>, PaginationInfo) {
        let total_count = documents.len();
        let page_size = pagination.page_size;
        let page_number = pagination.page_number;

        let total_pages = total_count.div_ceil(page_size);
        let start_index = page_number * page_size;
        let _end_index = (start_index + page_size).min(total_count);

        let paginated = if start_index < total_count {
            documents
                .into_iter()
                .skip(start_index)
                .take(page_size)
                .collect()
        } else {
            Vec::new()
        };

        let pagination_info = PaginationInfo {
            current_page: page_number,
            total_pages,
            has_next: page_number + 1 < total_pages,
            has_previous: page_number > 0,
            page_size,
        };

        (paginated, pagination_info)
    }

    /// Конвертировать результаты в SearchResultItem
    async fn convert_to_search_results(
        &self,
        document_ids: &[String],
        query: &AdvancedSearchQuery,
    ) -> Result<Vec<SearchResultItem>> {
        let fulltext_index = self.fulltext_index.read().await;
        let mut results = Vec::new();

        for doc_id in document_ids {
            if let Some(doc_entry) = fulltext_index.document_index.get(doc_id) {
                let highlights = if query.options.highlight_matches {
                    self.generate_highlights(&doc_entry.content, &query.query)
                } else {
                    Vec::new()
                };

                let search_item = SearchResultItem {
                    type_id: doc_id.clone(),
                    display_name: doc_entry.title.clone(),
                    description: doc_entry.content.clone(),
                    category: doc_entry.metadata.category.clone(),
                    source_type: DocumentationSourceType::Platform {
                        version: "8.3".to_string(),
                    },
                    relevance_score: 1.0, // TODO: Реальный расчет score
                    highlights,
                    breadcrumb: doc_entry
                        .metadata
                        .category
                        .split('/')
                        .map(|s| s.to_string())
                        .collect(),
                };

                results.push(search_item);
            }
        }

        Ok(results)
    }

    /// Сгенерировать подсветку совпадений
    fn generate_highlights(&self, content: &str, query: &str) -> Vec<HighlightFragment> {
        let query_words = self.tokenize_text(query);
        let mut highlights = Vec::new();

        for word in query_words {
            if content.to_lowercase().contains(&word.to_lowercase()) {
                highlights.push(HighlightFragment {
                    field: "content".to_string(),
                    highlighted_text: content.replace(&word, &format!("<mark>{}</mark>", word)),
                });
                break; // Только первое совпадение для простоты
            }
        }

        highlights
    }

    /// Построить фасеты для результатов поиска
    async fn build_search_facets(&self, _query: &AdvancedSearchQuery) -> Result<Vec<SearchFacet>> {
        let category_indexes = self.category_indexes.read().await;
        let facet_indexes = self.facet_indexes.read().await;
        let mut facets = Vec::new();

        // Фасет по категориям
        let mut category_values = Vec::new();
        for (category, index) in category_indexes.iter() {
            let count = index
                .category_to_types
                .get(category)
                .map(|types| types.len())
                .unwrap_or(0);
            category_values.push(FacetValue {
                value: category.clone(),
                count,
                selected: false,
            });
        }

        if !category_values.is_empty() {
            facets.push(SearchFacet {
                name: "Категории".to_string(),
                values: category_values,
            });
        }

        // Фасет по типам фасетов
        let mut facet_values = Vec::new();
        for (facet_kind, index) in facet_indexes.iter() {
            let count = index
                .facet_to_types
                .get(facet_kind)
                .map(|types| types.len())
                .unwrap_or(0);
            facet_values.push(FacetValue {
                value: format!("{:?}", facet_kind),
                count,
                selected: false,
            });
        }

        if !facet_values.is_empty() {
            facets.push(SearchFacet {
                name: "Фасеты".to_string(),
                values: facet_values,
            });
        }

        Ok(facets)
    }

    /// Сгенерировать предложения для исправления запроса
    async fn generate_suggestions(&self, query: &str) -> Result<Vec<String>> {
        // Простая реализация - предлагаем популярные термины
        let suggestions = vec![
            "ТаблицаЗначений".to_string(),
            "Справочники".to_string(),
            "Документы".to_string(),
            "СписокЗначений".to_string(),
        ];

        Ok(suggestions
            .into_iter()
            .filter(|s| s.to_lowercase().contains(&query.to_lowercase()))
            .collect())
    }

    /// Сгенерировать связанные запросы
    async fn generate_related_queries(&self, query: &str) -> Result<Vec<String>> {
        if query.contains("Таблица") {
            Ok(vec![
                "СписокЗначений".to_string(),
                "ДеревоЗначений".to_string(),
                "ТаблицаЗначений".to_string(),
            ])
        } else if query.contains("Справочник") {
            Ok(vec![
                "Справочники".to_string(),
                "СправочникМенеджер".to_string(),
                "СправочникОбъект".to_string(),
            ])
        } else {
            Ok(Vec::new())
        }
    }

    async fn update_search_statistics(&self, search_time_ms: u64) {
        let mut stats = self.search_statistics.write().await;
        stats.total_queries += 1;

        // Обновляем среднее время
        let total_time = stats.average_search_time_ms * (stats.total_queries - 1) as f64;
        stats.average_search_time_ms =
            (total_time + search_time_ms as f64) / stats.total_queries as f64;
    }

    /// Построить полнотекстовый индекс
    async fn build_fulltext_index(
        &self,
        types: &[super::core::hierarchy::TypeDocumentationFull],
    ) -> Result<()> {
        let mut fulltext_index = self.fulltext_index.write().await;

        for (i, type_doc) in types.iter().enumerate() {
            let document_id = format!("platform_{}", i);

            // Создаем запись в индексе документов
            let document_entry = DocumentIndexEntry {
                document_id: document_id.clone(),
                title: type_doc.russian_name.clone(),
                content: format!(
                    "{} {} {}",
                    type_doc.russian_name, type_doc.english_name, type_doc.description
                ),
                metadata: DocumentMetadata {
                    document_type: "PlatformType".to_string(),
                    category: type_doc.hierarchy_path.join("/"),
                    tags: type_doc.aliases.clone(),
                    created_at: chrono::Utc::now(),
                    popularity_score: 0.0,
                },
            };

            fulltext_index
                .document_index
                .insert(document_id.clone(), document_entry);

            // Индексируем слова
            self.index_words(
                &mut fulltext_index,
                &document_id,
                &type_doc.russian_name,
                3.0,
            )
            .await;
            self.index_words(
                &mut fulltext_index,
                &document_id,
                &type_doc.english_name,
                2.0,
            )
            .await;
            self.index_words(
                &mut fulltext_index,
                &document_id,
                &type_doc.description,
                1.0,
            )
            .await;

            // Индексируем альтернативные имена
            for alias in &type_doc.aliases {
                self.index_words(&mut fulltext_index, &document_id, alias, 2.5)
                    .await;
            }
        }

        println!(
            "📚 Индексировано {} документов в полнотекстовый индекс",
            types.len()
        );
        Ok(())
    }

    /// Индексировать слова в тексте
    async fn index_words(
        &self,
        index: &mut FullTextIndex,
        document_id: &str,
        text: &str,
        weight: f32,
    ) {
        let words = self.tokenize_text(text);

        for (position, word) in words.into_iter().enumerate() {
            if word.len() >= index.indexing_config.min_word_length
                && word.len() <= index.indexing_config.max_word_length
                && !index.indexing_config.stop_words.contains(&word)
            {
                let normalized_word = if index.indexing_config.case_sensitive {
                    word
                } else {
                    word.to_lowercase()
                };

                let indexed_doc = IndexedDocument {
                    document_id: document_id.to_string(),
                    weight,
                    positions: vec![position],
                };

                index
                    .word_index
                    .entry(normalized_word)
                    .or_default()
                    .push(indexed_doc);
            }
        }
    }

    /// Разбить текст на слова
    fn tokenize_text(&self, text: &str) -> Vec<String> {
        // Простая токенизация - разбиваем по пробелам и знакам препинания
        text.split_whitespace()
            .flat_map(|word| {
                word.split(&['.', ',', ';', ':', '!', '?', '(', ')', '[', ']', '{', '}'])
                    .filter(|w| !w.is_empty())
                    .map(|w| w.trim().to_string())
            })
            .filter(|w| !w.is_empty())
            .collect()
    }

    /// Построить индексы по категориям
    async fn build_category_indexes(
        &self,
        types: &[super::core::hierarchy::TypeDocumentationFull],
    ) -> Result<()> {
        let mut category_indexes = self.category_indexes.write().await;

        for (i, type_doc) in types.iter().enumerate() {
            let document_id = format!("platform_{}", i);
            let category_path = type_doc.hierarchy_path.join("/");

            let category_index = category_indexes
                .entry(category_path.clone())
                .or_insert_with(CategoryIndex::default);

            category_index
                .category_to_types
                .entry(category_path.clone())
                .or_insert_with(Vec::new)
                .push(document_id.clone());

            category_index
                .type_to_category
                .insert(document_id, category_path);
        }

        println!(
            "🗂️ Построено {} индексов по категориям",
            category_indexes.len()
        );
        Ok(())
    }

    /// Построить индексы по фасетам
    async fn build_facet_indexes(
        &self,
        types: &[super::core::hierarchy::TypeDocumentationFull],
    ) -> Result<()> {
        let mut facet_indexes = self.facet_indexes.write().await;

        for (i, type_doc) in types.iter().enumerate() {
            let document_id = format!("platform_{}", i);

            // Индексируем по фасетам типа
            for facet in &type_doc.available_facets {
                let facet_index = facet_indexes
                    .entry(*facet)
                    .or_insert_with(FacetIndex::default);

                facet_index
                    .facet_to_types
                    .entry(*facet)
                    .or_insert_with(Vec::new)
                    .push(document_id.clone());

                facet_index
                    .type_to_facets
                    .entry(document_id.clone())
                    .or_insert_with(Vec::new)
                    .push(*facet);
            }
        }

        println!("🏷️ Построено {} индексов по фасетам", facet_indexes.len());
        Ok(())
    }
}

#[allow(clippy::derivable_impls)]
impl Default for AdvancedSearchQuery {
    fn default() -> Self {
        Self {
            query: String::new(),
            filters: SearchFilters::default(),
            sort: SearchSort::default(),
            pagination: SearchPagination::default(),
            options: SearchOptions::default(),
        }
    }
}

impl Default for SearchFilters {
    fn default() -> Self {
        Self {
            source_types: Vec::new(),
            categories: Vec::new(),
            facets: Vec::new(),
            availability: Vec::new(),
            version_range: None,
            include_methods: true,
            include_properties: true,
            include_examples: false,
        }
    }
}

impl Default for SearchSort {
    fn default() -> Self {
        Self {
            field: SortField::Relevance,
            direction: SortDirection::Descending,
            secondary: None,
        }
    }
}

impl Default for SearchPagination {
    fn default() -> Self {
        Self {
            page_size: 20,
            page_number: 0,
            max_results: Some(1000),
        }
    }
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            fuzzy_search: true,
            include_synonyms: true,
            search_in_examples: false,
            min_score: 0.1,
            highlight_matches: true,
        }
    }
}

impl Default for SearchStatistics {
    fn default() -> Self {
        Self {
            fulltext_index_size: 0,
            total_indexed_documents: 0,
            total_queries: 0,
            average_search_time_ms: 0.0,
            popular_queries: Vec::new(),
            index_memory_mb: 0.0,
            query_type_stats: HashMap::new(),
        }
    }
}

impl Default for IndexingConfig {
    fn default() -> Self {
        Self {
            min_word_length: 2,
            max_word_length: 50,
            stop_words: vec![
                // Русские стоп-слова
                "и".to_string(),
                "в".to_string(),
                "на".to_string(),
                "с".to_string(),
                "для".to_string(),
                "к".to_string(),
                "от".to_string(),
                "по".to_string(),
                "или".to_string(),
                "но".to_string(),
                "как".to_string(),
                "что".to_string(),
                "это".to_string(),
                "то".to_string(),
                "же".to_string(),
                "не".to_string(),
                // Английские стоп-слова
                "a".to_string(),
                "an".to_string(),
                "the".to_string(),
                "is".to_string(),
                "at".to_string(),
                "for".to_string(),
                "to".to_string(),
                "of".to_string(),
                "by".to_string(),
                "in".to_string(),
                "on".to_string(),
                "with".to_string(),
                "and".to_string(),
                "or".to_string(),
                "but".to_string(),
                "as".to_string(),
            ],
            case_sensitive: false,
            index_code_examples: true,
        }
    }
}

// Реализация значения по умолчанию для корректности API с new()
impl Default for DocumentationSearchEngine {
    fn default() -> Self {
        Self::new()
    }
}
