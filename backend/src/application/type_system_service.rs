//! Application Layer: Type System Service
//!
//! Unified API для всех типов клиентов (LSP, Web, CLI)
//! Phase 4: API Unification - объединяет LspTypeService + WebTypeService + AnalysisService

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::info;

use crate::application::TypeInferenceService;
use crate::system::{AnalysisCache, AnalysisResult, IrCache, ParserCoordinator};
use bsl_shared::domain::types::{ResolutionResult, TypeResolution};
use bsl_shared::domain::{CompletionItem, CompletionKind, TypeMetadataLookup};
use bsl_shared::engine::AnalysisEngine;

/// Унифицированный сервис системы типов для Application Layer
///
/// Phase 4: Заменяет LspTypeService + WebTypeService + AnalysisService
/// единым unified API для всех презентационных слоев
pub struct TypeSystemService {
    // Application Layer: AnalysisEngine - чистая оркестрация анализа
    analysis_engine: Arc<AnalysisEngine>,

    // Application Layer: Type Inference Service для high-level операций
    inference_service: Arc<TypeInferenceService>,

    // Domain Layer: TypeMetadataLookup - мост между TypeResolution и RawTypeData
    metadata_lookup: TypeMetadataLookup,

    // System Layer компоненты
    cache: Arc<AnalysisCache>,
    ir_cache: Arc<IrCache>, // Milestone 2.13: IR кеширование для LSP hover
    parser: Arc<ParserCoordinator>,
}

impl TypeSystemService {
    /// Конструктор согласно архитектурной диаграмме
    pub fn new(
        analysis_engine: Arc<AnalysisEngine>,
        cache: Arc<AnalysisCache>,
        parser: Arc<ParserCoordinator>,
        ir_cache: Arc<IrCache>, // Milestone 2.13: IR кеширование
    ) -> Self {
        // Создаем TypeInferenceService на основе AnalysisEngine
        let resolver = analysis_engine.get_resolver();
        let repository = analysis_engine.get_repository();
        let inference_service = Arc::new(TypeInferenceService::new(
            resolver.clone(),
            repository.clone(),
        ));

        // Создаем TypeMetadataLookup для получения методов/свойств из RawTypeData
        let metadata_lookup = TypeMetadataLookup::new(repository);

        Self {
            analysis_engine,
            inference_service,
            metadata_lookup,
            cache,
            ir_cache,
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
        category_filter: Option<String>,
        certainty_filter: Option<String>,
        flow_sensitive_only: bool,
    ) -> bsl_shared::api::dtos::AnalysisResultDto {
        use bsl_shared::api::dtos::{
            AnalysisResultDto, CategoryDto, MetricsDto, PaginationDto, TabularSectionAttributeDto,
            TabularSectionDto, TypeDto, UnionComponentDto,
        };
        use bsl_shared::domain::types::{Certainty, ResolutionResult};

        // 1. Получаем все типы из Domain
        let all_types = self.inference_service.get_all_platform_globals();

        // 2. Сначала преобразуем в DTO (чтобы знать категорию), потом фильтруем, потом пагинируем
        let all_type_dtos: Vec<TypeDto> = all_types
            .iter()
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

                // Получаем методы и свойства через TypeMetadataLookup
                let methods = self.metadata_lookup.get_methods(res);
                let properties = self.metadata_lookup.get_properties(res);
                let raw_type = self.metadata_lookup.get_raw_type(res);

                // Извлекаем реальное описание из RawTypeData
                let description = raw_type
                    .as_ref()
                    .map(|rt| rt.description.clone())
                    .unwrap_or_else(|| self.generate_type_description(res));

                // Извлекаем enum values для платформенных перечислений
                let enum_values = raw_type.as_ref().and_then(|rt| {
                    if rt.enum_values.is_empty() {
                        None
                    } else {
                        Some(rt.enum_values.clone())
                    }
                });

                TypeDto {
                    id: name.clone(),
                    name: name.clone(),
                    category,
                    certainty: certainty_val,
                    certainty_text: format!("{:?} {}%", res.certainty, certainty_val),
                    facets: res
                        .available_facets
                        .iter()
                        .map(|f| format!("{:?}", f))
                        .collect(),
                    methods_count: Some(methods.len()),
                    methods: methods.iter().map(|m| m.name.clone()).collect(),
                    attributes_count: raw_type.as_ref().map(|rt| rt.attributes.len()),
                    properties: properties.iter().map(|p| p.name.clone()).collect(),
                    enum_values,
// Конвертируем табличные части из RawTypeData в DTO
                    tabular_sections: raw_type
                        .as_ref()
                        .map(|rt| {
                            rt.tabular_sections
                                .iter()
                                .map(|ts| TabularSectionDto {
                                    name: ts.name.clone(),
                                    attributes: ts
                                        .attributes
                                        .iter()
                                        .map(|attr| TabularSectionAttributeDto {
                                            name: attr.name.clone(),
                                            attr_type: Some(attr.attr_type.clone()),
                                        })
                                        .collect(),
                                })
                                .collect()
                        })
                        .unwrap_or_default(),
                    source,
                    flow_sensitive: false, // TODO: добавить flow-sensitive анализ
                    description,
                    union_types,
                    flow_analysis: None,
                    connections: None,
                    warning: None,
                    recommendation: None,
                }
            })
            .collect();

        // 3. Применяем фильтры
        let filtered_types: Vec<TypeDto> = all_type_dtos
            .into_iter()
            .filter(|t| {
                // Фильтр по категории
                if let Some(ref cat) = category_filter {
                    if &t.category != cat {
                        return false;
                    }
                }

                // Фильтр по определённости
                if let Some(ref cert) = certainty_filter {
                    let passes = match cert.as_str() {
                        "high" => t.certainty >= 80,
                        "medium" => t.certainty >= 30 && t.certainty < 80,
                        "low" => t.certainty < 30,
                        _ => true,
                    };
                    if !passes {
                        return false;
                    }
                }

                // Фильтр по flow-sensitive
                if flow_sensitive_only && !t.flow_sensitive {
                    return false;
                }

                true
            })
            .collect();

        // 4. Применяем пагинацию
        let type_dtos: Vec<TypeDto> = filtered_types
            .iter()
            .skip(offset)
            .take(limit)
            .cloned()
            .collect();

        // 5. Генерируем метрики (используем отфильтрованные данные)
        let metrics = MetricsDto {
            total_types: filtered_types.len(),
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
                count: type_dtos
                    .iter()
                    .filter(|t| t.category == "Platform")
                    .count(),
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

        // 6. Генерируем информацию о пагинации (используем отфильтрованные данные)
        let total_items = filtered_types.len();
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
            ResolutionResult::Intersection(types) => {
                format!("Intersection тип из {} типов", types.len())
            }
            ResolutionResult::Generic(gen) => {
                format!(
                    "Generic тип: {}<{}>",
                    gen.base_type,
                    gen.type_params
                        .iter()
                        .map(|t| format!("{:?}", t))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            ResolutionResult::Nullable(inner) => {
                format!("Nullable тип: {:?} | Null", inner)
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

    /// LSP операции - инкрементальный парсинг для textDocument/didChange
    pub async fn parse_incremental(
        &self,
        file_path: std::path::PathBuf,
        new_content: String,
        edits: Vec<crate::system::parser_coordinator::TextEdit>,
    ) -> Result<()> {
        info!("🔄 Инкрементальный парсинг файла: {:?}", file_path);

        let _result = self
            .parser
            .parse_incremental(file_path, new_content, edits)
            .map_err(|e| anyhow::anyhow!("Ошибка инкрементального парсинга: {}", e))?;

        info!("✅ Инкрементальный парсинг завершён успешно");
        Ok(())
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

        info!(
            "📝 Парсинг успешен, найдено операторов: {}",
            parse_result.program.statements.len()
        );

        // 3. Извлечение переменных и типов из AST
        let mut type_resolutions = HashMap::new();

        // Простая эвристика: извлекаем переменные с типами
        for line in content.lines() {
            // Паттерн: Перем ИмяПеременной: ТипДанных
            if line.trim().starts_with("Перем ") {
                if let Some(type_hint) = self.extract_type_from_var_declaration(line) {
                    let var_name = self.extract_var_name(line).unwrap_or("unknown".to_string());

                    // Используем TypeInferenceService для разрешения типа
                    let resolution = self
                        .inference_service
                        .resolve_expression_async(&type_hint)
                        .await;
                    type_resolutions.insert(var_name, resolution);
                }
            }

            // Паттерн: Функция ИмяФункции() Возврат Тип;
            if line.trim().starts_with("Функция ") || line.trim().starts_with("Процедура ")
            {
                if let Some(return_type) = self.extract_return_type(line) {
                    let func_name = self
                        .extract_function_name(line)
                        .unwrap_or("unknown".to_string());

                    let resolution = self
                        .inference_service
                        .resolve_expression_async(&return_type)
                        .await;
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
        self.cache
            .store_analysis(cache_key, analysis_result.clone());

        info!(
            "✅ Анализ содержимого {} завершён за {}ms",
            file_path, analysis_duration_ms
        );
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
        use tracing::{debug, info, warn};

        info!("🎯 Hover запрос: строка {}, колонка {}", line, column);

        // MILESTONE 2.13: IR Caching - проверяем кеш перед парсингом
        let content_hash = self.hash_content(file_content);

        // Проверяем IR кеш
        let ir_program = if let Some(cached_ir) = self.ir_cache.get(content_hash).await {
            info!("✅ IR cache HIT for hash {}", content_hash);
            cached_ir
        } else {
            info!("❌ IR cache MISS for hash {}, parsing...", content_hash);

            // Парсинг BSL кода (только при cache MISS)
            let parse_result = self
                .parser
                .parse(file_content)
                .map_err(|e| anyhow::anyhow!("Ошибка парсинга для hover: {}", e))?;

            // Конвертация AST → IR для Inline Scope Analysis
            let ir = crate::application::ast_to_ir::AstToIrConverter::convert(
                parse_result.program.clone(),
                file_content.to_string(),
                "hover_request.bsl".to_string(),
            )?;

            let ir_arc = std::sync::Arc::new(ir);

            // Сохраняем в кеш
            self.ir_cache.put(content_hash, ir_arc.clone()).await;
            debug!("Cached IR for hash {}", content_hash);

            ir_arc
        };

        // Milestone 2.11 Task B1: DEBUG логи для поиска узла
        debug!("Looking for node at position {}:{}", line, column);

        // НОВОЕ: Используем find_variable_at_position() для извлечения типа из scope
        if let Some((var_name, type_hint)) = ir_program.find_variable_at_position(line, column) {
            info!(
                "✅ find_variable_at_position({}, {}) нашла переменную: '{}' с типом {:?}",
                line, column, var_name, type_hint
            );

            // Форматируем hover с методами/свойствами из TypeRepository
            return Ok(Some(self.format_variable_hover(&var_name, &type_hint)));
        } else {
            // Milestone 2.11 Task B1: Логи когда переменная не найдена
            debug!(
                "find_variable_at_position({}, {}) не нашла переменную",
                line, column
            );
        }

        // Fallback 1: Пробуем find_node_at_position для других узлов (функции, циклы, etc.)
        if let Some(node) = ir_program.find_node_at_position(line, column) {
            info!(
                "✅ find_node_at_position({}, {}) нашёл узел (не переменная): span={:?}",
                line, column, node.span
            );
            debug!("Found node: {:?} at span {:?}", node.kind, node.span);
            return Ok(Some(self.format_semantic_node_info(node, file_content)));
        } else {
            // Milestone 2.11 Task B1: Предупреждение когда узел не найден
            warn!("❌ No node found at position {}:{} in IR", line, column);
        }

        // Fallback: старая логика по имени переменной (без AST, т.к. теперь используется IR кеш)
        if let Some(symbol_info) =
            self.extract_enhanced_symbol_info(file_content, line, column, None)
        {
            debug!("Fallback: using extract_enhanced_symbol_info");
            Ok(Some(symbol_info))
        } else {
            warn!("❌ Fallback also failed, returning generic BSL symbol message");
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

    /// Phase 5: Поиск типов с преобразованием в DTO (Web API)
    pub async fn search_types_as_dto(
        &self,
        query: &str,
    ) -> Result<bsl_shared::api::dtos::AnalysisResultDto> {
        use bsl_shared::api::dtos::{
            AnalysisResultDto, CategoryDto, MetricsDto, TabularSectionAttributeDto,
            TabularSectionDto, TypeDto, UnionComponentDto,
        };
        use bsl_shared::domain::types::{Certainty, ResolutionResult};

        info!("🌐 Web поиск типов с DTO: {}", query);

        // 1. Получаем все типы и фильтруем по запросу
        let all_types = self.inference_service.get_all_platform_globals();
        let query_lower = query.to_lowercase();

        let filtered_types: Vec<(&String, &TypeResolution)> = all_types
            .iter()
            .filter(|(name, _)| name.to_lowercase().contains(&query_lower))
            .collect();

        // 2. Преобразуем в DTO
        let type_dtos: Vec<TypeDto> = filtered_types
            .iter()
            .map(|(name, res)| {
                let (category, source) = match &res.source {
                    bsl_shared::domain::types::ResolutionSource::Static => {
                        ("Platform".to_string(), "Static Analysis".to_string())
                    }
                    _ => ("Configuration".to_string(), "Configuration".to_string()),
                };

                let certainty_val = match res.certainty {
                    Certainty::Known => 100,
                    Certainty::Inferred(val) => (val * 100.0) as u8,
                    Certainty::Unknown => 30,
                };

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

                // Получаем методы и свойства через TypeMetadataLookup
                let methods = self.metadata_lookup.get_methods(res);
                let properties = self.metadata_lookup.get_properties(res);
                let raw_type = self.metadata_lookup.get_raw_type(res);

                // Извлекаем реальное описание из RawTypeData
                let description = raw_type
                    .as_ref()
                    .map(|rt| rt.description.clone())
                    .unwrap_or_else(|| self.generate_type_description(res));

                // Извлекаем enum values для платформенных перечислений
                let enum_values = raw_type.as_ref().and_then(|rt| {
                    if rt.enum_values.is_empty() {
                        None
                    } else {
                        Some(rt.enum_values.clone())
                    }
                });

                TypeDto {
                    id: (*name).clone(),
                    name: (*name).clone(),
                    category,
                    certainty: certainty_val,
                    certainty_text: format!("{:?} {}%", res.certainty, certainty_val),
                    facets: res
                        .available_facets
                        .iter()
                        .map(|f| format!("{:?}", f))
                        .collect(),
                    methods_count: Some(methods.len()),
                    methods: methods.iter().map(|m| m.name.clone()).collect(),
                    attributes_count: raw_type.as_ref().map(|rt| rt.attributes.len()),
                    properties: properties.iter().map(|p| p.name.clone()).collect(),
                    enum_values,
// Конвертируем табличные части из RawTypeData в DTO
                    tabular_sections: raw_type
                        .as_ref()
                        .map(|rt| {
                            rt.tabular_sections
                                .iter()
                                .map(|ts| TabularSectionDto {
                                    name: ts.name.clone(),
                                    attributes: ts
                                        .attributes
                                        .iter()
                                        .map(|attr| TabularSectionAttributeDto {
                                            name: attr.name.clone(),
                                            attr_type: Some(attr.attr_type.clone()),
                                        })
                                        .collect(),
                                })
                                .collect()
                        })
                        .unwrap_or_default(),
                    source,
                    flow_sensitive: false,
                    description,
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
            total_types: type_dtos.len(),
            certainty_high: type_dtos.iter().filter(|t| t.certainty > 80).count(),
            certainty_medium: type_dtos
                .iter()
                .filter(|t| t.certainty > 40 && t.certainty <= 80)
                .count(),
            certainty_low: type_dtos.iter().filter(|t| t.certainty <= 40).count(),
            flow_sensitive: type_dtos.iter().filter(|t| t.flow_sensitive).count(),
            cache_hit_rate: format!("{:.1}%", self.cache.get_hit_rate()),
            analysis_speed: "125ms".to_string(),
        };

        // 4. Генерируем категории
        let mut categories = std::collections::HashMap::new();
        categories.insert(
            "Platform".to_string(),
            CategoryDto {
                color: "#3498db".to_string(),
                icon: "🔧".to_string(),
                count: type_dtos
                    .iter()
                    .filter(|t| t.category == "Platform")
                    .count(),
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

        Ok(AnalysisResultDto {
            types: type_dtos,
            categories,
            metrics,
            connections: vec![],
            pagination: None, // Поиск без пагинации
        })
    }

    /// Web операции - получить детали типа
    pub async fn get_type_details(&self, type_name: &str) -> Result<Option<TypeResolution>> {
        info!("🌐 Web детали типа: {}", type_name);
        let platform_globals = self.inference_service.get_all_platform_globals();
        Ok(platform_globals.get(type_name).cloned())
    }

    /// Web операции - получить автодополнения для выражения
    pub async fn get_type_completions(&self, expression: &str) -> Result<Vec<CompletionItem>> {
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

    /// Конвертирует UTF-16 offset (LSP character) в UTF-8 byte offset
    ///
    /// LSP протокол использует UTF-16 code units для позиций, но Rust строки в UTF-8.
    /// Эта функция корректно преобразует UTF-16 offset в byte offset для работы с &str[..].
    fn utf16_to_byte_offset(line: &str, utf16_offset: u32) -> usize {
        let mut utf16_count = 0;
        for (byte_offset, ch) in line.char_indices() {
            if utf16_count >= utf16_offset {
                return byte_offset;
            }
            // Кириллица и другие non-ASCII символы занимают 2 UTF-16 code units
            utf16_count += ch.len_utf16() as u32;
        }
        line.len() // Если offset за пределами строки, возвращаем конец
    }

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
        let (_current_line, line_prefix) = if line_index < lines.len() {
            let line_content = lines[line_index];
            // ✅ ИСПРАВЛЕНИЕ: Конвертируем UTF-16 offset → UTF-8 byte offset
            let column_index = Self::utf16_to_byte_offset(line_content, column);
            (line_content, &line_content[..column_index])
        } else {
            ("", "")
        };

        // Извлекаем текущее слово
        let current_word = self
            .extract_word_at_position(content, line, column)
            .unwrap_or_default();

        // Анализируем контекст
        let line_trimmed = line_prefix.trim();

        CompletionContext {
            current_word: current_word.clone(),
            can_add_statements: self.can_add_statements(line_trimmed),
            expects_type: self.expects_type_context(line_trimmed),
            can_add_functions: self.can_add_functions(line_trimmed),
        }
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
                    documentation: Some(
                        "Примитивный тип данных для текстовых значений".to_string(),
                    ),
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
                    documentation: Some(
                        "Функция для получения типа переданного значения".to_string(),
                    ),
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
                documentation: Some(
                    "Примитивный тип данных для значений даты и времени".to_string(),
                ),
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
                documentation: Some(
                    "Встроенная функция для вывода сообщения пользователю".to_string(),
                ),
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

    // ============================================================================
    // Validation API (Phase 4 - TypeValidator Integration)
    // ============================================================================

    /// Валидирует код 1С с использованием TypeValidator
    ///
    /// # Параметры
    /// * `code` - фрагмент кода для валидации (например "массив.Добавить()" или "таблица.НесуществующийМетод()")
    ///
    /// # Возвращает
    /// Список ошибок валидации или пустой вектор если код валиден
    ///
    /// # Примеры
    /// ```no_run
    /// # use bsl_backend::application::type_system_service::TypeSystemService;
    /// # async fn example(service: &TypeSystemService) -> anyhow::Result<()> {
    /// let errors = service.validate_code_fragment("массив.НесуществующийМетод()").await?;
    /// if errors.is_empty() {
    ///     println!("Код валиден!");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn validate_code_fragment(
        &self,
        code: &str,
    ) -> Result<Vec<bsl_shared::api::ValidationErrorDto>> {
        use bsl_shared::domain::types::DiagnosticSeverity;
        use bsl_shared::domain::validators::TypeValidator;
        use std::time::Instant;

        let start = Instant::now();
        let mut errors = Vec::new();

        // Упрощённая валидация для демонстрации
        // В реальной реализации нужен парсинг кода и анализ AST

        // Примитивный парсинг выражений вида "объект.метод()" или "объект.свойство"
        if let Some((object_expr, member)) = Self::parse_simple_member_access(code) {
            // Резолвим тип объекта
            let resolution = self
                .inference_service
                .resolve_expression_async(&object_expr)
                .await;
            let validator = TypeValidator::new(&self.metadata_lookup);

            // Проверяем методы (если есть скобки)
            if member.ends_with("()") || code.contains('(') {
                let method_name = member.trim_end_matches("()").trim();
                if let Some(error) = validator.validate_method_exists(&resolution, method_name) {
                    let diagnostic = error.to_diagnostic(1, 1);
                    errors.push(bsl_shared::api::ValidationErrorDto {
                        message: diagnostic.message,
                        severity: match diagnostic.severity {
                            DiagnosticSeverity::Error => "error".to_string(),
                            DiagnosticSeverity::Warning => "warning".to_string(),
                            DiagnosticSeverity::Info | DiagnosticSeverity::Hint => {
                                "info".to_string()
                            }
                        },
                        line: diagnostic.line,
                        column: diagnostic.column,
                        error_type: "NonExistentMethod".to_string(),
                    });
                }
            } else {
                // Проверяем свойства
                if let Some(error) = validator.validate_property_exists(&resolution, &member) {
                    let diagnostic = error.to_diagnostic(1, 1);
                    errors.push(bsl_shared::api::ValidationErrorDto {
                        message: diagnostic.message,
                        severity: match diagnostic.severity {
                            DiagnosticSeverity::Error => "error".to_string(),
                            DiagnosticSeverity::Warning => "warning".to_string(),
                            DiagnosticSeverity::Info | DiagnosticSeverity::Hint => {
                                "info".to_string()
                            }
                        },
                        line: diagnostic.line,
                        column: diagnostic.column,
                        error_type: "NonExistentProperty".to_string(),
                    });
                }
            }
        }

        info!("Валидация завершена за {:?}", start.elapsed());
        Ok(errors)
    }

    /// Примитивный парсер для выражений вида "объект.метод()" или "объект.свойство"
    fn parse_simple_member_access(code: &str) -> Option<(String, String)> {
        let trimmed = code.trim();
        if let Some(dot_pos) = trimmed.find('.') {
            let object = trimmed[..dot_pos].trim().to_string();
            let member = trimmed[dot_pos + 1..].trim().to_string();
            Some((object, member))
        } else {
            None
        }
    }

    /// Извлечь информацию о символе на указанной позиции из AST
    fn extract_enhanced_symbol_info(
        &self,
        file_content: &str,
        line: u32,
        column: u32,
        parse_result: Option<&crate::parsing::Program>,
    ) -> Option<String> {
        use crate::parsing::bsl::ast::{Expression, Statement};

        // Шаг 1: Извлечь слово под курсором
        let word_under_cursor = self.extract_word_at_position(file_content, line, column)?;

        // Шаг 2: Если есть AST, ищем информацию о слове в нём
        if let Some(parse_result) = parse_result {
            for statement in &parse_result.statements {
                match statement {
                    Statement::VarDeclaration { name, .. } if name == &word_under_cursor => {
                        return Some(format!("**Переменная:** `{}`\n\n*Тип:* Неопределено (требуется flow-sensitive анализ)", name));
                    }
                    Statement::Assignment { target, value, .. } => {
                        if let Expression::Identifier { name: var_name, .. } = target {
                            if var_name == &word_under_cursor {
                                // Application Layer: Маппинг AST → имя типа
                                if let Some(type_name) = self.expression_to_type_name(value) {
                                    // Domain Layer: Резолв через AnalysisEngine
                                    let resolution = self.analysis_engine.resolve_type(&type_name);
                                    let type_info =
                                        self.format_type_for_hover(&type_name, &resolution);
                                    return Some(format!(
                                        "**Присваивание:** `{} = ...`\n\n{}",
                                        var_name, type_info
                                    ));
                                } else {
                                    return Some(format!("**Присваивание:** `{} = ...`\n\n*Тип:* Требуется расширенный анализ", var_name));
                                }
                            }
                        }
                    }
                    Statement::FunctionDecl { name, params, .. } if name == &word_under_cursor => {
                        return Some(format!(
                            "**Функция:** `{}({})`\n\n*Параметры:* {}",
                            name,
                            params.join(", "),
                            params.len()
                        ));
                    }
                    Statement::ProcedureDecl { name, params, .. } if name == &word_under_cursor => {
                        return Some(format!(
                            "**Процедура:** `{}({})`\n\n*Параметры:* {}",
                            name,
                            params.join(", "),
                            params.len()
                        ));
                    }
                    _ => {}
                }
            }
        }

        // Шаг 3: Fallback - попытка резолва через TypeResolver
        self.resolve_type_for_identifier(&word_under_cursor)
    }

    /// Извлечь слово на указанной позиции (line, column)
    fn extract_word_at_position(
        &self,
        file_content: &str,
        line: u32,
        column: u32,
    ) -> Option<String> {
        let lines: Vec<&str> = file_content.lines().collect();
        let current_line = lines.get(line as usize)?;

        // ✅ ИСПРАВЛЕНИЕ: Конвертируем UTF-16 offset → UTF-8 byte offset
        let byte_offset = Self::utf16_to_byte_offset(current_line, column);

        // Ищем символ на позиции byte_offset (в терминах char indices, не bytes!)
        let mut char_index = 0;

        let chars: Vec<char> = current_line.chars().collect();
        for (idx, _ch) in current_line.char_indices() {
            if idx >= byte_offset {
                break;
            }
            char_index += 1;
        }

        if chars.is_empty() || char_index >= chars.len() {
            return None;
        }

        // Ищем начало слова
        let mut start = char_index;
        while start > 0 && Self::is_identifier_char(chars[start - 1]) {
            start -= 1;
        }

        // Ищем конец слова
        let mut end = char_index;
        while end < chars.len() && Self::is_identifier_char(chars[end]) {
            end += 1;
        }

        if start < end {
            Some(chars[start..end].iter().collect())
        } else {
            None
        }
    }

    /// Проверка, является ли символ частью идентификатора BSL
    fn is_identifier_char(c: char) -> bool {
        c.is_alphanumeric() || c == '_' || ('\u{0400}'..='\u{04FF}').contains(&c)
    }

    /// Попытка резолва типа для идентификатора через AnalysisEngine
    fn resolve_type_for_identifier(&self, identifier: &str) -> Option<String> {
        // Domain Layer: Резолв через AnalysisEngine
        let resolution = self.analysis_engine.resolve_type(identifier);

        // Проверяем, нашёлся ли тип
        use bsl_shared::domain::types::Certainty;
        if !matches!(resolution.certainty, Certainty::Unknown) {
            let type_info = self.format_type_for_hover(identifier, &resolution);
            return Some(format!(
                "**Тип платформы:** `{}`\n\n{}",
                identifier, type_info
            ));
        }

        // Если не нашлось — локальная переменная или неизвестный тип
        Some(format!(
            "**Идентификатор:** `{}`\n\n*Информация:* Локальная переменная или неизвестный тип\n\n💡 *Подсказка:* Для точного определения типа требуется flow-sensitive анализ",
            identifier
        ))
    }

    /// Форматировать ResolutionResult для отображения
    fn format_resolution_result(result: &ResolutionResult) -> String {
        match result {
            ResolutionResult::Concrete(concrete_type) => format!("{:?}", concrete_type),
            ResolutionResult::Union(types) => {
                let names: Vec<String> = types.iter().map(|t| format!("{:?}", t)).collect();
                format!("Union<{}>", names.join(" | "))
            }
            ResolutionResult::Dynamic => "Dynamic".to_string(),
            ResolutionResult::Intersection(types) => {
                let names: Vec<String> = types.iter().map(|t| format!("{:?}", t)).collect();
                format!("Intersection<{}>", names.join(" & "))
            }
            ResolutionResult::Generic(generic) => format!("Generic<{}>", generic.base_type),
            ResolutionResult::Nullable(inner) => format!("Nullable<{:?}>", inner),
        }
    }

    /// Форматирует информацию о SemanticNode для hover (Milestone 2.11)
    fn format_semantic_node_info(
        &self,
        node: &bsl_shared::ir::SemanticNode,
        _file_content: &str,
    ) -> String {
        use bsl_shared::ir::SemanticNodeKind;

        match &node.kind {
            SemanticNodeKind::VariableDeclaration {
                name,
                type_hint,
                initial_value_type,
                ..
            } => {
                let type_info = type_hint
                    .as_ref()
                    .or(initial_value_type.as_ref())
                    .map(|t| format!("*Тип:* {}", t))
                    .unwrap_or_else(|| "*Тип:* Неопределено".to_string());

                format!(
                    "**Переменная:** `{}`\n\n{}\n\n📍 Позиция: {}:{}-{}:{}",
                    name,
                    type_info,
                    node.span.start_line,
                    node.span.start_column,
                    node.span.end_line,
                    node.span.end_column
                )
            }
            SemanticNodeKind::Assignment {
                variable,
                value_type,
            } => {
                use bsl_shared::domain::types::Certainty;

                let resolution = self.analysis_engine.resolve_type(value_type);
                let raw_type = self.metadata_lookup.get_raw_type(&resolution);

                // ✅ ИСПРАВЛЕНИЕ: Показываем certainty в пользовательском формате (как в format_variable_hover)
                let mut output = format!("**Присваивание:** `{} = ...`\n", variable);
                output.push_str(&format!("*Тип:* `{}`\n", value_type));

                // ✅ ВСЕГДА показываем certainty
                let certainty_text = match resolution.certainty {
                    Certainty::Known => "🟢 Known (100%)".to_string(),
                    Certainty::Inferred(val) => format!("🟡 Inferred ({:.0}%)", val * 100.0),
                    Certainty::Unknown => "⚪ Unknown (0%)".to_string(),
                };
                output.push_str(&format!("*Уверенность:* {}\n\n", certainty_text));

                // Проверяем наличие RawTypeData для отображения методов/свойств
                if raw_type.is_none() {
                    output.push_str("⚠️ **Детали типа недоступны**\n\n");
                    output.push_str(&format!(
                        "📍 Позиция: {}:{}-{}:{}\n",
                        node.span.start_line,
                        node.span.start_column,
                        node.span.end_line,
                        node.span.end_column
                    ));
                    return output;
                }

                // Если Unknown — показываем дополнительную подсказку
                if matches!(resolution.certainty, Certainty::Unknown) {
                    output.push_str("⚠️ **Тип не распознан системой**\n\n");
                    output.push_str(&format!(
                        "📍 Позиция: {}:{}-{}:{}\n",
                        node.span.start_line,
                        node.span.start_column,
                        node.span.end_line,
                        node.span.end_column
                    ));
                    return output;
                }

                // RawTypeData найден — показываем полную информацию
                let methods = self.metadata_lookup.get_methods(&resolution);
                let properties = self.metadata_lookup.get_properties(&resolution);

                // Форматируем описание из RawTypeData
                let description = raw_type
                    .as_ref()
                    .map(|rt| rt.description.clone())
                    .filter(|d| !d.is_empty())
                    .unwrap_or_else(|| format!("Тип {}", value_type));

                output.push_str(&format!("📝 {}\n\n", description));

                // Добавляем методы (первые 10)
                if !methods.is_empty() {
                    output.push_str("📚 **Методы:**\n");
                    for method in methods.iter().take(10) {
                        let params_str = method
                            .params
                            .iter()
                            .map(|p| format!("{}: {}", p.name, p.param_type))
                            .collect::<Vec<_>>()
                            .join(", ");

                        if !method.return_type.is_empty() {
                            output.push_str(&format!(
                                "- `{}({})` → `{}`\n",
                                method.name, params_str, method.return_type
                            ));
                        } else {
                            output.push_str(&format!("- `{}({})`\n", method.name, params_str));
                        }
                    }
                    if methods.len() > 10 {
                        output.push_str(&format!("- ... и ещё {} методов\n", methods.len() - 10));
                    }
                    output.push('\n');
                }

                // Добавляем свойства (первые 10)
                if !properties.is_empty() {
                    output.push_str("📦 **Свойства:**\n");
                    for prop in properties.iter().take(10) {
                        output.push_str(&format!("- `{}`: `{}`\n", prop.name, prop.prop_type));
                    }
                    if properties.len() > 10 {
                        output
                            .push_str(&format!("- ... и ещё {} свойств\n", properties.len() - 10));
                    }
                    output.push('\n');
                }

                output.push_str(&format!(
                    "📍 Позиция: {}:{}-{}:{}\n",
                    node.span.start_line,
                    node.span.start_column,
                    node.span.end_line,
                    node.span.end_column
                ));

                output
            }
            SemanticNodeKind::FunctionDeclaration {
                name,
                params,
                return_type,
                body,
                ..
            } => {
                let params_str = params
                    .iter()
                    .map(|p| {
                        format!(
                            "{}: {}",
                            p.name,
                            p.type_hint.as_ref().unwrap_or(&"Неопределено".to_string())
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                let return_str = return_type
                    .as_ref()
                    .map(|t| format!("*Возвращает:* {}", t))
                    .unwrap_or_else(|| "*Возвращает:* Неопределено".to_string());

                // ✅ НОВОЕ: Добавляем количество statements в теле
                let body_info = if body.is_empty() {
                    "Тело пустое".to_string()
                } else {
                    format!("Содержит {} узлов", body.len())
                };

                format!(
                    "**Функция:** `{}({})`\n\n{}\n\n📦 {}\n\n📍 Позиция: {}:{}-{}:{}",
                    name,
                    params_str,
                    return_str,
                    body_info,
                    node.span.start_line,
                    node.span.start_column,
                    node.span.end_line,
                    node.span.end_column
                )
            }
            SemanticNodeKind::ProcedureDeclaration {
                name, params, body, ..
            } => {
                let params_str = params
                    .iter()
                    .map(|p| {
                        format!(
                            "{}: {}",
                            p.name,
                            p.type_hint.as_ref().unwrap_or(&"Неопределено".to_string())
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(", ");

                // ✅ НОВОЕ: Добавляем количество statements в теле
                let body_info = if body.is_empty() {
                    "Тело пустое".to_string()
                } else {
                    format!("Содержит {} узлов", body.len())
                };

                format!(
                    "**Процедура:** `{}({})`\n\n📦 {}\n\n📍 Позиция: {}:{}-{}:{}",
                    name,
                    params_str,
                    body_info,
                    node.span.start_line,
                    node.span.start_column,
                    node.span.end_line,
                    node.span.end_column
                )
            }
            SemanticNodeKind::IfStatement { condition_type, .. } => {
                format!(
                    "**Условие:** `Если ... Тогда`\n\n*Условие:* {}\n\n📍 Позиция: {}:{}-{}:{}",
                    condition_type,
                    node.span.start_line,
                    node.span.start_column,
                    node.span.end_line,
                    node.span.end_column
                )
            }
            SemanticNodeKind::WhileLoop { condition_type, .. } => {
                format!(
                    "**Цикл:** `Пока ... Цикл`\n\n*Условие:* {}\n\n📍 Позиция: {}:{}-{}:{}",
                    condition_type,
                    node.span.start_line,
                    node.span.start_column,
                    node.span.end_line,
                    node.span.end_column
                )
            }
            _ => {
                format!(
                    "**Узел IR:** {:?}\n\n📍 Позиция: {}:{}-{}:{}",
                    node.kind,
                    node.span.start_line,
                    node.span.start_column,
                    node.span.end_line,
                    node.span.end_column
                )
            }
        }
    }

    /// Форматирует информацию о переменной для hover (используя Inline Scope Analysis)
    ///
    /// # Аргументы
    /// - `var_name` - имя переменной из IR
    /// - `type_hint` - тип переменной из SymbolTable (Explicit/Inferred/Unknown)
    ///
    /// # Возвращает
    /// Форматированный markdown с информацией о типе, методах и свойствах
    fn format_variable_hover(
        &self,
        var_name: &str,
        type_hint: &bsl_shared::ir::TypeHint,
    ) -> String {
        use bsl_shared::domain::types::Certainty;
        use bsl_shared::ir::TypeHint;

        // Извлекаем имя типа из TypeHint
        let type_name = match type_hint {
            TypeHint::Explicit(t) | TypeHint::Inferred(t) => t.clone(),
            TypeHint::Generic {
                base_type,
                type_params,
                ..
            } => {
                // Generic тип: форматируем как "Массив<Строка>" или "Соответствие<Строка, Число>"
                if type_params.is_empty() {
                    base_type.clone()
                } else {
                    format!("{}<{}>", base_type, type_params.join(", "))
                }
            }
            TypeHint::Unknown => {
                return format!(
                    "**Переменная:** `{}`\n\n*Тип:* Неопределено\n\n💡 *Подсказка:* Переменная объявлена, но тип не выведен из присваивания",
                    var_name
                );
            }
        };

        // Резолвим тип через AnalysisEngine → TypeRepository
        let resolution = self.analysis_engine.resolve_type(&type_name);

        // ✅ НОВОЕ: Специальная обработка для Generic типов
        if let ResolutionResult::Generic(ref generic_type) = resolution.result {
            return self.format_generic_hover(var_name, generic_type, &resolution);
        }

        // Получаем RawTypeData для проверки существования типа
        let raw_type = self.metadata_lookup.get_raw_type(&resolution);

        // DEBUG: Выводим детальную информацию о resolution
        tracing::debug!(
            "Resolution для '{}': certainty={:?}, result={:?}",
            type_name,
            resolution.certainty,
            resolution.result
        );
        tracing::debug!(
            "RawTypeData для '{}': {}",
            type_name,
            if raw_type.is_some() {
                "НАЙДЕН"
            } else {
                "НЕ НАЙДЕН"
            }
        );

        // ✅ ИСПРАВЛЕНИЕ: Формируем базовую информацию ВСЕГДА (независимо от raw_type)
        let mut output = format!("**Переменная:** `{}`\n", var_name);
        output.push_str(&format!("*Тип:* `{}`\n", type_name));

        // ✅ ВСЕГДА показываем certainty (КЛЮЧЕВОЕ: ПЕРЕД проверкой raw_type)
        let certainty_text = match resolution.certainty {
            Certainty::Known => "🟢 Known (100%)".to_string(),
            Certainty::Inferred(val) => format!("🟡 Inferred ({:.0}%)", val * 100.0),
            Certainty::Unknown => "⚪ Unknown (0%)".to_string(),
        };
        output.push_str(&format!("*Уверенность:* {}\n\n", certainty_text));

        // ✅ ТЕПЕРЬ проверяем наличие RawTypeData для отображения методов/свойств
        if raw_type.is_none() {
            output.push_str("⚠️ **Детали типа недоступны**\n\n");
            output.push_str("💡 *Возможные причины:*\n");
            output.push_str("- Тип не загружен из Syntax Helper\n");
            output.push_str("- Требуется парсинг документации платформы\n");
            return output;
        }

        // ✅ Если Unknown — показываем дополнительную подсказку
        if matches!(resolution.certainty, Certainty::Unknown) {
            output.push_str("⚠️ **Тип не распознан системой**\n\n");
            output.push_str("💡 *Возможные причины:*\n");
            output.push_str("- Опечатка в имени типа\n");
            output.push_str("- Тип из конфигурации 1С (требуется Configuration Loader)\n");
            return output;
        }

        // ✅ RawTypeData найден — показываем полную информацию
        let methods = self.metadata_lookup.get_methods(&resolution);
        let properties = self.metadata_lookup.get_properties(&resolution);

        // Форматируем описание из RawTypeData
        let description = raw_type
            .as_ref()
            .map(|rt| rt.description.clone())
            .filter(|d| !d.is_empty())
            .unwrap_or_else(|| format!("Тип {}", type_name));

        output.push_str(&format!("📝 {}\n\n", description));

        // Добавляем методы (первые 10 для краткости)
        if !methods.is_empty() {
            output.push_str("📚 **Методы:**\n");
            for method in methods.iter().take(10) {
                // Форматируем сигнатуру метода: name(params) -> return_type
                let params_str = method
                    .params
                    .iter()
                    .map(|p| format!("{}: {}", p.name, p.param_type))
                    .collect::<Vec<_>>()
                    .join(", ");

                if !method.return_type.is_empty() {
                    output.push_str(&format!(
                        "- `{}({})` → `{}`\n",
                        method.name, params_str, method.return_type
                    ));
                } else {
                    output.push_str(&format!("- `{}({})`\n", method.name, params_str));
                }
            }
            if methods.len() > 10 {
                output.push_str(&format!("- ... и ещё {} методов\n", methods.len() - 10));
            }
            output.push('\n');
        }

        // Добавляем свойства (первые 10)
        if !properties.is_empty() {
            output.push_str("📦 **Свойства:**\n");
            for prop in properties.iter().take(10) {
                // Форматируем свойство: name: type
                output.push_str(&format!("- `{}`: `{}`\n", prop.name, prop.prop_type));
            }
            if properties.len() > 10 {
                output.push_str(&format!("- ... и ещё {} свойств\n", properties.len() - 10));
            }
        }

        output
    }

    /// Форматирует hover для Generic типа (например, ТабличнаяЧасть<СтрокаРаботы>)
    ///
    /// Специальное форматирование для табличных частей с отображением:
    /// - Базового типа и типового параметра
    /// - Методов коллекции (с подставленными типами)
    /// - Атрибутов строки табличной части
    fn format_generic_hover(
        &self,
        var_name: &str,
        generic_type: &bsl_shared::domain::types::GenericType,
        resolution: &TypeResolution,
    ) -> String {
        use bsl_shared::domain::types::{Certainty, ConcreteType};

        tracing::debug!(
            "🎨 Форматирование Generic hover: {} = {}<{}>",
            var_name,
            generic_type.base_type,
            generic_type.type_params.len()
        );

        let mut output = String::new();

        // Заголовок переменной
        output.push_str(&format!("**Переменная:** `{}`\n", var_name));

        // Форматируем полное имя Generic типа
        let full_type_name = if let Some(param_type) = generic_type.type_params.first() {
            let param_name = self.format_concrete_type_name(param_type);
            format!("{}<{}>", generic_type.base_type, param_name)
        } else {
            generic_type.base_type.clone()
        };

        output.push_str(&format!("*Тип:* `{}`\n", full_type_name));

        // Уверенность
        let certainty_text = match resolution.certainty {
            Certainty::Known => "🟢 Known (100%)".to_string(),
            Certainty::Inferred(val) => format!("🟡 Inferred ({:.0}%)", val * 100.0),
            Certainty::Unknown => "⚪ Unknown (0%)".to_string(),
        };
        output.push_str(&format!("*Уверенность:* {}\n\n", certainty_text));

        // ✅ НОВОЕ: Дополнительная информация для табличных частей
        if generic_type.base_type == "ТабличнаяЧасть" {
            if let Some(ConcreteType::TabularRow(row_type)) = generic_type.type_params.first() {
                output.push_str(&format!(
                    "📋 *Табличная часть:* `{}`\n",
                    row_type.tabular_section_name
                ));
                output.push_str(&format!("📄 *Родительский объект:* `{}`\n\n", row_type.parent_type));

                // Методы коллекции
                let methods = self.metadata_lookup.get_methods(resolution);
                if !methods.is_empty() {
                    output.push_str("📚 **Методы коллекции:**\n");
                    for method in methods.iter().take(10) {
                        let params_str = method
                            .params
                            .iter()
                            .map(|p| format!("{}: {}", p.name, p.param_type))
                            .collect::<Vec<_>>()
                            .join(", ");

                        if !method.return_type.is_empty() {
                            output.push_str(&format!(
                                "- `{}({})` → `{}`\n",
                                method.name, params_str, method.return_type
                            ));
                        } else {
                            output.push_str(&format!("- `{}({})`\n", method.name, params_str));
                        }
                    }
                    if methods.len() > 10 {
                        output.push_str(&format!("- ... и ещё {} методов\n", methods.len() - 10));
                    }
                    output.push('\n');
                }

                // ✅ НОВОЕ: Атрибуты строки табличной части
                if !row_type.attributes.is_empty() {
                    output.push_str("📦 **Атрибуты строки:**\n");
                    for attr in row_type.attributes.iter().take(15) {
                        if !attr.attr_type.is_empty() {
                            output.push_str(&format!("- `{}`: `{}`\n", attr.name, attr.attr_type));
                        } else {
                            output.push_str(&format!("- `{}`\n", attr.name));
                        }
                    }
                    if row_type.attributes.len() > 15 {
                        output.push_str(&format!(
                            "- ... и ещё {} атрибутов\n",
                            row_type.attributes.len() - 15
                        ));
                    }
                }
            }
        } else {
            // Для других Generic типов (не табличных частей) - базовый формат
            let methods = self.metadata_lookup.get_methods(resolution);
            if !methods.is_empty() {
                output.push_str("📚 **Методы:**\n");
                for method in methods.iter().take(10) {
                    let params_str = method
                        .params
                        .iter()
                        .map(|p| format!("{}: {}", p.name, p.param_type))
                        .collect::<Vec<_>>()
                        .join(", ");

                    if !method.return_type.is_empty() {
                        output.push_str(&format!(
                            "- `{}({})` → `{}`\n",
                            method.name, params_str, method.return_type
                        ));
                    } else {
                        output.push_str(&format!("- `{}({})`\n", method.name, params_str));
                    }
                }
                if methods.len() > 10 {
                    output.push_str(&format!("- ... и ещё {} методов\n", methods.len() - 10));
                }
            }
        }

        output
    }

    /// Форматирует имя ConcreteType для отображения
    fn format_concrete_type_name(&self, concrete: &bsl_shared::domain::types::ConcreteType) -> String {
        use bsl_shared::domain::types::ConcreteType;

        match concrete {
            ConcreteType::Platform(pt) => pt.name.clone(),
            ConcreteType::Configuration(ct) => {
                // Используем to_prefix() для корректного форматирования
                format!("{}.{}",
                    ct.kind.to_prefix(),
                    ct.name
                )
            }
            ConcreteType::Primitive(prim) => format!("{:?}", prim),
            ConcreteType::Special(spec) => format!("{:?}", spec),
            ConcreteType::GlobalFunction(gf) => gf.name.clone(),
            ConcreteType::TabularRow(tr) => tr.get_full_name(),
        }
    }

    /// Маппинг AST Expression → имя типа (Application Layer ответственность)
    ///
    /// Преобразует синтаксическую конструкцию (Expression) в доменное понятие (имя типа)
    /// для дальнейшего резолва через AnalysisEngine/TypeResolver.
    fn expression_to_type_name(
        &self,
        expr: &crate::parsing::bsl::ast::Expression,
    ) -> Option<String> {
        use crate::parsing::bsl::ast::Expression as Expr;

        match expr {
            Expr::Number { .. } => Some("Число".to_string()),
            Expr::String { .. } => Some("Строка".to_string()),
            Expr::Boolean { .. } => Some("Булево".to_string()),
            Expr::Identifier { name, .. } => Some(name.clone()),
            Expr::New { type_name, .. } => Some(type_name.clone()),
            _ => None, // Сложные выражения требуют расширенного анализа
        }
    }

    /// Форматирует TypeResolution для hover подсказки с полным описанием типа
    fn format_type_for_hover(&self, type_name: &str, resolution: &TypeResolution) -> String {
        let type_str = Self::format_resolution_result(&resolution.result);
        format!(
            "**Тип:** `{}`\n\n*Категория:* {:?}\n*Certainty:* {:?}\n*Структура:* {}",
            type_name, resolution.source, resolution.certainty, type_str
        )
    }

    /// MILESTONE 2.12: Получить семантическое дерево файла
    ///
    /// Парсит файл, конвертирует AST в IR, и возвращает SemanticTreeDto
    pub async fn get_semantic_tree(
        &self,
        file_content: &str,
        file_path: &str,
    ) -> Result<bsl_shared::api::semantic_dtos::SemanticTreeDto> {
        info!("🌳 Generating semantic tree for: {}", file_path);

        // 1. Парсим файл → AST → IR (используем готовый метод из ParserCoordinator)
        let semantic_program = self
            .parser
            .parse_to_ir(file_content, file_path)
            .map_err(|e| anyhow::anyhow!("Failed to parse file: {}", e))?;

        // 2. Конвертируем SemanticProgram → SemanticTreeDto
        let dto = semantic_program.to_dto(
            true, // include_call_graph
            true, // include_flow_sensitive
        );

        info!(
            "✅ Semantic tree generated: {} root nodes, {} symbols, {} metrics: {:?}",
            dto.root_nodes.len(),
            dto.symbol_table.len(),
            dto.metrics.node_count,
            dto.metrics
        );

        Ok(dto)
    }

    /// MILESTONE 2.13: Получить статистику IR Cache
    ///
    /// # Возвращает
    /// Статистику IR кеша (hits, misses, evictions, hit rate)
    pub async fn get_ir_cache_stats(&self) -> crate::system::IrCacheStats {
        self.ir_cache.get_stats().await
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
