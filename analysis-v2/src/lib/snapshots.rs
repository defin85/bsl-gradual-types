#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseChangedRange {
    pub start_byte: u32,
    pub old_end_byte: u32,
    pub new_end_byte: u32,
}

#[derive(Debug, Clone)]
pub struct ParseSnapshot {
    pub file_id: FileId,
    pub file_version: i32,
    pub parse_result: Arc<bsl_syntax::ast::ParseResult>,
    pub line_index: Arc<LineIndex>,
    pub backend_tree: Arc<Tree>,
    pub changed_ranges: Arc<Vec<ParseChangedRange>>,
    pub produced_at_millis: u128,
    pub backend_tree_hash: u64,
    pub incremental: bool,
    pub fallback_reason: Option<Arc<str>>,
}

#[salsa::input]
pub struct SourceFile {
    pub id: u32,
    #[returns(ref)]
    pub text: Arc<str>,
    pub version: i32,
    #[returns(ref)]
    pub path: Arc<str>,
}

#[salsa::input]
pub struct DepsSnapshot {
    #[returns(ref)]
    pub id: DepsSnapshotId,
    #[returns(ref)]
    pub data: DepsDataSnapshot,
}

#[salsa::input]
pub struct SettingsSnapshot {
    #[returns(ref)]
    pub id: SettingsId,
    pub diagnostics_detail_level: DetailLevel,
}

#[derive(Debug, Clone)]
pub struct ParseResultSnapshot(Arc<bsl_syntax::ast::ParseResult>);

impl PartialEq for ParseResultSnapshot {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for ParseResultSnapshot {}

unsafe impl salsa::Update for ParseResultSnapshot {
    unsafe fn maybe_update(old_pointer: *mut Self, new_value: Self) -> bool {
        // Always treat the parse result as updated. This avoids coupling the syntax layer
        // to salsa (via `Update`/`PartialEq` requirements) and is safe for correctness.
        let old_value: &mut Self = unsafe { &mut *old_pointer };
        *old_value = new_value;
        true
    }
}

#[derive(Debug, Clone)]
pub struct SemanticProgramSnapshot(Arc<SemanticProgram>);

impl PartialEq for SemanticProgramSnapshot {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for SemanticProgramSnapshot {}

unsafe impl salsa::Update for SemanticProgramSnapshot {
    unsafe fn maybe_update(old_pointer: *mut Self, new_value: Self) -> bool {
        let old_value: &mut Self = unsafe { &mut *old_pointer };
        *old_value = new_value;
        true
    }
}

#[derive(Debug, Clone)]
pub struct SyntaxDiagnosticsSnapshot(Arc<Vec<ParseError>>);

impl PartialEq for SyntaxDiagnosticsSnapshot {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for SyntaxDiagnosticsSnapshot {}

unsafe impl salsa::Update for SyntaxDiagnosticsSnapshot {
    unsafe fn maybe_update(old_pointer: *mut Self, new_value: Self) -> bool {
        let old_value: &mut Self = unsafe { &mut *old_pointer };
        *old_value = new_value;
        true
    }
}

#[derive(Debug, Clone)]
pub struct SemanticDiagnosticsSnapshot(Arc<Vec<TypeDiagnostic>>);

impl PartialEq for SemanticDiagnosticsSnapshot {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for SemanticDiagnosticsSnapshot {}

unsafe impl salsa::Update for SemanticDiagnosticsSnapshot {
    unsafe fn maybe_update(old_pointer: *mut Self, new_value: Self) -> bool {
        let old_value: &mut Self = unsafe { &mut *old_pointer };
        *old_value = new_value;
        true
    }
}

#[derive(Debug, Clone)]
pub struct TypeIndexSnapshot {
    index: Arc<type_inference_v2::TypeIndex>,
    parse_result_ms: u128,
    build_profile: type_inference_v2::TypeIndexBuildProfile,
    query_profile: TypeIndexQueryProfile,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TypeIndexQueryProfile {
    inputs_ms: u128,
    parse_result_query_ms: u128,
    build_ms: u128,
    total_ms: u128,
}

impl TypeIndexSnapshot {
    fn new(
        index: Arc<type_inference_v2::TypeIndex>,
        parse_result_ms: u128,
        build_profile: type_inference_v2::TypeIndexBuildProfile,
        query_profile: TypeIndexQueryProfile,
    ) -> Self {
        Self {
            index,
            parse_result_ms,
            build_profile,
            query_profile,
        }
    }

    fn index(&self) -> Arc<type_inference_v2::TypeIndex> {
        self.index.clone()
    }

    fn parse_result_ms(&self) -> u128 {
        self.parse_result_ms
    }

    fn build_profile(&self) -> type_inference_v2::TypeIndexBuildProfile {
        self.build_profile
    }

    fn query_profile(&self) -> TypeIndexQueryProfile {
        self.query_profile
    }
}

impl PartialEq for TypeIndexSnapshot {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.index, &other.index)
    }
}

impl Eq for TypeIndexSnapshot {}

unsafe impl salsa::Update for TypeIndexSnapshot {
    unsafe fn maybe_update(old_pointer: *mut Self, new_value: Self) -> bool {
        let old_value: &mut Self = unsafe { &mut *old_pointer };
        *old_value = new_value;
        true
    }
}

#[salsa::tracked]
pub fn file_text_len(db: &dyn salsa::Database, file: SourceFile) -> usize {
    file.text(db).len()
}

#[salsa::tracked]
pub fn line_index(db: &dyn salsa::Database, file: SourceFile) -> Arc<LineIndex> {
    Arc::new(LineIndex::new(file.text(db)))
}

#[salsa::tracked]
pub fn parse_result(
    db: &dyn salsa::Database,
    file: SourceFile,
    settings: SettingsSnapshot,
) -> ParseResultSnapshot {
    cancellation_checkpoint(db);
    let _settings_id = settings.id(db);
    let text = file.text(db);
    cancellation_checkpoint(db);
    let options = ParseOptions::default();
    match bsl_syntax::parse(text, &options) {
        Ok(parsed) => ParseResultSnapshot(Arc::new(parsed)),
        Err(err) => ParseResultSnapshot(Arc::new(bsl_syntax::ast::ParseResult::with_errors(
            bsl_syntax::ast::Program {
                statements: Vec::new(),
            },
            vec![bsl_syntax::ast::ParseError {
                message: err.to_string(),
                span: bsl_syntax::ast::Span::stub(),
                error_type: bsl_syntax::ast::ErrorType::ParseError,
                related: Vec::new(),
            }],
        ))),
    }
}

#[salsa::tracked]
pub fn ir(
    db: &dyn salsa::Database,
    file: SourceFile,
    deps: DepsSnapshot,
    settings: SettingsSnapshot,
) -> SemanticProgramSnapshot {
    cancellation_checkpoint(db);
    let _deps_id = deps.id(db);
    let deps_data = deps.data(db).0.clone();

    let parsed = parse_result(db, file, settings).0;
    cancellation_checkpoint(db);
    let source = file.text(db).to_string();
    let file_path = file.path(db).to_string();
    cancellation_checkpoint(db);
    let checkpoint = || cancellation_checkpoint(db);
    SemanticProgramSnapshot(
        build_ir_from_parsed_profiled_with_checkpoint(
            parsed,
            &source,
            &file_path,
            deps_data,
            Some(&checkpoint),
        )
        .program,
    )
}

#[salsa::tracked]
pub fn syntax_diagnostics(
    db: &dyn salsa::Database,
    file: SourceFile,
    settings: SettingsSnapshot,
) -> SyntaxDiagnosticsSnapshot {
    cancellation_checkpoint(db);
    let _settings_id = settings.id(db);
    let parsed = parse_result(db, file, settings).0;
    cancellation_checkpoint(db);
    SyntaxDiagnosticsSnapshot(Arc::new(parsed.syntax_errors.clone()))
}

#[salsa::tracked]
pub fn semantic_diagnostics(
    db: &dyn salsa::Database,
    file: SourceFile,
    deps: DepsSnapshot,
    settings: SettingsSnapshot,
) -> SemanticDiagnosticsSnapshot {
    cancellation_checkpoint(db);
    let _deps_id = deps.id(db);
    let _settings_id = settings.id(db);
    let deps_data = deps.data(db).0.clone();
    cancellation_checkpoint(db);

    let parsed = parse_result(db, file, settings).0;
    cancellation_checkpoint(db);
    if !parsed.syntax_errors.is_empty()
        && !syntax_errors_only_in_directives(file.text(db), &parsed.syntax_errors)
    {
        return SemanticDiagnosticsSnapshot(Arc::new(Vec::new()));
    }

    let program = ir(db, file, deps, settings).0;
    cancellation_checkpoint(db);
    let detail_level = settings.diagnostics_detail_level(db);
    let diagnostics = collect_semantic_diagnostics_from_program(program, deps_data, detail_level);
    SemanticDiagnosticsSnapshot(Arc::new(diagnostics))
}

#[salsa::tracked]
pub fn semantic_diagnostics_flow_sensitive(
    db: &dyn salsa::Database,
    file: SourceFile,
    deps: DepsSnapshot,
    settings: SettingsSnapshot,
) -> SemanticDiagnosticsSnapshot {
    cancellation_checkpoint(db);
    let _deps_id = deps.id(db);
    let _settings_id = settings.id(db);

    let base = semantic_diagnostics(db, file, deps, settings).0;
    cancellation_checkpoint(db);

    let parsed = parse_result(db, file, settings).0;
    cancellation_checkpoint(db);
    if !parsed.syntax_errors.is_empty()
        && !syntax_errors_only_in_directives(file.text(db), &parsed.syntax_errors)
    {
        return SemanticDiagnosticsSnapshot(base);
    }

    let program = ir(db, file, deps, settings).0;
    cancellation_checkpoint(db);
    let deps_data = deps.data(db).0.clone();
    let resolver = deps_data
        .resolver
        .clone()
        .unwrap_or_else(|| Arc::new(TypeResolver::new(deps_data.repository.clone())));

    let mut diagnostics = (*base).clone();
    cancellation_checkpoint(db);
    diagnostics.extend(flow_sensitive_null_safety_diagnostics(
        &program,
        resolver.as_ref(),
    ));
    diagnostics.sort_by(|a, b| {
        let severity_key = |severity: DiagnosticSeverity| match severity {
            DiagnosticSeverity::Error => 0_u8,
            DiagnosticSeverity::Warning => 1_u8,
            DiagnosticSeverity::Info => 2_u8,
            DiagnosticSeverity::Hint => 3_u8,
        };
        (
            a.span.start,
            a.span.end,
            severity_key(a.severity),
            &a.message,
        )
            .cmp(&(
                b.span.start,
                b.span.end,
                severity_key(b.severity),
                &b.message,
            ))
    });

    SemanticDiagnosticsSnapshot(Arc::new(diagnostics))
}

fn flow_type_at_byte_offset_impl(
    program: &SemanticProgram,
    byte_offset: u32,
    base_type: TypeResolution,
) -> Option<TypeResolution> {
    let (variable_name, _) = program.find_variable_at_byte_offset(byte_offset)?;
    let cfg = program.cfg.as_ref()?;
    let node_id = cfg.node_at_byte_offset(byte_offset, NodeAtByteOffsetBias::PreferLeft)?;

    let mut ctx = FlowAnalysisContext::new();
    ctx.set_variable(variable_name.as_str(), base_type.clone());

    // Убедимся, что все переменные из type-guards присутствуют в контексте (хотя бы как Unknown),
    // иначе NarrowingEngine может не применить narrowing к нужной переменной.
    for node in cfg.nodes() {
        match &node.kind {
            CfgNodeKind::Conditional { condition } | CfgNodeKind::LoopHeader { condition } => {
                for guard in detect_type_guards(condition) {
                    let var = guard.variable_name();
                    if ctx.get_variable(var).is_none() {
                        ctx.set_variable(var, TypeResolution::unknown());
                    }
                }
            }
            _ => {}
        }
    }

    let mut engine = NarrowingEngine::new(cfg.clone());
    engine.build_narrowing_contexts(ctx);

    let narrowed = engine
        .get_context(node_id)
        .and_then(|ctx| ctx.get_type(variable_name.as_str()))
        .cloned()
        .filter(|t| !t.is_unknown())?;

    (narrowed.type_name() != base_type.type_name()).then_some(narrowed)
}

fn flow_sensitive_null_safety_diagnostics(
    program: &SemanticProgram,
    resolver: &TypeResolver,
) -> Vec<TypeDiagnostic> {
    use bsl_shared::domain::types::{ConcreteType, PlatformType, ResolutionResult, SpecialType};
    use bsl_shared::ir::CfgNodeKind;
    use bsl_shared::ir::SemanticNodeKind;

    let Some(cfg) = program.cfg.as_ref() else {
        return Vec::new();
    };

    fn merge_var(ctx: &mut FlowAnalysisContext, name: &str, res: TypeResolution) {
        let mut other = FlowAnalysisContext::new();
        other.set_variable(name, res);
        ctx.merge(&other);
    }

    let mut ctx = FlowAnalysisContext::new();

    fn is_nullish_resolution(resolution: &TypeResolution) -> bool {
        if resolution.result.is_nullable() {
            return true;
        }
        matches!(
            &resolution.result,
            ResolutionResult::Concrete(ConcreteType::Special(SpecialType::Null))
                | ResolutionResult::Concrete(ConcreteType::Special(SpecialType::Undefined))
        ) || matches!(
            &resolution.result,
            ResolutionResult::Concrete(ConcreteType::Platform(PlatformType { name })) if {
                let lower = name.to_lowercase();
                lower == "null" || lower == "неопределено" || lower == "undefined"
            }
        )
    }

    fn leading_ident_token_lower(value: &str) -> Option<String> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return None;
        }
        let token: String = trimmed
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        (!token.is_empty()).then(|| token.to_lowercase())
    }

    // Минимальная и надёжная база для null-safety: фиксируем явные присваивания
    // `Null` / `Неопределено` из CFG (не завися от совпадения span-ов IR и type_index).
    for node in cfg.nodes() {
        let CfgNodeKind::Assignment { variable, value } = &node.kind else {
            continue;
        };

        let Some(v) = leading_ident_token_lower(value) else {
            continue;
        };
        if v == "null" {
            merge_var(
                &mut ctx,
                variable.as_str(),
                TypeResolution::primitive("Null"),
            );
        } else if v == "неопределено" || v == "undefined" {
            merge_var(
                &mut ctx,
                variable.as_str(),
                TypeResolution::primitive("Неопределено"),
            );
        }
    }

    // Инициализация контекста из type_index по rhs-span присваивания.
    for node in &program.nodes {
        let SemanticNodeKind::Assignment {
            variable,
            value_span,
            ..
        } = &node.kind
        else {
            continue;
        };
        let Some(resolution) = program.semantic_facts.type_resolution_for_span(*value_span) else {
            continue;
        };
        if is_nullish_resolution(&resolution) {
            merge_var(&mut ctx, variable.as_str(), resolution);
        }
    }

    for node in &program.nodes {
        let SemanticNodeKind::VariableDeclaration {
            name, type_hint, ..
        } = &node.kind
        else {
            continue;
        };
        if let Some(hint) = type_hint
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            let resolved = resolver.resolve_expression_sync(hint);
            if !resolved.is_unknown() {
                merge_var(&mut ctx, name.as_str(), resolved);
            }
        }
    }

    let mut analyzer = NullSafetyAnalyzer::new(cfg.clone());
    let result = analyzer.analyze(&ctx);

    result
        .warnings
        .into_iter()
        .filter_map(|w| {
            let span = w.span.or_else(|| {
                cfg.node_ir_node_index(w.node_id)
                    .and_then(|idx| program.nodes.get(idx).map(|n| n.span))
            })?;
            Some(TypeDiagnostic {
                severity: DiagnosticSeverity::Warning,
                message: w.message,
                span,
            })
        })
        .collect()
}

fn semantic_type_hints_from_program(program: &SemanticProgram) -> SemanticTypeHints {
    use bsl_shared::ir::SemanticNodeKind;

    fn receiver_span(
        program: &SemanticProgram,
        object_node: Option<usize>,
        object_span: Option<bsl_shared::ir::Span>,
    ) -> Option<bsl_shared::ir::Span> {
        object_span
            .or_else(|| object_node.and_then(|idx| program.nodes.get(idx).map(|node| node.span)))
    }

    let mut hints = SemanticTypeHints::default();

    for node in &program.nodes {
        match &node.kind {
            SemanticNodeKind::Assignment { value_span, .. } => {
                if let Some(resolution) =
                    program.semantic_facts.type_resolution_for_span(*value_span)
                {
                    hints
                        .assignment_value_type_by_span
                        .insert(node.span, resolution);
                }
            }
            SemanticNodeKind::FunctionCall {
                object_node,
                object_span,
                arg_spans,
                ..
            } => {
                let arg_types: Vec<TypeResolution> = arg_spans
                    .iter()
                    .filter_map(|span| program.semantic_facts.type_resolution_for_span(*span))
                    .collect();
                hints.call_arg_types_by_span.insert(node.span, arg_types);

                if let Some(span) = receiver_span(program, *object_node, *object_span) {
                    if let Some(receiver_type) =
                        program.semantic_facts.type_resolution_for_span(span)
                    {
                        hints
                            .call_receiver_type_by_span
                            .insert(node.span, receiver_type);
                    }
                }
            }
            SemanticNodeKind::MemberAccess {
                object_node,
                object_span,
                ..
            } => {
                if let Some(span) = receiver_span(program, *object_node, *object_span) {
                    if let Some(receiver_type) =
                        program.semantic_facts.type_resolution_for_span(span)
                    {
                        hints
                            .member_access_object_type_by_span
                            .insert(node.span, receiver_type);
                    }
                }
            }
            _ => {}
        }
    }

    hints
}

#[salsa::tracked]
pub fn type_index(
    db: &dyn salsa::Database,
    file: SourceFile,
    deps: DepsSnapshot,
    settings: SettingsSnapshot,
) -> TypeIndexSnapshot {
    let started = Instant::now();
    cancellation_checkpoint(db);
    let inputs_started = Instant::now();
    let _deps_id = deps.id(db);
    let _settings_id = settings.id(db);
    let deps_data = deps.data(db).0.clone();
    let inputs_ms = inputs_started.elapsed().as_millis();
    let ir_started = Instant::now();
    let program = ir(db, file, deps, settings).0;
    let ir_ms = ir_started.elapsed().as_millis();
    cancellation_checkpoint(db);
    let build_started = Instant::now();
    let mut profiled = type_inference_v2::build_type_index_from_semantic_program_with_path_profiled(
        program.as_ref(),
        file.path(db).as_ref(),
        deps_data,
    );
    let build_ms = build_started.elapsed().as_millis();
    let total_ms = started.elapsed().as_millis();
    if profiled.profile.total_ms < total_ms {
        profiled.profile.total_ms = total_ms;
    }
    TypeIndexSnapshot::new(
        Arc::new(profiled.index),
        0,
        profiled.profile,
        TypeIndexQueryProfile {
            inputs_ms: inputs_ms.saturating_add(ir_ms),
            parse_result_query_ms: 0,
            build_ms,
            total_ms,
        },
    )
}

fn syntax_errors_only_in_directives(code: &str, errors: &[ParseError]) -> bool {
    let index = LineIndex::new(code);
    errors.iter().all(|err| {
        let (line_no, _) = index.byte_offset_to_utf16_position(code, err.span.start as usize);
        let line = index.line_text(code, line_no as usize);
        line.trim_start().starts_with('&')
    })
}

fn build_ir_from_parsed_profiled_with_checkpoint(
    parsed: Arc<bsl_syntax::ast::ParseResult>,
    source: &str,
    file_path: &str,
    deps_data: Arc<SemanticDeps>,
    cancellation_checkpoint: Option<&dyn Fn()>,
) -> IrProfiledResult {
    #[inline(always)]
    fn checkpoint(cancellation_checkpoint: Option<&dyn Fn()>) {
        if let Some(checkpoint) = cancellation_checkpoint {
            checkpoint();
        }
    }

    fn maybe_inject_ir_build_delay_for_test() {
        if let Some(delay_ms) = std::env::var("BSL_TEST_ANALYSIS_IR_BUILD_DELAY_MS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .filter(|value| *value > 0)
        {
            std::thread::sleep(std::time::Duration::from_millis(delay_ms));
        }
    }

    let started = Instant::now();
    checkpoint(cancellation_checkpoint);
    maybe_inject_ir_build_delay_for_test();
    let convert_started = Instant::now();
    tracing::debug!(
        target: "bsl_backend::analysis_v2",
        file_path,
        source_len = source.len(),
        "ir_build: ast_to_ir start"
    );
    match AstToIrConverter::convert_with_resolver_and_checkpoint(
        parsed.program.clone(),
        source.to_string(),
        file_path.to_string(),
        deps_data.repository.clone(),
        deps_data.signature_index.clone(),
        deps_data.resolver.clone(),
        cancellation_checkpoint,
    ) {
        Ok(mut program) => {
            let ast_to_ir_convert_ms = convert_started.elapsed().as_millis();
            let materialize_started = Instant::now();
            tracing::debug!(
                target: "bsl_backend::analysis_v2",
                file_path,
                ast_to_ir_convert_ms,
                "ir_build: ast_to_ir finished"
            );
            tracing::debug!(
                target: "bsl_backend::analysis_v2",
                file_path,
                "ir_build: semantic_facts start"
            );
            checkpoint(cancellation_checkpoint);
            let profile = if let Some(cancellation_checkpoint) = cancellation_checkpoint {
                type_inference_v2::materialize_semantic_facts_with_path_profiled_and_checkpoint(
                    &mut program,
                    &parsed.program,
                    source,
                    file_path,
                    deps_data,
                    cancellation_checkpoint,
                )
            } else {
                type_inference_v2::materialize_semantic_facts_with_path_profiled(
                    &mut program,
                    &parsed.program,
                    source,
                    file_path,
                    deps_data,
                )
            };
            let semantic_facts_materialize_ms = materialize_started.elapsed().as_millis();
            let total_ms = started.elapsed().as_millis();
            tracing::debug!(
                target: "bsl_backend::analysis_v2",
                file_path,
                ast_to_ir_convert_ms,
                semantic_facts_materialize_ms,
                semantic_facts_seed_module_context_ms = profile.seed_module_context_ms,
                semantic_facts_local_function_summaries_ms = profile.local_function_summaries_ms,
                semantic_facts_visit_statements_ms = profile.visit_statements_ms,
                semantic_facts_visit_callable_body_ms = profile.visit_callable_body_ms,
                semantic_facts_visit_callable_body_count = profile.visit_callable_body_count,
                semantic_facts_merge_control_flow_env_ms = profile.merge_control_flow_env_ms,
                semantic_facts_merge_control_flow_env_count = profile.merge_control_flow_env_count,
                semantic_facts_statement_count = profile.statement_count,
                semantic_facts_local_function_summary_count = profile.local_function_summary_count,
                semantic_facts_index_entry_count = profile.index_entry_count,
                total_ms,
                "ir_build: semantic_facts finished"
            );
            IrProfiledResult {
                program: Arc::new(program),
                profile: IrBuildProfile {
                    ast_to_ir_convert_ms,
                    semantic_facts_materialize_ms,
                    semantic_facts_seed_module_context_ms: profile.seed_module_context_ms,
                    semantic_facts_local_function_summaries_ms: profile.local_function_summaries_ms,
                    semantic_facts_visit_statements_ms: profile.visit_statements_ms,
                    semantic_facts_visit_callable_body_ms: profile.visit_callable_body_ms,
                    semantic_facts_visit_callable_body_count: profile.visit_callable_body_count,
                    semantic_facts_merge_control_flow_env_ms: profile.merge_control_flow_env_ms,
                    semantic_facts_merge_control_flow_env_count: profile
                        .merge_control_flow_env_count,
                    semantic_facts_statement_count: profile.statement_count,
                    semantic_facts_local_function_summary_count: profile.local_function_summary_count,
                    semantic_facts_index_entry_count: profile.index_entry_count,
                    total_ms,
                },
                source: None,
            }
        }
        Err(_err) => {
            let mut program = SemanticProgram::new();
            program.source_info.path = file_path.to_string();
            program.source_info.content_hash = hash_content(source);
            IrProfiledResult {
                program: Arc::new(program),
                profile: IrBuildProfile {
                    ast_to_ir_convert_ms: convert_started.elapsed().as_millis(),
                    semantic_facts_materialize_ms: 0,
                    total_ms: started.elapsed().as_millis(),
                    ..IrBuildProfile::default()
                },
                source: None,
            }
        }
    }
}

fn collect_semantic_diagnostics_from_program(
    program: Arc<SemanticProgram>,
    deps_data: Arc<SemanticDeps>,
    detail_level: DetailLevel,
) -> Vec<TypeDiagnostic> {
    let type_hints = semantic_type_hints_from_program(&program);

    let resolver = deps_data
        .resolver
        .clone()
        .unwrap_or_else(|| Arc::new(TypeResolver::new(deps_data.repository.clone())));
    let metadata_lookup = TypeMetadataLookup::new(deps_data.repository.clone());
    let validator = TypeValidator::new(&metadata_lookup);

    let mut visitor = SemanticValidationVisitor::with_detail_level(
        &validator,
        &program,
        resolver.as_ref(),
        &deps_data.signature_index,
        detail_level,
    );
    visitor.set_platform_signatures_loaded(deps_data.platform_signatures_loaded);
    visitor.set_type_hints(Some(&type_hints));
    walk_program(&program, &mut visitor);

    let mut diagnostics = visitor.into_errors();
    diagnostics.sort_by(|a, b| {
        let severity_key = |severity: DiagnosticSeverity| match severity {
            DiagnosticSeverity::Error => 0_u8,
            DiagnosticSeverity::Warning => 1_u8,
            DiagnosticSeverity::Info => 2_u8,
            DiagnosticSeverity::Hint => 3_u8,
        };
        (
            a.span.start,
            a.span.end,
            severity_key(a.severity),
            &a.message,
        )
            .cmp(&(
                b.span.start,
                b.span.end,
                severity_key(b.severity),
                &b.message,
            ))
    });
    diagnostics.dedup_by(|left, right| {
        left.span.start == right.span.start
            && left.span.end == right.span.end
            && left.severity == right.severity
            && left.message == right.message
    });
    diagnostics
}
