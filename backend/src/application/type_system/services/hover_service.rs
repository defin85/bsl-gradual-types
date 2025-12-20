//! Hover Service - hover and type-at-position operations
//!
//! Functions for LSP hover requests and getting type information at cursor position.

use anyhow::Result;
use tracing::{debug, info, warn};

use bsl_shared::domain::types::TypeResolution;
use bsl_shared::domain::type_definition_location::TypeDefinitionLocation;
use bsl_shared::domain::TypeMetadataLookup;
use bsl_shared::engine::AnalysisEngine;
use bsl_shared::ir::{ScopeId, SemanticNodeKind, SemanticProgram};
use bsl_shared::utils::hash::hash_content;

use crate::application::ast_to_ir::AstToIrConverter;
use crate::helpers::hover_formatter::{HoverFormatConfig, HoverFormatter};
use crate::system::{IrCache, ParserCoordinator};

use super::super::extractors::symbol_extractor::extract_word_at_position;
use super::super::formatters::hover_formatters::format_semantic_node_info;

/// LSP operations - get symbol information at position (hover)
///
/// # Arguments
/// * `parser` - ParserCoordinator for parsing
/// * `analysis_engine` - AnalysisEngine for type resolution
/// * `ir_cache` - IR cache for performance
/// * `metadata_lookup` - Lookup for type metadata
/// * `hover_formatter` - Formatter for hover content
/// * `hover_count` - Counter for periodic stats
/// * `file_content` - File content
/// * `line` - Line number (0-based)
/// * `column` - Column number (UTF-16)
/// * `hover_config` - Optional hover configuration
///
/// # Returns
/// Formatted markdown hover content or None
#[allow(clippy::too_many_arguments)]
pub async fn get_hover_info(
    parser: &ParserCoordinator,
    analysis_engine: &AnalysisEngine,
    ir_cache: &IrCache,
    metadata_lookup: &TypeMetadataLookup,
    hover_formatter: &HoverFormatter,
    hover_count: &std::sync::atomic::AtomicU64,
    file_content: &str,
    line: u32,
    column: u32,
    hover_config: Option<HoverFormatConfig>,
) -> Result<Option<String>> {
    get_hover_info_with_file_path(
        parser,
        analysis_engine,
        ir_cache,
        metadata_lookup,
        hover_formatter,
        hover_count,
        file_content,
        "hover_request.bsl",
        line,
        column,
        hover_config,
    )
    .await
}

/// Hover с учётом пути к файлу.
///
/// Важно для модулей форм: `AstToIrConverter` засевает `Объект/Элементы/ЭтаФорма`
/// только если `file_path` распознаётся как `FormModule` по `CodeLocation`.
#[allow(clippy::too_many_arguments)]
pub async fn get_hover_info_with_file_path(
    parser: &ParserCoordinator,
    analysis_engine: &AnalysisEngine,
    ir_cache: &IrCache,
    metadata_lookup: &TypeMetadataLookup,
    hover_formatter: &HoverFormatter,
    hover_count: &std::sync::atomic::AtomicU64,
    file_content: &str,
    file_path: &str,
    line: u32,
    column: u32,
    hover_config: Option<HoverFormatConfig>,
) -> Result<Option<String>> {
    use std::sync::atomic::Ordering;

    // MILESTONE 2.13: Measure total time
    let start = std::time::Instant::now();

    info!("Hover request: line {}, column {}", line, column);

    // MILESTONE 2.13: IR Caching - check cache before parsing
    // Важно: путь влияет на CodeLocation (FormModule), поэтому включаем его в ключ кеша.
    let cache_key = hash_content(&format!("{}\n{}", file_path, file_content));

    // MILESTONE 2.13: Measure parsing time and cache hit flag
    let (ir_program, cache_hit, parse_time) =
        if let Some(cached_ir) = ir_cache.get(cache_key).await {
            let hit_time = start.elapsed();
            info!("IR cache HIT in {:?} for hash {}", hit_time, cache_key);
            (cached_ir, true, std::time::Duration::ZERO)
        } else {
            let parse_start = std::time::Instant::now();
            info!("IR cache MISS for hash {}, parsing...", cache_key);

            // Parse BSL code (only on cache MISS)
            let parse_result = parser
                .parse(file_content)
                .map_err(|e| anyhow::anyhow!("Parse error for hover: {}", e))?;

            // Convert AST -> IR for Inline Scope Analysis
            let repository = analysis_engine.get_repository();
            let signature_index = repository.get_signature_index_clone();
            let resolver = analysis_engine.get_resolver();
            let ir = AstToIrConverter::convert_with_resolver(
                parse_result.program.clone(),
                file_content.to_string(),
                file_path.to_string(),
                repository,
                signature_index,
                Some(resolver),
            )?;

            let ir_arc = std::sync::Arc::new(ir);

            let parse_duration = parse_start.elapsed();
            info!(
                "IR cache MISS, parsed in {:?} for hash {}",
                parse_duration, cache_key
            );

            // Save to cache
            ir_cache.put(cache_key, ir_arc.clone()).await;
            debug!("Cached IR for hash {}", cache_key);

            (ir_arc, false, parse_duration)
        };

    // MILESTONE 2.13: Measure lookup time
    let lookup_start = std::time::Instant::now();

    // Milestone 2.11 Task B1: DEBUG logs for node search
    debug!("Looking for node at position {}:{}", line, column);

    // Специальный кейс: hover на имени свойства (obj.Property) должен показывать тип свойства,
    // а не тип переменной-объекта слева от точки.
    if let Some(node) = ir_program.find_node_at_position(line, column) {
        if let SemanticNodeKind::MemberAccess {
            object_name,
            object_type,
            member_name,
            access_kind,
            result_type,
            ..
        } = &node.kind
        {
            if access_kind.is_property() {
                if let Some(word_under_cursor) =
                    extract_word_at_position(file_content, line, column)
                {
                    if word_under_cursor.eq_ignore_ascii_case(member_name) {
                        let resolver = analysis_engine.get_resolver();

                        // 1) Тип объекта-владельца (flow-sensitive, если объект - переменная)
                        let owner_resolution = if let Some(obj_name) = object_name.as_deref() {
                            if let Some(flow_type) = find_variable_type_at_position(
                                &ir_program,
                                obj_name,
                                node.scope_id,
                                line,
                            ) {
                                flow_type
                            } else {
                                resolver.resolve_variable_with_context(
                                    obj_name,
                                    &ir_program.symbols,
                                    node.scope_id,
                                )
                            }
                        } else {
                            object_type.clone()
                        };

                        // 2) Тип свойства из метаданных (если есть), иначе fallback на result_type узла
                        let (prop_type, is_readonly) = metadata_lookup
                            .get_properties(&owner_resolution)
                            .into_iter()
                            .find(|p| p.name.eq_ignore_ascii_case(member_name))
                            .map(|p| (p.prop_type, Some(p.is_readonly)))
                            .unwrap_or_else(|| (String::new(), None));

                        let property_resolution = if !prop_type.trim().is_empty() {
                            resolver.resolve_expression_sync(&prop_type)
                        } else {
                            resolver.resolve_expression_sync(&result_type.type_name())
                        };
                        let formatter = if let Some(config) = hover_config.clone() {
                            HoverFormatter::new(config, metadata_lookup.clone())
                        } else {
                            hover_formatter.clone()
                        };

                        return Ok(Some(formatter.format_property(
                            object_name.as_deref(),
                            &owner_resolution,
                            member_name,
                            &property_resolution,
                            is_readonly,
                        )));
                    }
                }
            }
        }
    }

    // Direction 2: Use find_variable_with_scope() for Generic inference
    let result = if let Some((var_name, _type_hint, scope_id)) =
        ir_program.find_variable_with_scope(line, column)
    {
        info!(
            "find_variable_with_scope({}, {}) found variable: '{}' in scope {:?}",
            line, column, var_name, scope_id
        );

        // FLOW-SENSITIVE: Find type at specific position (not final!)
        let resolution = if let Some(flow_type) =
            find_variable_type_at_position(&ir_program, &var_name, scope_id, line)
        {
            info!(
                "Flow-sensitive type for '{}' at line {}: {}",
                var_name,
                line,
                flow_type.type_name()
            );
            flow_type
        } else {
            // Fallback: use SymbolTable (final type)
            let resolver = analysis_engine.get_resolver();
            resolver.resolve_variable_with_context(&var_name, &ir_program.symbols, scope_id)
        };

        // MILESTONE 3.6 Phase 1: Use passed config or default
        let formatter = if let Some(config) = hover_config {
            HoverFormatter::new(config, metadata_lookup.clone())
        } else {
            hover_formatter.clone()
        };

        // Format hover via TypeResolution (instead of old TypeHint enum)
        Some(formatter.format_variable(&var_name, &resolution))
    } else {
        // Milestone 2.11 Task B1: Logs when variable not found
        debug!(
            "find_variable_with_scope({}, {}) did not find variable",
            line, column
        );

        // Fallback 1: Try find_node_at_position for other nodes (functions, loops, etc.)
        if let Some(node) = ir_program.find_node_at_position(line, column) {
            info!(
                "find_node_at_position({}, {}) found node (not variable): span={:?}",
                line, column, node.span
            );
            debug!("Found node: {:?} at span {:?}", node.kind, node.span);
            Some(format_semantic_node_info(
                node,
                file_content,
                metadata_lookup,
            ))
        } else {
            // Milestone 2.11 Task B1: Warning when node not found
            warn!("No node found at position {}:{} in IR", line, column);

            // Fallback 2: old logic by variable name (without AST, since IR cache is used now)
            if let Some(symbol_info) = extract_enhanced_symbol_info(
                analysis_engine,
                metadata_lookup,
                file_content,
                line,
                column,
                None,
            ) {
                debug!("Fallback: using extract_enhanced_symbol_info");
                Some(symbol_info)
            } else {
                warn!("Fallback also failed, returning generic BSL symbol message");
                Some(format!("BSL symbol at position {}:{}", line, column))
            }
        }
    };

    // MILESTONE 2.13: Log performance metrics
    let lookup_time = lookup_start.elapsed();
    let total_time = start.elapsed();

    info!(
        "Hover performance: total={:?}, cache_hit={}, parse={:?}, lookup={:?}",
        total_time, cache_hit, parse_time, lookup_time
    );

    // MILESTONE 2.13: Periodic cache stats output (every 100 hovers)
    let current_count = hover_count.fetch_add(1, Ordering::Relaxed);
    if current_count.is_multiple_of(100) {
        let stats = ir_cache.get_stats().await;
        let hit_rate = ir_cache.get_hit_rate().await;
        info!(
            "IR Cache stats after {} hovers: hit_rate={:.1}%, hits={}, misses={}, evictions={}",
            current_count, hit_rate, stats.hits, stats.misses, stats.evictions
        );
    }

    Ok(result)
}

/// Get TypeResolution for symbol at specified position (Go To Definition)
///
/// # Arguments
/// * `parser` - ParserCoordinator for parsing
/// * `analysis_engine` - AnalysisEngine for type resolution
/// * `ir_cache` - IR cache for performance
/// * `file_content` - File content
/// * `line` - Line number (0-based)
/// * `column` - Column number (UTF-16)
///
/// # Returns
/// TypeResolution if found
pub async fn get_type_at_position(
    parser: &ParserCoordinator,
    analysis_engine: &AnalysisEngine,
    ir_cache: &IrCache,
    file_content: &str,
    line: u32,
    column: u32,
) -> Result<Option<TypeResolution>> {
    info!("Get type at position: line {}, column {}", line, column);

    // MILESTONE 2.13: IR Caching - check cache before parsing
    let content_hash = hash_content(file_content);

    let ir_program = if let Some(cached_ir) = ir_cache.get(content_hash).await {
        debug!("IR cache HIT for get_type_at_position");
        cached_ir
    } else {
        debug!("IR cache MISS for get_type_at_position, parsing...");

        // Parse BSL code
        let parse_result = parser
            .parse(file_content)
            .map_err(|e| anyhow::anyhow!("Parse error for get_type_at_position: {}", e))?;

        // Convert AST -> IR
        let repository = analysis_engine.get_repository();
        let signature_index = repository.get_signature_index_clone();
        let ir = AstToIrConverter::convert(
            parse_result.program.clone(),
            file_content.to_string(),
            "definition_request.bsl".to_string(),
            repository,
            signature_index,
        )?;

        let ir_arc = std::sync::Arc::new(ir);
        ir_cache.put(content_hash, ir_arc.clone()).await;
        ir_arc
    };

    // Find variable at position via SymbolTable
    if let Some((var_name, _type_hint, scope_id)) =
        ir_program.find_variable_with_scope(line, column)
    {
        info!(
            "Found variable '{}' in scope {:?} at {}:{}",
            var_name, scope_id, line, column
        );

        // Resolve variable via TypeResolver with SymbolTable context
        let resolver = analysis_engine.get_resolver();
        let resolution =
            resolver.resolve_variable_with_context(&var_name, &ir_program.symbols, scope_id);

        return Ok(Some(resolution));
    }

    // Fallback: try find_node_at_position for other nodes
    if let Some(node) = ir_program.find_node_at_position(line, column) {
        debug!("Found node at position: {:?}", node.kind);

        // Extract type from node depending on its kind
        match &node.kind {
            SemanticNodeKind::VariableDeclaration {
                type_hint: Some(resolution),
                ..
            } => {
                // Phase 3: type_hint is now Option<TypeResolution>
                let resolver = analysis_engine.get_resolver();
                return Ok(Some(
                    resolver.resolve_expression_sync(&resolution.type_name()),
                ));
            }
            SemanticNodeKind::FunctionCall {
                object_type: Some(type_resolution),
                ..
            } => {
                // Phase 3: object_type is now Option<TypeResolution>
                let resolver = analysis_engine.get_resolver();
                return Ok(Some(
                    resolver.resolve_expression_sync(&type_resolution.type_name()),
                ));
            }
            SemanticNodeKind::MemberAccess { object_type, .. } => {
                // Phase 3: object_type is now TypeResolution
                let resolver = analysis_engine.get_resolver();
                return Ok(Some(
                    resolver.resolve_expression_sync(&object_type.type_name()),
                ));
            }
            SemanticNodeKind::NewExpression { type_name, .. } => {
                let resolver = analysis_engine.get_resolver();
                return Ok(Some(resolver.resolve_expression_sync(type_name)));
            }
            _ => {}
        }
    }

    debug!("No type found at position {}:{}", line, column);
    Ok(None)
}

/// Get definition location for method/function at specified position (Go To Definition for methods)
pub async fn get_method_definition_at_position(
    parser: &ParserCoordinator,
    analysis_engine: &AnalysisEngine,
    ir_cache: &IrCache,
    file_content: &str,
    file_path: Option<&str>,
    line: u32,
    column: u32,
) -> Result<Option<TypeDefinitionLocation>> {
    let content_hash = hash_content(file_content);

    let ir_program = if let Some(cached_ir) = ir_cache.get(content_hash).await {
        cached_ir
    } else {
        let parse_result = parser
            .parse(file_content)
            .map_err(|e| anyhow::anyhow!("Parse error for get_method_definition_at_position: {}", e))?;

        let repository = analysis_engine.get_repository();
        let signature_index = repository.get_signature_index_clone();
        let ir = AstToIrConverter::convert(
            parse_result.program.clone(),
            file_content.to_string(),
            "definition_request.bsl".to_string(),
            repository,
            signature_index,
        )?;

        let ir_arc = std::sync::Arc::new(ir);
        ir_cache.put(content_hash, ir_arc.clone()).await;
        ir_arc
    };

    let Some(node) = ir_program.find_node_at_position(line, column) else {
        return Ok(None);
    };

    let repo = analysis_engine.get_repository();

    match &node.kind {
        SemanticNodeKind::FunctionCall {
            function_name,
            object_type: Some(object_type),
            ..
        } => {
            let owner = object_type.type_name();
            if let Some(loc) = repo.find_method_definition_location(Some(&owner), function_name) {
                return Ok(Some(loc));
            }
        }
        SemanticNodeKind::FunctionCall {
            function_name,
            object_type: None,
            ..
        } => {
            if let Some(loc) = repo.find_method_definition_location(None, function_name) {
                return Ok(Some(loc));
            }

            // Fallback: локальная функция/процедура (в т.ч. приватная) в текущем файле.
            if let Some(file_path) = file_path {
                let parse_result = parser
                    .parse(file_content)
                    .map_err(|e| anyhow::anyhow!("Parse error for local method lookup: {}", e))?;

                for st in parse_result.program.statements {
                    match st {
                        crate::parsing::bsl::ast::Statement::FunctionDecl { name, span, .. }
                        | crate::parsing::bsl::ast::Statement::ProcedureDecl { name, span, .. } => {
                            if name.eq_ignore_ascii_case(function_name) {
                                return Ok(Some(TypeDefinitionLocation::user_defined(
                                    std::path::PathBuf::from(file_path),
                                    span.start_line,
                                    span.start_column,
                                    span.end_line,
                                    span.end_column,
                                )));
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        _ => {}
    }

    Ok(None)
}

/// Invalidate cache for changed file (MILESTONE 2.13)
///
/// Called from LSP on `didChange` notification.
/// Updates mapping URI -> Hash. Old IR stays in cache until eviction,
/// but won't be used since hash changed.
///
/// # Arguments
/// * `uri_to_hash` - RwLock mapping file URI to content hash
/// * `file_uri` - URI of the changed file
/// * `new_content` - New file content
pub async fn invalidate_file_cache(
    uri_to_hash: &tokio::sync::RwLock<std::collections::HashMap<String, u64>>,
    file_uri: &str,
    new_content: &str,
) {
    let old_hash = uri_to_hash.read().await.get(file_uri).copied();
    let new_hash = hash_content(new_content);

    // Log hash change (old IR not explicitly removed, will be evicted by LRU)
    if let Some(old) = old_hash {
        if old != new_hash {
            debug!(
                "Invalidated cache for {} (old hash: {}, new hash: {})",
                file_uri, old, new_hash
            );
        }
    }

    // Update mapping URI -> Hash
    uri_to_hash
        .write()
        .await
        .insert(file_uri.to_string(), new_hash);
}

// === Helper functions ===

/// Flow-sensitive search for variable type at specified position
///
/// Searches for the last assignment to the variable BEFORE the specified line,
/// to show the actual type at cursor position.
fn find_variable_type_at_position(
    ir_program: &SemanticProgram,
    var_name: &str,
    target_scope: ScopeId,
    line: u32,
) -> Option<TypeResolution> {
    let mut assignments: Vec<(u32, TypeResolution)> = Vec::new();

    for node in &ir_program.nodes {
        // Check scope visibility (current or parent)
        if !is_scope_visible(ir_program, node.scope_id, target_scope) {
            continue;
        }

        match &node.kind {
            // Variable assignment
            SemanticNodeKind::Assignment {
                variable,
                value_type,
                ..
            } if variable.eq_ignore_ascii_case(var_name) && node.span.start_line <= line => {
                assignments.push((node.span.start_line, value_type.clone()));
            }
            // Declaration with initialization
            SemanticNodeKind::VariableDeclaration {
                name,
                initial_value_type: Some(value_type),
                ..
            } if name.eq_ignore_ascii_case(var_name) && node.span.start_line <= line => {
                assignments.push((node.span.start_line, value_type.clone()));
            }
            _ => {}
        }
    }

    // Sort by line and take last assignment
    assignments.sort_by_key(|(line, _)| *line);
    assignments.last().map(|(_, res)| res.clone())
}

/// Check scope visibility from another scope
fn is_scope_visible(
    ir_program: &SemanticProgram,
    source_scope: ScopeId,
    target_scope: ScopeId,
) -> bool {
    if source_scope == target_scope {
        return true;
    }

    // Check parent chain
    let mut current = Some(target_scope);
    while let Some(scope_id) = current {
        if scope_id == source_scope {
            return true;
        }
        current = ir_program
            .symbols
            .scopes
            .get(&scope_id)
            .and_then(|s| s.parent);
    }
    false
}

/// Extract symbol information at specified position from AST
fn extract_enhanced_symbol_info(
    analysis_engine: &AnalysisEngine,
    _metadata_lookup: &TypeMetadataLookup,
    file_content: &str,
    line: u32,
    column: u32,
    parse_result: Option<&crate::parsing::Program>,
) -> Option<String> {
    use super::super::extractors::type_extractor::expression_to_type_name;
    use crate::parsing::bsl::ast::{Expression, Statement};

    // Step 1: Extract word under cursor
    let word_under_cursor = extract_word_at_position(file_content, line, column)?;

    // Step 2: If AST exists, look for word info in it
    if let Some(parse_result) = parse_result {
        for statement in &parse_result.statements {
            match statement {
                Statement::VarDeclaration { name, .. } if name == &word_under_cursor => {
                    return Some(format!("**Переменная:** `{}`\n\n*Тип:* Неопределено (требуется flow-sensitive анализ)", name));
                }
                #[allow(clippy::collapsible_match)]
                Statement::Assignment { target, value, .. } => {
                    if let Expression::Identifier { name: var_name, .. } = target {
                        if var_name == &word_under_cursor {
                            // Application Layer: Map AST -> type name
                            if let Some(type_name) = expression_to_type_name(value) {
                                // Domain Layer: Resolve via AnalysisEngine
                                let resolution = analysis_engine.resolve_type(&type_name);
                                let type_info = format_type_for_hover(&type_name, &resolution);
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
                Statement::FunctionDecl {
                    name,
                    params,
                    compiler_directive,
                    is_export: _,
                    ..
                } if name == &word_under_cursor => {
                    let directive_str = match compiler_directive {
                        Some(d) => format!("\n\n*Директива:* {:?}", d),
                        None => String::new(),
                    };
                    return Some(format!(
                        "**Функция:** `{}({})`\n\n*Параметры:* {}{}",
                        name,
                        params.join(", "),
                        params.len(),
                        directive_str
                    ));
                }
                Statement::ProcedureDecl {
                    name,
                    params,
                    compiler_directive,
                    is_export: _,
                    ..
                } if name == &word_under_cursor => {
                    let directive_str = match compiler_directive {
                        Some(d) => format!("\n\n*Директива:* {:?}", d),
                        None => String::new(),
                    };
                    return Some(format!(
                        "**Процедура:** `{}({})`\n\n*Параметры:* {}{}",
                        name,
                        params.join(", "),
                        params.len(),
                        directive_str
                    ));
                }
                _ => {}
            }
        }
    }

    // Step 3: Fallback - try to resolve type for identifier via AnalysisEngine
    resolve_type_for_identifier(analysis_engine, &word_under_cursor)
}

/// Try to resolve type for identifier via AnalysisEngine
fn resolve_type_for_identifier(
    analysis_engine: &AnalysisEngine,
    identifier: &str,
) -> Option<String> {
    use bsl_shared::domain::types::Certainty;

    // Domain Layer: Resolve via AnalysisEngine
    let resolution = analysis_engine.resolve_type(identifier);

    // Check if type was found
    if !matches!(resolution.certainty, Certainty::Unknown) {
        let type_info = format_type_for_hover(identifier, &resolution);
        return Some(format!(
            "**Тип платформы:** `{}`\n\n{}",
            identifier, type_info
        ));
    }

    // If not found - local variable or unknown type
    Some(format!(
        "**Идентификатор:** `{}`\n\n*Информация:* Локальная переменная или неизвестный тип\n\n*Подсказка:* Для точного определения типа требуется flow-sensitive анализ",
        identifier
    ))
}

/// Format TypeResolution for hover tooltip with full type description
fn format_type_for_hover(type_name: &str, resolution: &TypeResolution) -> String {
    use super::super::formatters::type_formatters::format_resolution_result;

    let type_str = format_resolution_result(&resolution.result);
    format!(
        "**Тип:** `{}`\n\n*Категория:* {:?}\n*Certainty:* {:?}\n*Структура:* {}",
        type_name, resolution.source, resolution.certainty, type_str
    )
}
