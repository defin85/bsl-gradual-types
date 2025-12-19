//! Индексация BSL модулей конфигурации (CommonModule/ObjectModule/ManagerModule/RecordSetModule)
//!
//! Цель: извлечь экспортные процедуры/функции из модулей конфигурации и
//! зарегистрировать их в SignatureIndex как `SignatureSource::Configuration`.
//!
//! На первом этапе извлекается только имя, список параметров и признак `Экспорт`.
//! Типы параметров и возвращаемых значений добавляются отдельными этапами (см. roadmap).

use crate::data::loaders::config_metadata_parser::types::CommonModuleProperties;
use crate::data::loaders::UniversalMetadataObject;
use crate::system::fs_utils::read_bsl_file;
use crate::system::tree_sitter_adapter::TreeSitterAdapter;
use crate::system::tree_sitter_adapter::directives::find_preceding_directive;
use crate::system::tree_sitter_adapter::span::{LineIndex, node_to_span_cached};
use crate::system::tree_sitter_adapter::utils::{convert_parameters, node_text};
use anyhow::{anyhow, Result};
use bsl_shared::domain::code_location::{CodeLocation, ModuleType};
use bsl_shared::domain::signature_index::{ContextRequirements, MethodSignature, SignatureSource};
use bsl_shared::domain::type_definition_location::TypeDefinitionLocation;
use bsl_shared::domain::types::{FacetKind, MetadataKind, ParameterInfo};
use rayon::prelude::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex, OnceLock,
};
use std::time::{Duration, Instant};
use tree_sitter::Parser;

#[derive(Debug, Clone)]
pub struct ModuleIndexProgress {
    pub current: usize,
    pub total: usize,
    pub module_path: PathBuf,
}

#[derive(Debug, Default)]
pub struct IndexedConfigSignatures {
    pub config_methods: Vec<(String, MethodSignature)>,
    pub global_functions: Vec<(String, MethodSignature)>,
    pub definition_locations: Vec<(String, String, TypeDefinitionLocation)>,
    pub global_definition_locations: Vec<(String, TypeDefinitionLocation)>,
}

#[derive(Debug)]
pub struct ModuleParseStats {
    pub decls: usize,
    pub export_decls: usize,
    pub call_sites: usize,
}

#[derive(Debug, Clone, Copy)]
pub enum SinglePassMode {
    Lite,
    Full,
}

#[derive(Debug)]
pub struct ModuleParseComparison {
    pub module_path: PathBuf,
    pub single_pass: ModuleParseStats,
    pub ast: ModuleParseStats,
    pub missing_decls: Vec<String>,
    pub extra_decls: Vec<String>,
    pub callsite_mismatches: Vec<String>,
}

#[derive(Debug)]
struct ParsedDecl {
    name: String,
    params: Vec<String>,
    is_export: bool,
    directive_ctx: Option<ContextRequirements>,
    return_type: Option<String>,
    span: crate::parsing::bsl::ast::Span,
}

#[derive(Debug)]
struct ParsedModuleData {
    decls: Vec<ParsedDecl>,
    call_sites: Vec<CallSite>,
}

#[derive(Debug)]
struct ParsedModule {
    owner_type_name: String,
    module_type: ModuleType,
    is_global_common_module: bool,
    module_path: PathBuf,
    decls: Vec<ParsedDecl>,
    call_sites: Vec<CallSite>,
}

#[derive(Debug)]
enum CallTarget {
    /// Невозможно различить между "локальной" функцией и глобальной (например, из Global common module),
    /// поэтому этот таргет резолвим только в рамках текущего модуля.
    LocalFunction { name: String },
    /// Вызов вида `ModuleName.Method(...)` или `Справочники.Номенклатура.Метод(...)`
    QualifiedMethod { receiver: Vec<String>, name: String },
}

#[derive(Debug)]
struct CallSite {
    target: CallTarget,
    arg_types: Vec<Option<String>>,
}

pub fn index_configuration_bsl_modules(
    config_root: &Path,
    metadata: &[UniversalMetadataObject],
) -> Result<IndexedConfigSignatures> {
    index_configuration_bsl_modules_with_progress::<fn(ModuleIndexProgress)>(
        config_root,
        metadata,
        None,
    )
}

pub fn index_configuration_bsl_modules_with_progress<F>(
    config_root: &Path,
    metadata: &[UniversalMetadataObject],
    mut progress_callback: Option<F>,
) -> Result<IndexedConfigSignatures>
where
    F: FnMut(ModuleIndexProgress),
{
    let metrics = parse_metrics_config();
    let common_module_props = collect_common_module_props(metadata);
    let all_module_paths = collect_module_paths(config_root, metadata);

    let mut out = IndexedConfigSignatures::default();

    let mut parsed_modules: Vec<ParsedModule> = Vec::new();
    let mut slow_modules: Vec<(Duration, PathBuf)> = Vec::new();

    let total = all_module_paths.len();
    for (idx, module_path) in all_module_paths.into_iter().enumerate() {
        if let Some(ref mut cb) = progress_callback {
            cb(ModuleIndexProgress {
                current: idx + 1,
                total,
                module_path: module_path.clone(),
            });
        }

        let Ok(location) = CodeLocation::determine_from_path(&module_path) else {
            continue;
        };

        let Some(owner_type_name) =
            resolve_owner_type_for_signature(&location.module_type, &common_module_props)
        else {
            continue;
        };

        let source = match read_bsl_file(&module_path) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!("Не удалось прочитать {:?}: {}", module_path, e);
                continue;
            }
        };

        let parse_started = Instant::now();
        let parsed = match parse_bsl_module(&source, &module_path) {
            Ok(p) => p,
            Err(e) => {
                let elapsed = parse_started.elapsed();
                if elapsed >= metrics.slow_threshold {
                    slow_modules.push((elapsed, module_path.clone()));
                }
                tracing::warn!("Не удалось распарсить модуль {:?}: {}", module_path, e);
                continue;
            }
        };
        let elapsed = parse_started.elapsed();
        if metrics.log_each {
            tracing::info!(
                "Парсинг модуля завершён ({}): {:?}",
                human_duration(elapsed),
                module_path
            );
        }
        if elapsed >= metrics.slow_threshold {
            slow_modules.push((elapsed, module_path.clone()));
        }

        let module_type = location.module_type;
        let is_global = is_global_common_module(&module_type, &common_module_props);

        parsed_modules.push(ParsedModule {
            owner_type_name,
            module_type,
            is_global_common_module: is_global,
            module_path: module_path.clone(),
            decls: parsed.decls,
            call_sites: parsed.call_sites,
        });
    }

    let inferred_param_types = infer_export_param_types_across_modules(&parsed_modules);

    for module in parsed_modules {
        let module_context = module_context_requirements(&module.module_type, &common_module_props);

        for decl in module.decls {
            if !decl.is_export {
                continue;
            }

            let method_name = decl.name.clone();
            let ctx = decl.directive_ctx.unwrap_or(module_context);
            let inferred_for_decl = inferred_param_types.get(&(module.owner_type_name.clone(), decl.name.clone()));

            let signature = MethodSignature::new(
                method_name.clone(),
                Some(module.owner_type_name.clone()),
                decl.params
                    .into_iter()
                    .enumerate()
                    .map(|(idx, p)| ParameterInfo {
                        name: p,
                        type_name: inferred_for_decl
                            .and_then(|v| v.get(idx))
                            .cloned()
                            .flatten(),
                        is_optional: false,
                        default_value: None,
                        description: None,
                    })
                    .collect(),
                decl.return_type.clone(),
                SignatureSource::Configuration,
                None, // return_facet: неизвестен на этапе извлечения
                ctx,
            );

            out.config_methods.push((module.owner_type_name.clone(), signature.clone()));

            if module.is_global_common_module {
                let mut global_sig = signature;
                global_sig.owner_type = None;
                out.global_functions.push((method_name.clone(), global_sig));
                out.global_definition_locations.push((
                    method_name.clone(),
                    TypeDefinitionLocation::user_defined(
                        module.module_path.clone(),
                        decl.span.start_line,
                        decl.span.start_column,
                        decl.span.end_line,
                        decl.span.end_column,
                    ),
                ));
            }

            out.definition_locations.push((
                module.owner_type_name.clone(),
                method_name,
                TypeDefinitionLocation::user_defined(
                    module.module_path.clone(),
                    decl.span.start_line,
                    decl.span.start_column,
                    decl.span.end_line,
                    decl.span.end_column,
                ),
            ));
        }
    }

    report_slow_modules(&mut slow_modules);
    sort_indexed_signatures(&mut out);
    Ok(out)
}

pub fn index_configuration_bsl_modules_with_progress_parallel<F>(
    config_root: &Path,
    metadata: &[UniversalMetadataObject],
    progress_callback: Option<F>,
) -> Result<IndexedConfigSignatures>
where
    F: Fn(ModuleIndexProgress) + Send + Sync + 'static,
{
    let metrics = parse_metrics_config();
    let common_module_props = collect_common_module_props(metadata);
    let all_module_paths = collect_module_paths(config_root, metadata);
    let total = all_module_paths.len();

    let progress_callback = progress_callback.map(Arc::new);
    let progress_counter = AtomicUsize::new(0);
    let slow_modules = Arc::new(Mutex::new(Vec::new()));

    let parsed_modules: Vec<ParsedModule> = all_module_paths
        .par_iter()
        .filter_map(|module_path| {
            let parse_started = Instant::now();
            let mut parsed_module: Option<ParsedModule> = None;

            let location = CodeLocation::determine_from_path(module_path);
            if let Ok(location) = location {
                if let Some(owner_type_name) =
                    resolve_owner_type_for_signature(&location.module_type, &common_module_props)
                {
                    let source = match read_bsl_file(module_path) {
                        Ok(s) => Some(s),
                        Err(e) => {
                            tracing::warn!("Не удалось прочитать {:?}: {}", module_path, e);
                            None
                        }
                    };

                    if let Some(source) = source {
                        let parsed = match parse_bsl_module(&source, module_path) {
                            Ok(p) => Some(p),
                            Err(e) => {
                                tracing::warn!(
                                    "Не удалось распарсить модуль {:?}: {}",
                                    module_path,
                                    e
                                );
                                None
                            }
                        };

                        if let Some(parsed) = parsed {
                            let module_type = location.module_type;
                            let is_global =
                                is_global_common_module(&module_type, &common_module_props);

                            parsed_module = Some(ParsedModule {
                                owner_type_name,
                                module_type,
                                is_global_common_module: is_global,
                                module_path: module_path.clone(),
                                decls: parsed.decls,
                                call_sites: parsed.call_sites,
                            });
                        }
                    }
                }
            }

            let elapsed = parse_started.elapsed();
            if metrics.log_each {
                tracing::info!(
                    "Парсинг модуля завершён ({}): {:?}",
                    human_duration(elapsed),
                    module_path
                );
            }
            if elapsed >= metrics.slow_threshold {
                if let Ok(mut guard) = slow_modules.lock() {
                    guard.push((elapsed, module_path.clone()));
                }
            }

            let current = progress_counter.fetch_add(1, Ordering::Relaxed) + 1;
            if let Some(ref cb) = progress_callback {
                cb(ModuleIndexProgress {
                    current,
                    total,
                    module_path: module_path.clone(),
                });
            }

            parsed_module
        })
        .collect();

    let mut out = IndexedConfigSignatures::default();
    let inferred_param_types = infer_export_param_types_across_modules(&parsed_modules);

    for module in parsed_modules {
        let module_context = module_context_requirements(&module.module_type, &common_module_props);

        for decl in module.decls {
            if !decl.is_export {
                continue;
            }

            let method_name = decl.name.clone();
            let ctx = decl.directive_ctx.unwrap_or(module_context);
            let inferred_for_decl =
                inferred_param_types.get(&(module.owner_type_name.clone(), decl.name.clone()));

            let signature = MethodSignature::new(
                method_name.clone(),
                Some(module.owner_type_name.clone()),
                decl.params
                    .into_iter()
                    .enumerate()
                    .map(|(idx, p)| ParameterInfo {
                        name: p,
                        type_name: inferred_for_decl
                            .and_then(|v| v.get(idx))
                            .cloned()
                            .flatten(),
                        is_optional: false,
                        default_value: None,
                        description: None,
                    })
                    .collect(),
                decl.return_type.clone(),
                SignatureSource::Configuration,
                None, // return_facet: неизвестен на этапе извлечения
                ctx,
            );

            out.config_methods
                .push((module.owner_type_name.clone(), signature.clone()));

            if module.is_global_common_module {
                let mut global_sig = signature;
                global_sig.owner_type = None;
                out.global_functions
                    .push((method_name.clone(), global_sig));
                out.global_definition_locations.push((
                    method_name.clone(),
                    TypeDefinitionLocation::user_defined(
                        module.module_path.clone(),
                        decl.span.start_line,
                        decl.span.start_column,
                        decl.span.end_line,
                        decl.span.end_column,
                    ),
                ));
            }

            out.definition_locations.push((
                module.owner_type_name.clone(),
                method_name,
                TypeDefinitionLocation::user_defined(
                    module.module_path.clone(),
                    decl.span.start_line,
                    decl.span.start_column,
                    decl.span.end_line,
                    decl.span.end_column,
                ),
            ));
        }
    }

    if let Ok(mut guard) = slow_modules.lock() {
        report_slow_modules(&mut guard);
    }
    sort_indexed_signatures(&mut out);

    Ok(out)
}

fn collect_common_module_props(
    metadata: &[UniversalMetadataObject],
) -> HashMap<String, CommonModuleProperties> {
    metadata
        .iter()
        .filter(|o| o.object_type == Some(MetadataKind::CommonModule))
        .filter_map(|o| o.common_module_properties.clone().map(|p| (o.name.clone(), p)))
        .collect()
}

fn collect_module_paths(
    config_root: &Path,
    metadata: &[UniversalMetadataObject],
) -> Vec<PathBuf> {
    let mut all_module_paths: Vec<PathBuf> = Vec::new();

    // Common modules: вычисляем путь напрямую по структуре конфигурации
    for obj in metadata {
        if obj.object_type == Some(MetadataKind::CommonModule) {
            let module_path = config_root
                .join("CommonModules")
                .join(&obj.name)
                .join("Ext")
                .join("Module.bsl");
            if module_path.exists() {
                all_module_paths.push(module_path);
            }
        }
    }

    // Object/Manager/RecordSet модули: берём пути из метаданных (если discovery их нашёл)
    for obj in metadata {
        if let Some(p) = obj.object_module_path.as_ref() {
            all_module_paths.push(p.clone());
        }
        if let Some(p) = obj.manager_module_path.as_ref() {
            all_module_paths.push(p.clone());
        }
        if let Some(p) = obj.record_set_module_path.as_ref() {
            all_module_paths.push(p.clone());
        }
    }

    all_module_paths.sort();
    all_module_paths.dedup();
    all_module_paths
}

fn sort_indexed_signatures(out: &mut IndexedConfigSignatures) {
    fn location_sort_key(location: &TypeDefinitionLocation) -> (String, u32, u32, u32, u32) {
        match location {
            TypeDefinitionLocation::UserDefined {
                file_path,
                start_line,
                start_column,
                end_line,
                end_column,
            } => (
                file_path.to_string_lossy().to_string(),
                *start_line,
                *start_column,
                *end_line,
                *end_column,
            ),
            _ => (String::new(), 0, 0, 0, 0),
        }
    }

    out.config_methods.sort_by(|a, b| {
        let a_key = (a.0.to_lowercase(), a.1.name.to_lowercase());
        let b_key = (b.0.to_lowercase(), b.1.name.to_lowercase());
        a_key.cmp(&b_key)
    });

    out.global_functions.sort_by(|a, b| {
        let a_key = a.0.to_lowercase();
        let b_key = b.0.to_lowercase();
        a_key.cmp(&b_key)
    });

    out.definition_locations.sort_by(|a, b| {
        let a_key = (
            a.0.to_lowercase(),
            a.1.to_lowercase(),
            location_sort_key(&a.2),
        );
        let b_key = (
            b.0.to_lowercase(),
            b.1.to_lowercase(),
            location_sort_key(&b.2),
        );
        a_key.cmp(&b_key)
    });

    out.global_definition_locations.sort_by(|a, b| {
        let a_key = (a.0.to_lowercase(), location_sort_key(&a.1));
        let b_key = (b.0.to_lowercase(), location_sort_key(&b.1));
        a_key.cmp(&b_key)
    });
}

fn is_global_common_module(
    module_type: &ModuleType,
    common_props: &HashMap<String, CommonModuleProperties>,
) -> bool {
    let ModuleType::CommonModule { name, .. } = module_type else {
        return false;
    };
    common_props.get(name).map(|p| p.global).unwrap_or(false)
}

fn module_context_requirements(
    module_type: &ModuleType,
    common_props: &HashMap<String, CommonModuleProperties>,
) -> ContextRequirements {
    match module_type {
        ModuleType::ObjectModule { .. }
        | ModuleType::ManagerModule { .. }
        | ModuleType::RecordSetModule { .. } => ContextRequirements::ServerOnly,

        ModuleType::CommonModule { name, .. } => common_props
            .get(name)
            .map(common_module_context_requirements)
            .unwrap_or_default(),

        // Формы сейчас не индексируем как источник методов конфигурации
        ModuleType::FormModule { .. } | ModuleType::Unknown => ContextRequirements::default(),
    }
}

fn common_module_context_requirements(props: &CommonModuleProperties) -> ContextRequirements {
    let allows_server = props.server || props.server_call || props.privileged;
    let allows_client = props.client_managed_application || props.client_ordinary_application;

    match (allows_server, allows_client) {
        (true, true) => ContextRequirements::Universal,
        (true, false) => ContextRequirements::ServerOnly,
        (false, true) => ContextRequirements::ClientOnly,
        (false, false) => ContextRequirements::Universal,
    }
}

fn resolve_owner_type_for_signature(
    module_type: &ModuleType,
    common_props: &HashMap<String, CommonModuleProperties>,
) -> Option<String> {
    match module_type {
        ModuleType::CommonModule { name, .. } => {
            let _ = common_props.get(name)?;
            Some(format!("{}.{}", MetadataKind::CommonModule.to_prefix(), name))
        }
        ModuleType::ObjectModule { owner_type } => owner_type_to_faceted_type(owner_type, FacetKind::Object),
        ModuleType::ManagerModule { owner_type } => owner_type_to_faceted_type(owner_type, FacetKind::Manager),
        ModuleType::RecordSetModule { owner_type } => owner_type_to_faceted_type(owner_type, FacetKind::Object),
        ModuleType::FormModule { .. } | ModuleType::Unknown => None,
    }
}

fn owner_type_to_faceted_type(owner_type: &str, facet: FacetKind) -> Option<String> {
    let (xml_kind, object_name) = owner_type.split_once('.')?;
    let kind = MetadataKind::from_xml_tag(xml_kind)?;
    let prefix = kind.faceted_type_prefix(&facet);
    Some(format!("{}.{}", prefix, object_name))
}

/// Возвращает список объявлений верхнего уровня:
/// (name, params, is_export, is_function, context_from_directive, return_type)
fn parse_bsl_module(source: &str, module_path: &Path) -> Result<ParsedModuleData> {
    let ts_parse_started = Instant::now();
    let tree = parse_with_thread_parser(source)?;
    let ts_parse_elapsed = ts_parse_started.elapsed();
    if ts_parse_elapsed >= Duration::from_secs(1) {
        tracing::debug!(
            "tree-sitter parse: {} ({} байт, {} строк) {:?}",
            human_duration(ts_parse_elapsed),
            source.len(),
            source.lines().count(),
            module_path
        );
    }

    let convert_started = Instant::now();
    let parse_result = parse_bsl_module_tree_sitter_with_mode(
        &tree,
        source,
        SinglePassMode::Full,
    );
    let convert_elapsed = convert_started.elapsed();
    if convert_elapsed >= Duration::from_secs(1) {
        tracing::debug!(
            "tree-sitter single-pass: {} ({} байт, {} строк) {:?}",
            human_duration(convert_elapsed),
            source.len(),
            source.lines().count(),
            module_path
        );
    }

    match parse_result {
        Ok(data) => Ok(data),
        Err(e) => {
            tracing::debug!(
                "Tree-sitter single-pass failed, fallback to AST ({}): {}",
                module_path.display(),
                e
            );

            let parse_result = TreeSitterAdapter::convert_tree_fast(&tree, source)
                .map_err(|e| anyhow!("tree-sitter convert_tree_fast failed: {}", e))?;
            let (decls, call_sites) = collect_decls_and_call_sites(&parse_result.program.statements);
            Ok(ParsedModuleData { decls, call_sites })
        }
    }
}

fn parse_with_thread_parser(source: &str) -> Result<tree_sitter::Tree> {
    thread_local! {
        static THREAD_PARSER: RefCell<Option<Parser>> = RefCell::new(None);
    }

    THREAD_PARSER.with(|cell| {
        let mut parser_opt = cell.borrow_mut();
        if parser_opt.is_none() {
            let mut parser = Parser::new();
            parser
                .set_language(&tree_sitter_bsl::LANGUAGE.into())
                .map_err(|e| anyhow!("tree-sitter-bsl language error: {:?}", e))?;
            *parser_opt = Some(parser);
        }

        let parser = parser_opt.as_mut().expect("parser is initialized");
        parser
            .parse(source, None)
            .ok_or_else(|| anyhow!("tree-sitter parse returned None"))
    })
}

pub fn compare_module_parsing_from_file(path: &Path) -> Result<ModuleParseComparison> {
    compare_module_parsing_from_file_with_progress_mode(path, SinglePassMode::Full, |_| {})
}

pub fn compare_module_parsing_from_file_with_progress(
    path: &Path,
    mut progress: impl FnMut(&str),
) -> Result<ModuleParseComparison> {
    compare_module_parsing_from_file_with_progress_mode(
        path,
        SinglePassMode::Full,
        &mut progress,
    )
}

pub fn compare_module_parsing_from_file_with_progress_mode(
    path: &Path,
    mode: SinglePassMode,
    mut progress: impl FnMut(&str),
) -> Result<ModuleParseComparison> {
    progress("Читаем исходник");
    let source = read_bsl_file(path)?;

    progress("Парсим tree-sitter");
    let tree = parse_with_thread_parser(&source)?;

    progress("Single-pass обход");
    let single_pass = parse_bsl_module_tree_sitter_with_mode(&tree, &source, mode)?;

    progress("AST конвертация: старт");
    let ast = parse_bsl_module_ast_with_progress(&tree, &source, |done, total| {
        let msg = format!("AST конвертация: {}/{}", done, total);
        progress(&msg);
    })?;

    progress("Сравнение результатов");
    Ok(compare_module_data(path.to_path_buf(), single_pass, ast))
}

pub fn single_pass_module_stats_from_file_with_progress_mode(
    path: &Path,
    mode: SinglePassMode,
    mut progress: impl FnMut(&str),
) -> Result<ModuleParseStats> {
    progress("Читаем исходник");
    let source = read_bsl_file(path)?;

    progress("Парсим tree-sitter");
    let tree = parse_with_thread_parser(&source)?;

    progress("Single-pass обход");
    let single_pass = parse_bsl_module_tree_sitter_with_mode(&tree, &source, mode)?;

    Ok(module_parse_stats(&single_pass))
}

fn parse_bsl_module_tree_sitter_with_mode(
    tree: &tree_sitter::Tree,
    source: &str,
    mode: SinglePassMode,
) -> Result<ParsedModuleData> {
    let line_index = LineIndex::new(source);
    let mut decls: Vec<ParsedDecl> = Vec::new();
    let mut call_sites: Vec<CallSite> = Vec::new();
    let mut env: HashMap<String, Vec<String>> = HashMap::new();
    let mut return_types: Vec<String> = Vec::new();
    let track_returns = matches!(mode, SinglePassMode::Full);

    let mut cursor = tree.root_node().walk();
    for child in tree.root_node().children(&mut cursor) {
        walk_stmt_ts(
            &child,
            source,
            &line_index,
            &mut decls,
            &mut call_sites,
            &mut env,
            &mut return_types,
            track_returns,
        );
    }

    Ok(ParsedModuleData { decls, call_sites })
}

fn module_parse_stats(parsed: &ParsedModuleData) -> ModuleParseStats {
    let export_decls = parsed.decls.iter().filter(|d| d.is_export).count();
    ModuleParseStats {
        decls: parsed.decls.len(),
        export_decls,
        call_sites: parsed.call_sites.len(),
    }
}

fn parse_bsl_module_ast_with_progress(
    tree: &tree_sitter::Tree,
    source: &str,
    mut progress: impl FnMut(usize, usize),
) -> Result<ParsedModuleData> {
    let parse_result =
        TreeSitterAdapter::convert_tree_fast_with_progress(tree, source, &mut progress)
            .map_err(|e| anyhow!("tree-sitter convert_tree_fast failed: {}", e))?;
    let (decls, call_sites) = collect_decls_and_call_sites(&parse_result.program.statements);
    Ok(ParsedModuleData { decls, call_sites })
}

fn compare_module_data(
    module_path: PathBuf,
    single_pass: ParsedModuleData,
    ast: ParsedModuleData,
) -> ModuleParseComparison {
    let (single_decl_keys, single_export_count) = decl_keys(&single_pass.decls);
    let (ast_decl_keys, ast_export_count) = decl_keys(&ast.decls);

    let mut missing_decls = Vec::new();
    let mut extra_decls = Vec::new();

    for key in ast_decl_keys.keys() {
        if !single_decl_keys.contains_key(key) {
            missing_decls.push(key.clone());
        }
    }
    for key in single_decl_keys.keys() {
        if !ast_decl_keys.contains_key(key) {
            extra_decls.push(key.clone());
        }
    }

    missing_decls.sort();
    extra_decls.sort();

    let mut callsite_mismatches = Vec::new();
    let single_calls = callsite_keys(&single_pass.call_sites);
    let ast_calls = callsite_keys(&ast.call_sites);

    for key in ast_calls.keys() {
        let a = ast_calls.get(key).copied().unwrap_or(0);
        let b = single_calls.get(key).copied().unwrap_or(0);
        if a != b {
            callsite_mismatches.push(format!(
                "{} (ast={}, single={})",
                key, a, b
            ));
        }
    }
    for key in single_calls.keys() {
        if !ast_calls.contains_key(key) {
            let b = single_calls.get(key).copied().unwrap_or(0);
            callsite_mismatches.push(format!(
                "{} (ast=0, single={})",
                key, b
            ));
        }
    }

    callsite_mismatches.sort();

    ModuleParseComparison {
        module_path,
        single_pass: ModuleParseStats {
            decls: single_pass.decls.len(),
            export_decls: single_export_count,
            call_sites: single_pass.call_sites.len(),
        },
        ast: ModuleParseStats {
            decls: ast.decls.len(),
            export_decls: ast_export_count,
            call_sites: ast.call_sites.len(),
        },
        missing_decls,
        extra_decls,
        callsite_mismatches,
    }
}

fn decl_keys(decls: &[ParsedDecl]) -> (HashMap<String, usize>, usize) {
    let mut out = HashMap::new();
    let mut export_count = 0;
    for d in decls {
        if d.is_export {
            export_count += 1;
        }
        let key = format!(
            "{}|{}|{}",
            d.name.to_lowercase(),
            d.params.len(),
            d.is_export
        );
        *out.entry(key).or_insert(0) += 1;
    }
    (out, export_count)
}

fn callsite_keys(call_sites: &[CallSite]) -> HashMap<String, usize> {
    let mut out = HashMap::new();
    for call in call_sites {
        let key = match &call.target {
            CallTarget::LocalFunction { name } => format!("local:{}", name.to_lowercase()),
            CallTarget::QualifiedMethod { receiver, name } => format!(
                "qual:{}:{}",
                receiver.join(".").to_lowercase(),
                name.to_lowercase()
            ),
        };
        *out.entry(key).or_insert(0) += 1;
    }
    out
}

fn walk_stmt_ts(
    node: &tree_sitter::Node,
    source: &str,
    line_index: &LineIndex,
    decls: &mut Vec<ParsedDecl>,
    calls: &mut Vec<CallSite>,
    env: &mut HashMap<String, Vec<String>>,
    return_types: &mut Vec<String>,
    track_returns: bool,
) {
    match node.kind() {
        "function_definition" => {
            if let Some(mut decl) = parse_decl_ts(node, source, line_index) {
                let mut nested_env = env.clone();
                let mut func_return_types = Vec::new();
                walk_function_body_ts(
                    node,
                    source,
                    line_index,
                    decls,
                    calls,
                    &mut nested_env,
                    &mut func_return_types,
                    track_returns,
                );
                if track_returns {
                    decl.return_type = finalize_return_types(func_return_types);
                } else {
                    decl.return_type = None;
                }
                decls.push(decl);
            }
        }
        "procedure_definition" => {
            if let Some(mut decl) = parse_decl_ts(node, source, line_index) {
                let mut nested_env = env.clone();
                let mut func_return_types = Vec::new();
                walk_function_body_ts(
                    node,
                    source,
                    line_index,
                    decls,
                    calls,
                    &mut nested_env,
                    &mut func_return_types,
                    false,
                );
                decl.return_type = None;
                decls.push(decl);
            }
        }
        "assignment_statement" => {
            if let Some((target, value_node)) = split_assignment_ts(node, source) {
                walk_expr_ts(&value_node, source, line_index, calls, env);
                if let Some(t) = infer_expr_type_ts(&value_node, source, env) {
                    env.entry(target.clone()).or_default().push(t);
                    if let Some(v) = env.get_mut(&target) {
                        *v = normalize_union_parts(std::mem::take(v));
                    }
                }
            }
        }
        "var_definition" | "var_statement" => {
            if let Some(name) = first_identifier(node, source) {
                env.entry(name).or_default();
            }
        }
        "call_statement" => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "expression"
                    || child.kind() == "call_expression"
                    || child.kind() == "method_call"
                {
                    walk_expr_ts(&child, source, line_index, calls, env);
                }
            }
        }
        "return_statement" => {
            let mut cursor = node.walk();
            let mut found_expr = false;
            for child in node.children(&mut cursor) {
                if child.kind() == "expression" || child.kind() == "method_call" {
                    found_expr = true;
                    if track_returns {
                        if let Some(t) = infer_expr_type_ts(&child, source, env) {
                            return_types.push(t);
                        }
                    }
                    walk_expr_ts(&child, source, line_index, calls, env);
                }
            }
            if !found_expr {
                if track_returns {
                    return_types.push("Неопределено".to_string());
                }
            }
        }
        "if_statement" => {
            walk_if_ts(
                node,
                source,
                line_index,
                decls,
                calls,
                env,
                return_types,
                track_returns,
            );
        }
        "for_statement" | "for_each_statement" | "while_statement" => {
            walk_loop_ts(
                node,
                source,
                line_index,
                decls,
                calls,
                env,
                return_types,
                track_returns,
            );
        }
        "try_statement" => {
            walk_try_ts(
                node,
                source,
                line_index,
                decls,
                calls,
                env,
                return_types,
                track_returns,
            );
        }
        "execute_statement" | "raise_error_statement" => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "expression" || child.kind() == "method_call" {
                    walk_expr_ts(&child, source, line_index, calls, env);
                }
            }
        }
        "add_handler_statement" | "remove_handler_statement" | "await_statement" => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "expression" || child.kind() == "method_call" {
                    walk_expr_ts(&child, source, line_index, calls, env);
                }
            }
        }
        _ => {}
    }
}

fn parse_decl_ts(
    node: &tree_sitter::Node,
    source: &str,
    line_index: &LineIndex,
) -> Option<ParsedDecl> {
    let mut cursor = node.walk();
    let mut name = String::new();
    let mut params: Vec<String> = Vec::new();
    let mut is_export = false;

    for child in node.children(&mut cursor) {
        match child.kind() {
            "identifier" if name.is_empty() => name = node_text(&child, source),
            "parameters" => {
                if let Ok(p) = convert_parameters(&child, source) {
                    params = p;
                }
            }
            _ if child.kind().ends_with("_KEYWORD") => {
                let kw = node_text(&child, source).trim().to_lowercase();
                if kw == "экспорт" || kw == "export" {
                    is_export = true;
                }
            }
            _ => {}
        }
    }

    if name.is_empty() {
        return None;
    }

    let directive_ctx = find_preceding_directive(node, source).map(context_from_directive);

    Some(ParsedDecl {
        name,
        params,
        is_export,
        directive_ctx,
        return_type: None,
        span: node_to_span_cached(node, source, line_index),
    })
}

fn walk_function_body_ts(
    node: &tree_sitter::Node,
    source: &str,
    line_index: &LineIndex,
    decls: &mut Vec<ParsedDecl>,
    calls: &mut Vec<CallSite>,
    env: &mut HashMap<String, Vec<String>>,
    return_types: &mut Vec<String>,
    track_returns: bool,
) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind().ends_with("_statement") || child.kind().ends_with("_definition") {
            walk_stmt_ts(
                &child,
                source,
                line_index,
                decls,
                calls,
                env,
                return_types,
                track_returns,
            );
        }
    }
}

fn walk_if_ts(
    node: &tree_sitter::Node,
    source: &str,
    line_index: &LineIndex,
    decls: &mut Vec<ParsedDecl>,
    calls: &mut Vec<CallSite>,
    env: &mut HashMap<String, Vec<String>>,
    return_types: &mut Vec<String>,
    track_returns: bool,
) {
    let mut cursor = node.walk();
    let mut in_then = false;
    let mut then_nodes: Vec<tree_sitter::Node> = Vec::new();
    let mut else_nodes: Vec<tree_sitter::Node> = Vec::new();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "THEN_KEYWORD" | "ТОГДА_KEYWORD" => in_then = true,
            "ENDIF_KEYWORD" | "КОНЕЦЕСЛИ_KEYWORD" => break,
            _ if !in_then
                && (child.kind().contains("expression")
                    || child.kind() == "call_expression"
                    || child.kind() == "method_call") =>
            {
                walk_expr_ts(&child, source, line_index, calls, env);
            }
            "else_clause" | "elseif_clause" => else_nodes.push(child),
            kind if in_then && (kind.ends_with("_statement") || kind.ends_with("_definition")) => {
                then_nodes.push(child);
            }
            _ => {}
        }
    }

    let mut then_env = env.clone();
    for node in then_nodes {
        walk_stmt_ts(
            &node,
            source,
            line_index,
            decls,
            calls,
            &mut then_env,
            return_types,
            track_returns,
        );
    }

    let mut else_env = env.clone();
    for node in else_nodes {
        let mut clause_cursor = node.walk();
        for child in node.children(&mut clause_cursor) {
            if child.kind().ends_with("_statement") || child.kind().ends_with("_definition") {
                walk_stmt_ts(
                    &child,
                    source,
                    line_index,
                    decls,
                    calls,
                    &mut else_env,
                    return_types,
                    track_returns,
                );
            }
        }
    }

    *env = HashMap::new();
    merge_envs(env, then_env);
    merge_envs(env, else_env);
}

fn walk_loop_ts(
    node: &tree_sitter::Node,
    source: &str,
    line_index: &LineIndex,
    decls: &mut Vec<ParsedDecl>,
    calls: &mut Vec<CallSite>,
    env: &mut HashMap<String, Vec<String>>,
    return_types: &mut Vec<String>,
    track_returns: bool,
) {
    let mut cursor = node.walk();
    let mut body_nodes: Vec<tree_sitter::Node> = Vec::new();

    for child in node.children(&mut cursor) {
        if child.kind().contains("expression")
            || child.kind() == "call_expression"
            || child.kind() == "method_call"
        {
            walk_expr_ts(&child, source, line_index, calls, env);
        } else if child.kind().ends_with("_statement") || child.kind().ends_with("_definition") {
            body_nodes.push(child);
        }
    }

    let mut body_env = env.clone();
    for node in body_nodes {
        walk_stmt_ts(
            &node,
            source,
            line_index,
            decls,
            calls,
            &mut body_env,
            return_types,
            track_returns,
        );
    }
    merge_envs(env, body_env);
}

fn walk_try_ts(
    node: &tree_sitter::Node,
    source: &str,
    line_index: &LineIndex,
    decls: &mut Vec<ParsedDecl>,
    calls: &mut Vec<CallSite>,
    env: &mut HashMap<String, Vec<String>>,
    return_types: &mut Vec<String>,
    track_returns: bool,
) {
    let mut cursor = node.walk();
    let mut try_nodes: Vec<tree_sitter::Node> = Vec::new();
    let mut except_nodes: Vec<tree_sitter::Node> = Vec::new();
    let mut in_except = false;

    for child in node.children(&mut cursor) {
        match child.kind() {
            "EXCEPT_KEYWORD" | "ИСКЛЮЧЕНИЕ_KEYWORD" => {
                in_except = true;
            }
            kind if kind.ends_with("_statement") || kind.ends_with("_definition") => {
                if in_except {
                    except_nodes.push(child);
                } else {
                    try_nodes.push(child);
                }
            }
            _ => {}
        }
    }

    let mut try_env = env.clone();
    for node in try_nodes {
        walk_stmt_ts(
            &node,
            source,
            line_index,
            decls,
            calls,
            &mut try_env,
            return_types,
            track_returns,
        );
    }

    let mut except_env = env.clone();
    for node in except_nodes {
        walk_stmt_ts(
            &node,
            source,
            line_index,
            decls,
            calls,
            &mut except_env,
            return_types,
            track_returns,
        );
    }

    *env = HashMap::new();
    merge_envs(env, try_env);
    merge_envs(env, except_env);
}

fn walk_expr_ts(
    node: &tree_sitter::Node,
    source: &str,
    line_index: &LineIndex,
    calls: &mut Vec<CallSite>,
    env: &HashMap<String, Vec<String>>,
) {
    if node.kind() == "call_expression" || node.kind() == "method_call" {
        if let Some(call) = parse_call_ts(node, source, env) {
            calls.push(call);
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "arguments" {
                let mut args_cursor = child.walk();
                for arg_child in child.children(&mut args_cursor) {
                    if arg_child.kind() == "expression" {
                        walk_expr_ts(&arg_child, source, line_index, calls, env);
                    }
                }
            }
        }
        return;
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_expr_ts(&child, source, line_index, calls, env);
    }
}

fn parse_call_ts(
    node: &tree_sitter::Node,
    source: &str,
    env: &HashMap<String, Vec<String>>,
) -> Option<CallSite> {
    let mut cursor = node.walk();
    let mut access_node: Option<tree_sitter::Node> = None;
    let mut method_call_node: Option<tree_sitter::Node> = None;
    let mut func_name: Option<String> = None;
    let mut args: Vec<tree_sitter::Node> = Vec::new();

    if node.kind() == "method_call" {
        method_call_node = Some(*node);
    }

    for child in node.children(&mut cursor) {
        match child.kind() {
            "access" => access_node = Some(child),
            "method_call" => method_call_node = Some(child),
            "identifier" if func_name.is_none() => {
                func_name = Some(node_text(&child, source));
            }
            "arguments" => {
                let mut args_cursor = child.walk();
                for arg_child in child.children(&mut args_cursor) {
                    if arg_child.kind() == "expression" {
                        args.push(arg_child);
                    }
                }
            }
            _ => {}
        }
    }

    if let Some(method_call) = method_call_node {
        let mut method_cursor = method_call.walk();
        for child in method_call.children(&mut method_cursor) {
            match child.kind() {
                "identifier" if func_name.is_none() => {
                    func_name = Some(node_text(&child, source));
                }
                "arguments" if args.is_empty() => {
                    let mut args_cursor = child.walk();
                    for arg_child in child.children(&mut args_cursor) {
                        if arg_child.kind() == "expression" {
                            args.push(arg_child);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    let arg_types = args
        .iter()
        .map(|arg| infer_expr_type_ts(arg, source, env))
        .collect();

    if let (Some(access), Some(method_call)) = (access_node, method_call_node) {
        let receiver = split_dotted_path(&access, source);
        let name = first_identifier(&method_call, source)?;
        return Some(CallSite {
            target: CallTarget::QualifiedMethod { receiver, name },
            arg_types,
        });
    }

    if func_name.is_none() {
        if let Some(access) = access_node {
            func_name = first_identifier(&access, source);
        }
    }

    if let Some(name) = func_name {
        return Some(CallSite {
            target: CallTarget::LocalFunction { name },
            arg_types,
        });
    }

    None
}

fn split_assignment_ts<'a>(
    node: &'a tree_sitter::Node<'a>,
    source: &str,
) -> Option<(String, tree_sitter::Node<'a>)> {
    let mut cursor = node.walk();
    let mut seen_eq = false;
    let mut target_name: Option<String> = None;
    let mut value_expr: Option<tree_sitter::Node> = None;

    for child in node.children(&mut cursor) {
        match child.kind() {
            "=" => seen_eq = true,
            "identifier" if !seen_eq && target_name.is_none() => {
                target_name = Some(node_text(&child, source));
            }
            "expression" | "call_expression" | "const_expression" | "method_call"
                if seen_eq && value_expr.is_none() =>
            {
                value_expr = Some(child);
            }
            _ => {}
        }
    }

    let target = target_name?;
    let value = value_expr?;
    Some((target, value))
}

fn infer_expr_type_ts(
    node: &tree_sitter::Node,
    source: &str,
    env: &HashMap<String, Vec<String>>,
) -> Option<String> {
    if node.kind() == "expression" || node.kind() == "const_expression" {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if let Some(t) = infer_expr_type_ts(&child, source, env) {
                return Some(t);
            }
        }
        return None;
    }

    match node.kind() {
        "string" => Some("Строка".to_string()),
        "number" => Some("Число".to_string()),
        "date" | "date_literal" => Some("Дата".to_string()),
        "identifier" => {
            let name = node_text(node, source);
            if name.eq_ignore_ascii_case("неопределено") {
                return Some("Неопределено".to_string());
            }
            env_get_union_ts(env, &name)
        }
        "new_expression" | "new_expression_method" => extract_new_type_ts(node, source),
        _ => {
            let text = node_text(node, source).to_lowercase();
            if text == "истина" || text == "true" {
                Some("Булево".to_string())
            } else if text == "ложь" || text == "false" {
                Some("Булево".to_string())
            } else {
                None
            }
        }
    }
}

fn extract_new_type_ts(node: &tree_sitter::Node, source: &str) -> Option<String> {
    let mut cursor = node.walk();
    let mut type_expr: Option<String> = None;

    for child in node.children(&mut cursor) {
        match child.kind() {
            "NEW_KEYWORD" | "НОВЫЙ_KEYWORD" => {}
            "identifier" | "property_access" => {
                if type_expr.is_none() {
                    type_expr = Some(node_text(&child, source));
                }
            }
            "arguments" => {
                let mut arg_cursor = child.walk();
                for arg_child in child.children(&mut arg_cursor) {
                    if arg_child.kind() == "expression" && type_expr.is_none() {
                        type_expr = Some(node_text(&arg_child, source));
                        break;
                    }
                }
            }
            _ => {}
        }
    }

    type_expr
}

fn split_dotted_path(node: &tree_sitter::Node, source: &str) -> Vec<String> {
    let text = node_text(node, source);
    text.split('.')
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        .map(|p| p.to_string())
        .collect()
}

fn first_identifier(node: &tree_sitter::Node, source: &str) -> Option<String> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "identifier" {
            return Some(node_text(&child, source));
        }
    }
    None
}

fn finalize_return_types(collected: Vec<String>) -> Option<String> {
    if collected.is_empty() {
        return Some("Неопределено".to_string());
    }

    Some(normalize_union_parts(collected).join(" | "))
}

fn env_get_union_ts(env: &HashMap<String, Vec<String>>, name: &str) -> Option<String> {
    let mut parts = env.get(name)?.clone();
    parts = normalize_union_parts(parts);
    (!parts.is_empty()).then(|| parts.join(" | "))
}

fn merge_envs(target: &mut HashMap<String, Vec<String>>, source: HashMap<String, Vec<String>>) {
    for (k, v) in source {
        for t in v {
            target.entry(k.clone()).or_default().push(t);
        }
        if let Some(values) = target.get_mut(&k) {
            *values = normalize_union_parts(std::mem::take(values));
        }
    }
}

#[allow(dead_code)]
fn extract_export_decls_from_tree(
    tree: &tree_sitter::Tree,
    source: &str,
) -> Result<Vec<ParsedDecl>> {
    fn node_text(node: &tree_sitter::Node, source: &str) -> String {
        source[node.byte_range()].to_string()
    }

    fn convert_parameters(node: &tree_sitter::Node, source: &str) -> Vec<String> {
        let mut params = Vec::new();
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            if child.kind() != "parameter" {
                continue;
            }

            let mut param_cursor = child.walk();
            for param_child in child.children(&mut param_cursor) {
                if param_child.kind() == "identifier" {
                    params.push(node_text(&param_child, source));
                    break;
                }
            }
        }

        params
    }

    fn collect_definition_nodes<'a>(node: tree_sitter::Node<'a>, out: &mut Vec<tree_sitter::Node<'a>>) {
        if matches!(node.kind(), "function_definition" | "procedure_definition") {
            out.push(node);
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            collect_definition_nodes(child, out);
        }
    }

    let line_index = LineIndex::new(source);

    let mut definition_nodes = Vec::new();
    collect_definition_nodes(tree.root_node(), &mut definition_nodes);

    let mut out = Vec::new();

    for node in definition_nodes {
        let mut cursor = node.walk();
        let mut name = String::new();
        let mut params: Vec<String> = Vec::new();
        let mut is_export = false;

        for child in node.children(&mut cursor) {
            match child.kind() {
                "identifier" if name.is_empty() => name = node_text(&child, source),
                "parameters" => params = convert_parameters(&child, source),
                _ if child.kind().ends_with("_KEYWORD") => {
                    let kw = node_text(&child, source).trim().to_lowercase();
                    if kw == "экспорт" || kw == "export" {
                        is_export = true;
                    }
                }
                _ => {}
            }
        }

        if name.is_empty() {
            continue;
        }

        out.push(ParsedDecl {
            name,
            params,
            is_export,
            directive_ctx: None,
            return_type: None,
            span: node_to_span_cached(&node, source, &line_index),
        });
    }

    Ok(out)
}

#[allow(dead_code)]
fn collect_call_sites(statements: &[crate::parsing::bsl::ast::Statement]) -> Vec<CallSite> {
    use crate::parsing::bsl::ast::{Expression, Statement};

    type VarEnv = HashMap<String, Vec<String>>;

    fn env_get_union(env: &VarEnv, name: &str) -> Option<String> {
        let mut parts = env.get(name)?.clone();
        parts = normalize_union_parts(parts);
        (!parts.is_empty()).then(|| parts.join(" | "))
    }

    fn env_add_type(env: &mut VarEnv, name: &str, t: String) {
        env.entry(name.to_string()).or_default().push(t);
        if let Some(v) = env.get_mut(name) {
            *v = normalize_union_parts(std::mem::take(v));
        }
    }

    fn expr_to_dotted_path(expr: &Expression) -> Option<Vec<String>> {
        match expr {
            Expression::Identifier { name, .. } => Some(vec![name.clone()]),
            Expression::PropertyAccess { object, property, .. } => {
                let mut base = expr_to_dotted_path(object)?;
                base.push(property.clone());
                Some(base)
            }
            _ => None,
        }
    }

    fn infer_expr_type_with_env(expr: &Expression, env: &VarEnv) -> Option<String> {
        match expr {
            Expression::Identifier { name, .. } => env_get_union(env, name).or_else(|| {
                name.eq_ignore_ascii_case("неопределено")
                    .then(|| "Неопределено".to_string())
            }),
            _ => infer_expr_type(expr),
        }
    }

    fn walk_expr(acc: &mut Vec<CallSite>, expr: &Expression, env: &VarEnv) {
        match expr {
            Expression::Call { function, args, .. } => {
                match function.as_ref() {
                    Expression::Identifier { name, .. } => acc.push(CallSite {
                        target: CallTarget::LocalFunction { name: name.clone() },
                        arg_types: args
                            .iter()
                            .map(|a| infer_expr_type_with_env(a, env))
                            .collect(),
                    }),
                    Expression::PropertyAccess {
                        object,
                        property,
                        ..
                    } => {
                        if let Some(receiver) = expr_to_dotted_path(object) {
                            acc.push(CallSite {
                                target: CallTarget::QualifiedMethod {
                                    receiver,
                                    name: property.clone(),
                                },
                                arg_types: args
                                    .iter()
                                    .map(|a| infer_expr_type_with_env(a, env))
                                    .collect(),
                            });
                        }
                    }
                    _ => {}
                }

                walk_expr(acc, function, env);
                for a in args {
                    walk_expr(acc, a, env);
                }
            }
            Expression::Binary { left, right, .. } => {
                walk_expr(acc, left, env);
                walk_expr(acc, right, env);
            }
            Expression::Unary { operand, .. } => walk_expr(acc, operand, env),
            Expression::Ternary {
                condition,
                then_expr,
                else_expr,
                ..
            } => {
                walk_expr(acc, condition, env);
                walk_expr(acc, then_expr, env);
                walk_expr(acc, else_expr, env);
            }
            Expression::New { args, .. } => {
                for a in args {
                    walk_expr(acc, a, env);
                }
            }
            Expression::PropertyAccess { object, .. } => walk_expr(acc, object, env),
            Expression::IndexAccess { object, index, .. } => {
                walk_expr(acc, object, env);
                walk_expr(acc, index, env);
            }
            Expression::Await { expression, .. } => walk_expr(acc, expression, env),
            Expression::Identifier { .. }
            | Expression::String { .. }
            | Expression::Number { .. }
            | Expression::Boolean { .. }
            | Expression::Date { .. } => {}
        }
    }

    fn merge_envs(a: &mut VarEnv, b: VarEnv) {
        for (k, v) in b {
            for t in v {
                env_add_type(a, &k, t);
            }
        }
    }

    fn walk_block(acc: &mut Vec<CallSite>, statements: &[Statement], env: &mut VarEnv) {
        for st in statements {
            walk_stmt(acc, st, env);
        }
    }

    fn walk_stmt(acc: &mut Vec<CallSite>, st: &Statement, env: &mut VarEnv) {
        match st {
            Statement::Assignment { target, value, .. } => {
                walk_expr(acc, target, env);
                walk_expr(acc, value, env);

                if let Expression::Identifier { name, .. } = target {
                    if let Some(t) = infer_expr_type_with_env(value, env) {
                        env_add_type(env, name, t);
                    }
                }
            }
            Statement::VarDeclaration { name, type_hint, .. } => {
                if let Some(hint) = type_hint.as_ref().filter(|s| !s.trim().is_empty()) {
                    env_add_type(env, name, hint.trim().to_string());
                }
            }
            Statement::FunctionDecl { body, .. } | Statement::ProcedureDecl { body, .. } => {
                let mut nested_env = env.clone();
                walk_block(acc, body, &mut nested_env);
            }
            Statement::If {
                condition,
                then_body,
                else_body,
                ..
            } => {
                walk_expr(acc, condition, env);

                let mut then_env = env.clone();
                walk_block(acc, then_body, &mut then_env);

                let mut else_env = env.clone();
                if let Some(else_body) = else_body {
                    walk_block(acc, else_body, &mut else_env);
                }

                *env = HashMap::new();
                merge_envs(env, then_env);
                merge_envs(env, else_env);
            }
            Statement::For {
                start, end, body, ..
            } => {
                walk_expr(acc, start, env);
                walk_expr(acc, end, env);

                let mut body_env = env.clone();
                walk_block(acc, body, &mut body_env);
                merge_envs(env, body_env);
            }
            Statement::ForEach { collection, body, .. } => {
                walk_expr(acc, collection, env);

                let mut body_env = env.clone();
                walk_block(acc, body, &mut body_env);
                merge_envs(env, body_env);
            }
            Statement::While { condition, body, .. } => {
                walk_expr(acc, condition, env);

                let mut body_env = env.clone();
                walk_block(acc, body, &mut body_env);
                merge_envs(env, body_env);
            }
            Statement::Return { value, .. } => {
                if let Some(v) = value {
                    walk_expr(acc, v, env);
                }
            }
            Statement::Try {
                try_body,
                except_body,
                ..
            } => {
                let mut try_env = env.clone();
                walk_block(acc, try_body, &mut try_env);

                let mut except_env = env.clone();
                walk_block(acc, except_body, &mut except_env);

                *env = HashMap::new();
                merge_envs(env, try_env);
                merge_envs(env, except_env);
            }
            Statement::Call { expression, .. } => walk_expr(acc, expression, env),
            Statement::Break { .. }
            | Statement::Continue { .. }
            | Statement::Goto { .. }
            | Statement::Label { .. } => {}
            Statement::Execute { code, .. } => walk_expr(acc, code, env),
            Statement::RaiseError { message, .. } => {
                if let Some(m) = message {
                    walk_expr(acc, m, env);
                }
            }
            Statement::AddHandler {
                event, handler, ..
            }
            | Statement::RemoveHandler {
                event, handler, ..
            } => {
                walk_expr(acc, event, env);
                walk_expr(acc, handler, env);
            }
            Statement::Await { expression, .. } => walk_expr(acc, expression, env),
        }
    }

    let mut out = Vec::new();
    let mut env: VarEnv = HashMap::new();
    walk_block(&mut out, statements, &mut env);
    out
}

fn collect_decls_and_call_sites(
    statements: &[crate::parsing::bsl::ast::Statement],
) -> (Vec<ParsedDecl>, Vec<CallSite>) {
    use crate::parsing::bsl::ast::{Expression, Statement};

    type VarEnv = HashMap<String, Vec<String>>;

    fn env_get_union(env: &VarEnv, name: &str) -> Option<String> {
        let mut parts = env.get(name)?.clone();
        parts = normalize_union_parts(parts);
        (!parts.is_empty()).then(|| parts.join(" | "))
    }

    fn env_add_type(env: &mut VarEnv, name: &str, t: String) {
        env.entry(name.to_string()).or_default().push(t);
        if let Some(v) = env.get_mut(name) {
            *v = normalize_union_parts(std::mem::take(v));
        }
    }

    fn expr_to_dotted_path(expr: &Expression) -> Option<Vec<String>> {
        match expr {
            Expression::Identifier { name, .. } => Some(vec![name.clone()]),
            Expression::PropertyAccess { object, property, .. } => {
                let mut base = expr_to_dotted_path(object)?;
                base.push(property.clone());
                Some(base)
            }
            _ => None,
        }
    }

    fn infer_expr_type_with_env(expr: &Expression, env: &VarEnv) -> Option<String> {
        match expr {
            Expression::Identifier { name, .. } => env_get_union(env, name).or_else(|| {
                name.eq_ignore_ascii_case("неопределено")
                    .then(|| "Неопределено".to_string())
            }),
            _ => infer_expr_type(expr),
        }
    }

    fn walk_expr(acc: &mut Vec<CallSite>, expr: &Expression, env: &VarEnv) {
        match expr {
            Expression::Call { function, args, .. } => {
                match function.as_ref() {
                    Expression::Identifier { name, .. } => acc.push(CallSite {
                        target: CallTarget::LocalFunction { name: name.clone() },
                        arg_types: args
                            .iter()
                            .map(|a| infer_expr_type_with_env(a, env))
                            .collect(),
                    }),
                    Expression::PropertyAccess {
                        object,
                        property,
                        ..
                    } => {
                        if let Some(receiver) = expr_to_dotted_path(object) {
                            acc.push(CallSite {
                                target: CallTarget::QualifiedMethod {
                                    receiver,
                                    name: property.clone(),
                                },
                                arg_types: args
                                    .iter()
                                    .map(|a| infer_expr_type_with_env(a, env))
                                    .collect(),
                            });
                        }
                    }
                    _ => {}
                }

                walk_expr(acc, function, env);
                for a in args {
                    walk_expr(acc, a, env);
                }
            }
            Expression::Binary { left, right, .. } => {
                walk_expr(acc, left, env);
                walk_expr(acc, right, env);
            }
            Expression::Unary { operand, .. } => walk_expr(acc, operand, env),
            Expression::Ternary {
                condition,
                then_expr,
                else_expr,
                ..
            } => {
                walk_expr(acc, condition, env);
                walk_expr(acc, then_expr, env);
                walk_expr(acc, else_expr, env);
            }
            Expression::New { args, .. } => {
                for a in args {
                    walk_expr(acc, a, env);
                }
            }
            Expression::PropertyAccess { object, .. } => walk_expr(acc, object, env),
            Expression::IndexAccess { object, index, .. } => {
                walk_expr(acc, object, env);
                walk_expr(acc, index, env);
            }
            Expression::Await { expression, .. } => walk_expr(acc, expression, env),
            Expression::Identifier { .. }
            | Expression::String { .. }
            | Expression::Number { .. }
            | Expression::Boolean { .. }
            | Expression::Date { .. } => {}
        }
    }

    fn merge_envs(a: &mut VarEnv, b: VarEnv) {
        for (k, v) in b {
            for t in v {
                env_add_type(a, &k, t);
            }
        }
    }

    fn walk_block(
        decls: &mut Vec<ParsedDecl>,
        calls: &mut Vec<CallSite>,
        statements: &[Statement],
        env: &mut VarEnv,
    ) {
        for st in statements {
            walk_stmt(decls, calls, st, env);
        }
    }

    fn walk_stmt(
        decls: &mut Vec<ParsedDecl>,
        calls: &mut Vec<CallSite>,
        st: &Statement,
        env: &mut VarEnv,
    ) {
        match st {
            Statement::Assignment { target, value, .. } => {
                walk_expr(calls, target, env);
                walk_expr(calls, value, env);

                if let Expression::Identifier { name, .. } = target {
                    if let Some(t) = infer_expr_type_with_env(value, env) {
                        env_add_type(env, name, t);
                    }
                }
            }
            Statement::VarDeclaration { name, type_hint, .. } => {
                if let Some(hint) = type_hint.as_ref().filter(|s| !s.trim().is_empty()) {
                    env_add_type(env, name, hint.trim().to_string());
                }
            }
            Statement::FunctionDecl {
                name,
                params,
                body,
                compiler_directive,
                is_export,
                span,
                ..
            } => {
                decls.push(ParsedDecl {
                    return_type: infer_return_type_from_body(body),
                    name: name.clone(),
                    params: params.clone(),
                    is_export: *is_export,
                    directive_ctx: compiler_directive.map(context_from_directive),
                    span: *span,
                });
                let mut nested_env = env.clone();
                walk_block(decls, calls, body, &mut nested_env);
            }
            Statement::ProcedureDecl {
                name,
                params,
                body,
                compiler_directive,
                is_export,
                span,
                ..
            } => {
                decls.push(ParsedDecl {
                    return_type: None,
                    name: name.clone(),
                    params: params.clone(),
                    is_export: *is_export,
                    directive_ctx: compiler_directive.map(context_from_directive),
                    span: *span,
                });
                let mut nested_env = env.clone();
                walk_block(decls, calls, body, &mut nested_env);
            }
            Statement::If {
                condition,
                then_body,
                else_body,
                ..
            } => {
                walk_expr(calls, condition, env);

                let mut then_env = env.clone();
                walk_block(decls, calls, then_body, &mut then_env);

                let mut else_env = env.clone();
                if let Some(else_body) = else_body {
                    walk_block(decls, calls, else_body, &mut else_env);
                }

                *env = HashMap::new();
                merge_envs(env, then_env);
                merge_envs(env, else_env);
            }
            Statement::For {
                start, end, body, ..
            } => {
                walk_expr(calls, start, env);
                walk_expr(calls, end, env);

                let mut body_env = env.clone();
                walk_block(decls, calls, body, &mut body_env);
                merge_envs(env, body_env);
            }
            Statement::ForEach { collection, body, .. } => {
                walk_expr(calls, collection, env);

                let mut body_env = env.clone();
                walk_block(decls, calls, body, &mut body_env);
                merge_envs(env, body_env);
            }
            Statement::While { condition, body, .. } => {
                walk_expr(calls, condition, env);

                let mut body_env = env.clone();
                walk_block(decls, calls, body, &mut body_env);
                merge_envs(env, body_env);
            }
            Statement::Return { value, .. } => {
                if let Some(v) = value {
                    walk_expr(calls, v, env);
                }
            }
            Statement::Try {
                try_body,
                except_body,
                ..
            } => {
                let mut try_env = env.clone();
                walk_block(decls, calls, try_body, &mut try_env);

                let mut except_env = env.clone();
                walk_block(decls, calls, except_body, &mut except_env);

                *env = HashMap::new();
                merge_envs(env, try_env);
                merge_envs(env, except_env);
            }
            Statement::Call { expression, .. } => walk_expr(calls, expression, env),
            Statement::Break { .. }
            | Statement::Continue { .. }
            | Statement::Goto { .. }
            | Statement::Label { .. } => {}
            Statement::Execute { code, .. } => walk_expr(calls, code, env),
            Statement::RaiseError { message, .. } => {
                if let Some(m) = message {
                    walk_expr(calls, m, env);
                }
            }
            Statement::AddHandler {
                event, handler, ..
            }
            | Statement::RemoveHandler {
                event, handler, ..
            } => {
                walk_expr(calls, event, env);
                walk_expr(calls, handler, env);
            }
            Statement::Await { expression, .. } => walk_expr(calls, expression, env),
        }
    }

    let mut decls = Vec::new();
    let mut calls = Vec::new();
    let mut env: VarEnv = HashMap::new();
    walk_block(&mut decls, &mut calls, statements, &mut env);
    (decls, calls)
}

fn infer_export_param_types_across_modules(
    modules: &[ParsedModule],
) -> HashMap<(String, String), Vec<Option<String>>> {
    let mut export_param_slots: HashMap<(String, String), usize> = HashMap::new();

    // 1) Собираем все экспортные сигнатуры (owner_type + method_name -> param_count)
    for m in modules {
        for d in &m.decls {
            if !d.is_export {
                continue;
            }
            export_param_slots.insert((m.owner_type_name.clone(), d.name.clone()), d.params.len());
        }
    }

    // 2) Собираем наблюдения по вызовам.
    // Ключ: (owner_type_name, method_name, param_index) -> vec типов аргумента.
    let mut observations: HashMap<(String, String, usize), Vec<String>> = HashMap::new();

    // Карта CommonModuleName -> owner_type_name ("ОбщиеМодули.<Name>")
    let mut common_module_owner_types: HashMap<String, String> = HashMap::new();
    for m in modules {
        if let ModuleType::CommonModule { name, .. } = &m.module_type {
            common_module_owner_types.insert(name.clone(), m.owner_type_name.clone());
        }
    }

    for module in modules {
        // Для локальных вызовов резолвим только внутри текущего модуля (иначе не отличить от глобального namespace).
        let local_exports: HashMap<String, usize> = module
            .decls
            .iter()
            .filter(|d| d.is_export)
            .map(|d| (d.name.clone(), d.params.len()))
            .collect();

        for call in &module.call_sites {
            let arg_types = &call.arg_types;

            match &call.target {
                CallTarget::LocalFunction { name } => {
                    let Some(_param_count) = local_exports.get(name) else {
                        continue;
                    };
                    for (idx, t) in arg_types.iter().enumerate() {
                        let Some(t) = t else { continue };
                        for part in split_union_string(t) {
                            observations
                                .entry((module.owner_type_name.clone(), name.clone(), idx))
                                .or_default()
                                .push(part);
                        }
                    }
                }
                CallTarget::QualifiedMethod { receiver, name } => {
                    // Common module call: "ИмяМодуля.Метод()"
                    if receiver.len() == 1 {
                        if let Some(owner) = common_module_owner_types.get(&receiver[0]) {
                            if export_param_slots.contains_key(&(owner.clone(), name.clone())) {
                                for (idx, t) in arg_types.iter().enumerate() {
                                    let Some(t) = t else { continue };
                                    for part in split_union_string(t) {
                                        observations
                                            .entry((owner.clone(), name.clone(), idx))
                                            .or_default()
                                            .push(part);
                                    }
                                }
                                continue;
                            }
                        }
                    }

                    // Manager call: "Справочники.<X>.Метод()"/"Documents.<X>.Method()"
                    if receiver.len() == 2 {
                        if let Some(owner) = resolve_manager_owner_type_from_receiver(receiver) {
                            if export_param_slots.contains_key(&(owner.clone(), name.clone())) {
                                for (idx, t) in arg_types.iter().enumerate() {
                                    let Some(t) = t else { continue };
                                    for part in split_union_string(t) {
                                        observations
                                            .entry((owner.clone(), name.clone(), idx))
                                            .or_default()
                                            .push(part);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // 3) Финализируем в (owner_type, method_name) -> Vec<Option<union>>
    let mut out: HashMap<(String, String), Vec<Option<String>>> = HashMap::new();
    for ((owner, name), param_count) in export_param_slots {
        out.insert((owner, name), vec![None; param_count]);
    }

    for ((owner, name, idx), types) in observations {
        let union = normalize_union_parts(types);
        let union = (!union.is_empty()).then(|| union.join(" | "));
        if let Some(v) = out.get_mut(&(owner, name)) {
            if idx < v.len() {
                v[idx] = union;
            }
        }
    }

    out
}

fn report_slow_modules(slow_modules: &mut Vec<(Duration, PathBuf)>) {
    let metrics = parse_metrics_config();
    if slow_modules.is_empty() {
        return;
    }
    slow_modules.sort_by(|a, b| b.0.cmp(&a.0));
    let top = slow_modules.iter().take(metrics.top_n);
    let table = format_slow_modules_table(top);
    tracing::info!(
        "Медленный парсинг модулей: всего={}\n{}",
        slow_modules.len(),
        table
    );
}

#[derive(Debug, Clone)]
struct ParseMetricsConfig {
    slow_threshold: Duration,
    top_n: usize,
    log_each: bool,
}

fn parse_metrics_config() -> &'static ParseMetricsConfig {
    static CONFIG: OnceLock<ParseMetricsConfig> = OnceLock::new();
    CONFIG.get_or_init(|| ParseMetricsConfig {
        slow_threshold: Duration::from_millis(env_u64("BSL_SLOW_MODULE_THRESHOLD_MS", 3000)),
        top_n: env_usize("BSL_SLOW_MODULE_TOP_N", 5).max(1),
        log_each: env_bool("BSL_MODULE_PARSE_LOG_EACH", false),
    })
}

fn env_u64(key: &str, default: u64) -> u64 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(default)
}

fn env_usize(key: &str, default: usize) -> usize {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(default)
}

fn env_bool(key: &str, default: bool) -> bool {
    match std::env::var(key) {
        Ok(v) => matches!(v.as_str(), "1" | "true" | "yes" | "on"),
        Err(_) => default,
    }
}

fn format_slow_modules_table<'a>(
    rows: impl Iterator<Item = &'a (Duration, PathBuf)>,
) -> String {
    let mut collected: Vec<(String, String)> = Vec::new();
    for (idx, (elapsed, path)) in rows.enumerate() {
        let rank = format!("{}", idx + 1);
        let duration = human_duration(*elapsed);
        let path_str = path.to_string_lossy().to_string();
        collected.push((rank, format!("{} | {}", duration, path_str)));
    }

    let rank_width = collected
        .iter()
        .map(|(rank, _)| rank.len())
        .max()
        .unwrap_or(1)
        .max(1);

    let detail_width = collected
        .iter()
        .map(|(_, detail)| detail.len())
        .max()
        .unwrap_or(1)
        .max(1);

    let mut out = String::new();
    let border = format!(
        "+-{:-<rank$}-+-{:-<detail$}-+",
        "",
        "",
        rank = rank_width,
        detail = detail_width
    );
    out.push_str(&border);
    out.push('\n');
    out.push_str(&format!(
        "| {:<rank$} | {:<detail$} |",
        "N",
        "duration | module_path",
        rank = rank_width,
        detail = detail_width
    ));
    out.push('\n');
    out.push_str(&border);
    out.push('\n');

    for (rank, detail) in collected {
        out.push_str(&format!(
            "| {:<rank$} | {:<detail$} |",
            rank,
            detail,
            rank = rank_width,
            detail = detail_width
        ));
        out.push('\n');
    }

    out.push_str(&border);
    out
}

fn human_duration(duration: Duration) -> String {
    let secs = duration.as_secs();
    let millis = duration.subsec_millis();
    if secs > 0 {
        format!("{}.{:03}s", secs, millis)
    } else {
        format!("{}ms", millis)
    }
}

fn normalize_union_parts(mut parts: Vec<String>) -> Vec<String> {
    parts.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
    parts.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
    parts
}

fn split_union_string(s: &str) -> Vec<String> {
    s.split('|')
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        .map(|p| p.to_string())
        .collect()
}

fn resolve_manager_owner_type_from_receiver(receiver: &[String]) -> Option<String> {
    // receiver: ["Справочники", "Контрагенты"] или ["Catalogs", "Контрагенты"]
    if receiver.len() != 2 {
        return None;
    }
    let kind = collection_name_to_metadata_kind(&receiver[0])?;
    let prefix = kind.faceted_type_prefix(&FacetKind::Manager);
    Some(format!("{}.{}", prefix, receiver[1]))
}

fn collection_name_to_metadata_kind(name: &str) -> Option<MetadataKind> {
    // Локальная таблица (чтобы не тянуть зависимость data layer -> application).
    match name {
        "Справочники" | "Catalogs" => Some(MetadataKind::Catalog),
        "Документы" | "Documents" => Some(MetadataKind::Document),
        "Перечисления" | "Enums" => Some(MetadataKind::Enum),
        "РегистрыСведений" | "InformationRegisters" => Some(MetadataKind::InformationRegister),
        "РегистрыНакопления" | "AccumulationRegisters" => Some(MetadataKind::AccumulationRegister),
        "РегистрыБухгалтерии" | "AccountingRegisters" => Some(MetadataKind::AccountingRegister),
        "РегистрыРасчета" | "CalculationRegisters" => Some(MetadataKind::CalculationRegister),
        "Отчеты" | "Reports" => Some(MetadataKind::Report),
        "Обработки" | "DataProcessors" => Some(MetadataKind::DataProcessor),
        "ПланыСчетов" | "ChartsOfAccounts" => Some(MetadataKind::ChartOfAccounts),
        "ПланыВидовХарактеристик" | "ChartsOfCharacteristicTypes" => {
            Some(MetadataKind::ChartOfCharacteristicTypes)
        }
        "ПланыВидовРасчета" | "ChartsOfCalculationTypes" => {
            Some(MetadataKind::ChartOfCalculationTypes)
        }
        "БизнесПроцессы" | "BusinessProcesses" => Some(MetadataKind::BusinessProcess),
        "Задачи" | "Tasks" => Some(MetadataKind::Task),
        _ => None,
    }
}

fn infer_return_type_from_body(body: &[crate::parsing::bsl::ast::Statement]) -> Option<String> {
    use crate::parsing::bsl::ast::Statement;

    fn collect_return_types(
        acc: &mut Vec<String>,
        statements: &[Statement],
    ) {
        for st in statements {
            match st {
                Statement::Return { value, .. } => {
                    let inferred = match value {
                        None => Some("Неопределено".to_string()),
                        Some(expr) => infer_expr_type(expr),
                    };
                    if let Some(t) = inferred {
                        acc.push(t);
                    }
                }
                Statement::If {
                    then_body,
                    else_body,
                    ..
                } => {
                    collect_return_types(acc, then_body);
                    if let Some(else_body) = else_body {
                        collect_return_types(acc, else_body);
                    }
                }
                Statement::For { body, .. }
                | Statement::ForEach { body, .. }
                | Statement::While { body, .. } => {
                    collect_return_types(acc, body);
                }
                Statement::Try {
                    try_body,
                    except_body,
                    ..
                } => {
                    collect_return_types(acc, try_body);
                    collect_return_types(acc, except_body);
                }
                _ => {}
            }
        }
    }

    let mut collected = Vec::new();
    collect_return_types(&mut collected, body);

    if collected.is_empty() {
        return Some("Неопределено".to_string());
    }

    Some(normalize_union_parts(collected).join(" | "))
}

fn infer_expr_type(expr: &crate::parsing::bsl::ast::Expression) -> Option<String> {
    use crate::parsing::bsl::ast::Expression;
    match expr {
        Expression::String { .. } => Some("Строка".to_string()),
        Expression::Number { .. } => Some("Число".to_string()),
        Expression::Boolean { .. } => Some("Булево".to_string()),
        Expression::Date { .. } => Some("Дата".to_string()),
        Expression::New { type_name, .. } => Some(type_name.clone()),
        Expression::Identifier { name, .. } if name.eq_ignore_ascii_case("неопределено") => {
            Some("Неопределено".to_string())
        }
        _ => None,
    }
}

fn context_from_directive(
    directive: crate::parsing::bsl::ast::CompilerDirective,
) -> ContextRequirements {
    use crate::parsing::bsl::ast::CompilerDirective as D;
    match directive {
        D::OnServer | D::OnServerNoContext => ContextRequirements::ServerOnly,
        D::OnClient => ContextRequirements::ClientOnly,
        D::OnClientOnServerNoContext => ContextRequirements::Universal,
        D::Unknown => ContextRequirements::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::loaders::config_metadata_parser::types::ReturnValuesReuse;
    use bsl_shared::domain::type_definition_location::TypeDefinitionLocation;
    use bsl_shared::domain::repository::InMemoryTypeRepository;
    use bsl_shared::domain::resolver::{TypeResolver, ValidationResultV2};
    use tempfile::TempDir;

    fn write(path: &Path, content: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
    }

    #[test]
    fn indexes_manager_module_exports() {
        let tmp = TempDir::new().unwrap();
        let module_path = tmp
            .path()
            .join("Catalogs")
            .join("Контрагенты")
            .join("Ext")
            .join("ManagerModule.bsl");

        write(
            &module_path,
            r#"
Процедура Тест(П1) Экспорт
КонецПроцедуры

Процедура Скрытая()
КонецПроцедуры
"#,
        );

        let mut obj = UniversalMetadataObject::new(
            "Catalog".to_string(),
            "Контрагенты".to_string(),
            "00000000-0000-0000-0000-000000000000".to_string(),
        );
        obj.manager_module_path = Some(module_path);

        let indexed = index_configuration_bsl_modules(tmp.path(), &[obj]).unwrap();
        assert!(
            indexed
                .config_methods
                .iter()
                .any(|(t, m)| t == "СправочникМенеджер.Контрагенты" && m.name == "Тест")
        );
        assert!(
            !indexed
                .config_methods
                .iter()
                .any(|(_, m)| m.name == "Скрытая")
        );
    }

    #[test]
    fn indexes_common_module_exports_and_global_functions() {
        let tmp = TempDir::new().unwrap();
        let module_path = tmp
            .path()
            .join("CommonModules")
            .join("ОбщийМодуль1")
            .join("Ext")
            .join("Module.bsl");

        write(
            &module_path,
            r#"
Функция Ф1() Экспорт
    Возврат 1;
КонецФункции
"#,
        );

        let mut cm = UniversalMetadataObject::new(
            "CommonModule".to_string(),
            "ОбщийМодуль1".to_string(),
            "00000000-0000-0000-0000-000000000000".to_string(),
        );
        cm.common_module_properties = Some(CommonModuleProperties {
            server: true,
            client_managed_application: false,
            client_ordinary_application: false,
            external_connection: false,
            server_call: false,
            global: true,
            privileged: false,
            compile: true,
            return_values_reuse: ReturnValuesReuse::DontUse,
        });

        let indexed = index_configuration_bsl_modules(tmp.path(), &[cm]).unwrap();
        assert!(
            indexed
                .config_methods
                .iter()
                .any(|(t, m)| t == "ОбщиеМодули.ОбщийМодуль1" && m.name == "Ф1")
        );
        let sig = indexed
            .config_methods
            .iter()
            .find(|(t, m)| t == "ОбщиеМодули.ОбщийМодуль1" && m.name == "Ф1")
            .map(|(_, m)| m)
            .unwrap();
        assert_eq!(sig.return_type.as_deref(), Some("Число"));
        assert!(indexed
            .global_functions
            .iter()
            .any(|(n, m)| n == "Ф1" && m.owner_type.is_none()));
    }

    #[test]
    fn infers_function_return_type_union() {
        let tmp = TempDir::new().unwrap();
        let module_path = tmp
            .path()
            .join("CommonModules")
            .join("ОбщийМодуль1")
            .join("Ext")
            .join("Module.bsl");

        write(
            &module_path,
            r#"
Функция Ф1(Флаг) Экспорт
    Если Флаг Тогда
        Возврат 1;
    Иначе
        Возврат "x";
    КонецЕсли;
КонецФункции
"#,
        );

        let mut cm = UniversalMetadataObject::new(
            "CommonModule".to_string(),
            "ОбщийМодуль1".to_string(),
            "00000000-0000-0000-0000-000000000000".to_string(),
        );
        cm.common_module_properties = Some(CommonModuleProperties {
            server: true,
            client_managed_application: true,
            client_ordinary_application: false,
            external_connection: false,
            server_call: false,
            global: false,
            privileged: false,
            compile: true,
            return_values_reuse: ReturnValuesReuse::DontUse,
        });

        let indexed = index_configuration_bsl_modules(tmp.path(), &[cm]).unwrap();
        let sig = indexed
            .config_methods
            .iter()
            .find(|(t, m)| t == "ОбщиеМодули.ОбщийМодуль1" && m.name == "Ф1")
            .map(|(_, m)| m)
            .unwrap();
        assert_eq!(sig.return_type.as_deref(), Some("Строка | Число"));
    }

    #[test]
    fn indexes_record_set_module_exports_as_object_facet() {
        let tmp = TempDir::new().unwrap();
        let module_path = tmp
            .path()
            .join("InformationRegisters")
            .join("РегистрСведений1")
            .join("Ext")
            .join("RecordSetModule.bsl");

        write(
            &module_path,
            r#"
Процедура Провести() Экспорт
КонецПроцедуры
"#,
        );

        let mut obj = UniversalMetadataObject::new(
            "InformationRegister".to_string(),
            "РегистрСведений1".to_string(),
            "00000000-0000-0000-0000-000000000000".to_string(),
        );
        obj.record_set_module_path = Some(module_path);

        let indexed = index_configuration_bsl_modules(tmp.path(), &[obj]).unwrap();
        assert!(
            indexed
                .config_methods
                .iter()
                .any(|(t, m)| t == "РегистрСведенийНаборЗаписей.РегистрСведений1" && m.name == "Провести")
        );
    }

    #[test]
    fn infers_param_types_from_call_sites_inside_module() {
        let tmp = TempDir::new().unwrap();
        let module_path = tmp
            .path()
            .join("CommonModules")
            .join("ОбщийМодуль1")
            .join("Ext")
            .join("Module.bsl");

        write(
            &module_path,
            r#"
Функция Ф1(П) Экспорт
    Возврат П;
КонецФункции

Процедура Тест()
    Ф1(1);
    Ф1("x");
КонецПроцедуры
"#,
        );

        let mut cm = UniversalMetadataObject::new(
            "CommonModule".to_string(),
            "ОбщийМодуль1".to_string(),
            "00000000-0000-0000-0000-000000000000".to_string(),
        );
        cm.common_module_properties = Some(CommonModuleProperties {
            server: true,
            client_managed_application: true,
            client_ordinary_application: false,
            external_connection: false,
            server_call: false,
            global: false,
            privileged: false,
            compile: true,
            return_values_reuse: ReturnValuesReuse::DontUse,
        });

        let indexed = index_configuration_bsl_modules(tmp.path(), &[cm]).unwrap();
        let sig = indexed
            .config_methods
            .iter()
            .find(|(t, m)| t == "ОбщиеМодули.ОбщийМодуль1" && m.name == "Ф1")
            .map(|(_, m)| m)
            .unwrap();

        assert_eq!(sig.params.len(), 1);
        assert_eq!(sig.params[0].type_name.as_deref(), Some("Строка | Число"));
    }

    #[test]
    fn infers_param_types_from_local_variables_in_call_args() {
        let tmp = TempDir::new().unwrap();
        let module_path = tmp
            .path()
            .join("CommonModules")
            .join("ОбщийМодуль1")
            .join("Ext")
            .join("Module.bsl");

        write(
            &module_path,
            r#"
Функция Ф1(П) Экспорт
    Возврат П;
КонецФункции

Процедура Тест()
    Х = Новый Массив();
    Ф1(Х);
КонецПроцедуры
"#,
        );

        let mut cm = UniversalMetadataObject::new(
            "CommonModule".to_string(),
            "ОбщийМодуль1".to_string(),
            "00000000-0000-0000-0000-000000000000".to_string(),
        );
        cm.common_module_properties = Some(CommonModuleProperties {
            server: true,
            client_managed_application: true,
            client_ordinary_application: false,
            external_connection: false,
            server_call: false,
            global: false,
            privileged: false,
            compile: true,
            return_values_reuse: ReturnValuesReuse::DontUse,
        });

        let indexed = index_configuration_bsl_modules(tmp.path(), &[cm]).unwrap();
        let sig = indexed
            .config_methods
            .iter()
            .find(|(t, m)| t == "ОбщиеМодули.ОбщийМодуль1" && m.name == "Ф1")
            .map(|(_, m)| m)
            .unwrap();

        assert_eq!(sig.params.len(), 1);
        assert_eq!(sig.params[0].type_name.as_deref(), Some("Массив"));
    }

    #[test]
    fn infers_param_types_from_common_module_qualified_calls_across_modules() {
        let tmp = TempDir::new().unwrap();

        let module1_path = tmp
            .path()
            .join("CommonModules")
            .join("ОбщийМодуль1")
            .join("Ext")
            .join("Module.bsl");
        write(
            &module1_path,
            r#"
Функция Ф1(П) Экспорт
    Возврат П;
КонецФункции
"#,
        );

        let module2_path = tmp
            .path()
            .join("CommonModules")
            .join("ОбщийМодуль2")
            .join("Ext")
            .join("Module.bsl");
        write(
            &module2_path,
            r#"
Процедура Тест()
    ОбщийМодуль1.Ф1(1);
    ОбщийМодуль1.Ф1("x");
КонецПроцедуры
"#,
        );

        let mut cm1 = UniversalMetadataObject::new(
            "CommonModule".to_string(),
            "ОбщийМодуль1".to_string(),
            "00000000-0000-0000-0000-000000000000".to_string(),
        );
        cm1.common_module_properties = Some(CommonModuleProperties {
            server: true,
            client_managed_application: true,
            client_ordinary_application: false,
            external_connection: false,
            server_call: false,
            global: false,
            privileged: false,
            compile: true,
            return_values_reuse: ReturnValuesReuse::DontUse,
        });

        let mut cm2 = UniversalMetadataObject::new(
            "CommonModule".to_string(),
            "ОбщийМодуль2".to_string(),
            "00000000-0000-0000-0000-000000000000".to_string(),
        );
        cm2.common_module_properties = Some(CommonModuleProperties {
            server: true,
            client_managed_application: true,
            client_ordinary_application: false,
            external_connection: false,
            server_call: false,
            global: false,
            privileged: false,
            compile: true,
            return_values_reuse: ReturnValuesReuse::DontUse,
        });

        let indexed = index_configuration_bsl_modules(tmp.path(), &[cm1, cm2]).unwrap();
        let sig = indexed
            .config_methods
            .iter()
            .find(|(t, m)| t == "ОбщиеМодули.ОбщийМодуль1" && m.name == "Ф1")
            .map(|(_, m)| m)
            .unwrap();

        assert_eq!(sig.params.len(), 1);
        assert_eq!(sig.params[0].type_name.as_deref(), Some("Строка | Число"));
    }

    #[test]
    fn infers_param_types_for_manager_module_from_catalog_collection_call() {
        let tmp = TempDir::new().unwrap();

        let manager_module_path = tmp
            .path()
            .join("Catalogs")
            .join("Контрагенты")
            .join("Ext")
            .join("ManagerModule.bsl");
        write(
            &manager_module_path,
            r#"
Функция М(П) Экспорт
    Возврат П;
КонецФункции
"#,
        );

        let common_module_path = tmp
            .path()
            .join("CommonModules")
            .join("ОбщийМодуль2")
            .join("Ext")
            .join("Module.bsl");
        write(
            &common_module_path,
            r#"
Процедура Тест()
    Х = 1;
    Справочники.Контрагенты.М(Х);
КонецПроцедуры
"#,
        );

        let mut catalog = UniversalMetadataObject::new(
            "Catalog".to_string(),
            "Контрагенты".to_string(),
            "00000000-0000-0000-0000-000000000000".to_string(),
        );
        catalog.manager_module_path = Some(manager_module_path);

        let mut cm2 = UniversalMetadataObject::new(
            "CommonModule".to_string(),
            "ОбщийМодуль2".to_string(),
            "00000000-0000-0000-0000-000000000000".to_string(),
        );
        cm2.common_module_properties = Some(CommonModuleProperties {
            server: true,
            client_managed_application: true,
            client_ordinary_application: false,
            external_connection: false,
            server_call: false,
            global: false,
            privileged: false,
            compile: true,
            return_values_reuse: ReturnValuesReuse::DontUse,
        });

        let indexed = index_configuration_bsl_modules(tmp.path(), &[catalog, cm2]).unwrap();
        let sig = indexed
            .config_methods
            .iter()
            .find(|(t, m)| t == "СправочникМенеджер.Контрагенты" && m.name == "М")
            .map(|(_, m)| m)
            .unwrap();

        assert_eq!(sig.params.len(), 1);
        assert_eq!(sig.params[0].type_name.as_deref(), Some("Число"));
    }

    #[test]
    fn inferred_param_types_are_used_by_call_validation_v2() {
        use bsl_shared::domain::repository::TypeRepository;

        let tmp = TempDir::new().unwrap();

        let manager_module_path = tmp
            .path()
            .join("Catalogs")
            .join("Контрагенты")
            .join("Ext")
            .join("ManagerModule.bsl");
        write(
            &manager_module_path,
            r#"
Функция М(П) Экспорт
    Возврат П;
КонецФункции
"#,
        );

        let common_module_path = tmp
            .path()
            .join("CommonModules")
            .join("ОбщийМодуль2")
            .join("Ext")
            .join("Module.bsl");
        write(
            &common_module_path,
            r#"
Процедура Тест()
    Справочники.Контрагенты.М(1);
КонецПроцедуры
"#,
        );

        let mut catalog = UniversalMetadataObject::new(
            "Catalog".to_string(),
            "Контрагенты".to_string(),
            "00000000-0000-0000-0000-000000000000".to_string(),
        );
        catalog.manager_module_path = Some(manager_module_path);

        let mut cm2 = UniversalMetadataObject::new(
            "CommonModule".to_string(),
            "ОбщийМодуль2".to_string(),
            "00000000-0000-0000-0000-000000000000".to_string(),
        );
        cm2.common_module_properties = Some(CommonModuleProperties {
            server: true,
            client_managed_application: true,
            client_ordinary_application: false,
            external_connection: false,
            server_call: false,
            global: false,
            privileged: false,
            compile: true,
            return_values_reuse: ReturnValuesReuse::DontUse,
        });

        let indexed = index_configuration_bsl_modules(tmp.path(), &[catalog, cm2]).unwrap();

        let repo = std::sync::Arc::new(InMemoryTypeRepository::new());
        for (owner_type, sig) in indexed.config_methods {
            repo.add_config_method_signature(&owner_type, sig);
        }

        let signature_index = repo.get_signature_index_clone();
        let resolver = TypeResolver::new(repo);

        let ok = resolver.validate_call_v2(
            Some("СправочникМенеджер.Контрагенты"),
            "М",
            &["Число".to_string()],
            &signature_index,
        );
        assert!(matches!(ok, ValidationResultV2::Ok(_)));

        let bad = resolver.validate_call_v2(
            Some("СправочникМенеджер.Контрагенты"),
            "М",
            &["Булево".to_string()],
            &signature_index,
        );
        assert!(
            matches!(bad, ValidationResultV2::TypeMismatch { .. }),
            "expected TypeMismatch for Булево vs inferred Число, got: {:?}",
            bad
        );
    }

    #[test]
    fn exports_include_definition_location() {
        let tmp = TempDir::new().unwrap();
        let module_path = tmp
            .path()
            .join("CommonModules")
            .join("ОбщийМодуль1")
            .join("Ext")
            .join("Module.bsl");

        write(
            &module_path,
            r#"
Функция Ф1() Экспорт
    Возврат 1;
КонецФункции
"#,
        );

        let mut cm = UniversalMetadataObject::new(
            "CommonModule".to_string(),
            "ОбщийМодуль1".to_string(),
            "00000000-0000-0000-0000-000000000000".to_string(),
        );
        cm.common_module_properties = Some(CommonModuleProperties {
            server: true,
            client_managed_application: false,
            client_ordinary_application: false,
            external_connection: false,
            server_call: false,
            global: false,
            privileged: false,
            compile: true,
            return_values_reuse: ReturnValuesReuse::DontUse,
        });

        let indexed = index_configuration_bsl_modules(tmp.path(), &[cm]).unwrap();
        let loc = indexed
            .definition_locations
            .iter()
            .find(|(owner, name, _)| owner == "ОбщиеМодули.ОбщийМодуль1" && name == "Ф1")
            .map(|(_, _, l)| l)
            .expect("expected definition location for Ф1");

        match loc {
            TypeDefinitionLocation::UserDefined { file_path, .. } => {
                assert!(file_path.ends_with("CommonModules/ОбщийМодуль1/Ext/Module.bsl"));
            }
            other => panic!("expected UserDefined location, got: {:?}", other),
        }
    }

    #[test]
    fn exports_include_global_definition_location_for_global_common_module() {
        let tmp = TempDir::new().unwrap();
        let module_path = tmp
            .path()
            .join("CommonModules")
            .join("ОбщийМодуль1")
            .join("Ext")
            .join("Module.bsl");

        write(
            &module_path,
            r#"
Процедура П1() Экспорт
КонецПроцедуры
"#,
        );

        let mut cm = UniversalMetadataObject::new(
            "CommonModule".to_string(),
            "ОбщийМодуль1".to_string(),
            "00000000-0000-0000-0000-000000000000".to_string(),
        );
        cm.common_module_properties = Some(CommonModuleProperties {
            server: true,
            client_managed_application: false,
            client_ordinary_application: false,
            external_connection: false,
            server_call: false,
            global: true,
            privileged: false,
            compile: true,
            return_values_reuse: ReturnValuesReuse::DontUse,
        });

        let indexed = index_configuration_bsl_modules(tmp.path(), &[cm]).unwrap();
        let loc = indexed
            .global_definition_locations
            .iter()
            .find(|(name, _)| name == "П1")
            .map(|(_, l)| l)
            .expect("expected global definition location for П1");

        match loc {
            TypeDefinitionLocation::UserDefined { file_path, .. } => {
                assert!(file_path.ends_with("CommonModules/ОбщийМодуль1/Ext/Module.bsl"));
            }
            other => panic!("expected UserDefined location, got: {:?}", other),
        }
    }

    #[test]
    fn indexes_common_module_exports_with_utf8_bom() {
        let tmp = TempDir::new().unwrap();
        let module_path = tmp
            .path()
            .join("CommonModules")
            .join("ОбщийМодуль1")
            .join("Ext")
            .join("Module.bsl");

        write(
            &module_path,
            "\u{FEFF}Процедура П1() Экспорт\r\nКонецПроцедуры\r\n",
        );

        let mut cm = UniversalMetadataObject::new(
            "CommonModule".to_string(),
            "ОбщийМодуль1".to_string(),
            "00000000-0000-0000-0000-000000000000".to_string(),
        );
        cm.common_module_properties = Some(CommonModuleProperties {
            server: true,
            client_managed_application: false,
            client_ordinary_application: false,
            external_connection: false,
            server_call: false,
            global: false,
            privileged: false,
            compile: true,
            return_values_reuse: ReturnValuesReuse::DontUse,
        });

        let indexed = index_configuration_bsl_modules(tmp.path(), &[cm]).unwrap();
        assert!(
            indexed
                .config_methods
                .iter()
                .any(|(owner, sig)| owner == "ОбщиеМодули.ОбщийМодуль1" && sig.name == "П1"),
            "expected to index exported procedure П1 from a UTF-8 BOM file"
        );
    }
}
