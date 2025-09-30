//! Application Layer: Type System Service
//! 
//! Unified API для всех типов клиентов (LSP, Web, CLI)
//! Phase 4: API Unification - объединяет LspTypeService + WebTypeService + AnalysisService

use std::sync::Arc;
use std::collections::HashMap;
use anyhow::Result;
use tracing::info;

use bsl_shared::engine::AnalysisEngine;
use bsl_shared::domain::types::{TypeResolution, ResolutionResult};
use bsl_shared::domain::{CompletionItem, CompletionKind};
use crate::system::{AnalysisCache, AnalysisResult, ParserCoordinator};
use crate::application::TypeInferenceService;

/// Унифицированный сервис системы типов для Application Layer
///
/// Phase 4: Заменяет LspTypeService + WebTypeService + AnalysisService
/// единым unified API для всех презентационных слоев
pub struct TypeSystemService {
    // Application Layer: Type Inference Service для high-level операций
    inference_service: Arc<TypeInferenceService>,

    // System Layer компоненты
    cache: Arc<AnalysisCache>,
    parser: Arc<ParserCoordinator>,
}

impl TypeSystemService {
    /// Конструктор согласно архитектурной диаграмме
    pub fn new(
        analysis_engine: Arc<AnalysisEngine>,
        cache: Arc<AnalysisCache>,
        parser: Arc<ParserCoordinator>,
    ) -> Self {
        // Создаем TypeInferenceService на основе AnalysisEngine
        let resolver = analysis_engine.get_resolver();
        let repository = analysis_engine.get_repository();
        let inference_service = Arc::new(TypeInferenceService::new(resolver, repository));

        Self {
            inference_service,
            cache,
            parser,
        }
    }

    pub fn initialize(&self) -> Result<()> {
        info!("🎭 TypeSystemService инициализирован (Phase 4: Unified API)");
        Ok(())
    }

    // === UNIFIED API FOR ALL CLIENTS ===

    /// Получить все платформенные глобальные типы (для Web API)
    pub fn get_all_platform_globals(&self) -> std::collections::HashMap<String, TypeResolution> {
        // Делегируем в TypeInferenceService (Application Layer)
        self.inference_service.get_all_platform_globals()
    }

    /// Phase 5: Получить все типы с преобразованием в DTO (Web API)
    pub fn get_all_types_as_dto(
        &self,
        limit: usize,
        offset: usize,
    ) -> bsl_shared::api::dtos::AnalysisResultDto {
        use bsl_shared::api::dtos::{AnalysisResultDto, TypeDto, CategoryDto, MetricsDto, PaginationDto, UnionComponentDto};
        use bsl_shared::domain::types::{Certainty, ResolutionResult};

        // 1. Получаем все типы из Domain
        let all_types = self.inference_service.get_all_platform_globals();

        // 2. Применяем пагинацию и преобразуем в DTO
        let type_dtos: Vec<TypeDto> = all_types
            .iter()
            .skip(offset)
            .take(limit)
            .map(|(name, res)| {
                // Определение категории и источника
                let (category, source) = match &res.source {
                    bsl_shared::domain::types::ResolutionSource::Static => {
                        ("Platform".to_string(), "Static Analysis".to_string())
                    }
                    _ => ("Configuration".to_string(), "Configuration".to_string()),
                };

                // Расчет certainty
                let certainty_val = match res.certainty {
                    Certainty::Known => 100,
                    Certainty::Inferred(val) => (val * 100.0) as u8,
                    Certainty::Unknown => 30,
                };

                // Извлечение union types
                let union_types = if let ResolutionResult::Union(types) = &res.result {
                    Some(
                        types
                            .iter()
                            .map(|wt| UnionComponentDto {
                                type_name: format!("{:?}", wt.type_),
                                probability: (wt.weight * 100.0) as u8,
                            })
                            .collect(),
                    )
                } else {
                    None
                };

                TypeDto {
                    id: name.clone(),
                    name: name.clone(),
                    category,
                    certainty: certainty_val,
                    certainty_text: format!("{:?} {}%", res.certainty, certainty_val),
                    facets: res.available_facets.iter().map(|f| format!("{:?}", f)).collect(),
                    methods_count: None,
                    methods: Vec::new(),
                    attributes_count: None,
                    source,
                    flow_sensitive: false, // TODO: добавить flow-sensitive анализ
                    description: self.generate_type_description(res),
                    union_types,
                    flow_analysis: None,
                    connections: None,
                    warning: None,
                    recommendation: None,
                }
            })
            .collect();

        // 3. Генерируем метрики
        let metrics = MetricsDto {
            total_types: all_types.len(),
            certainty_high: type_dtos.iter().filter(|t| t.certainty > 80).count(),
            certainty_medium: type_dtos
                .iter()
                .filter(|t| t.certainty > 40 && t.certainty <= 80)
                .count(),
            certainty_low: type_dtos.iter().filter(|t| t.certainty <= 40).count(),
            flow_sensitive: type_dtos.iter().filter(|t| t.flow_sensitive).count(),
            cache_hit_rate: format!("{:.1}%", self.cache.get_hit_rate()),
            analysis_speed: "125ms".to_string(), // TODO: реальная метрика
        };

        // 4. Генерируем категории
        let mut categories = std::collections::HashMap::new();
        categories.insert(
            "Platform".to_string(),
            CategoryDto {
                color: "#3498db".to_string(),
                icon: "🔧".to_string(),
                count: type_dtos.iter().filter(|t| t.category == "Platform").count(),
            },
        );
        categories.insert(
            "Configuration".to_string(),
            CategoryDto {
                color: "#e74c3c".to_string(),
                icon: "⚙️".to_string(),
                count: type_dtos
                    .iter()
                    .filter(|t| t.category == "Configuration")
                    .count(),
            },
        );

        // 5. Генерируем информацию о пагинации
        let total_items = all_types.len();
        let current_page = (offset / limit) + 1;
        let total_pages = total_items.div_ceil(limit);
        let has_prev = current_page > 1;
        let has_next = current_page < total_pages;

        let pagination = Some(PaginationDto {
            current_page,
            page_size: limit,
            total_items,
            total_pages,
            has_prev,
            has_next,
        });

        // 6. Возвращаем полную структуру
        AnalysisResultDto {
            types: type_dtos,
            categories,
            metrics,
            connections: Vec::new(),
            pagination,
        }
    }

    /// Генерация описания типа
    fn generate_type_description(&self, resolution: &TypeResolution) -> String {
        match &resolution.result {
            ResolutionResult::Concrete(concrete) => {
                format!("Конкретный тип: {:?}", concrete)
            }
            ResolutionResult::Union(types) => {
                format!("Union тип из {} вариантов", types.len())
            }
            ResolutionResult::Dynamic => "Динамический тип".to_string(),
        }
    }

    /// CLI операции - файловый анализ
    pub async fn analyze_file(&self, path: &str) -> Result<AnalysisResult> {
        info!("🔍 Анализируем файл: {}", path);

        let file_content = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("Не удалось прочитать файл {}: {}", path, e))?;

        let _parse_result = self
            .parser
            .parse(&file_content)
            .map_err(|e| anyhow::anyhow!("Ошибка парсинга файла {}: {}", path, e))?;

        let analysis_result = AnalysisResult {
            file_path: path.to_string(),
            type_resolutions: HashMap::new(),
            analysis_duration_ms: 0,
            cached_at: std::time::Instant::now(),
        };

        info!("✅ Анализ файла {} завершён", path);
        Ok(analysis_result)
    }

    /// Анализ содержимого файла без чтения с диска (Phase 4: улучшенная реализация)
    pub async fn analyze_file_content(
        &self,
        file_path: &str,
        content: &str,
    ) -> Result<AnalysisResult> {
        let start_time = std::time::Instant::now();
        info!("🔍 Анализ содержимого файла: {}", file_path);

        // 1. Проверка кэша (Application Layer логика)
        let cache_key = format!("{}:{}", file_path, self.hash_content(content));
        if let Some(cached_result) = self.cache.get_analysis(&cache_key) {
            info!("💾 Кэш попадание для файла: {}", file_path);
            return Ok(cached_result);
        }

        // 2. Парсинг файла
        let parse_result = self
            .parser
            .parse(content)
            .map_err(|e| anyhow::anyhow!("Ошибка парсинга содержимого {}: {}", file_path, e))?;

        info!("📝 Парсинг успешен, найдено операторов: {}", parse_result.statements.len());

        // 3. Извлечение переменных и типов из AST
        let mut type_resolutions = HashMap::new();

        // Простая эвристика: извлекаем переменные с типами
        for line in content.lines() {
            // Паттерн: Перем ИмяПеременной: ТипДанных
            if line.trim().starts_with("Перем ") {
                if let Some(type_hint) = self.extract_type_from_var_declaration(line) {
                    let var_name = self.extract_var_name(line).unwrap_or("unknown".to_string());

                    // Используем TypeInferenceService для разрешения типа
                    let resolution = self.inference_service.resolve_expression_async(&type_hint).await;
                    type_resolutions.insert(var_name, resolution);
                }
            }

            // Паттерн: Функция ИмяФункции() Возврат Тип;
            if line.trim().starts_with("Функция ") || line.trim().starts_with("Процедура ") {
                if let Some(return_type) = self.extract_return_type(line) {
                    let func_name = self.extract_function_name(line).unwrap_or("unknown".to_string());

                    let resolution = self.inference_service.resolve_expression_async(&return_type).await;
                    type_resolutions.insert(format!("return_{}", func_name), resolution);
                }
            }
        }

        let analysis_duration_ms = start_time.elapsed().as_millis();

        let analysis_result = AnalysisResult {
            file_path: file_path.to_string(),
            type_resolutions,
            analysis_duration_ms: analysis_duration_ms as u64,
            cached_at: std::time::Instant::now(),
        };

        // 4. Сохранение в кэш
        self.cache.store_analysis(cache_key, analysis_result.clone());

        info!("✅ Анализ содержимого {} завершён за {}ms", file_path, analysis_duration_ms);
        Ok(analysis_result)
    }

    // === HELPER METHODS FOR FILE ANALYSIS ===

    /// Хэширование содержимого для кэш-ключа
    fn hash_content(&self, content: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        hasher.finish()
    }

    /// Извлечение имени переменной из объявления
    fn extract_var_name(&self, line: &str) -> Option<String> {
        // Перем ИмяПеременной: Тип
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            let var_name = parts[1].trim_end_matches(':');
            return Some(var_name.to_string());
        }
        None
    }

    /// Извлечение типа из объявления переменной
    fn extract_type_from_var_declaration(&self, line: &str) -> Option<String> {
        // Перем ИмяПеременной: Тип
        if let Some(colon_pos) = line.find(':') {
            let type_part = &line[colon_pos + 1..];
            let type_name = type_part.split(';').next()?.trim();
            return Some(type_name.to_string());
        }
        None
    }

    /// Извлечение имени функции
    fn extract_function_name(&self, line: &str) -> Option<String> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            let func_name = parts[1].trim_end_matches('(');
            return Some(func_name.to_string());
        }
        None
    }

    /// Извлечение типа возврата функции
    fn extract_return_type(&self, line: &str) -> Option<String> {
        // Ищем "Возврат" в строке
        if let Some(return_pos) = line.find("Возврат") {
            let return_part = &line[return_pos + "Возврат".len()..];
            let type_name = return_part.split(';').next()?.trim();
            if !type_name.is_empty() {
                return Some(type_name.to_string());
            }
        }
        None
    }

    /// LSP операции - получить информацию о символе в позиции (hover)
    pub async fn get_hover_info(
        &self,
        file_content: &str,
        line: u32,
        column: u32,
    ) -> Result<Option<String>> {
        info!("🎯 Hover запрос: строка {}, колонка {}", line, column);

        let parse_result = self
            .parser
            .parse(file_content)
            .map_err(|e| anyhow::anyhow!("Ошибка парсинга для hover: {}", e))?;

        if let Some(symbol_info) =
            self.extract_enhanced_symbol_info(file_content, line, column, Some(&parse_result))
        {
            Ok(Some(symbol_info))
        } else {
            Ok(Some(format!("BSL символ на позиции {}:{}", line, column)))
        }
    }

    /// LSP операции - получить автодополнение в позиции
    pub async fn get_completion(
        &self,
        file_content: &str,
        line: u32,
        column: u32,
    ) -> Result<Vec<CompletionItem>> {
        info!("🎯 Completion запрос: строка {}, колонка {}", line, column);

        let _parse_result = self
            .parser
            .parse(file_content)
            .map_err(|e| anyhow::anyhow!("Ошибка парсинга для completion: {}", e))?;

        let context = self.analyze_completion_context(file_content, line, column);
        let mut completions = self.get_contextual_completions(&context);

        if context.can_add_statements {
            completions.extend(self.get_basic_bsl_constructs());
        }

        if context.expects_type || context.can_add_statements {
            completions.extend(self.get_bsl_types());
        }

        if context.can_add_functions {
            completions.extend(self.get_builtin_functions());
        }

        if completions.is_empty() || completions.len() < 5 {
            completions.extend(self.get_basic_bsl_constructs());
            completions.extend(self.get_bsl_types());
            completions.extend(self.get_builtin_functions());
        }

        Ok(completions)
    }

    /// Web операции - поиск типов
    pub async fn search_types(&self, query: &str) -> Result<Vec<String>> {
        info!("🌐 Web поиск типов: {}", query);
        let results = self.inference_service.search_types(query);
        Ok(results)
    }

    /// Web операции - получить детали типа
    pub async fn get_type_details(&self, type_name: &str) -> Result<Option<TypeResolution>> {
        info!("🌐 Web детали типа: {}", type_name);
        let platform_globals = self.inference_service.get_all_platform_globals();
        Ok(platform_globals.get(type_name).cloned())
    }

    /// Web операции - получить автодополнения для выражения
    pub async fn get_type_completions(
        &self,
        expression: &str,
    ) -> Result<Vec<CompletionItem>> {
        info!("🌐 Web автодополнения для: {}", expression);
        let completions = self.inference_service.get_completions(expression);
        Ok(completions)
    }

    /// Phase 5: Получить метрики типов для Web API
    pub fn get_metrics_summary(&self) -> serde_json::Value {
        use bsl_shared::domain::types::Certainty;

        let all_types = self.inference_service.get_all_platform_globals();

        let mut known = 0;
        let mut inferred = 0;
        let mut unknown = 0;

        for res in all_types.values() {
            match res.certainty {
                Certainty::Known => known += 1,
                Certainty::Inferred(_) => inferred += 1,
                Certainty::Unknown => unknown += 1,
            }
        }

        serde_json::json!({
            "total_types": all_types.len(),
            "known_types": known,
            "inferred_types": inferred,
            "unknown_types": unknown,
        })
    }

    // === МЕТОДЫ АНАЛИЗА КОНТЕКСТА ===

    /// Анализирует контекст для умного автодополнения
    pub fn analyze_completion_context(
        &self,
        content: &str,
        line: u32,
        column: u32,
    ) -> CompletionContext {
        let lines: Vec<&str> = content.lines().collect();
        let line_index = line as usize;

        // Получаем текущую строку и префикс
        let (current_line, line_prefix) = if line_index < lines.len() {
            let line_content = lines[line_index];
            let column_index = (column as usize).min(line_content.len());
            (line_content, &line_content[..column_index])
        } else {
            ("", "")
        };

        // Извлекаем текущее слово
        let current_word = self.extract_word_at_position(current_line, column as usize);

        // Анализируем контекст
        let line_trimmed = line_prefix.trim();

        CompletionContext {
            current_word: current_word.clone(),
            can_add_statements: self.can_add_statements(line_trimmed),
            expects_type: self.expects_type_context(line_trimmed),
            can_add_functions: self.can_add_functions(line_trimmed),
        }
    }

    /// Извлекает слово в указанной позиции
    fn extract_word_at_position(&self, line: &str, column: usize) -> String {
        if column == 0 || column > line.len() {
            return String::new();
        }

        let chars: Vec<char> = line.chars().collect();
        let mut start = column;
        let mut end = column;

        // Идём назад от позиции до начала слова
        while start > 0 {
            let ch = chars[start - 1];
            if ch.is_alphabetic() || ch == '_' || (start < column && ch.is_numeric()) {
                start -= 1;
            } else {
                break;
            }
        }

        // Идём вперёд до конца слова
        while end < chars.len() {
            let ch = chars[end];
            if ch.is_alphanumeric() || ch == '_' {
                end += 1;
            } else {
                break;
            }
        }

        chars[start..end].iter().collect()
    }

    /// Проверяет, можно ли добавлять операторы в данной позиции
    fn can_add_statements(&self, line_prefix: &str) -> bool {
        line_prefix.is_empty()
            || line_prefix.ends_with(';')
            || line_prefix.ends_with("Тогда")
            || line_prefix.ends_with("Иначе")
            || line_prefix.ends_with("КонецЕсли")
            || line_prefix.ends_with("КонецЦикла")
            || line_prefix.trim_start().is_empty()
    }

    /// Проверяет, ожидается ли тип в данной позиции
    fn expects_type_context(&self, line_prefix: &str) -> bool {
        line_prefix.contains(":")
            || line_prefix.contains("Тип(")
            || line_prefix.contains("ТипЗнч(")
            || line_prefix.contains("// ")
    }

    /// Проверяет, можно ли добавлять функции в данной позиции
    fn can_add_functions(&self, line_prefix: &str) -> bool {
        !line_prefix.contains("Процедура") && !line_prefix.contains("Функция")
    }

    /// Получает контекстные автодополнения
    pub fn get_contextual_completions(&self, context: &CompletionContext) -> Vec<CompletionItem> {
        let mut completions = Vec::new();

        // Фильтруем по текущему слову
        if !context.current_word.is_empty() {
            if context.current_word.to_lowercase().starts_with("п") {
                completions.push(CompletionItem {
                    label: "Процедура".to_string(),
                    kind: CompletionKind::Keyword,
                    detail: Some("🔧 Объявление процедуры".to_string()),
                    documentation: Some("Ключевое слово для объявления процедуры".to_string()),
                    insert_text: Some("Процедура ${1:ИмяПроцедуры}(${2:Параметры})\n\t${3:// тело процедуры}\nКонецПроцедуры".to_string()),
                    filter_text: Some("Процедура".to_string()),
                    sort_text: Some("Процедура".to_string()),
                });
            }

            if context.current_word.to_lowercase().starts_with("с") {
                completions.push(CompletionItem {
                    label: "Сообщить".to_string(),
                    kind: CompletionKind::Function,
                    detail: Some("📢 Вывод сообщения".to_string()),
                    documentation: Some("Функция для вывода сообщения пользователю".to_string()),
                    insert_text: Some("Сообщить(${1:\"текст\"})".to_string()),
                    filter_text: Some("Сообщить".to_string()),
                    sort_text: Some("Сообщить".to_string()),
                });
                completions.push(CompletionItem {
                    label: "Строка".to_string(),
                    kind: CompletionKind::Type,
                    detail: Some("📝 Тип данных: строка".to_string()),
                    documentation: Some("Примитивный тип данных для текстовых значений".to_string()),
                    insert_text: Some("Строка".to_string()),
                    filter_text: Some("Строка".to_string()),
                    sort_text: Some("Строка".to_string()),
                });
            }

            if context.current_word.to_lowercase().starts_with("т") {
                completions.push(CompletionItem {
                    label: "ТипЗнч".to_string(),
                    kind: CompletionKind::Function,
                    detail: Some("🔍 Получить тип значения".to_string()),
                    documentation: Some("Функция для получения типа переданного значения".to_string()),
                    insert_text: Some("ТипЗнч(${1:значение})".to_string()),
                    filter_text: Some("ТипЗнч".to_string()),
                    sort_text: Some("ТипЗнч".to_string()),
                });
            }
        }

        completions
    }

    /// Получает базовые BSL конструкции
    pub fn get_basic_bsl_constructs(&self) -> Vec<CompletionItem> {
        vec![
            CompletionItem {
                label: "Функция".to_string(),
                kind: CompletionKind::Keyword,
                detail: Some("🔧 Объявление функции".to_string()),
                documentation: Some("Ключевое слово для объявления функции".to_string()),
                insert_text: Some("Функция ${1:ИмяФункции}(${2:Параметры})\n\t${3:// тело функции}\nКонецФункции".to_string()),
                filter_text: Some("Функция".to_string()),
                sort_text: Some("Функция".to_string()),
            },
            CompletionItem {
                label: "Процедура".to_string(),
                kind: CompletionKind::Keyword,
                detail: Some("🔧 Объявление процедуры".to_string()),
                documentation: Some("Ключевое слово для объявления процедуры".to_string()),
                insert_text: Some("Процедура ${1:ИмяПроцедуры}(${2:Параметры})\n\t${3:// тело процедуры}\nКонецПроцедуры".to_string()),
                filter_text: Some("Процедура".to_string()),
                sort_text: Some("Процедура".to_string()),
            },
            CompletionItem {
                label: "Если".to_string(),
                kind: CompletionKind::Keyword,
                detail: Some("🔀 Условное выражение".to_string()),
                documentation: Some("Ключевое слово для условного выполнения".to_string()),
                insert_text: Some("Если ${1:условие} Тогда\n\t${2:// действия}\nКонецЕсли".to_string()),
                filter_text: Some("Если".to_string()),
                sort_text: Some("Если".to_string()),
            },
            CompletionItem {
                label: "Для".to_string(),
                kind: CompletionKind::Keyword,
                detail: Some("🔄 Цикл Для".to_string()),
                documentation: Some("Ключевое слово для циклического выполнения".to_string()),
                insert_text: Some("Для ${1:Счетчик} = ${2:НачальноеЗначение} По ${3:КонечноеЗначение} Цикл\n\t${4:// тело цикла}\nКонецЦикла".to_string()),
                filter_text: Some("Для".to_string()),
                sort_text: Some("Для".to_string()),
            },
        ]
    }

    /// Получает BSL типы данных
    pub fn get_bsl_types(&self) -> Vec<CompletionItem> {
        vec![
            CompletionItem {
                label: "Строка".to_string(),
                kind: CompletionKind::Type,
                detail: Some("📝 Строковый тип данных".to_string()),
                documentation: Some("Примитивный тип данных для текстовых значений".to_string()),
                insert_text: Some("Строка".to_string()),
                filter_text: Some("Строка".to_string()),
                sort_text: Some("Строка".to_string()),
            },
            CompletionItem {
                label: "Число".to_string(),
                kind: CompletionKind::Type,
                detail: Some("🔢 Числовой тип данных".to_string()),
                documentation: Some("Примитивный тип данных для числовых значений".to_string()),
                insert_text: Some("Число".to_string()),
                filter_text: Some("Число".to_string()),
                sort_text: Some("Число".to_string()),
            },
            CompletionItem {
                label: "Булево".to_string(),
                kind: CompletionKind::Type,
                detail: Some("✅ Булевый тип данных".to_string()),
                documentation: Some("Примитивный тип данных для логических значений".to_string()),
                insert_text: Some("Булево".to_string()),
                filter_text: Some("Булево".to_string()),
                sort_text: Some("Булево".to_string()),
            },
            CompletionItem {
                label: "Дата".to_string(),
                kind: CompletionKind::Type,
                detail: Some("📅 Тип данных дата/время".to_string()),
                documentation: Some("Примитивный тип данных для значений даты и времени".to_string()),
                insert_text: Some("Дата".to_string()),
                filter_text: Some("Дата".to_string()),
                sort_text: Some("Дата".to_string()),
            },
        ]
    }

    /// Получает встроенные функции BSL
    pub fn get_builtin_functions(&self) -> Vec<CompletionItem> {
        vec![
            CompletionItem {
                label: "Сообщить".to_string(),
                kind: CompletionKind::Function,
                detail: Some("📢 Вывести сообщение пользователю".to_string()),
                documentation: Some("Встроенная функция для вывода сообщения пользователю".to_string()),
                insert_text: Some("Сообщить(${1:\"текст\"})".to_string()),
                filter_text: Some("Сообщить".to_string()),
                sort_text: Some("Сообщить".to_string()),
            },
            CompletionItem {
                label: "ТипЗнч".to_string(),
                kind: CompletionKind::Function,
                detail: Some("🔍 Получить тип значения".to_string()),
                documentation: Some("Встроенная функция для получения типа значения".to_string()),
                insert_text: Some("ТипЗнч(${1:значение})".to_string()),
                filter_text: Some("ТипЗнч".to_string()),
                sort_text: Some("ТипЗнч".to_string()),
            },
            CompletionItem {
                label: "СтрДлина".to_string(),
                kind: CompletionKind::Function,
                detail: Some("📏 Получить длину строки".to_string()),
                documentation: Some("Встроенная функция для получения длины строки".to_string()),
                insert_text: Some("СтрДлина(${1:строка})".to_string()),
                filter_text: Some("СтрДлина".to_string()),
                sort_text: Some("СтрДлина".to_string()),
            },
        ]
    }

    /// Извлекает расширенную информацию о символе
    fn extract_enhanced_symbol_info(
        &self,
        _content: &str,
        line: u32,
        column: u32,
        _ast: Option<&crate::parsing::Program>,
    ) -> Option<String> {
        // Простая заглушка для hover информации
        Some(format!("BSL символ на позиции {}:{} (Phase 4)", line, column))
    }
}

/// Контекст для автодополнения
#[derive(Debug, Clone)]
pub struct CompletionContext {
    pub current_word: String,
    pub can_add_statements: bool,
    pub expects_type: bool,
    pub can_add_functions: bool,
}
