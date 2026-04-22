//! Объявления: function_definition, var_definition
//!
//! Модуль содержит конвертеры для объявлений функций, процедур и переменных.
//! Использует dispatcher для рекурсивной обработки тела функций.

use crate::ast::Statement;
use tree_sitter::Node;

use crate::tree_sitter_adapter::directives::find_preceding_directive;
use crate::tree_sitter_adapter::span::{node_to_span_cached, LineIndex};
use crate::tree_sitter_adapter::utils::{convert_parameters, node_text};
use crate::tree_sitter_adapter::RoutineBodyLoweringReusePlan;

/// Конвертировать function_definition с использованием кеша строк (Milestone 2.19)
///
/// Вызывает dispatcher для рекурсивной обработки тела функции/процедуры.
pub(crate) fn convert_function_definition_cached(
    node: &Node,
    source: &str,
    line_index: &LineIndex,
    progress: &mut super::LoweringProgressState,
    observer: &mut super::LoweringObserver<'_>,
    execution_attribution: Option<&mut super::LoweringExecutionAttribution>,
) -> Result<Statement, String> {
    convert_function_definition_cached_internal(
        node,
        source,
        line_index,
        progress,
        observer,
        None,
        execution_attribution,
    )
}

pub(crate) fn convert_function_definition_cached_with_body_reuse(
    node: &Node,
    source: &str,
    line_index: &LineIndex,
    progress: &mut super::LoweringProgressState,
    observer: &mut super::LoweringObserver<'_>,
    body_reuse: &mut RoutineBodyLoweringReusePlan,
    execution_attribution: &mut super::LoweringExecutionAttribution,
) -> Result<Statement, String> {
    convert_function_definition_cached_internal(
        node,
        source,
        line_index,
        progress,
        observer,
        Some(body_reuse),
        Some(execution_attribution),
    )
}

fn convert_function_definition_cached_internal(
    node: &Node,
    source: &str,
    line_index: &LineIndex,
    progress: &mut super::LoweringProgressState,
    observer: &mut super::LoweringObserver<'_>,
    mut body_reuse: Option<&mut RoutineBodyLoweringReusePlan>,
    mut execution_attribution: Option<&mut super::LoweringExecutionAttribution>,
) -> Result<Statement, String> {
    let callable_started = std::time::Instant::now();
    let span = node_to_span_cached(node, source, line_index);
    let mut cursor = node.walk();
    let mut name = String::new();
    let mut params = Vec::new();
    let mut body = Vec::new();
    let is_procedure = node.kind() == "procedure_definition";
    let mut is_export = false;
    let mut lowered_body_index = 0usize;
    let mut callable_body_dispatch_elapsed = std::time::Duration::default();
    let mut callable_body_dispatch_call_count = 0u64;

    // Ищем директиву компилятора перед функцией/процедурой
    let compiler_directive = find_preceding_directive(node, source)?;

    for child in node.children(&mut cursor) {
        match child.kind() {
            "identifier" => {
                if name.is_empty() {
                    name = node_text(&child, source)?;
                }
            }
            "parameters" => {
                params = convert_parameters(&child, source)?;
            }
            _ if child.kind().ends_with("_KEYWORD") => {
                let kw = node_text(&child, source)?.trim().to_lowercase();
                if kw == "экспорт" || kw == "export" {
                    is_export = true;
                }
            }
            _ => {
                if super::is_lowering_progress_unit_kind(child.kind()) {
                    if let Some(plan) = body_reuse.as_mut() {
                        let plan = &mut **plan;
                        if lowered_body_index < plan.reused_prefix_len {
                            let reused = plan
                                .reused_body_prefix
                                .pop_front()
                                .expect("reused body prefix entry must exist");
                            let started = std::time::Instant::now();
                            super::observe_reused_statement_progress(&reused, progress, observer)?;
                            if let Some(execution_attribution) = execution_attribution.as_mut() {
                                let execution_attribution = &mut **execution_attribution;
                                execution_attribution.reused_progress_elapsed =
                                    execution_attribution
                                        .reused_progress_elapsed
                                        .saturating_add(started.elapsed());
                                execution_attribution.reused_progress_call_count =
                                    execution_attribution
                                        .reused_progress_call_count
                                        .saturating_add(1);
                            }
                            body.push(reused);
                            lowered_body_index = lowered_body_index.saturating_add(1);
                            continue;
                        }
                        if lowered_body_index >= plan.reused_suffix_start {
                            let reused = plan
                                .reused_body_suffix
                                .pop_front()
                                .expect("reused body suffix entry must exist");
                            let started = std::time::Instant::now();
                            super::observe_reused_statement_progress(&reused, progress, observer)?;
                            if let Some(execution_attribution) = execution_attribution.as_mut() {
                                let execution_attribution = &mut **execution_attribution;
                                execution_attribution.reused_progress_elapsed =
                                    execution_attribution
                                        .reused_progress_elapsed
                                        .saturating_add(started.elapsed());
                                execution_attribution.reused_progress_call_count =
                                    execution_attribution
                                        .reused_progress_call_count
                                        .saturating_add(1);
                            }
                            body.push(reused);
                            lowered_body_index = lowered_body_index.saturating_add(1);
                            continue;
                        }
                    }
                    lowered_body_index = lowered_body_index.saturating_add(1);
                }
                // Собираем тело функции через dispatcher
                let node_kind = child.kind();
                let started = std::time::Instant::now();
                let maybe_stmt = if let Some(execution_attribution) = execution_attribution.as_mut()
                {
                    let execution_attribution = &mut **execution_attribution;
                    super::dispatch_statement_cached_internal_with_attribution(
                        &child,
                        source,
                        line_index,
                        progress,
                        observer,
                        Some(execution_attribution),
                    )?
                } else {
                    super::dispatch_statement_cached_internal(
                        &child, source, line_index, progress, observer,
                    )?
                };
                if let Some(stmt) = maybe_stmt {
                    body.push(stmt);
                }
                let elapsed = started.elapsed();
                callable_body_dispatch_elapsed =
                    callable_body_dispatch_elapsed.saturating_add(elapsed);
                callable_body_dispatch_call_count =
                    callable_body_dispatch_call_count.saturating_add(1);
                if let Some(execution_attribution) = execution_attribution.as_mut() {
                    let execution_attribution = &mut **execution_attribution;
                    super::record_rebuild_dispatch_attribution(
                        execution_attribution,
                        node_kind,
                        elapsed,
                    );
                }
            }
        }
    }

    if let Some(execution_attribution) = execution_attribution.as_mut() {
        let execution_attribution = &mut **execution_attribution;
        execution_attribution.rebuild_dispatch_callable_body_dispatch_elapsed =
            execution_attribution
                .rebuild_dispatch_callable_body_dispatch_elapsed
                .saturating_add(callable_body_dispatch_elapsed);
        execution_attribution.rebuild_dispatch_callable_body_dispatch_call_count =
            execution_attribution
                .rebuild_dispatch_callable_body_dispatch_call_count
                .saturating_add(callable_body_dispatch_call_count);
        execution_attribution.rebuild_dispatch_callable_non_body_dispatch_elapsed =
            execution_attribution
                .rebuild_dispatch_callable_non_body_dispatch_elapsed
                .saturating_add(
                    callable_started
                        .elapsed()
                        .saturating_sub(callable_body_dispatch_elapsed),
                );
    }

    if is_procedure {
        Ok(Statement::ProcedureDecl {
            name,
            params,
            body,
            compiler_directive,
            is_export,
            span,
        })
    } else {
        Ok(Statement::FunctionDecl {
            name,
            params,
            body,
            compiler_directive,
            is_export,
            span,
        })
    }
}

/// Конвертировать var_definition с использованием кеша строк (Milestone 2.19)
pub(crate) fn convert_var_definition_cached(
    node: &Node,
    source: &str,
    line_index: &LineIndex,
) -> Result<Statement, String> {
    let mut cursor = node.walk();
    let mut name = String::new();

    for child in node.children(&mut cursor) {
        if child.kind() == "identifier" {
            name = node_text(&child, source)?;
            break;
        }
    }

    Ok(Statement::VarDeclaration {
        name,
        type_hint: None,
        span: node_to_span_cached(node, source, line_index),
    })
}
