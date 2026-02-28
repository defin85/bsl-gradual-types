use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use salsa::Setter;

pub use bsl_line_index::{byte_offset_to_utf16, utf16_to_byte_offset, LineIndex};

pub mod ast_to_ir;
pub use ast_to_ir::AstToIrConverter;

mod implicit_bindings;
mod type_inference_v2;

use bsl_diagnostics::{SemanticTypeHints, SemanticValidationVisitor};
use bsl_shared::analysis::{detect_type_guards, NarrowingEngine};
use bsl_shared::domain::repository::{InMemoryTypeRepository, TypeRepository};
use bsl_shared::domain::resolver::TypeResolver;
use bsl_shared::domain::signature_index::SignatureIndex;
use bsl_shared::domain::types::TypeResolution;
use bsl_shared::domain::types::{DiagnosticSeverity, ParseError, TypeDiagnostic};
use bsl_shared::domain::validators::TypeValidator;
use bsl_shared::domain::TypeMetadataLookup;
use bsl_shared::domain::{FlowAnalysisContext, NullSafetyAnalyzer};
use bsl_shared::formatting::DetailLevel;
use bsl_shared::ir::walk_program;
use bsl_shared::ir::SemanticProgram;
use bsl_shared::ir::{CfgNodeKind, NodeAtByteOffsetBias};
use bsl_shared::utils::hash::hash_content;
use bsl_syntax::ParseOptions;
use tree_sitter::Tree;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FileId(pub u32);

pub const DEPS_SCHEMA_VERSION: &str = "deps-snapshot-v1";
pub const SETTINGS_SCHEMA_VERSION: &str = "settings-v1";

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DepsSnapshotId(String);

impl DepsSnapshotId {
    pub fn from_hash(hash: impl Into<String>) -> Self {
        Self(hash.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for DepsSnapshotId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SettingsId(String);

impl SettingsId {
    pub fn from_hash(hash: impl Into<String>) -> Self {
        Self(hash.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SettingsId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

pub struct SemanticDeps {
    pub repository: Arc<dyn TypeRepository>,
    pub signature_index: SignatureIndex,
    pub resolver: Option<Arc<TypeResolver>>,
    /// Явный флаг: платформа (Syntax Helper) загружена и SignatureIndex считается полным
    /// для целей диагностики "Неопределенная процедура или функция".
    pub platform_signatures_loaded: bool,
}

impl std::fmt::Debug for SemanticDeps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SemanticDeps")
            .field("has_resolver", &self.resolver.is_some())
            .field(
                "platform_signatures_loaded",
                &self.platform_signatures_loaded,
            )
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone)]
pub struct DepsDataSnapshot(pub Arc<SemanticDeps>);

impl PartialEq for DepsDataSnapshot {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for DepsDataSnapshot {}

unsafe impl salsa::Update for DepsDataSnapshot {
    unsafe fn maybe_update(old_pointer: *mut Self, new_value: Self) -> bool {
        let old_value: &mut Self = unsafe { &mut *old_pointer };
        *old_value = new_value;
        true
    }
}

#[derive(Debug, Clone)]
pub enum Change {
    SetFile {
        file_id: FileId,
        text: Arc<str>,
        version: i32,
        path: Arc<str>,
    },
    SetFileWithSnapshot {
        file_id: FileId,
        text: Arc<str>,
        version: i32,
        path: Arc<str>,
        parse_snapshot: ParseSnapshot,
    },
    RemoveFile {
        file_id: FileId,
    },
    SetDepsSnapshot {
        deps_id: DepsSnapshotId,
        deps: Arc<SemanticDeps>,
    },
    SetSettingsSnapshot {
        settings_id: SettingsId,
        diagnostics_detail_level: DetailLevel,
    },
}

pub type Cancellable<T> = Result<T, salsa::Cancelled>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TypeAtByteOffsetProfile {
    pub index_fetch_ms: u128,
    pub index_fetch_wait_ms: u128,
    pub index_parse_result_ms: u128,
    pub index_build_total_ms: u128,
    pub index_build_seed_module_context_ms: u128,
    pub index_build_local_function_summaries_ms: u128,
    pub index_build_visit_statements_ms: u128,
    pub index_scan_ms: u128,
    pub total_ms: u128,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeAtByteOffsetProfiledResult {
    pub resolution: Option<TypeResolution>,
    pub profile: TypeAtByteOffsetProfile,
}

fn cancellable<T>(op: impl FnOnce() -> T) -> Cancellable<T> {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(op)) {
        Ok(value) => Ok(value),
        Err(payload) => match payload.downcast::<salsa::Cancelled>() {
            Ok(cancelled) => Err(*cancelled),
            Err(payload) => std::panic::resume_unwind(payload),
        },
    }
}

#[inline(always)]
fn cancellation_checkpoint(db: &dyn salsa::Database) {
    db.unwind_if_revision_cancelled();
}

fn compute_index_fetch_wait_ms(
    index_fetch_ms: u128,
    index_parse_result_ms: u128,
    index_build_total_ms: u128,
) -> u128 {
    index_fetch_ms.saturating_sub(index_parse_result_ms.saturating_add(index_build_total_ms))
}

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

const DERIVED_CACHE_KEEP_VERSIONS: i32 = 2;

#[derive(Clone, Default)]
struct DerivedVersionArtifacts {
    ir_by_deps_id: HashMap<DepsSnapshotId, Arc<SemanticProgram>>,
}

#[derive(Clone, Default)]
struct DerivedArtifactsCache {
    by_file: HashMap<FileId, HashMap<i32, DerivedVersionArtifacts>>,
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
}

impl TypeIndexSnapshot {
    fn new(
        index: Arc<type_inference_v2::TypeIndex>,
        parse_result_ms: u128,
        build_profile: type_inference_v2::TypeIndexBuildProfile,
    ) -> Self {
        Self {
            index,
            parse_result_ms,
            build_profile,
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

    SemanticProgramSnapshot(build_ir_from_parsed(parsed, &source, &file_path, deps_data))
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
    let type_index = type_index(db, file, deps, settings).index();
    cancellation_checkpoint(db);
    let detail_level = settings.diagnostics_detail_level(db);
    let diagnostics = collect_semantic_diagnostics_from_program(
        parsed,
        program,
        type_index,
        deps_data,
        detail_level,
    );
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

    // Если base пустой из-за синтаксических ошибок, всё равно не пытаемся добавлять flow-sensitive.
    if base.is_empty() {
        return SemanticDiagnosticsSnapshot(base);
    }

    let deps_data = deps.data(db).0.clone();
    let program = ir(db, file, deps, settings).0;
    let type_index = type_index(db, file, deps, settings).index();
    cancellation_checkpoint(db);
    let resolver = deps_data
        .resolver
        .clone()
        .unwrap_or_else(|| Arc::new(TypeResolver::new(deps_data.repository.clone())));

    let mut diagnostics = (*base).clone();
    cancellation_checkpoint(db);
    diagnostics.extend(flow_sensitive_null_safety_diagnostics(
        &program,
        &type_index,
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
    type_index: &type_inference_v2::TypeIndex,
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
        let Some(resolution) = type_index_resolution_for_span(type_index, *value_span) else {
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

fn populate_assignment_value_hints(
    program: &SemanticProgram,
    type_index: &type_inference_v2::TypeIndex,
    out: &mut SemanticTypeHints,
) {
    use bsl_shared::ir::SemanticNodeKind;

    for node in &program.nodes {
        let SemanticNodeKind::Assignment { value_span, .. } = &node.kind else {
            continue;
        };
        if let Some(resolution) = type_index_resolution_for_span(type_index, *value_span) {
            out.assignment_value_type_by_span
                .insert(node.span, resolution);
        }
    }
}

fn populate_call_and_member_hints(
    program: &bsl_syntax::ast::Program,
    type_index: &type_inference_v2::TypeIndex,
    out: &mut SemanticTypeHints,
) {
    use bsl_syntax::ast::{Expression, Statement};

    fn visit_statement(
        stmt: &Statement,
        type_index: &type_inference_v2::TypeIndex,
        out: &mut SemanticTypeHints,
    ) {
        match stmt {
            Statement::VarDeclaration { .. } => {}
            Statement::Assignment { target, value, .. } => {
                visit_expression(target, type_index, out);
                visit_expression(value, type_index, out);
            }
            Statement::If {
                condition,
                then_body,
                else_body,
                ..
            } => {
                visit_expression(condition, type_index, out);
                for stmt in then_body {
                    visit_statement(stmt, type_index, out);
                }
                if let Some(else_body) = else_body {
                    for stmt in else_body {
                        visit_statement(stmt, type_index, out);
                    }
                }
            }
            Statement::While {
                condition, body, ..
            } => {
                visit_expression(condition, type_index, out);
                for stmt in body {
                    visit_statement(stmt, type_index, out);
                }
            }
            Statement::For {
                start, end, body, ..
            } => {
                visit_expression(start, type_index, out);
                visit_expression(end, type_index, out);
                for stmt in body {
                    visit_statement(stmt, type_index, out);
                }
            }
            Statement::ForEach {
                collection, body, ..
            } => {
                visit_expression(collection, type_index, out);
                for stmt in body {
                    visit_statement(stmt, type_index, out);
                }
            }
            Statement::Return {
                value: Some(value), ..
            } => {
                visit_expression(value, type_index, out);
            }
            Statement::Return { value: None, .. } => {}
            Statement::Try {
                try_body,
                except_body,
                ..
            } => {
                for stmt in try_body {
                    visit_statement(stmt, type_index, out);
                }
                for stmt in except_body {
                    visit_statement(stmt, type_index, out);
                }
            }
            Statement::Call { expression, .. } => {
                visit_expression(expression, type_index, out);
            }
            Statement::Execute { code, .. } => {
                visit_expression(code, type_index, out);
            }
            Statement::RaiseError {
                message: Some(message),
                ..
            } => {
                visit_expression(message, type_index, out);
            }
            Statement::RaiseError { message: None, .. } => {}
            Statement::AddHandler { event, handler, .. }
            | Statement::RemoveHandler { event, handler, .. } => {
                visit_expression(event, type_index, out);
                visit_expression(handler, type_index, out);
            }
            Statement::Await { expression, .. } => {
                visit_expression(expression, type_index, out);
            }
            Statement::FunctionDecl { body, .. } | Statement::ProcedureDecl { body, .. } => {
                for stmt in body {
                    visit_statement(stmt, type_index, out);
                }
            }
            _ => {}
        }
    }

    fn visit_expression(
        expr: &Expression,
        type_index: &type_inference_v2::TypeIndex,
        out: &mut SemanticTypeHints,
    ) {
        match expr {
            Expression::Call {
                function,
                args,
                span,
            } => {
                let key_span = call_ir_span(function, *span);

                let arg_types: Vec<TypeResolution> = args
                    .iter()
                    .filter_map(|arg| {
                        type_index_resolution_for_span(type_index, expression_span(arg))
                    })
                    .collect();
                out.call_arg_types_by_span.insert(key_span, arg_types);

                if let Expression::PropertyAccess { object, .. } = function.as_ref() {
                    if let Some(receiver_type) =
                        type_index_resolution_for_span(type_index, expression_span(object))
                    {
                        out.call_receiver_type_by_span
                            .insert(key_span, receiver_type);
                    }
                }

                visit_expression(function, type_index, out);
                for arg in args {
                    visit_expression(arg, type_index, out);
                }
            }
            Expression::PropertyAccess { object, span, .. } => {
                if let Some(receiver_type) =
                    type_index_resolution_for_span(type_index, expression_span(object))
                {
                    out.member_access_object_type_by_span
                        .insert(*span, receiver_type);
                }
                visit_expression(object, type_index, out);
            }
            Expression::New { args, .. } => {
                for arg in args {
                    visit_expression(arg, type_index, out);
                }
            }
            Expression::Binary { left, right, .. } => {
                visit_expression(left, type_index, out);
                visit_expression(right, type_index, out);
            }
            Expression::Unary { operand, .. } => {
                visit_expression(operand, type_index, out);
            }
            Expression::Ternary {
                condition,
                then_expr,
                else_expr,
                ..
            } => {
                visit_expression(condition, type_index, out);
                visit_expression(then_expr, type_index, out);
                visit_expression(else_expr, type_index, out);
            }
            Expression::IndexAccess { object, index, .. } => {
                visit_expression(object, type_index, out);
                visit_expression(index, type_index, out);
            }
            Expression::Await { expression, .. } => {
                visit_expression(expression, type_index, out);
            }
            _ => {}
        }
    }

    fn call_ir_span(function: &Expression, span: bsl_shared::ir::Span) -> bsl_shared::ir::Span {
        match function {
            Expression::PropertyAccess { object, .. } => match object.as_ref() {
                Expression::Identifier { span: obj_span, .. } => {
                    bsl_shared::ir::Span::new(obj_span.start, span.end)
                }
                _ => span,
            },
            _ => span,
        }
    }

    fn expression_span(expr: &Expression) -> bsl_shared::ir::Span {
        match expr {
            Expression::Identifier { span, .. }
            | Expression::String { span, .. }
            | Expression::Number { span, .. }
            | Expression::Boolean { span, .. }
            | Expression::Date { span, .. }
            | Expression::Call { span, .. }
            | Expression::Binary { span, .. }
            | Expression::Unary { span, .. }
            | Expression::Ternary { span, .. }
            | Expression::New { span, .. }
            | Expression::PropertyAccess { span, .. }
            | Expression::IndexAccess { span, .. }
            | Expression::Await { span, .. } => *span,
        }
    }

    for stmt in &program.statements {
        visit_statement(stmt, type_index, out);
    }
}

fn type_index_resolution_for_span(
    type_index: &type_inference_v2::TypeIndex,
    span: bsl_shared::ir::Span,
) -> Option<TypeResolution> {
    if let Some(exact) = type_index.type_for_exact_span(span) {
        return Some(exact);
    }
    if span.start == span.end {
        return type_index.type_at_byte_offset(span.start);
    }
    let end_inclusive = span.end.saturating_sub(1);
    type_index
        .type_at_byte_offset(end_inclusive)
        .or_else(|| type_index.type_at_byte_offset(span.start))
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
    let _deps_id = deps.id(db);
    let _settings_id = settings.id(db);
    let deps_data = deps.data(db).0.clone();
    let parse_result_started = Instant::now();
    let parsed = parse_result(db, file, settings).0;
    let parse_result_ms = parse_result_started.elapsed().as_millis();
    cancellation_checkpoint(db);
    let mut profiled = type_inference_v2::build_type_index_with_path_profiled(
        &parsed.program,
        file.path(db).as_ref(),
        deps_data,
    );
    let total_ms = started.elapsed().as_millis();
    if profiled.profile.total_ms < total_ms {
        profiled.profile.total_ms = total_ms;
    }
    TypeIndexSnapshot::new(Arc::new(profiled.index), parse_result_ms, profiled.profile)
}

fn syntax_errors_only_in_directives(code: &str, errors: &[ParseError]) -> bool {
    let index = LineIndex::new(code);
    errors.iter().all(|err| {
        let (line_no, _) = index.byte_offset_to_utf16_position(code, err.span.start as usize);
        let line = index.line_text(code, line_no as usize);
        line.trim_start().starts_with('&')
    })
}

fn build_ir_from_parsed(
    parsed: Arc<bsl_syntax::ast::ParseResult>,
    source: &str,
    file_path: &str,
    deps_data: Arc<SemanticDeps>,
) -> Arc<SemanticProgram> {
    match AstToIrConverter::convert_with_resolver(
        parsed.program.clone(),
        source.to_string(),
        file_path.to_string(),
        deps_data.repository.clone(),
        deps_data.signature_index.clone(),
        deps_data.resolver.clone(),
    ) {
        Ok(program) => Arc::new(program),
        Err(_err) => {
            let mut program = SemanticProgram::new();
            program.source_info.path = file_path.to_string();
            program.source_info.content_hash = hash_content(source);
            Arc::new(program)
        }
    }
}

fn collect_semantic_diagnostics_from_program(
    parsed: Arc<bsl_syntax::ast::ParseResult>,
    program: Arc<SemanticProgram>,
    type_index: Arc<type_inference_v2::TypeIndex>,
    deps_data: Arc<SemanticDeps>,
    detail_level: DetailLevel,
) -> Vec<TypeDiagnostic> {
    let mut type_hints = SemanticTypeHints::default();
    populate_assignment_value_hints(&program, &type_index, &mut type_hints);
    populate_call_and_member_hints(&parsed.program, &type_index, &mut type_hints);

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
    diagnostics
}

#[salsa::db]
#[derive(Clone, Default)]
pub struct AnalysisDatabase {
    storage: salsa::Storage<Self>,
}

#[salsa::db]
impl salsa::Database for AnalysisDatabase {}

pub struct AnalysisHostV2 {
    db: AnalysisDatabase,
    files: HashMap<FileId, SourceFile>,
    parse_snapshots: HashMap<FileId, ParseSnapshot>,
    derived_cache: Arc<std::sync::Mutex<DerivedArtifactsCache>>,
    deps: DepsSnapshot,
    settings: SettingsSnapshot,
}

impl Default for AnalysisHostV2 {
    fn default() -> Self {
        let db = AnalysisDatabase::default();
        let repository = Arc::new(InMemoryTypeRepository::new()) as Arc<dyn TypeRepository>;
        let platform_signatures_loaded = repository.platform_docs_loaded();
        let deps_data = Arc::new(SemanticDeps {
            signature_index: repository.get_signature_index_clone(),
            resolver: Some(Arc::new(TypeResolver::new(repository.clone()))),
            repository,
            platform_signatures_loaded,
        });
        let deps = DepsSnapshot::new(
            &db,
            DepsSnapshotId::from_hash(""),
            DepsDataSnapshot(deps_data),
        );
        let settings = SettingsSnapshot::new(&db, SettingsId::from_hash(""), DetailLevel::Full);
        Self {
            db,
            files: HashMap::new(),
            parse_snapshots: HashMap::new(),
            derived_cache: Arc::new(std::sync::Mutex::new(DerivedArtifactsCache::default())),
            deps,
            settings,
        }
    }
}

impl AnalysisHostV2 {
    pub fn apply_change(&mut self, change: Change) {
        match change {
            Change::SetFile {
                file_id,
                text,
                version,
                path,
            } => self.set_file(file_id, text, version, path),
            Change::SetFileWithSnapshot {
                file_id,
                text,
                version,
                path,
                parse_snapshot,
            } => self.set_file_with_snapshot(file_id, text, version, path, parse_snapshot),
            Change::RemoveFile { file_id } => {
                self.files.remove(&file_id);
                self.parse_snapshots.remove(&file_id);
                self.derived_cache
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .by_file
                    .remove(&file_id);
            }
            Change::SetDepsSnapshot { deps_id, deps } => {
                self.deps.set_id(&mut self.db).to(deps_id);
                self.deps.set_data(&mut self.db).to(DepsDataSnapshot(deps));
            }
            Change::SetSettingsSnapshot {
                settings_id,
                diagnostics_detail_level,
            } => {
                self.settings.set_id(&mut self.db).to(settings_id);
                self.settings
                    .set_diagnostics_detail_level(&mut self.db)
                    .to(diagnostics_detail_level);
            }
        }
    }

    pub fn set_file(&mut self, file_id: FileId, text: Arc<str>, version: i32, path: Arc<str>) {
        match self.files.get(&file_id).copied() {
            Some(file) => {
                file.set_text(&mut self.db).to(text);
                file.set_version(&mut self.db).to(version);
            }
            None => {
                let file = SourceFile::new(&self.db, file_id.0, text, version, path);
                self.files.insert(file_id, file);
            }
        }
        self.parse_snapshots.remove(&file_id);
        let min_version_to_keep = version.saturating_sub(DERIVED_CACHE_KEEP_VERSIONS);
        let mut cache = self
            .derived_cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut remove_file_entry = false;
        if let Some(versioned) = cache.by_file.get_mut(&file_id) {
            versioned.retain(|cached_version, _| *cached_version >= min_version_to_keep);
            remove_file_entry = versioned.is_empty();
        }
        if remove_file_entry {
            cache.by_file.remove(&file_id);
        }
    }

    pub fn set_file_with_snapshot(
        &mut self,
        file_id: FileId,
        text: Arc<str>,
        version: i32,
        path: Arc<str>,
        parse_snapshot: ParseSnapshot,
    ) {
        self.set_file(file_id, text, version, path);
        if parse_snapshot.file_id != file_id || parse_snapshot.file_version != version {
            return;
        }
        self.parse_snapshots.insert(file_id, parse_snapshot);
    }

    pub fn has_file(&self, file_id: FileId) -> bool {
        self.files.contains_key(&file_id)
    }

    pub fn deps_id(&self) -> DepsSnapshotId {
        self.deps.id(&self.db).clone()
    }

    pub fn settings_id(&self) -> SettingsId {
        self.settings.id(&self.db).clone()
    }

    pub fn snapshot(&self) -> AnalysisV2 {
        AnalysisV2 {
            db: self.db.clone(),
            files: self.files.clone(),
            parse_snapshots: self.parse_snapshots.clone(),
            derived_cache: self.derived_cache.clone(),
            deps: self.deps,
            settings: self.settings,
        }
    }

    pub fn analysis(&self) -> AnalysisV2 {
        self.snapshot()
    }
}

pub struct AnalysisV2 {
    db: AnalysisDatabase,
    files: HashMap<FileId, SourceFile>,
    parse_snapshots: HashMap<FileId, ParseSnapshot>,
    derived_cache: Arc<std::sync::Mutex<DerivedArtifactsCache>>,
    deps: DepsSnapshot,
    settings: SettingsSnapshot,
}

impl AnalysisV2 {
    fn parse_snapshot_for_file(&self, file_id: FileId, file: SourceFile) -> Option<&ParseSnapshot> {
        let snapshot = self.parse_snapshots.get(&file_id)?;
        let file_version = file.version(&self.db);
        (snapshot.file_version == file_version).then_some(snapshot)
    }

    fn parse_snapshot_can_reuse_previous_ir(snapshot: &ParseSnapshot, current_text: &str) -> bool {
        if !snapshot.incremental || snapshot.fallback_reason.is_some() {
            return false;
        }
        if snapshot.changed_ranges.is_empty() {
            return true;
        }
        if snapshot.changed_ranges.len() != 1 {
            return false;
        }
        let range = &snapshot.changed_ranges[0];
        if range.start_byte != range.old_end_byte {
            return false;
        }
        let start = range.start_byte as usize;
        let new_end = range.new_end_byte as usize;
        if new_end != current_text.len() || start > new_end {
            return false;
        }
        let Ok(inserted) = std::str::from_utf8(&current_text.as_bytes()[start..new_end]) else {
            return false;
        };
        !inserted.is_empty() && inserted.chars().all(char::is_whitespace)
    }

    fn try_reuse_ir_from_previous_version(
        &self,
        file_id: FileId,
        file_version: i32,
        snapshot: &ParseSnapshot,
        current_text: &str,
        deps_id: &DepsSnapshotId,
    ) -> Option<Arc<SemanticProgram>> {
        if file_version <= 0 || !Self::parse_snapshot_can_reuse_previous_ir(snapshot, current_text)
        {
            return None;
        }
        let previous_version = file_version.saturating_sub(1);
        let cache = self
            .derived_cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        cache
            .by_file
            .get(&file_id)?
            .get(&previous_version)?
            .ir_by_deps_id
            .get(deps_id)
            .cloned()
    }

    fn remember_ir_artifact(
        &self,
        file_id: FileId,
        file_version: i32,
        deps_id: DepsSnapshotId,
        program: Arc<SemanticProgram>,
    ) {
        let min_version_to_keep = file_version.saturating_sub(DERIVED_CACHE_KEEP_VERSIONS);
        let mut cache = self
            .derived_cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let file_cache = cache.by_file.entry(file_id).or_default();
        file_cache.retain(|version, _| *version >= min_version_to_keep);
        file_cache
            .entry(file_version)
            .or_default()
            .ir_by_deps_id
            .insert(deps_id, program);
    }

    pub fn file_text(&self, file_id: FileId) -> Cancellable<Option<Arc<str>>> {
        let Some(&file) = self.files.get(&file_id) else {
            return Ok(None);
        };
        cancellable(|| file.text(&self.db).clone()).map(Some)
    }

    pub fn file_version(&self, file_id: FileId) -> Cancellable<Option<i32>> {
        let Some(&file) = self.files.get(&file_id) else {
            return Ok(None);
        };
        cancellable(|| file.version(&self.db)).map(Some)
    }

    pub fn file_path(&self, file_id: FileId) -> Cancellable<Option<Arc<str>>> {
        let Some(&file) = self.files.get(&file_id) else {
            return Ok(None);
        };
        cancellable(|| file.path(&self.db).clone()).map(Some)
    }

    pub fn file_text_len(&self, file_id: FileId) -> Cancellable<Option<usize>> {
        let Some(&file) = self.files.get(&file_id) else {
            return Ok(None);
        };
        cancellable(|| file_text_len(&self.db, file)).map(Some)
    }

    pub fn line_index(&self, file_id: FileId) -> Cancellable<Option<Arc<LineIndex>>> {
        let Some(&file) = self.files.get(&file_id) else {
            return Ok(None);
        };
        if let Some(snapshot) = self.parse_snapshot_for_file(file_id, file) {
            return Ok(Some(snapshot.line_index.clone()));
        }
        cancellable(|| line_index(&self.db, file)).map(Some)
    }

    pub fn parse_result(
        &self,
        file_id: FileId,
    ) -> Cancellable<Option<Arc<bsl_syntax::ast::ParseResult>>> {
        let Some(&file) = self.files.get(&file_id) else {
            return Ok(None);
        };
        if let Some(snapshot) = self.parse_snapshot_for_file(file_id, file) {
            return Ok(Some(snapshot.parse_result.clone()));
        }
        cancellable(|| parse_result(&self.db, file, self.settings).0).map(Some)
    }

    pub fn ir(&self, file_id: FileId) -> Cancellable<Option<Arc<SemanticProgram>>> {
        let Some(&file) = self.files.get(&file_id) else {
            return Ok(None);
        };
        let file_version = file.version(&self.db);
        let deps_id = self.deps.id(&self.db).clone();
        if let Some(snapshot) = self.parse_snapshot_for_file(file_id, file) {
            let source = file.text(&self.db);
            if let Some(reused) = self.try_reuse_ir_from_previous_version(
                file_id,
                file_version,
                snapshot,
                source.as_ref(),
                &deps_id,
            ) {
                self.remember_ir_artifact(file_id, file_version, deps_id, reused.clone());
                return Ok(Some(reused));
            }
            let deps_data = self.deps.data(&self.db).0.clone();
            let file_path = file.path(&self.db);
            let program = build_ir_from_parsed(
                snapshot.parse_result.clone(),
                source.as_ref(),
                file_path.as_ref(),
                deps_data,
            );
            self.remember_ir_artifact(file_id, file_version, deps_id, program.clone());
            return Ok(Some(program));
        }
        let program = cancellable(|| ir(&self.db, file, self.deps, self.settings).0)?;
        self.remember_ir_artifact(file_id, file_version, deps_id, program.clone());
        Ok(Some(program))
    }

    pub fn syntax_diagnostics(&self, file_id: FileId) -> Cancellable<Option<Arc<Vec<ParseError>>>> {
        let Some(&file) = self.files.get(&file_id) else {
            return Ok(None);
        };
        if let Some(snapshot) = self.parse_snapshot_for_file(file_id, file) {
            return Ok(Some(Arc::new(snapshot.parse_result.syntax_errors.clone())));
        }
        cancellable(|| syntax_diagnostics(&self.db, file, self.settings).0).map(Some)
    }

    pub fn semantic_diagnostics(
        &self,
        file_id: FileId,
    ) -> Cancellable<Option<Arc<Vec<TypeDiagnostic>>>> {
        let Some(&file) = self.files.get(&file_id) else {
            return Ok(None);
        };
        if let Some(snapshot) = self.parse_snapshot_for_file(file_id, file) {
            let parsed = snapshot.parse_result.clone();
            let source = file.text(&self.db).clone();
            if !parsed.syntax_errors.is_empty()
                && !syntax_errors_only_in_directives(source.as_ref(), &parsed.syntax_errors)
            {
                return Ok(Some(Arc::new(Vec::new())));
            }
            let file_path = file.path(&self.db).clone();
            let deps_data = self.deps.data(&self.db).0.clone();
            let program = build_ir_from_parsed(
                parsed.clone(),
                source.as_ref(),
                file_path.as_ref(),
                deps_data.clone(),
            );
            let type_index = Arc::new(type_inference_v2::build_type_index_with_path(
                &parsed.program,
                file_path.as_ref(),
                deps_data.clone(),
            ));
            let detail_level = self.settings.diagnostics_detail_level(&self.db);
            let diagnostics = collect_semantic_diagnostics_from_program(
                parsed,
                program,
                type_index,
                deps_data,
                detail_level,
            );
            return Ok(Some(Arc::new(diagnostics)));
        }
        cancellable(|| semantic_diagnostics(&self.db, file, self.deps, self.settings).0).map(Some)
    }

    pub fn semantic_diagnostics_flow_sensitive(
        &self,
        file_id: FileId,
    ) -> Cancellable<Option<Arc<Vec<TypeDiagnostic>>>> {
        let Some(&file) = self.files.get(&file_id) else {
            return Ok(None);
        };
        if let Some(snapshot) = self.parse_snapshot_for_file(file_id, file) {
            let base = self
                .semantic_diagnostics(file_id)?
                .unwrap_or_else(|| Arc::new(Vec::new()));
            if base.is_empty() {
                return Ok(Some(base));
            }
            let parsed = snapshot.parse_result.clone();
            let source = file.text(&self.db).clone();
            let file_path = file.path(&self.db).clone();
            let deps_data = self.deps.data(&self.db).0.clone();
            let program = build_ir_from_parsed(
                parsed.clone(),
                source.as_ref(),
                file_path.as_ref(),
                deps_data.clone(),
            );
            let type_index = Arc::new(type_inference_v2::build_type_index_with_path(
                &parsed.program,
                file_path.as_ref(),
                deps_data.clone(),
            ));
            let resolver = deps_data
                .resolver
                .clone()
                .unwrap_or_else(|| Arc::new(TypeResolver::new(deps_data.repository.clone())));
            let mut diagnostics = (*base).clone();
            diagnostics.extend(flow_sensitive_null_safety_diagnostics(
                &program,
                &type_index,
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
            return Ok(Some(Arc::new(diagnostics)));
        }
        cancellable(|| {
            semantic_diagnostics_flow_sensitive(&self.db, file, self.deps, self.settings).0
        })
        .map(Some)
    }

    pub fn utf16_position_to_byte_offset(
        &self,
        file_id: FileId,
        line: u32,
        character: u32,
    ) -> Cancellable<Option<usize>> {
        let Some(&file) = self.files.get(&file_id) else {
            return Ok(None);
        };
        cancellable(|| {
            let text = file.text(&self.db);
            let index = line_index(&self.db, file);
            index.utf16_position_to_byte_offset(text, line, character)
        })
        .map(Some)
    }

    pub fn utf16_position_to_point(
        &self,
        file_id: FileId,
        line: u32,
        character: u32,
    ) -> Cancellable<Option<(usize, usize)>> {
        let Some(&file) = self.files.get(&file_id) else {
            return Ok(None);
        };
        cancellable(|| {
            let text = file.text(&self.db);
            let index = line_index(&self.db, file);
            index.utf16_position_to_point(text, line, character)
        })
        .map(Some)
    }

    pub fn deps_id(&self) -> Cancellable<DepsSnapshotId> {
        cancellable(|| self.deps.id(&self.db).clone())
    }

    pub fn deps_data(&self) -> Cancellable<Arc<SemanticDeps>> {
        cancellable(|| self.deps.data(&self.db).0.clone())
    }

    pub fn settings_id(&self) -> Cancellable<SettingsId> {
        cancellable(|| self.settings.id(&self.db).clone())
    }

    pub fn completion(&self, _file_id: FileId, _line: u32, _character: u32) -> Cancellable<()> {
        Ok(())
    }

    pub fn hover(&self, _file_id: FileId, _line: u32, _character: u32) -> Cancellable<()> {
        Ok(())
    }

    pub fn signature_help(&self, _file_id: FileId, _line: u32, _character: u32) -> Cancellable<()> {
        Ok(())
    }

    pub fn type_at_byte_offset(
        &self,
        file_id: FileId,
        byte_offset: u32,
    ) -> Cancellable<Option<TypeResolution>> {
        self.type_at_byte_offset_profiled(file_id, byte_offset)
            .map(|profiled| profiled.resolution)
    }

    pub fn type_at_byte_offset_profiled(
        &self,
        file_id: FileId,
        byte_offset: u32,
    ) -> Cancellable<TypeAtByteOffsetProfiledResult> {
        let Some(&file) = self.files.get(&file_id) else {
            return Ok(TypeAtByteOffsetProfiledResult {
                resolution: None,
                profile: TypeAtByteOffsetProfile::default(),
            });
        };
        let started = Instant::now();
        let index_fetch_started = Instant::now();
        let index_snapshot = cancellable(|| type_index(&self.db, file, self.deps, self.settings))?;
        let index_fetch_ms = index_fetch_started.elapsed().as_millis();
        let index = index_snapshot.index();
        let index_build_profile = index_snapshot.build_profile();
        let clip_to_index_fetch = |value_ms: u128| value_ms.min(index_fetch_ms);
        let index_parse_result_ms = clip_to_index_fetch(index_snapshot.parse_result_ms());
        let index_build_total_ms = clip_to_index_fetch(index_build_profile.total_ms);
        let index_fetch_wait_ms = compute_index_fetch_wait_ms(
            index_fetch_ms,
            index_parse_result_ms,
            index_build_total_ms,
        );
        let index_build_seed_module_context_ms =
            clip_to_index_fetch(index_build_profile.seed_module_context_ms);
        let index_build_local_function_summaries_ms =
            clip_to_index_fetch(index_build_profile.local_function_summaries_ms);
        let index_build_visit_statements_ms =
            clip_to_index_fetch(index_build_profile.visit_statements_ms);

        let index_scan_started = Instant::now();
        let resolution = index.type_at_byte_offset(byte_offset);
        let index_scan_ms = index_scan_started.elapsed().as_millis();
        let total_ms = started.elapsed().as_millis();

        Ok(TypeAtByteOffsetProfiledResult {
            resolution,
            profile: TypeAtByteOffsetProfile {
                index_fetch_ms,
                index_fetch_wait_ms,
                index_parse_result_ms,
                index_build_total_ms,
                index_build_seed_module_context_ms,
                index_build_local_function_summaries_ms,
                index_build_visit_statements_ms,
                index_scan_ms,
                total_ms,
            },
        })
    }

    pub fn flow_type_at_byte_offset(
        &self,
        file_id: FileId,
        byte_offset: u32,
    ) -> Cancellable<Option<TypeResolution>> {
        let Some(&file) = self.files.get(&file_id) else {
            return Ok(None);
        };
        cancellable(|| {
            let program = ir(&self.db, file, self.deps, self.settings).0;
            let index = type_index(&self.db, file, self.deps, self.settings).index();
            let base = index
                .type_at_byte_offset(byte_offset)
                .unwrap_or_else(TypeResolution::unknown);
            flow_type_at_byte_offset_impl(program.as_ref(), byte_offset, base)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tree_sitter::Parser as TreeSitterParser;

    fn parse_backend_tree_for_test(text: &str) -> Arc<Tree> {
        let mut parser = TreeSitterParser::new();
        parser
            .set_language(&tree_sitter_bsl::LANGUAGE.into())
            .expect("tree-sitter-bsl language");
        Arc::new(
            parser
                .parse(text, None)
                .expect("tree-sitter parse for snapshot"),
        )
    }

    fn normalize_json(value: &mut serde_json::Value) {
        match value {
            serde_json::Value::Object(map) => {
                let mut entries: Vec<(String, serde_json::Value)> =
                    std::mem::take(map).into_iter().collect();
                entries.sort_by(|(a, _), (b, _)| a.cmp(b));

                let mut sorted = serde_json::Map::new();
                for (key, mut value) in entries {
                    normalize_json(&mut value);
                    sorted.insert(key, value);
                }
                *map = sorted;
            }
            serde_json::Value::Array(items) => {
                for item in items {
                    normalize_json(item);
                }
            }
            _ => {}
        }
    }

    fn parse_snapshot_for_test(
        file_id: FileId,
        file_version: i32,
        text: &str,
        changed_ranges: Vec<ParseChangedRange>,
        incremental: bool,
        fallback_reason: Option<&str>,
    ) -> ParseSnapshot {
        ParseSnapshot {
            file_id,
            file_version,
            parse_result: Arc::new(
                bsl_syntax::parse(text, &ParseOptions::default()).expect("snapshot parse"),
            ),
            line_index: Arc::new(LineIndex::new(text)),
            backend_tree: parse_backend_tree_for_test(text),
            changed_ranges: Arc::new(changed_ranges),
            produced_at_millis: 0,
            backend_tree_hash: 0,
            incremental,
            fallback_reason: fallback_reason.map(Arc::from),
        }
    }

    #[test]
    fn compute_index_fetch_wait_ms_subtracts_parse_and_build_time() {
        assert_eq!(compute_index_fetch_wait_ms(156_207, 0, 97), 156_110);
        assert_eq!(compute_index_fetch_wait_ms(10, 8, 5), 0);
    }

    #[test]
    fn file_text_and_version_update_after_set_file() {
        let mut host = AnalysisHostV2::default();
        let file_id = FileId(1);

        host.apply_change(Change::SetFile {
            file_id,
            text: Arc::from("abc"),
            version: 1,
            path: Arc::from("test.bsl"),
        });

        {
            let analysis = host.analysis();
            assert_eq!(analysis.file_text(file_id).unwrap().as_deref(), Some("abc"));
            assert_eq!(analysis.file_version(file_id).unwrap(), Some(1));
            assert_eq!(analysis.file_text_len(file_id).unwrap(), Some(3));
        }

        host.apply_change(Change::SetFile {
            file_id,
            text: Arc::from("abcd"),
            version: 2,
            path: Arc::from("test.bsl"),
        });

        {
            let analysis = host.analysis();
            assert_eq!(
                analysis.file_text(file_id).unwrap().as_deref(),
                Some("abcd")
            );
            assert_eq!(analysis.file_version(file_id).unwrap(), Some(2));
            assert_eq!(analysis.file_text_len(file_id).unwrap(), Some(4));
        }
    }

    #[test]
    fn remove_file_makes_queries_return_none() {
        let mut host = AnalysisHostV2::default();
        let file_id = FileId(1);

        host.apply_change(Change::SetFile {
            file_id,
            text: Arc::from("abc"),
            version: 1,
            path: Arc::from("test.bsl"),
        });
        host.apply_change(Change::RemoveFile { file_id });

        let analysis = host.analysis();
        assert_eq!(analysis.file_text(file_id).unwrap(), None);
        assert_eq!(analysis.file_version(file_id).unwrap(), None);
        assert_eq!(analysis.file_text_len(file_id).unwrap(), None);
    }

    #[test]
    fn deps_and_settings_ids_are_read_from_snapshot() {
        use std::sync::{Arc, Mutex};
        use std::time::Duration;

        let host = Arc::new(Mutex::new(AnalysisHostV2::default()));
        host.lock().unwrap().apply_change(Change::SetDepsSnapshot {
            deps_id: DepsSnapshotId::from_hash("deps-a"),
            deps: Arc::new(SemanticDeps {
                repository: Arc::new(InMemoryTypeRepository::new()),
                signature_index: SignatureIndex::new(),
                resolver: None,
                platform_signatures_loaded: false,
            }),
        });
        host.lock()
            .unwrap()
            .apply_change(Change::SetSettingsSnapshot {
                settings_id: SettingsId::from_hash("settings-a"),
                diagnostics_detail_level: DetailLevel::Full,
            });

        let analysis_a = host.lock().unwrap().snapshot();
        assert_eq!(analysis_a.deps_id().unwrap().as_str(), "deps-a");
        assert_eq!(analysis_a.settings_id().unwrap().as_str(), "settings-a");

        let (locked_tx, locked_rx) = std::sync::mpsc::channel::<()>();
        let (done_tx, done_rx) = std::sync::mpsc::channel::<()>();
        let host_for_update = host.clone();
        let update_thread = std::thread::spawn(move || {
            let mut host = host_for_update.lock().unwrap();
            locked_tx.send(()).unwrap();
            host.apply_change(Change::SetDepsSnapshot {
                deps_id: DepsSnapshotId::from_hash("deps-b"),
                deps: Arc::new(SemanticDeps {
                    repository: Arc::new(InMemoryTypeRepository::new()),
                    signature_index: SignatureIndex::new(),
                    resolver: None,
                    platform_signatures_loaded: false,
                }),
            });
            host.apply_change(Change::SetSettingsSnapshot {
                settings_id: SettingsId::from_hash("settings-b"),
                diagnostics_detail_level: DetailLevel::Full,
            });
            done_tx.send(()).unwrap();
        });

        locked_rx.recv_timeout(Duration::from_secs(1)).unwrap();

        assert!(done_rx.recv_timeout(Duration::from_millis(200)).is_err());
        assert_eq!(analysis_a.deps_id().unwrap().as_str(), "deps-a");
        assert_eq!(analysis_a.settings_id().unwrap().as_str(), "settings-a");

        drop(analysis_a);
        done_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        update_thread.join().unwrap();

        let analysis_b = host.lock().unwrap().snapshot();
        assert_eq!(analysis_b.deps_id().unwrap().as_str(), "deps-b");
        assert_eq!(analysis_b.settings_id().unwrap().as_str(), "settings-b");
    }

    #[test]
    fn line_index_and_positioning_are_read_from_snapshot() {
        let mut host = AnalysisHostV2::default();
        let file_id = FileId(1);

        host.apply_change(Change::SetFile {
            file_id,
            text: Arc::from("abc\ndef"),
            version: 1,
            path: Arc::from("test.bsl"),
        });

        let analysis = host.snapshot();
        let index = analysis.line_index(file_id).unwrap().unwrap();
        assert_eq!(index.line_count(), 2);

        assert_eq!(
            analysis
                .utf16_position_to_byte_offset(file_id, 0, 999)
                .unwrap(),
            Some(3)
        );
        assert_eq!(
            analysis.utf16_position_to_point(file_id, 0, 999).unwrap(),
            Some((0, 3))
        );
    }

    #[test]
    fn set_file_with_snapshot_uses_snapshot_parse_result_and_line_index() {
        let mut host = AnalysisHostV2::default();
        let file_id = FileId(7);
        let text: Arc<str> = Arc::from("Процедура Тест()\nКонецПроцедуры");
        let parsed = Arc::new(
            bsl_syntax::parse(text.as_ref(), &ParseOptions::default()).expect("snapshot parse"),
        );
        let index = Arc::new(LineIndex::new(text.as_ref()));
        let snapshot = ParseSnapshot {
            file_id,
            file_version: 3,
            parse_result: parsed.clone(),
            line_index: index.clone(),
            backend_tree: parse_backend_tree_for_test(text.as_ref()),
            changed_ranges: Arc::new(Vec::new()),
            produced_at_millis: 0,
            backend_tree_hash: 0,
            incremental: false,
            fallback_reason: None,
        };

        host.apply_change(Change::SetFileWithSnapshot {
            file_id,
            text: text.clone(),
            version: 3,
            path: Arc::from("snapshot-test.bsl"),
            parse_snapshot: snapshot,
        });

        let analysis = host.snapshot();
        let parse_result = analysis.parse_result(file_id).unwrap().unwrap();
        let line_index = analysis.line_index(file_id).unwrap().unwrap();

        assert!(Arc::ptr_eq(&parsed, &parse_result));
        assert!(Arc::ptr_eq(&index, &line_index));
    }

    #[test]
    fn set_file_with_snapshot_ignores_mismatched_snapshot_version() {
        let mut host = AnalysisHostV2::default();
        let file_id = FileId(8);
        let text: Arc<str> = Arc::from("Процедура Тест()\nКонецПроцедуры");
        let snapshot_parsed = Arc::new(
            bsl_syntax::parse(text.as_ref(), &ParseOptions::default()).expect("snapshot parse"),
        );
        let snapshot = ParseSnapshot {
            file_id,
            file_version: 99,
            parse_result: snapshot_parsed.clone(),
            line_index: Arc::new(LineIndex::new(text.as_ref())),
            backend_tree: parse_backend_tree_for_test(text.as_ref()),
            changed_ranges: Arc::new(Vec::new()),
            produced_at_millis: 0,
            backend_tree_hash: 0,
            incremental: true,
            fallback_reason: Some(Arc::from("version_mismatch")),
        };

        host.apply_change(Change::SetFileWithSnapshot {
            file_id,
            text,
            version: 1,
            path: Arc::from("snapshot-mismatch.bsl"),
            parse_snapshot: snapshot,
        });

        let analysis = host.snapshot();
        let parsed = analysis.parse_result(file_id).unwrap().unwrap();
        assert!(
            !Arc::ptr_eq(&parsed, &snapshot_parsed),
            "snapshot with mismatched version must not be used"
        );
    }

    #[test]
    fn parse_result_recomputes_when_file_text_changes() {
        let mut host = AnalysisHostV2::default();
        let file_id = FileId(1);

        host.apply_change(Change::SetFile {
            file_id,
            text: Arc::from("Procedure Test()\nEndProcedure"),
            version: 1,
            path: Arc::from("test.bsl"),
        });

        let parsed_a = {
            let analysis = host.snapshot();
            analysis.parse_result(file_id).unwrap().unwrap()
        };

        host.apply_change(Change::SetFile {
            file_id,
            text: Arc::from("Procedure Test(\nEndProcedure"),
            version: 2,
            path: Arc::from("test.bsl"),
        });

        let parsed_b = {
            let analysis = host.snapshot();
            analysis.parse_result(file_id).unwrap().unwrap()
        };

        assert!(!Arc::ptr_eq(&parsed_a, &parsed_b));
        assert!(parsed_a.syntax_errors.is_empty());
        assert!(!parsed_b.syntax_errors.is_empty());
    }

    #[test]
    fn parse_result_recomputes_when_settings_id_changes() {
        let mut host = AnalysisHostV2::default();
        let file_id = FileId(1);

        host.apply_change(Change::SetFile {
            file_id,
            text: Arc::from("Procedure Test()\nEndProcedure"),
            version: 1,
            path: Arc::from("test.bsl"),
        });

        let parsed_a = {
            let analysis = host.snapshot();
            analysis.parse_result(file_id).unwrap().unwrap()
        };

        host.apply_change(Change::SetSettingsSnapshot {
            settings_id: SettingsId::from_hash("settings-b"),
            diagnostics_detail_level: DetailLevel::Full,
        });

        let parsed_b = {
            let analysis = host.snapshot();
            analysis.parse_result(file_id).unwrap().unwrap()
        };

        assert!(!Arc::ptr_eq(&parsed_a, &parsed_b));
    }

    #[test]
    fn remove_file_makes_parse_result_return_none() {
        let mut host = AnalysisHostV2::default();
        let file_id = FileId(1);

        host.apply_change(Change::SetFile {
            file_id,
            text: Arc::from("Procedure Test()\nEndProcedure"),
            version: 1,
            path: Arc::from("test.bsl"),
        });
        host.apply_change(Change::RemoveFile { file_id });

        let analysis = host.snapshot();
        assert!(analysis.parse_result(file_id).unwrap().is_none());
    }

    #[test]
    fn ir_recomputes_when_file_text_changes() {
        let mut host = AnalysisHostV2::default();
        let file_id = FileId(1);

        host.apply_change(Change::SetFile {
            file_id,
            text: Arc::from("Procedure Test()\nEndProcedure"),
            version: 1,
            path: Arc::from("test.bsl"),
        });

        let ir_a = {
            let analysis = host.snapshot();
            analysis.ir(file_id).unwrap().unwrap()
        };

        host.apply_change(Change::SetFile {
            file_id,
            text: Arc::from("Procedure Test(\nEndProcedure"),
            version: 2,
            path: Arc::from("test.bsl"),
        });

        let ir_b = {
            let analysis = host.snapshot();
            analysis.ir(file_id).unwrap().unwrap()
        };

        assert!(!Arc::ptr_eq(&ir_a, &ir_b));
    }

    #[test]
    fn ir_reuses_previous_version_for_tail_whitespace_append_snapshot() {
        let mut host = AnalysisHostV2::default();
        let file_id = FileId(11);
        let text_v1: Arc<str> = Arc::from("Procedure Test()\n    x = 1;\nEndProcedure");

        host.apply_change(Change::SetFile {
            file_id,
            text: text_v1.clone(),
            version: 1,
            path: Arc::from("tail-whitespace.bsl"),
        });

        let ir_v1 = host.analysis().ir(file_id).unwrap().unwrap();

        let text_v2 = Arc::<str>::from(format!("{}\n", text_v1.as_ref()));
        let old_len = text_v1.len() as u32;
        host.apply_change(Change::SetFileWithSnapshot {
            file_id,
            text: text_v2.clone(),
            version: 2,
            path: Arc::from("tail-whitespace.bsl"),
            parse_snapshot: parse_snapshot_for_test(
                file_id,
                2,
                text_v2.as_ref(),
                vec![ParseChangedRange {
                    start_byte: old_len,
                    old_end_byte: old_len,
                    new_end_byte: text_v2.len() as u32,
                }],
                true,
                None,
            ),
        });

        let ir_v2 = host.analysis().ir(file_id).unwrap().unwrap();
        assert!(
            Arc::ptr_eq(&ir_v1, &ir_v2),
            "tail whitespace append must reuse previous IR"
        );
    }

    #[test]
    fn ir_does_not_reuse_previous_version_for_non_tail_snapshot_change() {
        let mut host = AnalysisHostV2::default();
        let file_id = FileId(12);
        let text_v1: Arc<str> = Arc::from("Procedure Test()\n    x = 1;\nEndProcedure");

        host.apply_change(Change::SetFile {
            file_id,
            text: text_v1.clone(),
            version: 1,
            path: Arc::from("non-tail-change.bsl"),
        });

        let ir_v1 = host.analysis().ir(file_id).unwrap().unwrap();

        let edit_start = text_v1.find('1').expect("edit marker") as u32;
        let text_v2: Arc<str> = Arc::from(text_v1.replacen('1', "2", 1));
        host.apply_change(Change::SetFileWithSnapshot {
            file_id,
            text: text_v2.clone(),
            version: 2,
            path: Arc::from("non-tail-change.bsl"),
            parse_snapshot: parse_snapshot_for_test(
                file_id,
                2,
                text_v2.as_ref(),
                vec![ParseChangedRange {
                    start_byte: edit_start,
                    old_end_byte: edit_start + 1,
                    new_end_byte: edit_start + 1,
                }],
                true,
                None,
            ),
        });

        let ir_v2 = host.analysis().ir(file_id).unwrap().unwrap();
        assert!(
            !Arc::ptr_eq(&ir_v1, &ir_v2),
            "non-tail change must trigger full IR recompute"
        );
    }

    fn build_large_burst_module(marker: u32) -> String {
        let mut text = String::from("Процедура СтрессТест()\n");
        text.push_str("    ЛокМассив = Новый Массив;\n");
        for idx in 0..800_u32 {
            text.push_str(&format!("    ЛокПер{idx} = {idx};\n"));
        }
        text.push_str(&format!("    Маркер = {marker};\n"));
        text.push_str("    ЛокМассив.НесуществующийМетод();\n");
        text.push_str("КонецПроцедуры\n");
        text
    }

    #[test]
    fn large_module_snapshot_edit_burst_preserves_semantic_diagnostics_parity() {
        let file_id = FileId(77);
        let path: Arc<str> = Arc::from("large-burst-parity.bsl");
        let mut host_snapshot = AnalysisHostV2::default();
        let mut host_full = AnalysisHostV2::default();
        let mut current_text = build_large_burst_module(0);

        host_snapshot.apply_change(Change::SetFile {
            file_id,
            text: Arc::from(current_text.clone()),
            version: 1,
            path: path.clone(),
        });
        host_full.apply_change(Change::SetFile {
            file_id,
            text: Arc::from(current_text.clone()),
            version: 1,
            path: path.clone(),
        });

        for step in 1..=16_i32 {
            let previous_marker = format!("    Маркер = {};", step - 1);
            let next_marker = format!("    Маркер = {step};");
            let start = current_text
                .find(&previous_marker)
                .expect("marker from previous step");
            let old_end = start + previous_marker.len();
            let updated_text = current_text.replacen(&previous_marker, &next_marker, 1);
            let new_end = start + next_marker.len();
            let version = step + 1;

            host_snapshot.apply_change(Change::SetFileWithSnapshot {
                file_id,
                text: Arc::from(updated_text.clone()),
                version,
                path: path.clone(),
                parse_snapshot: parse_snapshot_for_test(
                    file_id,
                    version,
                    updated_text.as_ref(),
                    vec![ParseChangedRange {
                        start_byte: start as u32,
                        old_end_byte: old_end as u32,
                        new_end_byte: new_end as u32,
                    }],
                    true,
                    None,
                ),
            });
            host_full.apply_change(Change::SetFile {
                file_id,
                text: Arc::from(updated_text.clone()),
                version,
                path: path.clone(),
            });

            let snapshot_analysis = host_snapshot.snapshot();
            let full_analysis = host_full.snapshot();

            let syntax_snapshot = snapshot_analysis
                .syntax_diagnostics(file_id)
                .unwrap()
                .unwrap();
            let syntax_full = full_analysis.syntax_diagnostics(file_id).unwrap().unwrap();
            let mut syntax_snapshot_json =
                serde_json::to_value(syntax_snapshot.as_ref()).expect("serialize snapshot syntax");
            let mut syntax_full_json =
                serde_json::to_value(syntax_full.as_ref()).expect("serialize full syntax");
            normalize_json(&mut syntax_snapshot_json);
            normalize_json(&mut syntax_full_json);
            assert_eq!(
                syntax_snapshot_json, syntax_full_json,
                "syntax diagnostics drift at burst step {step}"
            );

            let semantic_snapshot = snapshot_analysis
                .semantic_diagnostics(file_id)
                .unwrap()
                .unwrap();
            let semantic_full = full_analysis
                .semantic_diagnostics(file_id)
                .unwrap()
                .unwrap();
            let mut semantic_snapshot_json = serde_json::to_value(semantic_snapshot.as_ref())
                .expect("serialize snapshot semantic diagnostics");
            let mut semantic_full_json = serde_json::to_value(semantic_full.as_ref())
                .expect("serialize full semantic diagnostics");
            normalize_json(&mut semantic_snapshot_json);
            normalize_json(&mut semantic_full_json);
            assert_eq!(
                semantic_snapshot_json, semantic_full_json,
                "semantic diagnostics drift at burst step {step}"
            );

            current_text = updated_text;
        }
    }

    #[test]
    fn ir_recomputes_when_deps_id_changes() {
        use salsa::Setter;

        let mut host = AnalysisHostV2::default();
        let file_id = FileId(1);

        host.apply_change(Change::SetFile {
            file_id,
            text: Arc::from("Procedure Test()\nEndProcedure"),
            version: 1,
            path: Arc::from("test.bsl"),
        });

        let ir_a = {
            let analysis = host.snapshot();
            analysis.ir(file_id).unwrap().unwrap()
        };

        host.deps
            .set_id(&mut host.db)
            .to(DepsSnapshotId::from_hash("deps-b"));

        let ir_b = {
            let analysis = host.snapshot();
            analysis.ir(file_id).unwrap().unwrap()
        };

        assert!(!Arc::ptr_eq(&ir_a, &ir_b));
    }

    #[test]
    fn ir_recomputes_when_settings_id_changes() {
        let mut host = AnalysisHostV2::default();
        let file_id = FileId(1);

        host.apply_change(Change::SetFile {
            file_id,
            text: Arc::from("Procedure Test()\nEndProcedure"),
            version: 1,
            path: Arc::from("test.bsl"),
        });

        let ir_a = {
            let analysis = host.snapshot();
            analysis.ir(file_id).unwrap().unwrap()
        };

        host.apply_change(Change::SetSettingsSnapshot {
            settings_id: SettingsId::from_hash("settings-b"),
            diagnostics_detail_level: DetailLevel::Full,
        });

        let ir_b = {
            let analysis = host.snapshot();
            analysis.ir(file_id).unwrap().unwrap()
        };

        assert!(!Arc::ptr_eq(&ir_a, &ir_b));
    }

    #[test]
    fn ir_is_deterministic_for_same_input_across_hosts() {
        let file_id = FileId(1);
        let text: Arc<str> = Arc::from(
            "Procedure Test()\n\
             x = 1;\n\
             x = x + 1;\n\
             EndProcedure",
        );
        let path: Arc<str> = Arc::from("test.bsl");

        let program_a = {
            let mut host = AnalysisHostV2::default();
            host.apply_change(Change::SetFile {
                file_id,
                text: text.clone(),
                version: 1,
                path: path.clone(),
            });
            let analysis = host.snapshot();
            analysis.ir(file_id).unwrap().unwrap()
        };

        let program_b = {
            let mut host = AnalysisHostV2::default();
            host.apply_change(Change::SetFile {
                file_id,
                text: text.clone(),
                version: 1,
                path: path.clone(),
            });
            let analysis = host.snapshot();
            analysis.ir(file_id).unwrap().unwrap()
        };

        let mut json_a = serde_json::to_value(&*program_a).expect("serialize SemanticProgram");
        let mut json_b = serde_json::to_value(&*program_b).expect("serialize SemanticProgram");
        normalize_json(&mut json_a);
        normalize_json(&mut json_b);
        assert_eq!(json_a, json_b);
    }

    #[test]
    fn remove_file_makes_ir_return_none() {
        let mut host = AnalysisHostV2::default();
        let file_id = FileId(1);

        host.apply_change(Change::SetFile {
            file_id,
            text: Arc::from("Procedure Test()\nEndProcedure"),
            version: 1,
            path: Arc::from("test.bsl"),
        });
        host.apply_change(Change::RemoveFile { file_id });

        let analysis = host.snapshot();
        assert!(analysis.ir(file_id).unwrap().is_none());
    }

    #[test]
    fn syntax_diagnostics_are_read_from_parse_result() {
        let file_id = FileId(1);

        let syntax_a = {
            let mut host = AnalysisHostV2::default();
            host.apply_change(Change::SetFile {
                file_id,
                text: Arc::from("Procedure Test(\nEndProcedure"),
                version: 1,
                path: Arc::from("test.bsl"),
            });
            let analysis = host.snapshot();
            analysis.syntax_diagnostics(file_id).unwrap().unwrap()
        };

        let syntax_b = {
            let mut host = AnalysisHostV2::default();
            host.apply_change(Change::SetFile {
                file_id,
                text: Arc::from("Procedure Test(\nEndProcedure"),
                version: 1,
                path: Arc::from("test.bsl"),
            });
            let analysis = host.snapshot();
            analysis.syntax_diagnostics(file_id).unwrap().unwrap()
        };

        assert!(!syntax_a.is_empty());
        let json_a = serde_json::to_string(&*syntax_a).expect("serialize syntax diagnostics");
        let json_b = serde_json::to_string(&*syntax_b).expect("serialize syntax diagnostics");
        assert_eq!(json_a, json_b);
    }

    #[test]
    fn semantic_diagnostics_skip_when_syntax_errors_present() {
        let mut host = AnalysisHostV2::default();
        let file_id = FileId(1);

        host.apply_change(Change::SetFile {
            file_id,
            text: Arc::from("Procedure Test(\nEndProcedure"),
            version: 1,
            path: Arc::from("test.bsl"),
        });

        let analysis = host.snapshot();
        let semantic = analysis.semantic_diagnostics(file_id).unwrap().unwrap();
        assert!(semantic.is_empty());
    }

    #[test]
    fn semantic_diagnostics_depend_on_deps_id() {
        let mut host = AnalysisHostV2::default();
        let file_id = FileId(1);

        host.apply_change(Change::SetFile {
            file_id,
            text: Arc::from("Procedure Test()\nEndProcedure"),
            version: 1,
            path: Arc::from("test.bsl"),
        });

        let repository = Arc::new(InMemoryTypeRepository::new()) as Arc<dyn TypeRepository>;
        let platform_signatures_loaded = repository.platform_docs_loaded();
        let deps = Arc::new(SemanticDeps {
            signature_index: repository.get_signature_index_clone(),
            resolver: Some(Arc::new(TypeResolver::new(repository.clone()))),
            repository,
            platform_signatures_loaded,
        });

        host.apply_change(Change::SetDepsSnapshot {
            deps_id: DepsSnapshotId::from_hash("deps-a"),
            deps: deps.clone(),
        });
        let diagnostics_a = host
            .analysis()
            .semantic_diagnostics(file_id)
            .unwrap()
            .unwrap();

        host.apply_change(Change::SetDepsSnapshot {
            deps_id: DepsSnapshotId::from_hash("deps-b"),
            deps,
        });
        let diagnostics_b = host
            .analysis()
            .semantic_diagnostics(file_id)
            .unwrap()
            .unwrap();

        assert!(
            !Arc::ptr_eq(&diagnostics_a, &diagnostics_b),
            "semantic diagnostics should be recomputed when deps_id changes"
        );
    }

    #[test]
    fn semantic_diagnostics_respect_diagnostics_detail_level() {
        let mut host = AnalysisHostV2::default();
        let file_id = FileId(1);

        host.apply_change(Change::SetFile {
            file_id,
            text: Arc::from(
                "Procedure Test()\n\
                 x = 1;\n\
                 x.UnknownMethod();\n\
                 EndProcedure",
            ),
            version: 1,
            path: Arc::from("test.bsl"),
        });

        host.apply_change(Change::SetSettingsSnapshot {
            settings_id: SettingsId::from_hash("settings-compact"),
            diagnostics_detail_level: DetailLevel::Compact,
        });
        let compact = host
            .analysis()
            .semantic_diagnostics(file_id)
            .unwrap()
            .unwrap();
        assert!(!compact.is_empty());

        host.apply_change(Change::SetSettingsSnapshot {
            settings_id: SettingsId::from_hash("settings-detailed"),
            diagnostics_detail_level: DetailLevel::Detailed,
        });
        let detailed = host
            .analysis()
            .semantic_diagnostics(file_id)
            .unwrap()
            .unwrap();
        assert_eq!(compact.len(), detailed.len());
        assert_ne!(compact[0].message, detailed[0].message);
    }

    #[test]
    fn semantic_diagnostics_do_not_include_flow_sensitive_null_safety_by_default() {
        let mut host = AnalysisHostV2::default();
        let file_id = FileId(1);

        host.apply_change(Change::SetFile {
            file_id,
            text: Arc::from(
                "Procedure Test()\n\
                 x = Null;\n\
                 x.Method();\n\
                 EndProcedure",
            ),
            version: 1,
            path: Arc::from("test.bsl"),
        });

        let repository = Arc::new(InMemoryTypeRepository::new()) as Arc<dyn TypeRepository>;
        let platform_signatures_loaded = repository.platform_docs_loaded();
        let deps = Arc::new(SemanticDeps {
            signature_index: repository.get_signature_index_clone(),
            resolver: Some(Arc::new(TypeResolver::new(repository.clone()))),
            repository,
            platform_signatures_loaded,
        });

        host.apply_change(Change::SetDepsSnapshot {
            deps_id: DepsSnapshotId::from_hash("deps"),
            deps,
        });

        let base = host
            .analysis()
            .semantic_diagnostics(file_id)
            .unwrap()
            .unwrap();

        assert!(
            base.iter().all(|d| !d.message.contains("может быть Null")),
            "base diagnostics unexpectedly contain flow-sensitive null-safety: {:?}",
            base
        );

        let flow = host
            .analysis()
            .semantic_diagnostics_flow_sensitive(file_id)
            .unwrap()
            .unwrap();

        assert!(
            flow.iter().any(|d| d.message.contains("может быть Null")),
            "flow-sensitive diagnostics should contain null-safety warning: {:?}",
            flow
        );
    }

    #[test]
    fn cancellable_propagates_panics() {
        let result = std::panic::catch_unwind(|| {
            let _: Cancellable<()> = cancellable(|| panic!("test panic"));
        });
        assert!(result.is_err());
    }
}
