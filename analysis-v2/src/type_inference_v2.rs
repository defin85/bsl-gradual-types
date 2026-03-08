use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use bsl_shared::domain::is_configuration_type_pattern;
use bsl_shared::domain::resolver::TypeResolver;
use bsl_shared::domain::signature_index::SignatureIndex;
use bsl_shared::domain::types::MetadataKind;
use bsl_shared::domain::types::{
    Certainty, ContextualTypeDescriptor, FacetKind, GenericType, ResolutionMetadata,
    ResolutionResult, ResolutionSource,
};
use bsl_shared::domain::types::{ConcreteType, TypeResolution, UncertaintyReason, WeightedType};
use bsl_shared::domain::TypeMetadataLookup;
use bsl_shared::domain::{CodeLocation, ModuleType};
use bsl_syntax::ast::{Expression, Program, Statement};

use crate::ast_to_ir::{is_global_collection, lookup_global_collection};
use crate::implicit_bindings::{
    directive_disables_form_context, ImplicitBindingResolver, FORM_CONTEXT_BOUND_SYMBOL_KEYS,
};
use crate::SemanticDeps;

#[derive(Debug, Clone)]
pub(crate) struct TypeIndexEntry {
    pub span: bsl_shared::ir::Span,
    pub resolution: TypeResolution,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct TypeIndex {
    entries: Vec<TypeIndexEntry>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct TypeIndexBuildProfile {
    pub seed_module_context_ms: u128,
    pub local_function_summaries_ms: u128,
    pub visit_statements_ms: u128,
    pub total_ms: u128,
    pub statement_count: u64,
    pub local_function_summary_count: u64,
    pub index_entry_count: u64,
}

#[derive(Debug, Clone)]
pub(crate) struct TypeIndexBuildProfiled {
    pub index: TypeIndex,
    pub profile: TypeIndexBuildProfile,
}

impl TypeIndex {
    pub(crate) fn type_for_exact_span(&self, span: bsl_shared::ir::Span) -> Option<TypeResolution> {
        self.entries
            .iter()
            .find(|entry| entry.span == span)
            .map(|entry| entry.resolution.clone())
    }

    pub(crate) fn type_at_byte_offset(&self, byte_offset: u32) -> Option<TypeResolution> {
        let find = |offset: u32| {
            self.entries
                .iter()
                .filter(|entry| entry.span.contains(offset))
                .min_by_key(|entry| entry.span.len())
                .map(|entry| entry.resolution.clone())
        };

        // Аналогично IR `find_node_at_byte_offset`: если курсор на границе `end`,
        // пробуем сместиться на 1 байт влево.
        find(byte_offset).or_else(|| byte_offset.checked_sub(1).and_then(find))
    }
}

#[derive(Clone)]
struct TypeEnv {
    variables: HashMap<String, TypeResolution>,
    instance_bindings: HashMap<String, InstanceBinding>,
    description_type_bindings: HashMap<String, TypeResolution>,
    instance_effects: InstanceEffectStore,
    local_function_summaries: Arc<HashMap<String, LocalFunctionSummary>>,
    module_type: Option<ModuleType>,
}

impl Default for TypeEnv {
    fn default() -> Self {
        Self {
            variables: HashMap::new(),
            instance_bindings: HashMap::new(),
            description_type_bindings: HashMap::new(),
            instance_effects: InstanceEffectStore::default(),
            local_function_summaries: Arc::new(HashMap::new()),
            module_type: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct LocalFunctionSummary {
    return_type: TypeResolution,
    may_fallthrough: bool,
}

struct TypeInferencer {
    deps: Arc<SemanticDeps>,
    resolver: Arc<TypeResolver>,
    signature_index: SignatureIndex,
    metadata_lookup: TypeMetadataLookup,
}

#[path = "type_inference_v2/expression_helpers.rs"]
mod expression_helpers;
#[path = "type_inference_v2/instance_effects.rs"]
mod instance_effects;
#[path = "type_inference_v2/local_function_summaries.rs"]
mod local_function_summaries;

use self::expression_helpers::{expr_span, signature_lookup_type_name};
use self::instance_effects::{
    arbitrary_resolution, merge_resolutions, normalize_schema_value_type, strip_structural_members,
    InstanceBinding, InstanceEffectStore, InstanceId,
};

impl TypeInferencer {
    fn new(deps: Arc<SemanticDeps>) -> Self {
        let resolver = deps
            .resolver
            .clone()
            .unwrap_or_else(|| Arc::new(TypeResolver::new(deps.repository.clone())));
        let signature_index = deps.signature_index.clone();
        let metadata_lookup = TypeMetadataLookup::new(deps.repository.clone());
        Self {
            deps,
            resolver,
            signature_index,
            metadata_lookup,
        }
    }

    fn build_index(&self, program: &Program, file_path: &str) -> TypeIndex {
        self.build_index_profiled(program, file_path).index
    }

    fn build_index_profiled(&self, program: &Program, file_path: &str) -> TypeIndexBuildProfiled {
        let started = Instant::now();
        let mut env = TypeEnv::default();
        let mut index = TypeIndex::default();

        let seed_started = Instant::now();
        self.seed_module_context(file_path, &mut env);
        let seed_module_context_ms = seed_started.elapsed().as_millis();

        let local_function_summaries_started = Instant::now();
        let local_function_summaries = self.infer_local_function_summaries(program, &env);
        let local_function_summary_count = local_function_summaries.len() as u64;
        env.local_function_summaries = Arc::new(local_function_summaries);
        let local_function_summaries_ms = local_function_summaries_started.elapsed().as_millis();

        let visit_statements_started = Instant::now();
        for stmt in &program.statements {
            self.visit_statement(stmt, &mut env, &mut index);
        }
        let visit_statements_ms = visit_statements_started.elapsed().as_millis();

        TypeIndexBuildProfiled {
            profile: TypeIndexBuildProfile {
                seed_module_context_ms,
                local_function_summaries_ms,
                visit_statements_ms,
                total_ms: started.elapsed().as_millis(),
                statement_count: program.statements.len() as u64,
                local_function_summary_count,
                index_entry_count: index.entries.len() as u64,
            },
            index,
        }
    }

    fn seed_module_context(&self, file_path: &str, env: &mut TypeEnv) {
        let path = Path::new(file_path);
        let Ok(location) = CodeLocation::determine_from_path(path) else {
            return;
        };
        env.module_type = Some(location.module_type.clone());

        let binding_resolver = ImplicitBindingResolver::new();
        for binding in binding_resolver.bindings_for_module(&location.module_type) {
            let resolution = binding
                .descriptor
                .as_ref()
                .map(|descriptor| self.resolve_contextual_descriptor(descriptor))
                .unwrap_or_else(TypeResolution::unknown);
            env.variables
                .insert(binding.name.to_lowercase(), resolution);
        }

        let ModuleType::FormModule {
            ref form_name,
            ref owner_type,
        } = location.module_type
        else {
            return;
        };

        let Some(type_names) = binding_resolver.form_module_type_names(owner_type, form_name)
        else {
            return;
        };
        let Some(form_type) = self.deps.repository.find_type(&type_names.form_type_name) else {
            return;
        };

        for prop in form_type.properties {
            let key = prop.name.to_lowercase();
            if env.variables.contains_key(&key) {
                continue;
            }
            if prop.prop_type.is_empty() {
                continue;
            }
            if prop.prop_type.contains("cfg:") {
                env.variables
                    .insert(key, TypeResolution::inferred(&prop.prop_type));
                continue;
            }
            if is_configuration_type_pattern(&prop.prop_type) {
                let resolved = self.resolver.resolve_expression_sync(&prop.prop_type);
                let resolved = if resolved.is_unknown() {
                    TypeResolution::inferred(&prop.prop_type)
                } else {
                    resolved
                };
                env.variables.insert(key, resolved);
                continue;
            }
            let resolved = if self.deps.repository.find_type(&prop.prop_type).is_some() {
                TypeResolution::explicit(&prop.prop_type)
            } else {
                self.try_resolve_configuration_type(&prop.prop_type)
                    .unwrap_or_else(|| self.resolver.resolve_expression_sync(&prop.prop_type))
            };
            env.variables.insert(key, resolved);
        }
    }

    fn resolve_contextual_descriptor(
        &self,
        descriptor: &ContextualTypeDescriptor,
    ) -> TypeResolution {
        match descriptor {
            ContextualTypeDescriptor::PlatformType { type_name } => {
                self.resolve_platform_descriptor_type(type_name)
            }
            ContextualTypeDescriptor::ConfigurationFacet { kind, name, facet } => {
                self.resolve_configuration_facet_descriptor(*kind, name, *facet)
            }
            ContextualTypeDescriptor::FormType { .. }
            | ContextualTypeDescriptor::FormElementsType { .. } => {
                self.resolve_platform_descriptor_type(&descriptor.canonical_type_name())
            }
            ContextualTypeDescriptor::FormDataObject {
                kind, owner_name, ..
            } => {
                let mut resolution = self.resolve_configuration_descriptor(*kind, owner_name);
                for note in descriptor.resolution_metadata_notes() {
                    if !resolution.metadata.notes.contains(&note) {
                        resolution.metadata.notes.push(note);
                    }
                }
                resolution
            }
        }
    }

    fn resolve_platform_descriptor_type(&self, type_name: &str) -> TypeResolution {
        let resolved = self.resolver.resolve_expression_sync(type_name);
        if !resolved.is_unknown() {
            return resolved;
        }

        if self.deps.repository.find_type(type_name).is_some() {
            TypeResolution::explicit(type_name)
        } else {
            TypeResolution::inferred_weak(type_name)
        }
    }

    fn resolve_configuration_descriptor(&self, kind: MetadataKind, name: &str) -> TypeResolution {
        let mut resolution = TypeResolution::metadata_type(kind, name, None);
        let metadata_type_name = format!("{}.{}", kind.to_prefix(), name);

        if let Some(raw) = self.deps.repository.find_type(&metadata_type_name) {
            resolution.available_facets = raw.facets.clone();
            return resolution;
        }

        resolution.certainty = Certainty::InferredWeak;
        resolution.source = ResolutionSource::Inferred;
        if !self.metadata_lookup.is_configuration_loaded() {
            resolution.metadata.uncertainty_reason =
                Some(UncertaintyReason::ConfigurationNotLoaded);
        }
        resolution
    }

    fn resolve_configuration_facet_descriptor(
        &self,
        kind: MetadataKind,
        name: &str,
        facet: FacetKind,
    ) -> TypeResolution {
        let mut resolution = TypeResolution::metadata_type(kind, name, Some(facet));
        let metadata_type_name = format!("{}.{}", kind.to_prefix(), name);

        if let Some(raw) = self.deps.repository.find_type(&metadata_type_name) {
            resolution.available_facets = raw.facets.clone();
            return resolution;
        }

        resolution.certainty = Certainty::InferredWeak;
        resolution.source = ResolutionSource::Inferred;
        if !self.metadata_lookup.is_configuration_loaded() {
            resolution.metadata.uncertainty_reason =
                Some(UncertaintyReason::ConfigurationNotLoaded);
        }
        resolution
    }

    fn visit_statement(&self, stmt: &Statement, env: &mut TypeEnv, index: &mut TypeIndex) {
        match stmt {
            Statement::VarDeclaration {
                name, type_hint, ..
            } => {
                let resolution = type_hint
                    .as_deref()
                    .map(TypeResolution::explicit)
                    .unwrap_or_else(TypeResolution::unknown);
                env.set_variable_value(name.to_lowercase(), resolution, None);
            }
            Statement::Assignment { target, value, .. } => {
                let value_type = self.infer_expr(value, env, index);
                if let Expression::Identifier { name, .. } = target {
                    let key = name.to_lowercase();
                    let description_type = self.extract_type_from_description_expr(value, env);
                    let binding = self.binding_for_assignment_value(value, &value_type, env);
                    let base_resolution = binding
                        .as_ref()
                        .map(|(base_resolution, _binding)| base_resolution.clone())
                        .unwrap_or_else(|| value_type.clone());
                    env.set_variable_value(
                        key.clone(),
                        base_resolution,
                        binding.map(|(_base_resolution, binding)| binding),
                    );
                    env.set_description_type_resolution(key.clone(), description_type);
                    // Hover/type-at-position на имени переменной после присваивания
                    // должен видеть новый тип.
                    if let Some(updated) = env.variable_resolution(&key) {
                        self.record(expr_span(target), updated, index);
                    }
                }
            }
            Statement::If {
                condition,
                then_body,
                else_body,
                ..
            } => {
                let _ = self.infer_expr(condition, env, index);
                let base_env = env.clone();
                let mut then_env = base_env.clone();
                for stmt in then_body {
                    self.visit_statement(stmt, &mut then_env, index);
                }
                let mut else_env = base_env.clone();
                if let Some(else_body) = else_body {
                    for stmt in else_body {
                        self.visit_statement(stmt, &mut else_env, index);
                    }
                }
                *env = self.merge_control_flow_env(&base_env, &then_env, &else_env);
            }
            Statement::While {
                condition, body, ..
            } => {
                let _ = self.infer_expr(condition, env, index);
                let base_env = env.clone();
                let mut body_env = base_env.clone();
                for stmt in body {
                    self.visit_statement(stmt, &mut body_env, index);
                }
                *env = self.merge_control_flow_env(&base_env, &body_env, &base_env);
            }
            Statement::For {
                variable,
                start,
                end,
                body,
                ..
            } => {
                let _ = self.infer_expr(start, env, index);
                let _ = self.infer_expr(end, env, index);
                let base_env = env.clone();
                let mut body_env = base_env.clone();
                body_env.set_variable_value(
                    variable.to_lowercase(),
                    TypeResolution::primitive("Число"),
                    None,
                );
                for stmt in body {
                    self.visit_statement(stmt, &mut body_env, index);
                }
                *env = self.merge_control_flow_env(&base_env, &body_env, &base_env);
            }
            Statement::ForEach {
                variable,
                collection,
                body,
                ..
            } => {
                let collection_type = self.infer_expr(collection, env, index);
                let base_env = env.clone();
                let mut body_env = base_env.clone();
                let foreach_binding = self.binding_for_foreach_collection(collection, env);
                body_env.set_variable_value(
                    variable.to_lowercase(),
                    foreach_binding
                        .as_ref()
                        .map(|(resolution, _binding)| resolution.clone())
                        .unwrap_or(collection_type),
                    foreach_binding.map(|(_resolution, binding)| binding),
                );
                for stmt in body {
                    self.visit_statement(stmt, &mut body_env, index);
                }
                *env = self.merge_control_flow_env(&base_env, &body_env, &base_env);
            }
            Statement::Return {
                value: Some(value), ..
            } => {
                let _ = self.infer_expr(value, env, index);
            }
            Statement::Return { value: None, .. } => {}
            Statement::Try {
                try_body,
                except_body,
                ..
            } => {
                let base_env = env.clone();
                let mut try_env = base_env.clone();
                for stmt in try_body {
                    self.visit_statement(stmt, &mut try_env, index);
                }
                let mut except_env = base_env.clone();
                for stmt in except_body {
                    self.visit_statement(stmt, &mut except_env, index);
                }
                *env = self.merge_control_flow_env(&base_env, &try_env, &except_env);
            }
            Statement::Call { expression, .. } => {
                let _ = self.infer_expr(expression, env, index);
            }
            Statement::Execute { code, .. } => {
                let _ = self.infer_expr(code, env, index);
            }
            Statement::RaiseError {
                message: Some(message),
                ..
            } => {
                let _ = self.infer_expr(message, env, index);
            }
            Statement::RaiseError { message: None, .. } => {}
            Statement::AddHandler { event, handler, .. }
            | Statement::RemoveHandler { event, handler, .. } => {
                let _ = self.infer_expr(event, env, index);
                let _ = self.infer_expr(handler, env, index);
            }
            Statement::Await { expression, .. } => {
                let _ = self.infer_expr(expression, env, index);
            }
            Statement::FunctionDecl {
                params,
                body,
                compiler_directive,
                ..
            }
            | Statement::ProcedureDecl {
                params,
                body,
                compiler_directive,
                ..
            } => {
                // TODO(v2): полноценное вычисление типов внутри функций на основе call graph.
                // Пока строим индекс внутри тела, наследуя module-level окружение
                // (например, implicit переменные модуля формы) и добавляя параметры.
                let mut fn_env = env.clone();
                if directive_disables_form_context(*compiler_directive) {
                    for key in FORM_CONTEXT_BOUND_SYMBOL_KEYS {
                        fn_env.variables.remove(key);
                        fn_env.instance_bindings.remove(key);
                    }
                }
                for param in params {
                    fn_env.set_variable_value(
                        param.to_lowercase(),
                        TypeResolution::unknown(),
                        None,
                    );
                }
                for stmt in body {
                    self.visit_statement(stmt, &mut fn_env, index);
                }
            }
            _ => {}
        }
    }

    fn record(
        &self,
        span: bsl_shared::ir::Span,
        resolution: TypeResolution,
        index: &mut TypeIndex,
    ) {
        index.entries.push(TypeIndexEntry { span, resolution });
    }

    fn infer_expr(
        &self,
        expr: &Expression,
        env: &mut TypeEnv,
        index: &mut TypeIndex,
    ) -> TypeResolution {
        let resolution = match expr {
            Expression::Number { .. } => TypeResolution::primitive("Число"),
            Expression::String { .. } => TypeResolution::primitive("Строка"),
            Expression::Boolean { .. } => TypeResolution::primitive("Булево"),
            Expression::Date { .. } => TypeResolution::primitive("Дата"),
            Expression::Identifier { name, .. } => self.infer_identifier(name, env),
            Expression::New {
                type_name, args, ..
            } => self.infer_new_expression(type_name, args, env, index),
            Expression::PropertyAccess {
                object, property, ..
            } => {
                let object_resolution = self.infer_expr(object, env, index);
                self.infer_property_access(&object_resolution, property)
            }
            Expression::Call { function, args, .. } => self.infer_call(function, args, env, index),
            Expression::Binary {
                left,
                operator,
                right,
                ..
            } => {
                let left_type = self.infer_expr(left, env, index);
                let right_type = self.infer_expr(right, env, index);
                self.infer_binary(operator, &left_type, &right_type)
            }
            Expression::Unary { operand, .. } => self.infer_expr(operand, env, index),
            Expression::Ternary {
                condition,
                then_expr,
                else_expr,
                ..
            } => {
                let _ = self.infer_expr(condition, env, index);
                let then_type = self.infer_expr(then_expr, env, index);
                let else_type = self.infer_expr(else_expr, env, index);
                // TODO(v2): union типов.
                if then_type
                    .type_name()
                    .eq_ignore_ascii_case(&else_type.type_name())
                {
                    then_type
                } else {
                    TypeResolution::unknown()
                }
            }
            Expression::IndexAccess {
                object,
                index: index_expr,
                ..
            } => {
                let object_resolution = self.infer_expr(object, env, index);
                let _ = self.infer_expr(index_expr, env, index);
                self.resolve_index_access(expr, object, index_expr, &object_resolution, env)
            }
            Expression::Await { expression, .. } => self.infer_expr(expression, env, index),
        };

        self.record(expr_span(expr), resolution.clone(), index);
        resolution
    }

    fn infer_identifier(&self, name: &str, env: &TypeEnv) -> TypeResolution {
        let name_lower = name.to_lowercase();
        if name_lower == "неопределено" || name_lower == "undefined" {
            return TypeResolution::primitive("Неопределено");
        }
        if name_lower == "null" {
            return TypeResolution::primitive("Null");
        }
        if matches!(name_lower.as_str(), "истина" | "ложь" | "true" | "false") {
            return TypeResolution::primitive("Булево");
        }

        if let Some(value) = env.variable_resolution(&name_lower) {
            return value.clone();
        }

        if is_global_collection(name).is_some() {
            return TypeResolution::inferred(name);
        }

        let common_module_type = format!("ОбщиеМодули.{}", name);
        if self
            .deps
            .repository
            .find_type(&common_module_type)
            .is_some()
            || !self
                .signature_index
                .get_type_methods(&common_module_type)
                .is_empty()
        {
            return TypeResolution::metadata_type(MetadataKind::CommonModule, name, None);
        }

        if let Some(resolved) = self.infer_applied_owner_member_identifier(name, &name_lower, env) {
            return resolved;
        }

        TypeResolution::undeclared_variable(name)
    }

    fn infer_new_expression(
        &self,
        type_name: &str,
        args: &[Expression],
        env: &mut TypeEnv,
        index: &mut TypeIndex,
    ) -> TypeResolution {
        let clean = type_name.trim().trim_end_matches("()").trim();
        let mut resolution = match clean {
            "Массив" => TypeResolution::generic("Массив", &["?"], Certainty::InferredWeak),
            "Соответствие" => {
                TypeResolution::generic("Соответствие", &["?", "?"], Certainty::InferredWeak)
            }
            "Структура" => TypeResolution::explicit("Структура"),
            "ТаблицаЗначений" => TypeResolution::explicit("ТаблицаЗначений"),
            "Список" => TypeResolution::generic("Список", &["?"], Certainty::InferredWeak),
            _ => {
                if self.deps.repository.find_type(clean).is_some() {
                    TypeResolution::explicit(clean)
                } else {
                    let mut res = TypeResolution::primitive(clean);
                    res.certainty = Certainty::Unknown;
                    res.metadata.uncertainty_reason = Some(UncertaintyReason::TypeNotFound {
                        name: clean.to_string(),
                    });
                    res
                }
            }
        };

        let arg_types: Vec<TypeResolution> = args
            .iter()
            .map(|arg| self.infer_expr(arg, env, index))
            .collect();

        if clean == "Структура" {
            self.apply_structure_constructor_members(&mut resolution, args, &arg_types);
        }

        resolution
    }

    fn infer_property_access(
        &self,
        object_type: &TypeResolution,
        property: &str,
    ) -> TypeResolution {
        let base_type = object_type.type_name();
        if let Some(info) = lookup_global_collection(&base_type) {
            // Справочники.Контрагенты -> СправочникМенеджер.Контрагенты
            let manager = format!("{}.{}", info.item_manager_type, property);
            return self.resolver.resolve_expression_sync(&manager);
        }

        let property_key = property.to_lowercase();
        if let Some(resolved) =
            self.resolve_property_type_by_name(object_type, property_key.as_str())
        {
            return resolved;
        }

        TypeResolution::unknown()
    }

    fn infer_call(
        &self,
        function: &Expression,
        args: &[Expression],
        env: &mut TypeEnv,
        index: &mut TypeIndex,
    ) -> TypeResolution {
        let arg_types: Vec<TypeResolution> = args
            .iter()
            .map(|arg| self.infer_expr(arg, env, index))
            .collect();

        match function {
            Expression::Identifier { name, .. } => self.infer_global_function_call(name, env),
            Expression::PropertyAccess {
                object, property, ..
            } => {
                let receiver = self.infer_expr(object, env, index);
                if let Some(resolved) = self.try_apply_universal_collection_method(
                    object, property, args, &arg_types, env, index,
                ) {
                    return resolved;
                }

                self.infer_method_call(&receiver, property)
            }
            _ => TypeResolution::unknown(),
        }
    }

    fn binding_for_assignment_value(
        &self,
        expr: &Expression,
        value_type: &TypeResolution,
        env: &mut TypeEnv,
    ) -> Option<(TypeResolution, InstanceBinding)> {
        match expr {
            Expression::Identifier { name, .. } => {
                let key = name.to_lowercase();
                let base_resolution = env.variable_base_resolution(&key)?.clone();
                let binding = env.variable_binding(&key)?.clone();
                Some((base_resolution, binding))
            }
            Expression::New {
                type_name, args, ..
            } => {
                let clean = type_name.trim().trim_end_matches("()").trim();
                let base_resolution = strip_structural_members(value_type.clone());
                let binding = match clean {
                    "Соответствие" => {
                        env.instance_effects.new_map_instance(&base_resolution)
                    }
                    "Структура" => {
                        let binding = env.instance_effects.new_structure_instance();
                        if let Some(instance_id) = InstanceEffectStore::direct_instance(&binding) {
                            for member in value_type.structural_members() {
                                env.instance_effects.insert_structure_field(
                                    instance_id,
                                    &member.canonical_name,
                                    (*member.member_type).clone(),
                                    bsl_shared::ir::Span::new(
                                        member
                                            .source_span
                                            .map(|span| span.start)
                                            .unwrap_or_else(|| expr_span(expr).start),
                                        member
                                            .source_span
                                            .map(|span| span.end)
                                            .unwrap_or_else(|| expr_span(expr).end),
                                    ),
                                    Some(member.member_id.clone()),
                                );
                            }
                        }
                        binding
                    }
                    "ТаблицаЗначений" => {
                        env.instance_effects.new_value_table_instance()
                    }
                    _ => return None,
                };

                let _ = args;
                Some((base_resolution, binding))
            }
            Expression::Call { function, .. } => {
                let Expression::PropertyAccess {
                    object, property, ..
                } = function.as_ref()
                else {
                    return None;
                };

                if !property.eq_ignore_ascii_case("Добавить") {
                    return None;
                }

                let table_instance = self.direct_instance_for_expr(object, env)?;
                if !env.instance_effects.is_value_table_instance(table_instance) {
                    return None;
                }

                let base_resolution = TypeResolution::explicit("СтрокаТаблицыЗначений");
                let binding = env.instance_effects.value_table_row_binding(table_instance);
                Some((base_resolution, binding))
            }
            _ => None,
        }
    }

    fn binding_for_foreach_collection(
        &self,
        collection: &Expression,
        env: &TypeEnv,
    ) -> Option<(TypeResolution, InstanceBinding)> {
        let table_instance = self.direct_instance_for_expr(collection, env)?;
        if !env.instance_effects.is_value_table_instance(table_instance) {
            return None;
        }

        let base_resolution = TypeResolution::explicit("СтрокаТаблицыЗначений");
        let binding = env.instance_effects.value_table_row_binding(table_instance);
        Some((base_resolution, binding))
    }

    fn direct_instance_for_expr(&self, expr: &Expression, env: &TypeEnv) -> Option<InstanceId> {
        match expr {
            Expression::Identifier { name, .. } => {
                let key = name.to_lowercase();
                env.variable_binding(&key)
                    .and_then(InstanceEffectStore::direct_instance)
            }
            _ => None,
        }
    }

    fn try_apply_universal_collection_method(
        &self,
        object: &Expression,
        property: &str,
        args: &[Expression],
        arg_types: &[TypeResolution],
        env: &mut TypeEnv,
        _index: &mut TypeIndex,
    ) -> Option<TypeResolution> {
        if property.eq_ignore_ascii_case("Вставить") || property.eq_ignore_ascii_case("Установить")
        {
            if let Some(instance_id) = self.direct_instance_for_expr(object, env) {
                if env.instance_effects.is_map_instance(instance_id) && args.len() >= 2 {
                    let value_type = normalize_schema_value_type(arg_types[1].clone());
                    env.instance_effects.insert_map_value(
                        instance_id,
                        self.extract_literal_key_with_normalized(&args[0]),
                        arg_types.first().cloned(),
                        value_type,
                        expr_span(&args[0]),
                    );
                    let receiver = self.infer_expr(object, env, _index);
                    return Some(self.infer_method_call(&receiver, property));
                }

                if env.instance_effects.is_structure_instance(instance_id) && args.len() >= 2 {
                    if let Some(field_name) = self.extract_literal_key(&args[0]) {
                        env.instance_effects.insert_structure_field(
                            instance_id,
                            &field_name,
                            normalize_schema_value_type(arg_types[1].clone()),
                            expr_span(&args[0]),
                            None,
                        );
                    }
                    let receiver = self.infer_expr(object, env, _index);
                    return Some(self.infer_method_call(&receiver, property));
                }
            }
        }

        if property.eq_ignore_ascii_case("Добавить") {
            if let Some(table_instance) = self.value_table_columns_owner_instance(object, env) {
                let column_name = args.first().and_then(|expr| self.extract_literal_key(expr));
                let column_type = args
                    .get(1)
                    .and_then(|expr| self.extract_type_from_description_expr(expr, env))
                    .map(normalize_schema_value_type);
                if let Some(column_name) = column_name {
                    env.instance_effects.insert_value_table_column(
                        table_instance,
                        &column_name,
                        column_type.unwrap_or_else(arbitrary_resolution),
                        expr_span(args.first().unwrap_or(object)),
                    );
                }
                let receiver = self.infer_expr(object, env, _index);
                return Some(self.infer_method_call(&receiver, property));
            }

            if let Some(table_instance) = self.direct_instance_for_expr(object, env) {
                if env.instance_effects.is_value_table_instance(table_instance) {
                    let binding = env.instance_effects.value_table_row_binding(table_instance);
                    let base_resolution = TypeResolution::explicit("СтрокаТаблицыЗначений");
                    return Some(
                        env.instance_effects
                            .materialize(&base_resolution, Some(&binding)),
                    );
                }
            }
        }

        None
    }

    fn value_table_columns_owner_instance(
        &self,
        expr: &Expression,
        env: &TypeEnv,
    ) -> Option<InstanceId> {
        let Expression::PropertyAccess {
            object, property, ..
        } = expr
        else {
            return None;
        };

        if !property.eq_ignore_ascii_case("Колонки") {
            return None;
        }

        let instance_id = self.direct_instance_for_expr(object, env)?;
        env.instance_effects
            .is_value_table_instance(instance_id)
            .then_some(instance_id)
    }

    fn extract_literal_key_with_normalized(&self, expr: &Expression) -> Option<(String, String)> {
        let canonical = self.extract_literal_key(expr)?;
        Some((
            bsl_shared::domain::type_id::normalize(&canonical),
            canonical,
        ))
    }

    fn extract_literal_key(&self, expr: &Expression) -> Option<String> {
        match expr {
            Expression::String { value, .. } => Some(value.clone()),
            Expression::Number { value, .. } => Some(value.to_string()),
            Expression::Boolean { value, .. } => Some(value.to_string()),
            _ => None,
        }
    }

    fn extract_type_from_description_expr(
        &self,
        expr: &Expression,
        env: &TypeEnv,
    ) -> Option<TypeResolution> {
        match expr {
            Expression::String { value, .. } => Some(self.resolve_declared_type_name(value)),
            Expression::New {
                type_name, args, ..
            } if type_name.trim().eq_ignore_ascii_case("ОписаниеТипов") => args
                .first()
                .and_then(|arg| self.extract_type_from_description_expr(arg, env)),
            Expression::Identifier { name, .. } => {
                env.description_type_resolution(&name.to_lowercase())
            }
            Expression::PropertyAccess {
                object, property, ..
            } => self.extract_type_from_description_qualifier(object, property),
            _ => None,
        }
    }

    fn extract_type_from_description_qualifier(
        &self,
        object: &Expression,
        property: &str,
    ) -> Option<TypeResolution> {
        let Expression::Identifier { name, .. } = object else {
            return None;
        };

        if name.eq_ignore_ascii_case("КвалификаторыСтрок")
            && property.eq_ignore_ascii_case("StringType")
        {
            return Some(TypeResolution::primitive("Строка"));
        }

        None
    }

    fn resolve_declared_type_name(&self, type_name: &str) -> TypeResolution {
        if let Some(resolved) = self.try_resolve_configuration_type(type_name) {
            return resolved;
        }

        let resolved = self.resolver.resolve_expression_sync(type_name);
        if resolved.is_unknown() && self.deps.repository.find_type(type_name).is_some() {
            return TypeResolution::explicit(type_name);
        }

        resolved
    }

    fn apply_structure_constructor_members(
        &self,
        resolution: &mut TypeResolution,
        args: &[Expression],
        arg_types: &[TypeResolution],
    ) {
        let Some(Expression::String { value: names, .. }) = args.first() else {
            return;
        };

        for (position, field_name) in names
            .split(',')
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .enumerate()
        {
            let value_type = arg_types
                .get(position + 1)
                .cloned()
                .map(normalize_schema_value_type)
                .unwrap_or_else(arbitrary_resolution);
            resolution.add_structural_member(bsl_shared::domain::types::StructuralMember::new(
                field_name.to_string(),
                value_type,
                args.get(position + 1).map(|expr| {
                    let span = expr_span(expr);
                    bsl_shared::domain::types::StructuralMemberSpan::new(span.start, span.end)
                }),
                Certainty::Inferred,
            ));
        }
    }

    fn merge_control_flow_env(&self, base: &TypeEnv, left: &TypeEnv, right: &TypeEnv) -> TypeEnv {
        let mut merged = base.clone();
        merged.instance_effects = InstanceEffectStore::merge_branch(
            &base.instance_effects,
            &left.instance_effects,
            &right.instance_effects,
        );

        let variable_keys: BTreeSet<String> = base
            .variables
            .keys()
            .chain(left.variables.keys())
            .chain(right.variables.keys())
            .cloned()
            .collect();

        for key in variable_keys {
            let left_base = left.variable_base_resolution(&key);
            let right_base = right.variable_base_resolution(&key);
            let merged_base = match (left_base, right_base) {
                (Some(left_base), Some(right_base)) => merge_resolutions(left_base, right_base),
                (Some(left_base), None) => left_base.clone(),
                (None, Some(right_base)) => right_base.clone(),
                (None, None) => continue,
            };

            let merged_binding = merged.instance_effects.merge_variable_binding(
                &left.instance_effects,
                left.variable_binding(&key),
                &right.instance_effects,
                right.variable_binding(&key),
            );
            let merged_description = match (
                left.description_type_resolution(&key),
                right.description_type_resolution(&key),
            ) {
                (Some(left_desc), Some(right_desc))
                    if left_desc
                        .type_name()
                        .eq_ignore_ascii_case(&right_desc.type_name()) =>
                {
                    Some(left_desc)
                }
                _ => None,
            };

            merged.set_variable_value(key.clone(), merged_base, merged_binding);
            merged.set_description_type_resolution(key, merged_description);
        }

        merged
    }

    fn resolve_index_access(
        &self,
        expr: &Expression,
        object: &Expression,
        index_expr: &Expression,
        object_resolution: &TypeResolution,
        env: &TypeEnv,
    ) -> TypeResolution {
        if let Some(instance_id) = self.direct_instance_for_expr(object, env) {
            if env.instance_effects.is_map_instance(instance_id) {
                let literal_key = self.extract_literal_key(index_expr);
                if let Some(resolved) = env
                    .instance_effects
                    .resolve_map_value(instance_id, literal_key.as_deref())
                {
                    return resolved;
                }
            }
        }

        if let ResolutionResult::Generic(GenericType {
            base_type,
            type_params,
        }) = &object_resolution.result
        {
            if base_type.eq_ignore_ascii_case("Соответствие") {
                if let Some(value_type) = type_params.get(1).and_then(|param| {
                    (!matches!(
                        param,
                        ConcreteType::Special(bsl_shared::domain::types::SpecialType::Undefined)
                    ))
                    .then(|| TypeResolution::known(param.clone()))
                }) {
                    return value_type;
                }
            }
        }

        let _ = expr;
        arbitrary_resolution()
    }

    fn infer_global_function_call(&self, name: &str, env: &TypeEnv) -> TypeResolution {
        let name_lower = name.to_lowercase();
        if let Some(local) = env.local_function_summaries.get(&name_lower) {
            return local.return_type.clone();
        }

        if let Some(sig) = self.signature_index.find_global_function(name) {
            if let Some(return_type) = sig.return_type.as_deref().filter(|s| !s.is_empty()) {
                if let Some(resolved) = self.try_resolve_configuration_type(return_type) {
                    return resolved;
                }
                return self.resolver.resolve_expression_sync(return_type);
            }
        }
        TypeResolution::unknown()
    }

    fn infer_method_call(&self, receiver: &TypeResolution, method: &str) -> TypeResolution {
        let type_name = signature_lookup_type_name(receiver);
        let metadata_name = SignatureIndex::extract_metadata_name(&type_name);
        let concretize_return_type = |return_type: &str| -> String {
            let Some(metadata_name) = metadata_name else {
                return return_type.to_string();
            };

            // Подставляем имя объекта только когда return type действительно шаблонный:
            // - содержит placeholder "<...>" / "&lt;...&gt;"
            // - или является фасетным базовым типом без ".Имя"
            //
            // Это снижает риск перезаписать уже-конкретизированный return type
            // (например "СправочникСсылка.Номенклатура").
            if return_type.contains('<')
                || return_type.contains("&lt;")
                || !return_type.contains('.')
            {
                let substituted = SignatureIndex::substitute_type_name(return_type, metadata_name);
                if substituted != return_type {
                    return substituted;
                }
            }

            return_type.to_string()
        };

        if let Some(sig) = self.signature_index.find_method(&type_name, method) {
            if let Some(return_type) = sig.return_type.as_deref().filter(|s| !s.is_empty()) {
                let return_type = concretize_return_type(return_type);
                if let Some(resolved) = self.try_resolve_configuration_type(&return_type) {
                    return resolved;
                }
                return self.resolver.resolve_expression_sync(&return_type);
            }
        }

        let methods = self.metadata_lookup.get_methods(receiver);
        let method_key = method.to_lowercase();
        if let Some(m) = methods
            .into_iter()
            .find(|m| m.name.to_lowercase() == method_key)
        {
            if let Some(return_type) = (!m.return_type.is_empty()).then_some(m.return_type) {
                let return_type = concretize_return_type(&return_type);
                if let Some(resolved) = self.try_resolve_configuration_type(&return_type) {
                    return resolved;
                }
                return self.resolver.resolve_expression_sync(&return_type);
            }
        }

        TypeResolution::unknown()
    }
}

pub(crate) fn build_type_index_with_path(
    program: &Program,
    file_path: &str,
    deps: Arc<SemanticDeps>,
) -> TypeIndex {
    TypeInferencer::new(deps).build_index(program, file_path)
}

pub(crate) fn build_type_index_with_path_profiled(
    program: &Program,
    file_path: &str,
    deps: Arc<SemanticDeps>,
) -> TypeIndexBuildProfiled {
    TypeInferencer::new(deps).build_index_profiled(program, file_path)
}

#[cfg(test)]
#[path = "type_inference_v2/tests.rs"]
mod tests;
