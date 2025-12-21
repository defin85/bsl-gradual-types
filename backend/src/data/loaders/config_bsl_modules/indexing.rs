use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};
use std::time::{Duration, Instant};

use anyhow::Result;
use bsl_shared::domain::code_location::{CodeLocation, ModuleType};
use bsl_shared::domain::signature_index::{ContextRequirements, MethodSignature, SignatureSource};
use bsl_shared::domain::type_definition_location::TypeDefinitionLocation;
use bsl_shared::domain::types::{FacetKind, MetadataKind, ParameterInfo};
use rayon::prelude::*;

use crate::data::loaders::config_metadata_parser::types::CommonModuleProperties;
use crate::data::loaders::UniversalMetadataObject;
use crate::system::fs_utils::read_bsl_file;

use super::inference::infer_export_param_types_across_modules;
use super::metrics::{human_duration, parse_metrics_config, report_slow_modules};
use super::parsing::parse_bsl_module;
use super::types::{
    IndexedConfigSignatures, ModuleIndexProgress, ParsedModule,
};

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
            let inferred_for_decl =
                inferred_param_types.get(&(module.owner_type_name.clone(), decl.name.clone()));

            let signature = MethodSignature::new(
                method_name.clone(),
                Some(module.owner_type_name.clone()),
                decl.params
                    .into_iter()
                    .enumerate()
                    .map(|(idx, p)| ParameterInfo {
                        name: p.name,
                        type_name: inferred_for_decl
                            .and_then(|v| v.get(idx))
                            .cloned()
                            .flatten(),
                        is_optional: p.is_optional,
                        default_value: None,
                        description: None,
                    })
                    .collect(),
                decl.return_type.clone(),
                None,
                None,
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
                        name: p.name,
                        type_name: inferred_for_decl
                            .and_then(|v| v.get(idx))
                            .cloned()
                            .flatten(),
                        is_optional: p.is_optional,
                        default_value: None,
                        description: None,
                    })
                    .collect(),
                decl.return_type.clone(),
                None,
                None,
                SignatureSource::Configuration,
                None,
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
        .filter_map(|obj| {
            obj.common_module_properties
                .as_ref()
                .map(|props| (obj.name.clone(), props.clone()))
        })
        .collect()
}

fn collect_module_paths(config_root: &Path, metadata: &[UniversalMetadataObject]) -> Vec<PathBuf> {
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
