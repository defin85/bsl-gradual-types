//! Индексация BSL модулей конфигурации (CommonModule/ObjectModule/ManagerModule/RecordSetModule)
//!
//! Цель: извлечь экспортные процедуры/функции из модулей конфигурации и
//! зарегистрировать их в SignatureIndex как `SignatureSource::Configuration`.
//!
//! На первом этапе извлекается только имя, список параметров и признак `Экспорт`.
//! Типы параметров и возвращаемых значений добавляются отдельными этапами (см. roadmap).

use crate::data::loaders::config_metadata_parser::types::CommonModuleProperties;
use crate::data::loaders::UniversalMetadataObject;
use crate::system::tree_sitter_adapter::TreeSitterAdapter;
use anyhow::{anyhow, Result};
use bsl_shared::domain::code_location::{CodeLocation, ModuleType};
use bsl_shared::domain::signature_index::{ContextRequirements, MethodSignature, SignatureSource};
use bsl_shared::domain::type_definition_location::TypeDefinitionLocation;
use bsl_shared::domain::types::{FacetKind, MetadataKind, ParameterInfo};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tree_sitter::Parser;

#[derive(Debug, Default)]
pub struct IndexedConfigSignatures {
    pub config_methods: Vec<(String, MethodSignature)>,
    pub global_functions: Vec<(String, MethodSignature)>,
    pub definition_locations: Vec<(String, String, TypeDefinitionLocation)>,
    pub global_definition_locations: Vec<(String, TypeDefinitionLocation)>,
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
    let common_module_props = collect_common_module_props(metadata);

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

    let mut out = IndexedConfigSignatures::default();

    let mut parsed_modules: Vec<ParsedModule> = Vec::new();

    for module_path in all_module_paths {
        let Ok(location) = CodeLocation::determine_from_path(&module_path) else {
            continue;
        };

        let Some(owner_type_name) =
            resolve_owner_type_for_signature(&location.module_type, &common_module_props)
        else {
            continue;
        };

        let source = match std::fs::read_to_string(&module_path) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!("Не удалось прочитать {:?}: {}", module_path, e);
                continue;
            }
        };

        let parsed = match parse_bsl_module(&source) {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!("Не удалось распарсить модуль {:?}: {}", module_path, e);
                continue;
            }
        };

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
fn parse_bsl_module(
    source: &str,
) -> Result<ParsedModuleData> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_bsl::LANGUAGE.into())
        .map_err(|e| anyhow!("tree-sitter-bsl language error: {:?}", e))?;

    let tree = parser
        .parse(source, None)
        .ok_or_else(|| anyhow!("tree-sitter parse returned None"))?;

    let parse_result =
        TreeSitterAdapter::convert_tree(&tree, source).map_err(|e| anyhow!(e))?;

    let call_sites = collect_call_sites(&parse_result.program.statements);
    let mut decls = Vec::new();

    for st in parse_result.program.statements {
        match st {
            crate::parsing::bsl::ast::Statement::FunctionDecl {
                name,
                params,
                body,
                compiler_directive,
                is_export,
                span,
                ..
            } => decls.push(ParsedDecl {
                return_type: infer_return_type_from_body(&body),
                name,
                params,
                is_export,
                directive_ctx: compiler_directive.map(context_from_directive),
                span,
            }),
            crate::parsing::bsl::ast::Statement::ProcedureDecl {
                name,
                params,
                body: _,
                compiler_directive,
                is_export,
                span,
                ..
            } => decls.push(ParsedDecl {
                return_type: None,
                name,
                params,
                is_export,
                directive_ctx: compiler_directive.map(context_from_directive),
                span,
            }),
            _ => {}
        }
    }

    Ok(ParsedModuleData { decls, call_sites })
}

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
}
