use std::cell::Cell;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use bsl_shared::domain::is_configuration_type_pattern;
use bsl_shared::domain::resolver::TypeResolver;
use bsl_shared::domain::signature_index::{MethodSignature, SignatureIndex, SignatureSource};
use bsl_shared::domain::types::MetadataKind;
use bsl_shared::domain::types::{
    Certainty, ContextualTypeDescriptor, FacetKind, GenericType, ResolutionMetadata,
    ResolutionResult, ResolutionSource,
};
use bsl_shared::domain::types::{
    ConcreteType, ParameterInfo, TypeResolution, UncertaintyReason, WeightedType,
};
use bsl_shared::domain::TypeDefinitionLocation;
use bsl_shared::domain::TypeMetadataLookup;
use bsl_shared::domain::{CodeLocation, ModuleType};
use bsl_shared::ir::{
    SemanticConstructorTarget, SemanticFacts, SemanticMethodTarget, SemanticProgram,
    SemanticTypeEntry, Span,
};
use bsl_shared::FORM_DATA_SEMANTICS_NOTE;
use bsl_syntax::ast::{CompilerDirective, Expression, ParseError, Program, Statement};

use crate::ast_to_ir::{is_global_collection, lookup_global_collection};
use crate::implicit_bindings::{
    directive_disables_form_context, ImplicitBindingResolver, FORM_CONTEXT_BOUND_SYMBOL_KEYS,
};
use crate::SemanticDeps;

#[derive(Debug, Clone, Default)]
pub(crate) struct TypeIndex {
    entries: Vec<SemanticTypeEntry>,
    definition_locations_by_span: HashMap<Span, TypeDefinitionLocation>,
    assignment_value_type_by_span: HashMap<Span, TypeResolution>,
    call_receiver_type_by_span: HashMap<Span, TypeResolution>,
    call_arg_types_by_span: HashMap<Span, Vec<TypeResolution>>,
    member_access_object_type_by_span: HashMap<Span, TypeResolution>,
    call_method_targets_by_span: HashMap<Span, SemanticMethodTarget>,
    member_method_targets_by_span: HashMap<Span, SemanticMethodTarget>,
    constructor_targets_by_span: HashMap<Span, SemanticConstructorTarget>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct TypeIndexBuildProfile {
    pub seed_module_context_ms: u128,
    pub local_function_summaries_ms: u128,
    pub visit_statements_ms: u128,
    pub visit_callable_body_ms: u128,
    pub visit_callable_body_count: u64,
    pub merge_control_flow_env_ms: u128,
    pub merge_control_flow_env_count: u64,
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

#[derive(Debug, Clone)]
struct SemanticFactsBuildProfiled {
    facts: SemanticFacts,
    profile: TypeIndexBuildProfile,
}

impl TypeIndex {
    fn from_semantic_facts(facts: &SemanticFacts) -> Self {
        Self {
            entries: facts.type_entries.clone(),
            definition_locations_by_span: facts.definition_locations_by_span.clone(),
            assignment_value_type_by_span: facts.assignment_value_type_by_span.clone(),
            call_receiver_type_by_span: facts.call_receiver_type_by_span.clone(),
            call_arg_types_by_span: facts.call_arg_types_by_span.clone(),
            member_access_object_type_by_span: facts.member_access_object_type_by_span.clone(),
            call_method_targets_by_span: facts.call_method_targets_by_span.clone(),
            member_method_targets_by_span: facts.member_method_targets_by_span.clone(),
            constructor_targets_by_span: facts.constructor_targets_by_span.clone(),
        }
    }

    pub(crate) fn type_at_byte_offset(&self, byte_offset: u32) -> Option<TypeResolution> {
        let find = |offset: u32| {
            self.entries
                .iter()
                .filter(|entry| entry.span.contains(offset))
                .min_by_key(|entry| entry.span.len())
                .map(|entry| entry.resolution.clone())
        };

        find(byte_offset).or_else(|| byte_offset.checked_sub(1).and_then(find))
    }

    pub(crate) fn type_for_exact_span(&self, span: Span) -> Option<TypeResolution> {
        self.entries
            .iter()
            .find(|entry| entry.span == span)
            .map(|entry| entry.resolution.clone())
    }

    pub(crate) fn type_resolution_for_span(&self, span: Span) -> Option<TypeResolution> {
        if let Some(exact) = self.type_for_exact_span(span) {
            return Some(exact);
        }
        if span.start == span.end {
            return self.type_at_byte_offset(span.start);
        }
        let end_inclusive = span.end.saturating_sub(1);
        self.type_at_byte_offset(end_inclusive)
            .or_else(|| self.type_at_byte_offset(span.start))
    }

    pub(crate) fn definition_location_for_exact_span(
        &self,
        span: Span,
    ) -> Option<TypeDefinitionLocation> {
        self.definition_locations_by_span.get(&span).cloned()
    }

    pub(crate) fn definition_location_at_byte_offset(
        &self,
        byte_offset: u32,
    ) -> Option<TypeDefinitionLocation> {
        self.closest_fact_by_offset(&self.definition_locations_by_span, byte_offset)
    }

    pub(crate) fn definition_location_for_span(
        &self,
        span: Span,
    ) -> Option<TypeDefinitionLocation> {
        if let Some(exact) = self.definition_location_for_exact_span(span) {
            return Some(exact);
        }
        if span.start == span.end {
            return self.definition_location_at_byte_offset(span.start);
        }
        let end_inclusive = span.end.saturating_sub(1);
        self.definition_location_at_byte_offset(end_inclusive)
            .or_else(|| self.definition_location_at_byte_offset(span.start))
    }

    pub(crate) fn assignment_value_type_for_span(&self, span: Span) -> Option<TypeResolution> {
        self.assignment_value_type_by_span.get(&span).cloned()
    }

    pub(crate) fn assignment_value_type_at_byte_offset(
        &self,
        byte_offset: u32,
    ) -> Option<TypeResolution> {
        self.closest_fact_by_offset(&self.assignment_value_type_by_span, byte_offset)
    }

    pub(crate) fn call_receiver_type_for_span(&self, span: Span) -> Option<TypeResolution> {
        self.call_receiver_type_by_span.get(&span).cloned()
    }

    pub(crate) fn call_receiver_type_at_byte_offset(
        &self,
        byte_offset: u32,
    ) -> Option<TypeResolution> {
        self.closest_fact_by_offset(&self.call_receiver_type_by_span, byte_offset)
    }

    pub(crate) fn call_arg_types_for_span(&self, span: Span) -> Option<Vec<TypeResolution>> {
        self.call_arg_types_by_span.get(&span).cloned()
    }

    pub(crate) fn call_arg_types_at_byte_offset(
        &self,
        byte_offset: u32,
    ) -> Option<Vec<TypeResolution>> {
        self.closest_fact_by_offset(&self.call_arg_types_by_span, byte_offset)
    }

    pub(crate) fn member_access_object_type_for_span(&self, span: Span) -> Option<TypeResolution> {
        self.member_access_object_type_by_span.get(&span).cloned()
    }

    pub(crate) fn member_access_object_type_at_byte_offset(
        &self,
        byte_offset: u32,
    ) -> Option<TypeResolution> {
        self.closest_fact_by_offset(&self.member_access_object_type_by_span, byte_offset)
    }

    pub(crate) fn call_method_target_for_span(&self, span: Span) -> Option<SemanticMethodTarget> {
        self.call_method_targets_by_span.get(&span).cloned()
    }

    pub(crate) fn call_method_target_at_byte_offset(
        &self,
        byte_offset: u32,
    ) -> Option<SemanticMethodTarget> {
        self.closest_fact_by_offset(&self.call_method_targets_by_span, byte_offset)
    }

    pub(crate) fn member_method_target_for_span(&self, span: Span) -> Option<SemanticMethodTarget> {
        self.member_method_targets_by_span.get(&span).cloned()
    }

    pub(crate) fn member_method_target_at_byte_offset(
        &self,
        byte_offset: u32,
    ) -> Option<SemanticMethodTarget> {
        self.closest_fact_by_offset(&self.member_method_targets_by_span, byte_offset)
    }

    pub(crate) fn constructor_target_for_span(
        &self,
        span: Span,
    ) -> Option<SemanticConstructorTarget> {
        self.constructor_targets_by_span.get(&span).cloned()
    }

    pub(crate) fn constructor_target_at_byte_offset(
        &self,
        byte_offset: u32,
    ) -> Option<SemanticConstructorTarget> {
        self.closest_fact_by_offset(&self.constructor_targets_by_span, byte_offset)
    }

    fn closest_fact_by_offset<T: Clone>(
        &self,
        facts: &HashMap<Span, T>,
        byte_offset: u32,
    ) -> Option<T> {
        let find = |offset: u32| {
            facts
                .iter()
                .filter(|(span, _)| span.contains(offset))
                .min_by_key(|(span, _)| span.len())
                .map(|(_, value)| value.clone())
        };

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
    current_file_path: Arc<str>,
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
            current_file_path: Arc::from(""),
            module_type: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct LocalFunctionSummary {
    return_type: TypeResolution,
    may_fallthrough: bool,
    params: Vec<String>,
    declaration_span: bsl_shared::ir::Span,
    is_function: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ResolutionLookupCacheBaseKey {
    type_name: String,
    active_facet: Option<FacetKind>,
    has_form_data_semantics: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ResolutionMethodCacheKey {
    base: ResolutionLookupCacheBaseKey,
    method_name_lower: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ResolutionPropertyCacheKey {
    base: ResolutionLookupCacheBaseKey,
    property_name_lower: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct MethodTargetCacheKey {
    owner_type: String,
    method_name_lower: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum DefinitionLocationCacheKey {
    Configuration(String),
    TabularRowParent(String),
}

#[derive(Default)]
struct TypeInferencerStats {
    visit_callable_body_ms: Cell<u128>,
    visit_callable_body_count: Cell<u64>,
    merge_control_flow_env_ms: Cell<u128>,
    merge_control_flow_env_count: Cell<u64>,
    source_incomplete_member_access_recovery_ms: Cell<u128>,
    source_incomplete_member_access_recovery_count: Cell<u64>,
    syntax_incomplete_member_access_recovery_ms: Cell<u128>,
    syntax_incomplete_member_access_recovery_count: Cell<u64>,
    incomplete_call_target_recovery_ms: Cell<u128>,
    incomplete_call_target_recovery_count: Cell<u64>,
}

struct TypeInferencer {
    deps: Arc<SemanticDeps>,
    resolver: Arc<TypeResolver>,
    signature_index: SignatureIndex,
    metadata_lookup: TypeMetadataLookup,
    property_type_cache: RefCell<HashMap<ResolutionPropertyCacheKey, Option<TypeResolution>>>,
    method_return_cache: RefCell<HashMap<ResolutionMethodCacheKey, TypeResolution>>,
    method_target_cache: RefCell<HashMap<MethodTargetCacheKey, Option<SemanticMethodTarget>>>,
    definition_location_cache:
        RefCell<HashMap<DefinitionLocationCacheKey, Option<TypeDefinitionLocation>>>,
    stats: TypeInferencerStats,
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
            property_type_cache: RefCell::new(HashMap::new()),
            method_return_cache: RefCell::new(HashMap::new()),
            method_target_cache: RefCell::new(HashMap::new()),
            definition_location_cache: RefCell::new(HashMap::new()),
            stats: TypeInferencerStats::default(),
        }
    }

    fn add_u128_stat(stat: &Cell<u128>, value: u128) {
        stat.set(stat.get().saturating_add(value));
    }

    fn add_u64_stat(stat: &Cell<u64>, value: u64) {
        stat.set(stat.get().saturating_add(value));
    }

    fn lookup_cache_base_key(resolution: &TypeResolution) -> ResolutionLookupCacheBaseKey {
        ResolutionLookupCacheBaseKey {
            type_name: resolution.type_name(),
            active_facet: resolution.active_facet,
            has_form_data_semantics: resolution
                .metadata
                .notes
                .iter()
                .any(|note| note == FORM_DATA_SEMANTICS_NOTE),
        }
    }

    #[cfg(test)]
    fn build_index(&self, program: &Program, file_path: &str) -> TypeIndex {
        self.build_index_profiled(program, file_path).index
    }

    #[cfg(test)]
    fn build_index_profiled(&self, program: &Program, file_path: &str) -> TypeIndexBuildProfiled {
        let profiled = self.build_facts_internal(program, file_path, None, None);
        TypeIndexBuildProfiled {
            index: TypeIndex::from_semantic_facts(&profiled.facts),
            profile: profiled.profile,
        }
    }

    #[cfg(test)]
    fn build_index_from_parse_result_profiled(
        &self,
        parsed: &bsl_syntax::ast::ParseResult,
        source_text: &str,
        file_path: &str,
    ) -> TypeIndexBuildProfiled {
        let profiled = self.build_facts_internal(
            &parsed.program,
            file_path,
            Some(source_text),
            Some(RecoveryContext {
                source_text,
                syntax_errors: &parsed.syntax_errors,
            }),
        );
        TypeIndexBuildProfiled {
            index: TypeIndex::from_semantic_facts(&profiled.facts),
            profile: profiled.profile,
        }
    }

    fn build_index_from_semantic_program_profiled(
        &self,
        program: &SemanticProgram,
        file_path: &str,
        recovery: Option<RecoveryContext<'_>>,
    ) -> TypeIndexBuildProfiled {
        let _ = (file_path, recovery);
        TypeIndexBuildProfiled {
            index: TypeIndex::from_semantic_facts(&program.semantic_facts),
            profile: projection_build_profile(program),
        }
    }

    fn build_facts_internal(
        &self,
        program: &Program,
        file_path: &str,
        source_text: Option<&str>,
        recovery: Option<RecoveryContext<'_>>,
    ) -> SemanticFactsBuildProfiled {
        let started = Instant::now();
        let mut env = TypeEnv::default();
        let mut facts = SemanticFacts::default();
        env.current_file_path = Arc::from(file_path.to_string());

        let seed_started = Instant::now();
        self.seed_module_context(file_path, &mut env);
        let seed_module_context_ms = seed_started.elapsed().as_millis();

        let local_function_summaries_started = Instant::now();
        let local_function_summaries = self.infer_local_function_summaries(program, &env);
        let local_function_summary_count = local_function_summaries.len() as u64;
        env.local_function_summaries = Arc::new(local_function_summaries);
        let local_function_summaries_ms = local_function_summaries_started.elapsed().as_millis();
        let source_incomplete_member_access_offsets =
            source_text.map(incomplete_member_access_dot_offsets);

        let visit_statements_started = Instant::now();
        for stmt in &program.statements {
            match stmt {
                Statement::FunctionDecl {
                    params,
                    body,
                    compiler_directive,
                    span,
                    ..
                }
                | Statement::ProcedureDecl {
                    params,
                    body,
                    compiler_directive,
                    span,
                    ..
                } => {
                    let fn_env = self.visit_callable_body(
                        params,
                        body,
                        *compiler_directive,
                        &env,
                        &mut facts,
                    );
                    if let Some(source_text) = source_text {
                        let started = Instant::now();
                        self.record_source_incomplete_member_access_entries(
                            source_text,
                            *span,
                            source_incomplete_member_access_offsets.as_deref(),
                            &fn_env,
                            &mut facts,
                        );
                        Self::add_u128_stat(
                            &self.stats.source_incomplete_member_access_recovery_ms,
                            started.elapsed().as_millis(),
                        );
                        Self::add_u64_stat(
                            &self.stats.source_incomplete_member_access_recovery_count,
                            1,
                        );
                    }
                    if let Some(recovery) = recovery {
                        let member_recovery_started = Instant::now();
                        self.record_incomplete_member_access_recovery_entries(
                            recovery, *span, &fn_env, &mut facts,
                        );
                        Self::add_u128_stat(
                            &self.stats.syntax_incomplete_member_access_recovery_ms,
                            member_recovery_started.elapsed().as_millis(),
                        );
                        Self::add_u64_stat(
                            &self.stats.syntax_incomplete_member_access_recovery_count,
                            1,
                        );
                        let call_target_started = Instant::now();
                        self.record_incomplete_call_target_recovery_entries(
                            recovery, *span, &fn_env, &mut facts,
                        );
                        Self::add_u128_stat(
                            &self.stats.incomplete_call_target_recovery_ms,
                            call_target_started.elapsed().as_millis(),
                        );
                        Self::add_u64_stat(&self.stats.incomplete_call_target_recovery_count, 1);
                    }
                }
                _ => self.visit_statement(stmt, &mut env, &mut facts),
            }
        }
        if let Some(source_text) = source_text {
            let started = Instant::now();
            self.record_source_incomplete_member_access_entries(
                source_text,
                bsl_shared::ir::Span::new(0, source_text.len() as u32),
                source_incomplete_member_access_offsets.as_deref(),
                &env,
                &mut facts,
            );
            Self::add_u128_stat(
                &self.stats.source_incomplete_member_access_recovery_ms,
                started.elapsed().as_millis(),
            );
            Self::add_u64_stat(
                &self.stats.source_incomplete_member_access_recovery_count,
                1,
            );
        }
        if let Some(recovery) = recovery {
            let member_recovery_started = Instant::now();
            self.record_incomplete_member_access_recovery_entries(
                recovery,
                bsl_shared::ir::Span::new(0, recovery.source_text.len() as u32),
                &env,
                &mut facts,
            );
            Self::add_u128_stat(
                &self.stats.syntax_incomplete_member_access_recovery_ms,
                member_recovery_started.elapsed().as_millis(),
            );
            Self::add_u64_stat(
                &self.stats.syntax_incomplete_member_access_recovery_count,
                1,
            );
            let call_target_started = Instant::now();
            self.record_incomplete_call_target_recovery_entries(
                recovery,
                bsl_shared::ir::Span::new(0, recovery.source_text.len() as u32),
                &env,
                &mut facts,
            );
            Self::add_u128_stat(
                &self.stats.incomplete_call_target_recovery_ms,
                call_target_started.elapsed().as_millis(),
            );
            Self::add_u64_stat(&self.stats.incomplete_call_target_recovery_count, 1);
        }
        let visit_statements_ms = visit_statements_started.elapsed().as_millis();
        let visit_callable_body_ms = self.stats.visit_callable_body_ms.get();
        let visit_callable_body_count = self.stats.visit_callable_body_count.get();
        let merge_control_flow_env_ms = self.stats.merge_control_flow_env_ms.get();
        let merge_control_flow_env_count = self.stats.merge_control_flow_env_count.get();
        let source_incomplete_member_access_recovery_ms =
            self.stats.source_incomplete_member_access_recovery_ms.get();
        let source_incomplete_member_access_recovery_count = self
            .stats
            .source_incomplete_member_access_recovery_count
            .get();
        let syntax_incomplete_member_access_recovery_ms =
            self.stats.syntax_incomplete_member_access_recovery_ms.get();
        let syntax_incomplete_member_access_recovery_count = self
            .stats
            .syntax_incomplete_member_access_recovery_count
            .get();
        let incomplete_call_target_recovery_ms =
            self.stats.incomplete_call_target_recovery_ms.get();
        let incomplete_call_target_recovery_count =
            self.stats.incomplete_call_target_recovery_count.get();
        tracing::debug!(
            target: "bsl_backend::analysis_v2",
            file_path,
            source_text_len = source_text.map(|text| text.len()).unwrap_or(0),
            seed_module_context_ms,
            local_function_summaries_ms,
            visit_statements_ms,
            visit_callable_body_ms,
            visit_callable_body_count,
            merge_control_flow_env_ms,
            merge_control_flow_env_count,
            source_incomplete_member_access_recovery_ms,
            source_incomplete_member_access_recovery_count,
            syntax_incomplete_member_access_recovery_ms,
            syntax_incomplete_member_access_recovery_count,
            incomplete_call_target_recovery_ms,
            incomplete_call_target_recovery_count,
            statement_count = program.statements.len(),
            local_function_summary_count,
            index_entry_count = facts.type_entries.len(),
            "semantic_facts: build_facts_internal finished"
        );

        SemanticFactsBuildProfiled {
            profile: TypeIndexBuildProfile {
                seed_module_context_ms,
                local_function_summaries_ms,
                visit_statements_ms,
                visit_callable_body_ms,
                visit_callable_body_count,
                merge_control_flow_env_ms,
                merge_control_flow_env_count,
                total_ms: started.elapsed().as_millis(),
                statement_count: program.statements.len() as u64,
                local_function_summary_count,
                index_entry_count: facts.type_entries.len() as u64,
            },
            facts,
        }
    }

    fn visit_callable_body(
        &self,
        params: &[String],
        body: &[Statement],
        compiler_directive: Option<CompilerDirective>,
        env: &TypeEnv,
        facts: &mut SemanticFacts,
    ) -> TypeEnv {
        let started = Instant::now();
        let mut fn_env = env.clone();
        if directive_disables_form_context(compiler_directive) {
            for key in FORM_CONTEXT_BOUND_SYMBOL_KEYS {
                fn_env.variables.remove(key);
                fn_env.instance_bindings.remove(key);
            }
        }
        for param in params {
            fn_env.set_variable_value(param.to_lowercase(), TypeResolution::unknown(), None);
        }
        for stmt in body {
            self.visit_statement(stmt, &mut fn_env, facts);
        }
        Self::add_u128_stat(
            &self.stats.visit_callable_body_ms,
            started.elapsed().as_millis(),
        );
        Self::add_u64_stat(&self.stats.visit_callable_body_count, 1);
        fn_env
    }

    fn record_incomplete_member_access_recovery_entries(
        &self,
        recovery: RecoveryContext<'_>,
        container_span: bsl_shared::ir::Span,
        env: &TypeEnv,
        facts: &mut SemanticFacts,
    ) {
        if recovery.syntax_errors.is_empty() {
            return;
        }

        let mut dot_offsets = recovery_incomplete_member_access_dot_offsets_within_span(
            recovery.source_text,
            container_span,
        );
        for error in recovery.syntax_errors {
            if let Some(dot_offset) =
                find_incomplete_member_access_dot_offset(recovery.source_text, error.span)
            {
                if !dot_offsets.contains(&dot_offset) {
                    dot_offsets.push(dot_offset);
                }
            }
        }

        for dot_offset in dot_offsets {
            self.record_incomplete_member_access_receiver_entries_at_dot_offset(
                recovery.source_text,
                dot_offset,
                env,
                facts,
                false,
            );

            let Some((member_span, member_expr_text)) =
                extract_incomplete_member_access_target_slice_at_dot_offset(
                    recovery.source_text,
                    dot_offset,
                )
            else {
                continue;
            };

            let resolution = parse_recovery_expression_snippet(member_expr_text)
                .map(|member_expr| {
                    let mut scratch_env = env.clone();
                    let mut scratch_facts = SemanticFacts::default();
                    self.infer_expr(&member_expr, &mut scratch_env, &mut scratch_facts)
                })
                .unwrap_or_else(|| self.resolver.resolve_expression_sync(member_expr_text));
            if resolution.is_unknown() || resolution.is_dynamic() {
                continue;
            }

            self.record(member_span, resolution.clone(), facts);
            self.record_definition_location(member_span, &resolution, facts);
        }
    }

    fn record_source_incomplete_member_access_entries(
        &self,
        source_text: &str,
        container_span: bsl_shared::ir::Span,
        candidate_offsets: Option<&[usize]>,
        env: &TypeEnv,
        facts: &mut SemanticFacts,
    ) {
        let dot_offsets = candidate_offsets
            .map(|offsets| {
                incomplete_member_access_dot_offsets_within_span_from_candidates(
                    offsets,
                    container_span,
                )
            })
            .unwrap_or_else(|| {
                incomplete_member_access_dot_offsets_within_span(source_text, container_span)
            });
        for dot_offset in dot_offsets {
            self.record_incomplete_member_access_receiver_entries_at_dot_offset(
                source_text,
                dot_offset,
                env,
                facts,
                true,
            );
        }
    }

    fn record_incomplete_member_access_receiver_entries_at_dot_offset(
        &self,
        source_text: &str,
        dot_offset: usize,
        env: &TypeEnv,
        facts: &mut SemanticFacts,
        skip_identifier_receivers: bool,
    ) {
        for (receiver_span, receiver_expr_text) in
            extract_incomplete_member_access_receiver_slices_at_dot_offset(source_text, dot_offset)
        {
            if facts.type_resolution_for_span(receiver_span).is_some() {
                continue;
            }

            let Some(receiver_expr) = parse_recovery_expression_snippet(receiver_expr_text) else {
                continue;
            };
            if skip_identifier_receivers {
                if let Expression::Identifier { name, .. } = &receiver_expr {
                    if !self.allow_source_recovered_identifier_receiver(name, env) {
                        continue;
                    }
                }
            }

            let mut scratch_env = env.clone();
            let mut scratch_facts = SemanticFacts::default();
            let resolution = self.infer_expr(&receiver_expr, &mut scratch_env, &mut scratch_facts);
            if resolution.is_unknown() || resolution.is_dynamic() {
                continue;
            }

            self.record(receiver_span, resolution, facts);
        }
    }

    fn allow_source_recovered_identifier_receiver(&self, name: &str, env: &TypeEnv) -> bool {
        if is_global_collection(name).is_some() {
            return true;
        }

        let name_lower = name.to_lowercase();
        match env.module_type {
            Some(ModuleType::FormModule { .. }) => {
                FORM_CONTEXT_BOUND_SYMBOL_KEYS.contains(&name_lower.as_str())
            }
            Some(ModuleType::ManagerModule { .. })
            | Some(ModuleType::ObjectModule { .. })
            | Some(ModuleType::RecordSetModule { .. }) => {
                matches!(name_lower.as_str(), "этотобъект" | "объект")
            }
            _ => false,
        }
    }

    fn record_incomplete_call_target_recovery_entries(
        &self,
        recovery: RecoveryContext<'_>,
        container_span: bsl_shared::ir::Span,
        env: &TypeEnv,
        facts: &mut SemanticFacts,
    ) {
        if recovery.syntax_errors.is_empty() {
            return;
        }

        let mut candidates = Vec::new();
        for error in recovery.syntax_errors {
            for candidate in
                incomplete_call_recovery_candidates_on_error_line(recovery.source_text, error.span)
            {
                if !span_contains(container_span, candidate.call_span.start) {
                    continue;
                }
                if candidates
                    .iter()
                    .any(|existing: &IncompleteCallRecoveryCandidate| {
                        existing.call_span == candidate.call_span
                            && existing.member_span == candidate.member_span
                    })
                {
                    continue;
                }
                candidates.push(candidate);
            }
            for anchor in recovery_call_anchor_offsets(recovery.source_text, error.span) {
                let Some(candidate) =
                    incomplete_call_recovery_candidate_at_offset(recovery.source_text, anchor)
                else {
                    continue;
                };
                if !span_contains(container_span, candidate.call_span.start) {
                    continue;
                }
                if candidates
                    .iter()
                    .any(|existing: &IncompleteCallRecoveryCandidate| {
                        existing.call_span == candidate.call_span
                            && existing.member_span == candidate.member_span
                    })
                {
                    continue;
                }
                candidates.push(candidate);
            }
        }

        for candidate in candidates {
            match candidate.kind {
                IncompleteCallRecoveryKind::Constructor { type_name } => {
                    self.record_constructor_target(candidate.call_span, &type_name, facts);
                }
                IncompleteCallRecoveryKind::Method {
                    receiver_expr,
                    method_name,
                } => {
                    let Some(receiver_expr) =
                        parse_recovery_expression_snippet(receiver_expr.as_str())
                    else {
                        continue;
                    };
                    let mut scratch_env = env.clone();
                    let mut scratch_facts = SemanticFacts::default();
                    let receiver =
                        self.infer_expr(&receiver_expr, &mut scratch_env, &mut scratch_facts);
                    if receiver.is_unknown() || receiver.is_dynamic() {
                        continue;
                    }

                    let Some(target) =
                        self.semantic_method_target(&receiver, &method_name)
                            .filter(|target| {
                                target.signature.is_some() || target.definition_location.is_some()
                            })
                    else {
                        continue;
                    };
                    self.record_method_target(
                        candidate.member_span,
                        candidate.call_span,
                        target,
                        facts,
                    );
                }
            }
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

    fn visit_statement(&self, stmt: &Statement, env: &mut TypeEnv, facts: &mut SemanticFacts) {
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
            Statement::Assignment {
                target,
                value,
                span,
            } => {
                let value_type = self.infer_expr(value, env, facts);
                facts
                    .assignment_value_type_by_span
                    .insert(*span, value_type.clone());
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
                        self.record(expr_span(target), updated, facts);
                    }
                }
            }
            Statement::If {
                condition,
                then_body,
                else_body,
                ..
            } => {
                let _ = self.infer_expr(condition, env, facts);
                let base_env = env.clone();
                let mut then_env = base_env.clone();
                for stmt in then_body {
                    self.visit_statement(stmt, &mut then_env, facts);
                }
                let mut else_env = base_env.clone();
                if let Some(else_body) = else_body {
                    for stmt in else_body {
                        self.visit_statement(stmt, &mut else_env, facts);
                    }
                }
                *env = self.merge_control_flow_env(&base_env, &then_env, &else_env);
            }
            Statement::While {
                condition, body, ..
            } => {
                let _ = self.infer_expr(condition, env, facts);
                let base_env = env.clone();
                let mut body_env = base_env.clone();
                for stmt in body {
                    self.visit_statement(stmt, &mut body_env, facts);
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
                let _ = self.infer_expr(start, env, facts);
                let _ = self.infer_expr(end, env, facts);
                let base_env = env.clone();
                let mut body_env = base_env.clone();
                body_env.set_variable_value(
                    variable.to_lowercase(),
                    TypeResolution::primitive("Число"),
                    None,
                );
                for stmt in body {
                    self.visit_statement(stmt, &mut body_env, facts);
                }
                *env = self.merge_control_flow_env(&base_env, &body_env, &base_env);
            }
            Statement::ForEach {
                variable,
                collection,
                body,
                ..
            } => {
                let collection_type = self.infer_expr(collection, env, facts);
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
                    self.visit_statement(stmt, &mut body_env, facts);
                }
                *env = self.merge_control_flow_env(&base_env, &body_env, &base_env);
            }
            Statement::Return {
                value: Some(value), ..
            } => {
                let _ = self.infer_expr(value, env, facts);
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
                    self.visit_statement(stmt, &mut try_env, facts);
                }
                let mut except_env = base_env.clone();
                for stmt in except_body {
                    self.visit_statement(stmt, &mut except_env, facts);
                }
                *env = self.merge_control_flow_env(&base_env, &try_env, &except_env);
            }
            Statement::Call { expression, .. } => {
                let _ = self.infer_expr(expression, env, facts);
            }
            Statement::Execute { code, .. } => {
                let _ = self.infer_expr(code, env, facts);
            }
            Statement::RaiseError {
                message: Some(message),
                ..
            } => {
                let _ = self.infer_expr(message, env, facts);
            }
            Statement::RaiseError { message: None, .. } => {}
            Statement::AddHandler { event, handler, .. }
            | Statement::RemoveHandler { event, handler, .. } => {
                let _ = self.infer_expr(event, env, facts);
                let _ = self.infer_expr(handler, env, facts);
            }
            Statement::Await { expression, .. } => {
                let _ = self.infer_expr(expression, env, facts);
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
                let _ = self.visit_callable_body(params, body, *compiler_directive, env, facts);
            }
            _ => {}
        }
    }

    fn record(
        &self,
        span: bsl_shared::ir::Span,
        resolution: TypeResolution,
        facts: &mut SemanticFacts,
    ) {
        facts
            .type_entries
            .push(SemanticTypeEntry { span, resolution });
    }

    fn record_definition_location(
        &self,
        span: bsl_shared::ir::Span,
        resolution: &TypeResolution,
        facts: &mut SemanticFacts,
    ) {
        let Some(location) = self.semantic_definition_location(resolution) else {
            return;
        };
        facts.definition_locations_by_span.insert(span, location);
    }

    fn infer_expr(
        &self,
        expr: &Expression,
        env: &mut TypeEnv,
        facts: &mut SemanticFacts,
    ) -> TypeResolution {
        let resolution = match expr {
            Expression::Number { .. } => TypeResolution::primitive("Число"),
            Expression::String { .. } => TypeResolution::primitive("Строка"),
            Expression::Boolean { .. } => TypeResolution::primitive("Булево"),
            Expression::Date { .. } => TypeResolution::primitive("Дата"),
            Expression::Identifier { name, .. } => self.infer_identifier(name, env),
            Expression::New {
                type_name, args, ..
            } => {
                let resolution = self.infer_new_expression(type_name, args, env, facts);
                self.record_constructor_target(expr_span(expr), type_name, facts);
                resolution
            }
            Expression::PropertyAccess {
                object, property, ..
            } => {
                let object_resolution = self.infer_expr(object, env, facts);
                let resolution = self.infer_property_access(&object_resolution, property);
                facts
                    .member_access_object_type_by_span
                    .insert(expr_span(expr), object_resolution);
                resolution
            }
            Expression::Call { function, args, .. } => {
                self.infer_call(function, args, env, facts, expr_span(expr))
            }
            Expression::Binary {
                left,
                operator,
                right,
                ..
            } => {
                let left_type = self.infer_expr(left, env, facts);
                let right_type = self.infer_expr(right, env, facts);
                self.infer_binary(operator, &left_type, &right_type)
            }
            Expression::Unary { operand, .. } => self.infer_expr(operand, env, facts),
            Expression::Ternary {
                condition,
                then_expr,
                else_expr,
                ..
            } => {
                let _ = self.infer_expr(condition, env, facts);
                let then_type = self.infer_expr(then_expr, env, facts);
                let else_type = self.infer_expr(else_expr, env, facts);
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
                let object_resolution = self.infer_expr(object, env, facts);
                let _ = self.infer_expr(index_expr, env, facts);
                self.resolve_index_access(expr, object, index_expr, &object_resolution, env)
            }
            Expression::Await { expression, .. } => self.infer_expr(expression, env, facts),
        };

        self.record(expr_span(expr), resolution.clone(), facts);
        self.record_definition_location(expr_span(expr), &resolution, facts);
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

        TypeResolution::undeclared_variable(name)
    }

    fn infer_new_expression(
        &self,
        type_name: &str,
        args: &[Expression],
        env: &mut TypeEnv,
        facts: &mut SemanticFacts,
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
            .map(|arg| self.infer_expr(arg, env, facts))
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
        facts: &mut SemanticFacts,
        call_span: bsl_shared::ir::Span,
    ) -> TypeResolution {
        let arg_types: Vec<TypeResolution> = args
            .iter()
            .map(|arg| self.infer_expr(arg, env, facts))
            .collect();
        facts
            .call_arg_types_by_span
            .insert(call_span, arg_types.clone());

        match function {
            Expression::Identifier { name, .. } => {
                if let Some(target) =
                    self.semantic_identifier_call_target(name, env)
                        .filter(|target| {
                            target.signature.is_some() || target.definition_location.is_some()
                        })
                {
                    facts.call_method_targets_by_span.insert(call_span, target);
                }
                self.infer_global_function_call(name, env)
            }
            Expression::PropertyAccess {
                object, property, ..
            } => {
                let receiver = self.infer_expr(object, env, facts);
                let method_target =
                    self.semantic_method_target(&receiver, property)
                        .filter(|target| {
                            target.signature.is_some() || target.definition_location.is_some()
                        });
                facts
                    .call_receiver_type_by_span
                    .insert(call_span, receiver.clone());
                if let Some(resolved) = self.try_apply_universal_collection_method(
                    object, property, args, &arg_types, env, facts,
                ) {
                    if let Some(target) = method_target {
                        self.record_method_target(expr_span(function), call_span, target, facts);
                    }
                    return resolved;
                }

                let resolved = self.infer_method_call(&receiver, property);
                if let Some(target) = method_target {
                    self.record_method_target(expr_span(function), call_span, target, facts);
                }
                resolved
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
        facts: &mut SemanticFacts,
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
                    let receiver = self.infer_expr(object, env, facts);
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
                    let receiver = self.infer_expr(object, env, facts);
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
                let receiver = self.infer_expr(object, env, facts);
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
        let started = Instant::now();
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

        Self::add_u128_stat(
            &self.stats.merge_control_flow_env_ms,
            started.elapsed().as_millis(),
        );
        Self::add_u64_stat(&self.stats.merge_control_flow_env_count, 1);
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

    fn semantic_identifier_call_target(
        &self,
        name: &str,
        env: &TypeEnv,
    ) -> Option<SemanticMethodTarget> {
        let name_lower = name.to_lowercase();
        if let Some(local) = env.local_function_summaries.get(&name_lower) {
            let return_type = (!local.return_type.is_unknown() && !local.return_type.is_dynamic())
                .then(|| local.return_type.type_name());
            let signature = MethodSignature::new(
                name.to_string(),
                None,
                local
                    .params
                    .iter()
                    .map(|param_name| ParameterInfo {
                        name: param_name.clone(),
                        type_name: None,
                        is_optional: false,
                        default_value: None,
                        description: None,
                    })
                    .collect(),
                if local.is_function { return_type } else { None },
                None,
                None,
                SignatureSource::UserCode,
                local.return_type.active_facet,
                Default::default(),
            );
            let definition_location = TypeDefinitionLocation::user_defined(
                PathBuf::from(env.current_file_path.as_ref()),
                local.declaration_span.start,
                local.declaration_span.end,
            );
            return Some(SemanticMethodTarget {
                owner_type: None,
                method_name: name.to_string(),
                signature: Some(signature),
                definition_location: Some(definition_location),
            });
        }

        let signature = self.deps.repository.find_method_signature(None, name);
        let definition_location = self
            .deps
            .repository
            .find_method_definition_location(None, name);
        if signature.is_none() && definition_location.is_none() {
            return None;
        }

        Some(SemanticMethodTarget {
            owner_type: None,
            method_name: name.to_string(),
            signature,
            definition_location,
        })
    }

    fn infer_method_call(&self, receiver: &TypeResolution, method: &str) -> TypeResolution {
        let cache_key = ResolutionMethodCacheKey {
            base: Self::lookup_cache_base_key(receiver),
            method_name_lower: method.to_lowercase(),
        };
        if let Some(cached) = self.method_return_cache.borrow().get(&cache_key).cloned() {
            return cached;
        }

        let type_name = signature_lookup_type_name(receiver);
        let metadata_name = SignatureIndex::extract_metadata_name(&type_name);
        let method_key = cache_key.method_name_lower.clone();
        if let Some(resolved) = self.try_resolve_tabular_section_row_method(receiver, &method_key) {
            self.method_return_cache
                .borrow_mut()
                .insert(cache_key.clone(), resolved.clone());
            return resolved;
        }
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

        let resolve_lookup_method = |methods: Vec<bsl_shared::domain::types::RawMethodData>| {
            methods
                .into_iter()
                .find(|item| item.name.to_lowercase() == method_key)
                .and_then(|item| (!item.return_type.is_empty()).then_some(item.return_type))
                .map(|return_type| concretize_return_type(&return_type))
                .and_then(|return_type| {
                    self.try_resolve_configuration_type(&return_type)
                        .or_else(|| Some(self.resolver.resolve_expression_sync(&return_type)))
                })
        };

        if matches!(receiver.result, ResolutionResult::Generic(_)) {
            if let Some(resolved) =
                resolve_lookup_method(self.metadata_lookup.get_methods(receiver))
            {
                self.method_return_cache
                    .borrow_mut()
                    .insert(cache_key, resolved.clone());
                return resolved;
            }
        }

        if let Some(sig) = self.signature_index.find_method(&type_name, method) {
            if let Some(return_type) = sig.return_type.as_deref().filter(|s| !s.is_empty()) {
                let return_type = concretize_return_type(return_type);
                if let Some(resolved) = self.try_resolve_configuration_type(&return_type) {
                    self.method_return_cache
                        .borrow_mut()
                        .insert(cache_key.clone(), resolved.clone());
                    return resolved;
                }
                let resolved = self.resolver.resolve_expression_sync(&return_type);
                self.method_return_cache
                    .borrow_mut()
                    .insert(cache_key.clone(), resolved.clone());
                return resolved;
            }
        }

        if let Some(resolved) = resolve_lookup_method(self.metadata_lookup.get_methods(receiver)) {
            self.method_return_cache
                .borrow_mut()
                .insert(cache_key.clone(), resolved.clone());
            return resolved;
        }

        let resolved = TypeResolution::unknown();
        self.method_return_cache
            .borrow_mut()
            .insert(cache_key, resolved.clone());
        resolved
    }

    fn try_resolve_tabular_section_row_method(
        &self,
        receiver: &TypeResolution,
        method_key: &str,
    ) -> Option<TypeResolution> {
        if !matches!(method_key, "добавить" | "получить") {
            return None;
        }

        let type_name = receiver.type_name();
        let item_type = type_name
            .strip_prefix("ТабличнаяЧасть<")
            .and_then(|tail| tail.strip_suffix('>'))?
            .trim();
        if item_type.is_empty() {
            return None;
        }

        let resolved_type_name = if self.deps.repository.find_type(item_type).is_some() {
            item_type.to_string()
        } else {
            let row_type_name = format!("Строка{item_type}");
            if self.deps.repository.find_type(&row_type_name).is_some() {
                row_type_name
            } else {
                return None;
            }
        };

        Some(TypeResolution::explicit(&resolved_type_name))
    }

    fn semantic_method_target(
        &self,
        receiver: &TypeResolution,
        method_name: &str,
    ) -> Option<SemanticMethodTarget> {
        if receiver.is_unknown() || receiver.is_dynamic() {
            return None;
        }

        let owner_type = signature_lookup_type_name(receiver);
        let owner_type = owner_type.trim();
        if owner_type.is_empty() {
            return None;
        }

        let cache_key = MethodTargetCacheKey {
            owner_type: owner_type.to_string(),
            method_name_lower: method_name.to_lowercase(),
        };
        if let Some(cached) = self.method_target_cache.borrow().get(&cache_key).cloned() {
            return cached;
        }

        let target = Some(SemanticMethodTarget {
            owner_type: Some(owner_type.to_string()),
            method_name: method_name.to_string(),
            signature: self
                .deps
                .repository
                .find_method_signature(Some(owner_type), method_name),
            definition_location: self
                .deps
                .repository
                .find_method_definition_location(Some(owner_type), method_name),
        });
        self.method_target_cache
            .borrow_mut()
            .insert(cache_key, target.clone());
        target
    }

    fn semantic_definition_location(
        &self,
        resolution: &TypeResolution,
    ) -> Option<TypeDefinitionLocation> {
        match &resolution.result {
            ResolutionResult::Concrete(ConcreteType::Configuration(cfg)) => {
                let type_key = format!("{}.{}", cfg.kind.to_prefix(), cfg.name);
                let cache_key = DefinitionLocationCacheKey::Configuration(type_key.clone());
                if let Some(cached) = self
                    .definition_location_cache
                    .borrow()
                    .get(&cache_key)
                    .cloned()
                {
                    return cached;
                }
                let location = self.deps.repository.find_type(&type_key).and_then(|raw| {
                    let metadata_path = raw.metadata_path?;
                    Some(TypeDefinitionLocation::configuration_with_modules(
                        metadata_path,
                        raw.module_paths.unwrap_or_default(),
                    ))
                });
                self.definition_location_cache
                    .borrow_mut()
                    .insert(cache_key, location.clone());
                location
            }
            ResolutionResult::Concrete(ConcreteType::TabularRow(tabular_row)) => {
                let cache_key =
                    DefinitionLocationCacheKey::TabularRowParent(tabular_row.parent_type.clone());
                if let Some(cached) = self
                    .definition_location_cache
                    .borrow()
                    .get(&cache_key)
                    .cloned()
                {
                    return cached;
                }
                let location = self
                    .deps
                    .repository
                    .find_type(&tabular_row.parent_type)
                    .and_then(|raw| {
                        let metadata_path = raw.metadata_path?;
                        Some(TypeDefinitionLocation::configuration_with_modules(
                            metadata_path,
                            raw.module_paths.unwrap_or_default(),
                        ))
                    });
                self.definition_location_cache
                    .borrow_mut()
                    .insert(cache_key, location.clone());
                location
            }
            ResolutionResult::Concrete(ConcreteType::Primitive(_))
            | ResolutionResult::Concrete(ConcreteType::Special(_))
            | ResolutionResult::Generic(_)
            | ResolutionResult::Nullable(_)
            | ResolutionResult::Union(_)
            | ResolutionResult::Intersection(_) => resolution.get_definition_location(),
            ResolutionResult::Concrete(ConcreteType::Platform(_))
            | ResolutionResult::Concrete(ConcreteType::GlobalFunction(_))
            | ResolutionResult::Dynamic => None,
        }
    }

    fn record_method_target(
        &self,
        member_span: bsl_shared::ir::Span,
        call_span: bsl_shared::ir::Span,
        target: SemanticMethodTarget,
        facts: &mut SemanticFacts,
    ) {
        facts
            .call_method_targets_by_span
            .insert(call_span, target.clone());
        facts
            .member_method_targets_by_span
            .insert(member_span, target);
    }

    fn record_constructor_target(
        &self,
        span: bsl_shared::ir::Span,
        type_name: &str,
        facts: &mut SemanticFacts,
    ) {
        let type_name = type_name.trim().trim_end_matches("()").trim();
        if type_name.is_empty() {
            return;
        }

        facts.constructor_targets_by_span.insert(
            span,
            SemanticConstructorTarget {
                type_name: type_name.to_string(),
                signature: self.deps.repository.find_constructor(type_name),
            },
        );
    }
}

#[derive(Clone, Copy)]
struct RecoveryContext<'a> {
    source_text: &'a str,
    syntax_errors: &'a [ParseError],
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct IncompleteCallRecoveryCandidate {
    member_span: bsl_shared::ir::Span,
    call_span: bsl_shared::ir::Span,
    kind: IncompleteCallRecoveryKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum IncompleteCallRecoveryKind {
    Constructor {
        type_name: String,
    },
    Method {
        receiver_expr: String,
        method_name: String,
    },
}

fn projection_build_profile(program: &SemanticProgram) -> TypeIndexBuildProfile {
    TypeIndexBuildProfile {
        statement_count: program.nodes.len() as u64,
        index_entry_count: program.semantic_facts.type_entries.len() as u64,
        ..TypeIndexBuildProfile::default()
    }
}

fn span_contains(span: bsl_shared::ir::Span, offset: u32) -> bool {
    span.start <= offset && offset < span.end
}

fn recovery_call_anchor_offsets(source_text: &str, error_span: bsl_shared::ir::Span) -> Vec<usize> {
    let len = source_text.len();
    let mut offsets = Vec::new();
    for offset in [
        error_span.start as usize,
        error_span.end as usize,
        error_span.start.saturating_sub(1) as usize,
        error_span.end.saturating_sub(1) as usize,
    ] {
        let clamped = offset.min(len);
        if !offsets.contains(&clamped) {
            offsets.push(clamped);
        }
    }
    offsets
}

fn incomplete_call_recovery_candidate_at_offset(
    source_text: &str,
    anchor_offset: usize,
) -> Option<IncompleteCallRecoveryCandidate> {
    let prefix = source_text.get(..anchor_offset)?;
    let (open_paren_offset, call_span_end) =
        if let Some(open_paren_offset) = find_last_unclosed_call_paren_offset(prefix) {
            (open_paren_offset, open_paren_offset.saturating_add(1))
        } else {
            let open_paren_offset =
                find_last_incomplete_argument_call_paren_offset(source_text, anchor_offset)?;
            let (_, line_end) = trimmed_line_bounds_at_offset(source_text, open_paren_offset)?;
            (open_paren_offset, line_end)
        };
    let before_paren = source_text.get(..open_paren_offset)?;
    let head_text = extract_expression_suffix(before_paren)?;
    let head_span = source_slice_span(source_text, head_text)?;
    let call_span =
        bsl_shared::ir::Span::new(head_span.start, call_span_end.min(u32::MAX as usize) as u32);
    let head = extract_recovery_call_head(head_text)?;

    Some(IncompleteCallRecoveryCandidate {
        member_span: head_span,
        call_span,
        kind: head,
    })
}

fn incomplete_call_recovery_candidates_on_error_line(
    source_text: &str,
    error_span: bsl_shared::ir::Span,
) -> Vec<IncompleteCallRecoveryCandidate> {
    let Some((line_start, line_end)) =
        line_bounds_for_offset(source_text, error_span.start as usize)
    else {
        return Vec::new();
    };
    let Some(line_text) = source_text.get(line_start..line_end) else {
        return Vec::new();
    };

    let chars: Vec<(usize, char)> = line_text.char_indices().collect();
    let mut out = Vec::new();
    let mut in_string = false;
    let mut idx = 0usize;
    while idx < chars.len() {
        let (byte_idx, ch) = chars[idx];
        let next = chars.get(idx + 1).map(|(_, ch)| *ch);

        if in_string {
            if ch == '"' {
                if next == Some('"') {
                    idx += 2;
                    continue;
                }
                in_string = false;
            }
            idx += 1;
            continue;
        }

        if ch == '/' && next == Some('/') {
            break;
        }

        if ch == '"' {
            in_string = true;
            idx += 1;
            continue;
        }

        if ch == '(' {
            let prefix_end = line_start.saturating_add(byte_idx);
            let Some(before_paren) = source_text.get(line_start..prefix_end) else {
                idx += 1;
                continue;
            };
            let Some(head_text) = extract_expression_suffix(before_paren) else {
                idx += 1;
                continue;
            };
            let Some(head_span) = source_slice_span(source_text, head_text) else {
                idx += 1;
                continue;
            };
            let Some(kind) = extract_recovery_call_head(head_text) else {
                idx += 1;
                continue;
            };
            let open_paren_u32 = line_start.saturating_add(byte_idx).min(u32::MAX as usize) as u32;
            out.push(IncompleteCallRecoveryCandidate {
                member_span: head_span,
                call_span: bsl_shared::ir::Span::new(
                    head_span.start,
                    open_paren_u32.saturating_add(1),
                ),
                kind,
            });
        }

        idx += 1;
    }

    out
}

fn line_bounds_for_offset(source_text: &str, offset: usize) -> Option<(usize, usize)> {
    if source_text.is_empty() {
        return None;
    }

    let clamped = offset.min(source_text.len().saturating_sub(1));
    let line_start = source_text
        .get(..clamped)
        .and_then(|prefix| prefix.rfind('\n').map(|idx| idx + 1))
        .unwrap_or_default();
    let line_end = source_text
        .get(clamped..)
        .and_then(|suffix| suffix.find('\n').map(|idx| clamped + idx))
        .unwrap_or(source_text.len());
    Some((line_start, line_end))
}

fn find_last_unclosed_call_paren_offset(prefix: &str) -> Option<usize> {
    let chars: Vec<(usize, char)> = prefix.char_indices().collect();
    let mut stack: Vec<usize> = Vec::new();
    let mut in_string = false;
    let mut in_block_comment = false;
    let mut idx = 0usize;

    while idx < chars.len() {
        let (byte_idx, ch) = chars[idx];
        let next = chars.get(idx + 1).map(|(_, ch)| *ch);

        if in_string {
            if ch == '"' {
                if next == Some('"') {
                    idx += 2;
                    continue;
                }
                in_string = false;
            }
            idx += 1;
            continue;
        }

        if in_block_comment {
            if ch == '*' && next == Some('/') {
                in_block_comment = false;
                idx += 2;
                continue;
            }
            idx += 1;
            continue;
        }

        if ch == '/' && next == Some('/') {
            while idx < chars.len() && chars[idx].1 != '\n' {
                idx += 1;
            }
            continue;
        }

        if ch == '/' && next == Some('*') {
            in_block_comment = true;
            idx += 2;
            continue;
        }

        if ch == '"' {
            in_string = true;
            idx += 1;
            continue;
        }

        match ch {
            '(' => stack.push(byte_idx),
            ')' => {
                stack.pop();
            }
            _ => {}
        }

        idx += 1;
    }

    stack.pop()
}

fn find_last_incomplete_argument_call_paren_offset(
    source_text: &str,
    anchor_offset: usize,
) -> Option<usize> {
    let (line_start, line_end) = trimmed_line_bounds_at_offset(source_text, anchor_offset)?;
    let line_text = source_text.get(line_start..line_end)?;

    line_text
        .char_indices()
        .rev()
        .find_map(|(offset, ch)| (ch == '(').then_some(offset))
        .and_then(|offset| {
            call_tail_has_missing_argument(line_text.get(offset..)?)
                .then_some(line_start.saturating_add(offset))
        })
}

fn trimmed_line_bounds_at_offset(
    source_text: &str,
    anchor_offset: usize,
) -> Option<(usize, usize)> {
    if source_text.is_empty() {
        return None;
    }

    let clamped = anchor_offset.min(source_text.len().saturating_sub(1));
    let line_start = source_text
        .get(..clamped)
        .and_then(|prefix| prefix.rfind('\n').map(|idx| idx + 1))
        .unwrap_or_default();
    let line_end_raw = source_text
        .get(clamped..)
        .and_then(|suffix| suffix.find('\n').map(|idx| clamped + idx))
        .unwrap_or(source_text.len());
    let line_text = source_text.get(line_start..line_end_raw)?;
    let trimmed_end = line_start.saturating_add(line_text.trim_end().len());
    Some((line_start, trimmed_end))
}

fn call_tail_has_missing_argument(tail: &str) -> bool {
    let mut depth = 0i32;
    let mut in_string = false;
    let mut pending_argument = false;

    for ch in tail.chars() {
        if in_string {
            if ch == '"' {
                in_string = false;
            }
            continue;
        }

        if ch == '"' {
            in_string = true;
            if depth == 1 {
                pending_argument = false;
            }
            continue;
        }

        match ch {
            '(' => {
                depth += 1;
                if depth == 1 {
                    pending_argument = false;
                }
            }
            ')' => {
                if depth == 1 && pending_argument {
                    return true;
                }
                if depth > 0 {
                    depth -= 1;
                }
            }
            ',' if depth == 1 => pending_argument = true,
            _ if depth == 1 && !ch.is_whitespace() => pending_argument = false,
            _ => {}
        }
    }

    pending_argument
}

fn extract_recovery_call_head(text: &str) -> Option<IncompleteCallRecoveryKind> {
    let trimmed = text.trim_end();

    if let Some(type_name) = extract_recovery_constructor_name(trimmed) {
        return Some(IncompleteCallRecoveryKind::Constructor { type_name });
    }

    if let Some(dot_byte_pos) = trimmed.rfind('.') {
        let after_dot = trimmed.get(dot_byte_pos + 1..)?.trim_start();
        let method_name = after_dot
            .chars()
            .take_while(|ch| recovery_call_identifier_char(*ch))
            .collect::<String>();
        if method_name.is_empty() {
            return None;
        }

        let receiver_expr = trimmed.get(..dot_byte_pos)?.trim_end();
        if receiver_expr.is_empty() {
            return None;
        }

        return Some(IncompleteCallRecoveryKind::Method {
            receiver_expr: receiver_expr.to_string(),
            method_name,
        });
    }

    let function_name = trimmed
        .chars()
        .rev()
        .take_while(|ch| recovery_call_identifier_char(*ch))
        .collect::<String>()
        .chars()
        .rev()
        .collect::<String>();
    if function_name.is_empty() || recovery_call_control_keyword(&function_name) {
        return None;
    }

    None
}

fn extract_recovery_constructor_name(text: &str) -> Option<String> {
    let mut iter = text.split_whitespace();
    let keyword = iter.next()?;
    if !keyword.eq_ignore_ascii_case("Новый") {
        return None;
    }

    let remainder: String = iter.collect::<Vec<_>>().join(" ");
    let normalized: String = remainder.chars().filter(|ch| !ch.is_whitespace()).collect();
    if normalized.is_empty() {
        return None;
    }

    normalized
        .chars()
        .all(|ch| ch == '.' || recovery_call_identifier_char(ch))
        .then_some(normalized)
}

fn recovery_call_control_keyword(value: &str) -> bool {
    matches!(
        value.to_lowercase().as_str(),
        "если"
            | "иначеесли"
            | "пока"
            | "для"
            | "каждого"
            | "попытка"
            | "исключение"
            | "конецесли"
            | "конеццикла"
            | "конецпопытки"
            | "конецпроцедуры"
            | "конецфункции"
            | "возврат"
            | "выбор"
            | "когда"
            | "иначе"
    )
}

fn recovery_call_identifier_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_' || ('А'..='я').contains(&ch) || ch == 'Ё' || ch == 'ё'
}

fn extract_incomplete_member_access_receiver_slices_at_dot_offset(
    source_text: &str,
    dot_offset: usize,
) -> Vec<(bsl_shared::ir::Span, &str)> {
    let Some(receiver_slices) =
        extract_incomplete_member_access_receiver_slice_texts(source_text, dot_offset)
    else {
        return Vec::new();
    };

    receiver_slices
        .into_iter()
        .filter_map(|slice| source_slice_span(source_text, slice).map(|span| (span, slice)))
        .collect()
}

fn extract_incomplete_member_access_target_slice_at_dot_offset(
    source_text: &str,
    dot_offset: usize,
) -> Option<(bsl_shared::ir::Span, &str)> {
    let line_start = source_text
        .get(..dot_offset)?
        .rfind('\n')
        .map(|idx| idx + 1)
        .unwrap_or_default();
    let line_end = source_text
        .get(dot_offset..)?
        .find('\n')
        .map(|idx| dot_offset + idx)
        .unwrap_or(source_text.len());
    let line_text = source_text.get(line_start..line_end)?;
    let dot_in_line = dot_offset.checked_sub(line_start)?;
    let tail = line_text.get(dot_in_line + 1..)?;
    let leading_ws = tail.len().saturating_sub(tail.trim_start().len());
    let tail = tail.get(leading_ws..)?;
    let member_len = tail
        .char_indices()
        .take_while(|(_, ch)| recovery_call_identifier_char(*ch))
        .last()
        .map(|(idx, ch)| idx + ch.len_utf8())
        .unwrap_or_default();
    if member_len == 0 {
        return None;
    }

    let receiver_slices =
        extract_incomplete_member_access_receiver_slice_texts(source_text, dot_offset)?;
    let receiver_expr_text = *receiver_slices.first()?;
    let receiver_span = source_slice_span(source_text, receiver_expr_text)?;
    let member_end = line_start
        .saturating_add(dot_in_line)
        .saturating_add(1)
        .saturating_add(leading_ws)
        .saturating_add(member_len);
    let full_text = source_text.get(receiver_span.start as usize..member_end)?;
    Some((
        bsl_shared::ir::Span::new(
            receiver_span.start,
            member_end.min(u32::MAX as usize) as u32,
        ),
        full_text,
    ))
}

fn incomplete_member_access_dot_offsets_within_span(
    source_text: &str,
    container_span: bsl_shared::ir::Span,
) -> Vec<usize> {
    let global_offsets = incomplete_member_access_dot_offsets(source_text);
    incomplete_member_access_dot_offsets_within_span_from_candidates(
        &global_offsets,
        container_span,
    )
}

fn incomplete_member_access_dot_offsets(source_text: &str) -> Vec<usize> {
    let container_span =
        bsl_shared::ir::Span::new(0, source_text.len().min(u32::MAX as usize) as u32);
    scan_incomplete_member_access_dot_offsets_within_span(source_text, container_span)
}

fn incomplete_member_access_dot_offsets_within_span_from_candidates(
    candidate_offsets: &[usize],
    container_span: bsl_shared::ir::Span,
) -> Vec<usize> {
    let start = container_span.start as usize;
    let end = container_span.end as usize;
    let start_idx = candidate_offsets.partition_point(|offset| *offset < start);
    let end_idx = candidate_offsets.partition_point(|offset| *offset < end);
    candidate_offsets[start_idx..end_idx].to_vec()
}

fn scan_incomplete_member_access_dot_offsets_within_span(
    source_text: &str,
    container_span: bsl_shared::ir::Span,
) -> Vec<usize> {
    scan_member_access_dot_offsets_within_span(
        source_text,
        container_span,
        looks_like_source_incomplete_member_access_tail,
    )
}

fn recovery_incomplete_member_access_dot_offsets_within_span(
    source_text: &str,
    container_span: bsl_shared::ir::Span,
) -> Vec<usize> {
    scan_member_access_dot_offsets_within_span(
        source_text,
        container_span,
        looks_like_recovery_incomplete_member_access_tail,
    )
}

fn scan_member_access_dot_offsets_within_span(
    source_text: &str,
    container_span: bsl_shared::ir::Span,
    tail_predicate: fn(&str) -> bool,
) -> Vec<usize> {
    let start = container_span.start as usize;
    let end = (container_span.end as usize).min(source_text.len());
    let Some(container_text) = source_text.get(start..end) else {
        return Vec::new();
    };
    let mut offsets = Vec::new();
    let mut cursor = start;
    for chunk in container_text.split_inclusive('\n') {
        let line_text = chunk.strip_suffix('\n').unwrap_or(chunk);
        let line_text = line_text.strip_suffix('\r').unwrap_or(line_text);
        if line_text.trim_start().starts_with("//") {
            cursor = cursor.saturating_add(chunk.len());
            continue;
        }
        if let Some(dot_in_line) = line_text.rfind('.') {
            let valid_tail = line_text.get(dot_in_line + 1..).is_some_and(tail_predicate);
            if valid_tail {
                offsets.push(cursor.saturating_add(dot_in_line));
            }
        }
        cursor = cursor.saturating_add(chunk.len());
    }

    offsets
}

fn looks_like_source_incomplete_member_access_tail(tail: &str) -> bool {
    tail.trim().trim_end_matches(';').trim().is_empty()
}

fn looks_like_recovery_incomplete_member_access_tail(tail: &str) -> bool {
    let trimmed = tail.trim();
    if trimmed.is_empty() {
        return true;
    }

    let mut saw_identifier_char = false;
    for ch in trimmed.chars() {
        if ch == ';' {
            continue;
        }
        if ch == '_' || ch.is_alphanumeric() {
            saw_identifier_char = true;
            continue;
        }
        if ch.is_whitespace() {
            continue;
        }
        return false;
    }

    saw_identifier_char
}

fn find_incomplete_member_access_dot_offset(
    source_text: &str,
    error_span: bsl_shared::ir::Span,
) -> Option<usize> {
    let start = error_span.start as usize;
    let end = (error_span.end as usize).min(source_text.len());

    if let Some(snippet) = source_text.get(start..end) {
        let trimmed_start = snippet
            .char_indices()
            .find_map(|(idx, ch)| (!ch.is_whitespace()).then_some(idx))
            .unwrap_or_default();
        if let Some(trimmed) = snippet.get(trimmed_start..) {
            let trimmed = trimmed.trim_end();
            if trimmed.ends_with('.') {
                return Some(
                    start
                        .saturating_add(trimmed_start)
                        .saturating_add(trimmed.len().saturating_sub(1)),
                );
            }
        }
    }

    line_end_dot_offset(source_text, end.saturating_sub(1))
        .or_else(|| line_end_dot_offset(source_text, start))
}

fn line_end_dot_offset(source_text: &str, offset: usize) -> Option<usize> {
    if source_text.is_empty() {
        return None;
    }

    let clamped = offset.min(source_text.len().saturating_sub(1));
    let line_start = source_text
        .get(..clamped)
        .and_then(|prefix| prefix.rfind('\n').map(|idx| idx + 1))
        .unwrap_or_default();
    let line_end = source_text
        .get(clamped..)
        .and_then(|suffix| suffix.find('\n').map(|idx| clamped + idx))
        .unwrap_or(source_text.len());
    let line_text = source_text.get(line_start..line_end)?;
    let trimmed = line_text.trim_end();
    let dot_in_line = trimmed.rfind('.')?;
    if trimmed
        .get(dot_in_line + 1..)
        .is_some_and(|tail| !tail.trim().is_empty())
    {
        return None;
    }

    Some(line_start.saturating_add(dot_in_line))
}

fn extract_incomplete_member_access_receiver_slice_texts(
    source_text: &str,
    dot_offset: usize,
) -> Option<Vec<&str>> {
    let file_prefix = source_text.get(..dot_offset)?;
    if let Some(choice_text) = extract_choice_expression(file_prefix) {
        return extract_choice_result_expression_slices(choice_text);
    }

    let line_start = file_prefix
        .rfind('\n')
        .map(|idx| idx + 1)
        .unwrap_or_default();
    let line_prefix = source_text.get(line_start..dot_offset)?;
    let receiver_expr_text = extract_expression_suffix(line_prefix)?;
    let receiver_expr_text = strip_wrapping_parentheses(receiver_expr_text.trim());
    if receiver_expr_text.is_empty() {
        return None;
    }

    if let Some(choice_text) = extract_choice_expression(receiver_expr_text) {
        return extract_choice_result_expression_slices(choice_text);
    }

    Some(
        extract_ternary_result_expression_slices(receiver_expr_text)
            .unwrap_or_else(|| vec![receiver_expr_text]),
    )
}

fn parse_recovery_expression_snippet(expr_text: &str) -> Option<Expression> {
    let synthetic = format!(
        "Procedure __Recovery__()\n    __tmp = {};\nEndProcedure\n",
        expr_text
    );
    let parse = bsl_syntax::parse_fast(&synthetic).ok()?;
    find_first_assignment_value(&parse.program)
}

fn find_first_assignment_value(program: &Program) -> Option<Expression> {
    for stmt in &program.statements {
        if let Some(value) = find_first_assignment_value_in_statement(stmt) {
            return Some(value);
        }
    }

    None
}

fn find_first_assignment_value_in_statement(stmt: &Statement) -> Option<Expression> {
    match stmt {
        Statement::Assignment { value, .. } => Some(value.clone()),
        Statement::FunctionDecl { body, .. } | Statement::ProcedureDecl { body, .. } => body
            .iter()
            .find_map(find_first_assignment_value_in_statement),
        _ => None,
    }
}

fn strip_wrapping_parentheses(text: &str) -> &str {
    let mut out = text.trim();
    loop {
        let Some(stripped) = try_strip_one_pair_of_parens(out) else {
            return out;
        };
        out = stripped.trim();
    }
}

fn try_strip_one_pair_of_parens(text: &str) -> Option<&str> {
    if !text.starts_with('(') || !text.ends_with(')') {
        return None;
    }

    let mut depth: i32 = 0;
    let mut in_string = false;

    for (idx, ch) in text.char_indices() {
        if in_string {
            if ch == '"' {
                in_string = false;
            }
            continue;
        }

        if ch == '"' {
            in_string = true;
            continue;
        }

        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth < 0 {
                    return None;
                }
                if depth == 0 && idx.saturating_add(ch.len_utf8()) != text.len() {
                    return None;
                }
            }
            _ => {}
        }
    }

    if depth != 0 {
        return None;
    }

    text.get(1..text.len().saturating_sub(1))
}

fn extract_expression_suffix(prefix: &str) -> Option<&str> {
    let trimmed = prefix.trim_end();
    if trimmed.is_empty() {
        return None;
    }

    let start = find_expression_start(trimmed);
    let expr = trimmed.get(start..)?.trim();
    if expr.is_empty() {
        None
    } else {
        Some(expr)
    }
}

fn find_expression_start(prefix: &str) -> usize {
    let mut paren_depth: i32 = 0;
    let mut bracket_depth: i32 = 0;
    let mut in_string = false;

    let chars: Vec<(usize, char)> = prefix.char_indices().collect();
    for &(idx, ch) in chars.iter().rev() {
        if in_string {
            if ch == '"' {
                in_string = false;
            }
            continue;
        }

        match ch {
            '"' => {
                in_string = true;
                continue;
            }
            ')' => {
                paren_depth += 1;
                continue;
            }
            '(' => {
                if paren_depth > 0 {
                    paren_depth -= 1;
                    continue;
                }
                return idx + ch.len_utf8();
            }
            ']' => {
                bracket_depth += 1;
                continue;
            }
            '[' => {
                if bracket_depth > 0 {
                    bracket_depth -= 1;
                    continue;
                }
                return idx + ch.len_utf8();
            }
            _ => {}
        }

        if paren_depth != 0 || bracket_depth != 0 {
            continue;
        }

        match ch {
            ';' | ',' | '=' | '+' | '-' | '*' | '/' => return idx + ch.len_utf8(),
            _ => {}
        }
    }

    0
}

fn extract_choice_result_expression_slices(receiver_expr_text: &str) -> Option<Vec<&str>> {
    let start_offset = receiver_expr_text
        .char_indices()
        .find_map(|(idx, ch)| (!ch.is_whitespace()).then_some(idx))
        .unwrap_or(receiver_expr_text.len());

    let lower = receiver_expr_text.to_lowercase();
    if lower.len() != receiver_expr_text.len() {
        return None;
    }

    if keyword_at(&lower, start_offset, "выбор").is_none()
        && keyword_at(&lower, start_offset, "case").is_none()
    {
        return None;
    }

    let keywords = collect_choice_keywords(receiver_expr_text, &lower, start_offset);
    if keywords.is_empty() {
        return None;
    }

    let end_start = keywords
        .iter()
        .rfind(|kw| kw.kind == ChoiceKeywordKind::End)
        .map(|kw| kw.start)?;

    let mut out: Vec<&str> = Vec::new();

    for kw in &keywords {
        if kw.kind != ChoiceKeywordKind::Then || kw.end > end_start {
            continue;
        }

        let expr_start = skip_ws(receiver_expr_text, kw.end);
        let expr_end = keywords
            .iter()
            .filter(|next| next.start >= expr_start)
            .filter(|next| {
                matches!(
                    next.kind,
                    ChoiceKeywordKind::When | ChoiceKeywordKind::Else | ChoiceKeywordKind::End
                )
            })
            .map(|next| next.start)
            .min()
            .unwrap_or(receiver_expr_text.len());

        let expr = receiver_expr_text.get(expr_start..expr_end)?.trim();
        if !expr.is_empty() {
            out.push(expr);
        }
    }

    if let Some(else_kw) = keywords
        .iter()
        .find(|kw| kw.kind == ChoiceKeywordKind::Else && kw.end <= end_start)
        .copied()
    {
        let expr_start = skip_ws(receiver_expr_text, else_kw.end);
        let expr = receiver_expr_text.get(expr_start..end_start)?.trim();
        if !expr.is_empty() {
            out.push(expr);
        }
    }

    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

fn extract_ternary_result_expression_slices(receiver_expr_text: &str) -> Option<Vec<&str>> {
    let trimmed = receiver_expr_text.trim();
    if !trimmed.starts_with("?(") || !trimmed.ends_with(')') {
        return None;
    }

    let inner = trimmed.get(2..trimmed.len().saturating_sub(1))?;
    let parts = split_top_level_csv(inner);
    if parts.len() != 3 {
        return None;
    }

    let then_expr = parts.get(1)?.trim();
    let else_expr = parts.get(2)?.trim();
    if then_expr.is_empty() || else_expr.is_empty() {
        return None;
    }

    Some(vec![then_expr, else_expr])
}

fn split_top_level_csv(text: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0usize;
    let mut paren_depth: i32 = 0;
    let mut bracket_depth: i32 = 0;
    let mut in_string = false;

    for (idx, ch) in text.char_indices() {
        if in_string {
            if ch == '"' {
                in_string = false;
            }
            continue;
        }

        match ch {
            '"' => in_string = true,
            '(' => paren_depth += 1,
            ')' => paren_depth -= 1,
            '[' => bracket_depth += 1,
            ']' => bracket_depth -= 1,
            ',' if paren_depth == 0 && bracket_depth == 0 => {
                if let Some(part) = text.get(start..idx) {
                    parts.push(part);
                }
                start = idx.saturating_add(ch.len_utf8());
            }
            _ => {}
        }
    }

    if let Some(part) = text.get(start..) {
        parts.push(part);
    }

    parts
}

fn source_slice_span(source: &str, slice: &str) -> Option<bsl_shared::ir::Span> {
    let slice_start = slice.as_ptr() as usize;
    let source_start = source.as_ptr() as usize;
    let start = slice_start.checked_sub(source_start)?;
    let end = start.checked_add(slice.len())?;
    if end > source.len() {
        return None;
    }

    Some(bsl_shared::ir::Span {
        start: start.min(u32::MAX as usize) as u32,
        end: end.min(u32::MAX as usize) as u32,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChoiceKeywordKind {
    Case,
    When,
    Then,
    Else,
    End,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ChoiceKeyword {
    kind: ChoiceKeywordKind,
    start: usize,
    end: usize,
}

fn extract_choice_expression(receiver_expr_text: &str) -> Option<&str> {
    let trimmed = receiver_expr_text.trim_end();
    let lower = trimmed.to_lowercase();
    if lower.len() != trimmed.len() {
        return None;
    }

    let keywords = collect_choice_keywords(trimmed, &lower, 0);
    if keywords.is_empty() {
        return None;
    }

    let mut stack: Vec<usize> = Vec::new();
    let mut matched_start: Option<usize> = None;
    for kw in &keywords {
        match kw.kind {
            ChoiceKeywordKind::Case => stack.push(kw.start),
            ChoiceKeywordKind::End => {
                let Some(case_start) = stack.pop() else {
                    continue;
                };
                if kw.end == trimmed.len() {
                    matched_start = Some(case_start);
                }
            }
            _ => {}
        }
    }

    let start = matched_start?;
    trimmed.get(start..)
}

fn collect_choice_keywords(
    receiver_expr_text: &str,
    lower: &str,
    start_offset: usize,
) -> Vec<ChoiceKeyword> {
    let mut keywords: Vec<ChoiceKeyword> = Vec::new();
    let mut i = start_offset;
    let mut in_string = false;
    let mut paren_depth: i32 = 0;
    let mut bracket_depth: i32 = 0;

    while i < receiver_expr_text.len() {
        let ch = receiver_expr_text[i..].chars().next().unwrap_or('\0');
        let ch_len = ch.len_utf8().max(1);

        if in_string {
            if ch == '"' {
                let next_i = i.saturating_add(ch_len);
                let is_escaped_quote = receiver_expr_text
                    .get(next_i..)
                    .and_then(|rest| rest.chars().next())
                    .is_some_and(|next_ch| next_ch == '"');

                if is_escaped_quote {
                    i = next_i.saturating_add(1);
                    continue;
                }
                in_string = false;
            }

            i = i.saturating_add(ch_len);
            continue;
        }

        match ch {
            '"' => {
                in_string = true;
                i = i.saturating_add(ch_len);
                continue;
            }
            '(' => {
                paren_depth = paren_depth.saturating_add(1);
                i = i.saturating_add(ch_len);
                continue;
            }
            ')' => {
                if paren_depth > 0 {
                    paren_depth -= 1;
                }
                i = i.saturating_add(ch_len);
                continue;
            }
            '[' => {
                bracket_depth = bracket_depth.saturating_add(1);
                i = i.saturating_add(ch_len);
                continue;
            }
            ']' => {
                if bracket_depth > 0 {
                    bracket_depth -= 1;
                }
                i = i.saturating_add(ch_len);
                continue;
            }
            _ => {}
        }

        if paren_depth == 0 && bracket_depth == 0 {
            if let Some(end) =
                keyword_at(lower, i, "выбор").or_else(|| keyword_at(lower, i, "case"))
            {
                keywords.push(ChoiceKeyword {
                    kind: ChoiceKeywordKind::Case,
                    start: i,
                    end,
                });
                i = end;
                continue;
            }

            if let Some(end) =
                keyword_at(lower, i, "когда").or_else(|| keyword_at(lower, i, "when"))
            {
                keywords.push(ChoiceKeyword {
                    kind: ChoiceKeywordKind::When,
                    start: i,
                    end,
                });
                i = end;
                continue;
            }

            if let Some(end) =
                keyword_at(lower, i, "тогда").or_else(|| keyword_at(lower, i, "then"))
            {
                keywords.push(ChoiceKeyword {
                    kind: ChoiceKeywordKind::Then,
                    start: i,
                    end,
                });
                i = end;
                continue;
            }

            if let Some(end) =
                keyword_at(lower, i, "иначе").or_else(|| keyword_at(lower, i, "else"))
            {
                keywords.push(ChoiceKeyword {
                    kind: ChoiceKeywordKind::Else,
                    start: i,
                    end,
                });
                i = end;
                continue;
            }

            if let Some(end) = keyword_at(lower, i, "конецвыбора")
                .or_else(|| keyword_at(lower, i, "endcase"))
                .or_else(|| keyword_at(lower, i, "конец"))
                .or_else(|| keyword_at(lower, i, "end"))
            {
                keywords.push(ChoiceKeyword {
                    kind: ChoiceKeywordKind::End,
                    start: i,
                    end,
                });
                i = end;
                continue;
            }
        }

        i = i.saturating_add(ch_len);
    }

    keywords
}

fn skip_ws(text: &str, mut idx: usize) -> usize {
    while let Some(ch) = text.get(idx..).and_then(|rest| rest.chars().next()) {
        if ch.is_whitespace() {
            idx = idx.saturating_add(ch.len_utf8());
        } else {
            break;
        }
    }

    idx
}

fn keyword_at(lower: &str, idx: usize, keyword: &str) -> Option<usize> {
    let rest = lower.get(idx..)?;
    if !rest.starts_with(keyword) {
        return None;
    }

    let before = lower.get(..idx)?.chars().next_back();
    if before.is_some_and(is_word_char) {
        return None;
    }

    let end = idx.saturating_add(keyword.len());
    let after = lower.get(end..)?.chars().next();
    if after.is_some_and(is_word_char) {
        return None;
    }

    Some(end)
}

fn is_word_char(ch: char) -> bool {
    ch == '_' || ch.is_alphanumeric()
}

#[cfg(test)]
pub(crate) fn build_type_index_with_path(
    program: &Program,
    file_path: &str,
    deps: Arc<SemanticDeps>,
) -> TypeIndex {
    TypeInferencer::new(deps).build_index(program, file_path)
}

pub(crate) fn build_type_index_from_semantic_program_with_path_profiled(
    program: &SemanticProgram,
    file_path: &str,
    deps: Arc<SemanticDeps>,
) -> TypeIndexBuildProfiled {
    TypeInferencer::new(deps).build_index_from_semantic_program_profiled(program, file_path, None)
}

pub(crate) fn materialize_semantic_facts_with_path_profiled(
    program: &mut SemanticProgram,
    parsed_program: &Program,
    source_text: &str,
    file_path: &str,
    deps: Arc<SemanticDeps>,
) -> TypeIndexBuildProfile {
    let profiled = TypeInferencer::new(deps).build_facts_internal(
        parsed_program,
        file_path,
        Some(source_text),
        None,
    );
    program.semantic_facts = profiled.facts;
    profiled.profile
}

#[cfg(test)]
pub(crate) fn materialize_semantic_facts_with_recovery_with_path_profiled(
    program: &mut SemanticProgram,
    parsed: &bsl_syntax::ast::ParseResult,
    source_text: &str,
    file_path: &str,
    deps: Arc<SemanticDeps>,
) -> TypeIndexBuildProfile {
    let profiled = TypeInferencer::new(deps).build_facts_internal(
        &parsed.program,
        file_path,
        Some(source_text),
        Some(RecoveryContext {
            source_text,
            syntax_errors: &parsed.syntax_errors,
        }),
    );
    program.semantic_facts = profiled.facts;
    profiled.profile
}

#[cfg(test)]
pub(crate) fn build_type_index_from_parse_result_with_path(
    parsed: &bsl_syntax::ast::ParseResult,
    source_text: &str,
    file_path: &str,
    deps: Arc<SemanticDeps>,
) -> TypeIndex {
    TypeInferencer::new(deps)
        .build_index_from_parse_result_profiled(parsed, source_text, file_path)
        .index
}

#[cfg(test)]
pub(crate) fn build_type_index_from_semantic_program_with_path(
    program: &SemanticProgram,
    file_path: &str,
    deps: Arc<SemanticDeps>,
) -> TypeIndex {
    TypeInferencer::new(deps)
        .build_index_from_semantic_program_profiled(program, file_path, None)
        .index
}

#[cfg(test)]
pub(crate) fn build_type_index_from_semantic_program_with_recovery_with_path(
    program: &SemanticProgram,
    source_text: &str,
    syntax_errors: &[ParseError],
    file_path: &str,
    deps: Arc<SemanticDeps>,
) -> TypeIndex {
    TypeInferencer::new(deps)
        .build_index_from_semantic_program_profiled(
            program,
            file_path,
            Some(RecoveryContext {
                source_text,
                syntax_errors,
            }),
        )
        .index
}

#[cfg(test)]
#[path = "type_inference_v2/tests.rs"]
mod tests;
