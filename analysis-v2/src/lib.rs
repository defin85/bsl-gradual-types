use std::collections::HashMap;
use std::sync::Arc;

use salsa::Setter;

pub use bsl_line_index::{byte_offset_to_utf16, utf16_to_byte_offset, LineIndex};

pub mod ast_to_ir;
pub use ast_to_ir::AstToIrConverter;

mod open_file_overlay;
mod type_inference_v2;

use bsl_diagnostics::{SemanticTypeHints, SemanticValidationVisitor};
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
use bsl_shared::utils::hash::hash_content;
use bsl_syntax::ParseOptions;

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

#[derive(Clone)]
pub struct OpenFilesSnapshotData(pub Arc<Vec<SourceFile>>);

impl PartialEq for OpenFilesSnapshotData {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for OpenFilesSnapshotData {}

unsafe impl salsa::Update for OpenFilesSnapshotData {
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

fn cancellable<T>(op: impl FnOnce() -> T) -> Cancellable<T> {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(op)) {
        Ok(value) => Ok(value),
        Err(payload) => match payload.downcast::<salsa::Cancelled>() {
            Ok(cancelled) => Err(*cancelled),
            Err(payload) => std::panic::resume_unwind(payload),
        },
    }
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

#[salsa::input]
pub struct OpenFilesSnapshot {
    #[returns(ref)]
    pub data: OpenFilesSnapshotData,
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
pub struct FlowTypeAtOffsetSnapshot(Arc<Option<TypeResolution>>);

impl PartialEq for FlowTypeAtOffsetSnapshot {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for FlowTypeAtOffsetSnapshot {}

unsafe impl salsa::Update for FlowTypeAtOffsetSnapshot {
    unsafe fn maybe_update(old_pointer: *mut Self, new_value: Self) -> bool {
        let old_value: &mut Self = unsafe { &mut *old_pointer };
        *old_value = new_value;
        true
    }
}

#[derive(Debug, Clone)]
pub struct TypeIndexSnapshot(Arc<type_inference_v2::TypeIndex>);

impl PartialEq for TypeIndexSnapshot {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
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

#[derive(Debug, Clone)]
pub struct OpenFilesReturnOverlaySnapshot(Arc<open_file_overlay::OpenFilesReturnOverlay>);

impl PartialEq for OpenFilesReturnOverlaySnapshot {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for OpenFilesReturnOverlaySnapshot {}

unsafe impl salsa::Update for OpenFilesReturnOverlaySnapshot {
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
    let _settings_id = settings.id(db);
    let text = file.text(db);
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
pub fn open_files_return_overlay(
    db: &dyn salsa::Database,
    open_files: OpenFilesSnapshot,
    deps: DepsSnapshot,
    settings: SettingsSnapshot,
) -> OpenFilesReturnOverlaySnapshot {
    let _deps_id = deps.id(db);
    let _settings_id = settings.id(db);
    let deps_data = deps.data(db).0.clone();
    let signature_index = deps_data.signature_index.clone();
    let files = open_files.data(db).0.clone();
    OpenFilesReturnOverlaySnapshot(Arc::new(
        open_file_overlay::build_return_overlay_for_open_files(
            db,
            files.as_slice(),
            &signature_index,
            settings,
        ),
    ))
}

#[salsa::tracked]
pub fn ir(
    db: &dyn salsa::Database,
    file: SourceFile,
    deps: DepsSnapshot,
    settings: SettingsSnapshot,
) -> SemanticProgramSnapshot {
    let _deps_id = deps.id(db);
    let deps_data = deps.data(db).0.clone();

    let parsed = parse_result(db, file, settings).0;
    let source = file.text(db).to_string();
    let file_path = file.path(db).to_string();

    match AstToIrConverter::convert_with_resolver(
        parsed.program.clone(),
        source,
        file_path.clone(),
        deps_data.repository.clone(),
        deps_data.signature_index.clone(),
        deps_data.resolver.clone(),
    ) {
        Ok(program) => SemanticProgramSnapshot(Arc::new(program)),
        Err(_err) => {
            let mut program = SemanticProgram::new();
            program.source_info.path = file_path;
            program.source_info.content_hash = hash_content(file.text(db));
            SemanticProgramSnapshot(Arc::new(program))
        }
    }
}

#[salsa::tracked]
pub fn syntax_diagnostics(
    db: &dyn salsa::Database,
    file: SourceFile,
    settings: SettingsSnapshot,
) -> SyntaxDiagnosticsSnapshot {
    let _settings_id = settings.id(db);
    let parsed = parse_result(db, file, settings).0;
    SyntaxDiagnosticsSnapshot(Arc::new(parsed.syntax_errors.clone()))
}

#[salsa::tracked]
pub fn semantic_diagnostics(
    db: &dyn salsa::Database,
    file: SourceFile,
    deps: DepsSnapshot,
    settings: SettingsSnapshot,
    open_files: OpenFilesSnapshot,
) -> SemanticDiagnosticsSnapshot {
    let _deps_id = deps.id(db);
    let _settings_id = settings.id(db);
    let deps_data = deps.data(db).0.clone();

    let parsed = parse_result(db, file, settings).0;
    if !parsed.syntax_errors.is_empty()
        && !syntax_errors_only_in_directives(file.text(db), &parsed.syntax_errors)
    {
        return SemanticDiagnosticsSnapshot(Arc::new(Vec::new()));
    }

    let program = ir(db, file, deps, settings).0;
    let type_index = type_index(db, file, deps, settings, open_files).0;

    let mut type_hints = SemanticTypeHints::default();
    populate_assignment_value_hints(&program, &type_index, &mut type_hints);
    populate_call_and_member_hints(&parsed.program, &type_index, &mut type_hints);

    let resolver = deps_data
        .resolver
        .clone()
        .unwrap_or_else(|| Arc::new(TypeResolver::new(deps_data.repository.clone())));
    let metadata_lookup = TypeMetadataLookup::new(deps_data.repository.clone());
    let validator = TypeValidator::new(&metadata_lookup);

    let detail_level = settings.diagnostics_detail_level(db);
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

    SemanticDiagnosticsSnapshot(Arc::new(diagnostics))
}

#[salsa::tracked]
pub fn semantic_diagnostics_flow_sensitive(
    db: &dyn salsa::Database,
    file: SourceFile,
    deps: DepsSnapshot,
    settings: SettingsSnapshot,
    open_files: OpenFilesSnapshot,
) -> SemanticDiagnosticsSnapshot {
    let _deps_id = deps.id(db);
    let _settings_id = settings.id(db);
    let deps_data = deps.data(db).0.clone();

    let parsed = parse_result(db, file, settings).0;
    if !parsed.syntax_errors.is_empty()
        && !syntax_errors_only_in_directives(file.text(db), &parsed.syntax_errors)
    {
        return SemanticDiagnosticsSnapshot(Arc::new(Vec::new()));
    }

    let program = ir(db, file, deps, settings).0;
    let type_index = type_index(db, file, deps, settings, open_files).0;

    let mut type_hints = SemanticTypeHints::default();
    populate_assignment_value_hints(&program, &type_index, &mut type_hints);
    populate_call_and_member_hints(&parsed.program, &type_index, &mut type_hints);

    let resolver = deps_data
        .resolver
        .clone()
        .unwrap_or_else(|| Arc::new(TypeResolver::new(deps_data.repository.clone())));
    let metadata_lookup = TypeMetadataLookup::new(deps_data.repository.clone());
    let validator = TypeValidator::new(&metadata_lookup);

    let detail_level = settings.diagnostics_detail_level(db);
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

fn cfg_node_at_byte_offset(
    cfg: &bsl_shared::ir::ControlFlowGraph,
    byte_offset: u32,
) -> Option<usize> {
    let find = |offset: u32| {
        (0..cfg.nodes().len())
            .filter_map(|node_id| cfg.node_span(node_id).map(|span| (node_id, span)))
            .filter(|(_, span)| span.contains(offset))
            .min_by_key(|(_, span)| span.len())
            .map(|(node_id, _)| node_id)
    };

    // Hover/completion часто вызываются на границе токена (например, сразу после '.'),
    // поэтому пробуем небольшое окно назад.
    for delta in 0..=32_u32 {
        if let Some(offset) = byte_offset.checked_sub(delta) {
            if let Some(node_id) = find(offset) {
                return Some(node_id);
            }
        }
    }
    None
}

fn conditional_branch_node_at_byte_offset(
    program: &bsl_shared::ir::SemanticProgram,
    cfg: &bsl_shared::ir::ControlFlowGraph,
    conditional_node_id: usize,
    byte_offset: u32,
) -> usize {
    use bsl_shared::ir::EdgeKind;
    use bsl_shared::ir::SemanticNodeKind;

    let mut edge_kind = EdgeKind::ConditionalTrue;

    if let Some(ir_node_idx) = cfg.node_ir_node_index(conditional_node_id) {
        if let Some(ir_node) = program.nodes.get(ir_node_idx) {
            if let SemanticNodeKind::IfStatement { else_branch, .. } = &ir_node.kind {
                if let Some(else_branch) = else_branch.as_ref().filter(|b| !b.is_empty()) {
                    let else_start = else_branch
                        .iter()
                        .filter_map(|idx| program.nodes.get(*idx).map(|n| n.span.start))
                        .min();

                    if else_start.is_some_and(|start| byte_offset >= start) {
                        edge_kind = EdgeKind::ConditionalFalse;
                    }
                }
            }
        }
    }

    cfg.edges()
        .iter()
        .find(|e| e.from == conditional_node_id && e.kind == edge_kind)
        .map(|e| e.to)
        .unwrap_or(conditional_node_id)
}

fn build_initial_flow_context_for_narrowing(
    cfg: &bsl_shared::ir::ControlFlowGraph,
    variable_name: &str,
    base_type: TypeResolution,
) -> FlowAnalysisContext {
    use bsl_shared::analysis::detect_type_guards;
    use bsl_shared::ir::CfgNodeKind;

    let mut ctx = FlowAnalysisContext::new();
    ctx.set_variable(variable_name, base_type);

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

    ctx
}

fn narrow_type_for_variable_at(
    program: &bsl_shared::ir::SemanticProgram,
    byte_offset: u32,
    variable_name: &str,
    base_type: TypeResolution,
) -> Option<TypeResolution> {
    use bsl_shared::analysis::NarrowingEngine;
    use bsl_shared::ir::{CfgNodeKind, EdgeKind};

    let cfg = program.cfg.as_ref()?;
    let mut node_id = cfg_node_at_byte_offset(cfg, byte_offset)?;

    // Если позиция попала в span условного узла (например, внутри then/else блока),
    // смещаемся на соответствующую ветку, чтобы получить корректный контекст narrowing.
    match &cfg.nodes().get(node_id)?.kind {
        CfgNodeKind::Conditional { .. } => {
            node_id = conditional_branch_node_at_byte_offset(program, cfg, node_id, byte_offset);
        }
        CfgNodeKind::LoopHeader { .. } => {
            node_id = cfg
                .edges()
                .iter()
                .find(|e| e.from == node_id && e.kind == EdgeKind::ConditionalTrue)
                .map(|e| e.to)
                .unwrap_or(node_id);
        }
        _ => {}
    }

    let initial = build_initial_flow_context_for_narrowing(cfg, variable_name, base_type);
    let mut engine = NarrowingEngine::new(cfg.clone());
    engine.build_narrowing_contexts(initial);

    engine
        .get_context(node_id)
        .and_then(|ctx| ctx.get_type(variable_name))
        .cloned()
        .filter(|t| !t.is_unknown())
}

#[salsa::tracked]
pub fn flow_type_at_byte_offset(
    db: &dyn salsa::Database,
    file: SourceFile,
    deps: DepsSnapshot,
    settings: SettingsSnapshot,
    open_files: OpenFilesSnapshot,
    byte_offset: u32,
) -> FlowTypeAtOffsetSnapshot {
    let _deps_id = deps.id(db);
    let _settings_id = settings.id(db);

    let program = ir(db, file, deps, settings).0;
    let type_index = type_index(db, file, deps, settings, open_files).0;

    let base = type_index.type_at_byte_offset(byte_offset);
    let (var_name, _state) = match program.find_variable_at_byte_offset(byte_offset) {
        Some(v) => v,
        None => return FlowTypeAtOffsetSnapshot(Arc::new(base)),
    };

    let base_for_narrowing = base.clone().unwrap_or_else(TypeResolution::unknown);
    if let Some(narrowed) =
        narrow_type_for_variable_at(&program, byte_offset, &var_name, base_for_narrowing)
    {
        return FlowTypeAtOffsetSnapshot(Arc::new(Some(narrowed)));
    }

    FlowTypeAtOffsetSnapshot(Arc::new(base))
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
    open_files: OpenFilesSnapshot,
) -> TypeIndexSnapshot {
    let _deps_id = deps.id(db);
    let _settings_id = settings.id(db);
    let deps_data = deps.data(db).0.clone();
    let parsed = parse_result(db, file, settings).0;
    let overlay = open_files_return_overlay(db, open_files, deps, settings).0;
    TypeIndexSnapshot(Arc::new(type_inference_v2::build_type_index_with_path(
        &parsed.program,
        file.path(db).as_ref(),
        deps_data,
        Some(overlay.clone()),
    )))
}

fn syntax_errors_only_in_directives(code: &str, errors: &[ParseError]) -> bool {
    let index = LineIndex::new(code);
    errors.iter().all(|err| {
        let (line_no, _) = index.byte_offset_to_utf16_position(code, err.span.start as usize);
        let line = index.line_text(code, line_no as usize);
        line.trim_start().starts_with('&')
    })
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
    deps: DepsSnapshot,
    settings: SettingsSnapshot,
    open_files: OpenFilesSnapshot,
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
        let open_files = OpenFilesSnapshot::new(&db, OpenFilesSnapshotData(Arc::new(Vec::new())));
        Self {
            db,
            files: HashMap::new(),
            deps,
            settings,
            open_files,
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
            Change::RemoveFile { file_id } => {
                self.files.remove(&file_id);
                self.refresh_open_files_snapshot();
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
                self.refresh_open_files_snapshot();
            }
        }
    }

    fn refresh_open_files_snapshot(&mut self) {
        let mut files: Vec<SourceFile> = self.files.values().copied().collect();
        files.sort_by_key(|file| file.id(&self.db));
        self.open_files
            .set_data(&mut self.db)
            .to(OpenFilesSnapshotData(Arc::new(files)));
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
            deps: self.deps,
            settings: self.settings,
            open_files: self.open_files,
        }
    }

    pub fn analysis(&self) -> AnalysisV2 {
        self.snapshot()
    }
}

pub struct AnalysisV2 {
    db: AnalysisDatabase,
    files: HashMap<FileId, SourceFile>,
    deps: DepsSnapshot,
    settings: SettingsSnapshot,
    open_files: OpenFilesSnapshot,
}

impl AnalysisV2 {
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
        cancellable(|| line_index(&self.db, file)).map(Some)
    }

    pub fn parse_result(
        &self,
        file_id: FileId,
    ) -> Cancellable<Option<Arc<bsl_syntax::ast::ParseResult>>> {
        let Some(&file) = self.files.get(&file_id) else {
            return Ok(None);
        };
        cancellable(|| parse_result(&self.db, file, self.settings).0).map(Some)
    }

    pub fn ir(&self, file_id: FileId) -> Cancellable<Option<Arc<SemanticProgram>>> {
        let Some(&file) = self.files.get(&file_id) else {
            return Ok(None);
        };
        cancellable(|| ir(&self.db, file, self.deps, self.settings).0).map(Some)
    }

    pub fn syntax_diagnostics(&self, file_id: FileId) -> Cancellable<Option<Arc<Vec<ParseError>>>> {
        let Some(&file) = self.files.get(&file_id) else {
            return Ok(None);
        };
        cancellable(|| syntax_diagnostics(&self.db, file, self.settings).0).map(Some)
    }

    pub fn semantic_diagnostics(
        &self,
        file_id: FileId,
    ) -> Cancellable<Option<Arc<Vec<TypeDiagnostic>>>> {
        let Some(&file) = self.files.get(&file_id) else {
            return Ok(None);
        };
        cancellable(|| {
            semantic_diagnostics(&self.db, file, self.deps, self.settings, self.open_files).0
        })
        .map(Some)
    }

    pub fn semantic_diagnostics_flow_sensitive(
        &self,
        file_id: FileId,
    ) -> Cancellable<Option<Arc<Vec<TypeDiagnostic>>>> {
        let Some(&file) = self.files.get(&file_id) else {
            return Ok(None);
        };
        cancellable(|| {
            semantic_diagnostics_flow_sensitive(
                &self.db,
                file,
                self.deps,
                self.settings,
                self.open_files,
            )
            .0
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
        let Some(&file) = self.files.get(&file_id) else {
            return Ok(None);
        };
        cancellable(|| {
            type_index(&self.db, file, self.deps, self.settings, self.open_files)
                .0
                .clone()
        })
        .map(|index| index.type_at_byte_offset(byte_offset))
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
            flow_type_at_byte_offset(
                &self.db,
                file,
                self.deps,
                self.settings,
                self.open_files,
                byte_offset,
            )
            .0
            .clone()
        })
        .map(|s| (*s).clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn type_index_uses_open_file_return_overlay_across_files() {
        let repository = Arc::new(InMemoryTypeRepository::new()) as Arc<dyn TypeRepository>;
        let platform_signatures_loaded = repository.platform_docs_loaded();
        let deps = Arc::new(SemanticDeps {
            signature_index: SignatureIndex::new(),
            resolver: Some(Arc::new(TypeResolver::new(repository.clone()))),
            repository,
            platform_signatures_loaded,
        });

        let mut host = AnalysisHostV2::default();
        host.apply_change(Change::SetDepsSnapshot {
            deps_id: DepsSnapshotId::from_hash("deps"),
            deps,
        });
        host.apply_change(Change::SetSettingsSnapshot {
            settings_id: SettingsId::from_hash("settings"),
            diagnostics_detail_level: DetailLevel::Full,
        });

        let file_a = FileId(1);
        let file_b = FileId(2);

        let source_a: Arc<str> = Arc::from(
            "Процедура Тест()\n\
             Р = ОбщийМодуль1.Ф1();\n\
             КонецПроцедуры",
        );
        let offset = source_a.find("Ф1").expect("Ф1 offset") as u32;

        host.apply_change(Change::SetFile {
            file_id: file_a,
            text: source_a.clone(),
            version: 1,
            path: Arc::from("test.bsl"),
        });

        host.apply_change(Change::SetFile {
            file_id: file_b,
            text: Arc::from(
                "Функция Ф1() Экспорт\n\
                 Возврат 1;\n\
                 КонецФункции",
            ),
            version: 1,
            path: Arc::from("CommonModules/ОбщийМодуль1/Ext/Module.bsl"),
        });

        let analysis = host.analysis();
        let ty = analysis
            .type_at_byte_offset(file_a, offset)
            .unwrap()
            .unwrap();
        assert_eq!(ty.type_name(), "Число");
        drop(analysis);

        // Несохранённое изменение в другом open file должно обновлять overlay.
        host.apply_change(Change::SetFile {
            file_id: file_b,
            text: Arc::from(
                "Функция Ф1() Экспорт\n\
                 Возврат \"x\";\n\
                 КонецФункции",
            ),
            version: 2,
            path: Arc::from("CommonModules/ОбщийМодуль1/Ext/Module.bsl"),
        });

        let analysis = host.analysis();
        let ty = analysis
            .type_at_byte_offset(file_a, offset)
            .unwrap()
            .unwrap();
        assert_eq!(ty.type_name(), "Строка");
    }

    #[test]
    fn type_index_uses_open_file_return_overlay_across_files_object_module() {
        let repository = Arc::new(InMemoryTypeRepository::new()) as Arc<dyn TypeRepository>;
        let platform_signatures_loaded = repository.platform_docs_loaded();
        let mut signature_index = SignatureIndex::new();
        signature_index.add_platform_method(
            bsl_shared::domain::TypeId::new("СправочникМенеджер.Контрагенты"),
            bsl_shared::domain::signature_index::MethodSignature::new(
                "ПолучитьОбъект".to_string(),
                Some("СправочникМенеджер.Контрагенты".to_string()),
                vec![],
                Some("СправочникОбъект.Контрагенты".to_string()),
                None,
                None,
                bsl_shared::domain::signature_index::SignatureSource::Platform,
                None,
                Default::default(),
            ),
        );
        let deps = Arc::new(SemanticDeps {
            signature_index,
            resolver: Some(Arc::new(TypeResolver::new(repository.clone()))),
            repository,
            platform_signatures_loaded,
        });

        let mut host = AnalysisHostV2::default();
        host.apply_change(Change::SetDepsSnapshot {
            deps_id: DepsSnapshotId::from_hash("deps"),
            deps,
        });
        host.apply_change(Change::SetSettingsSnapshot {
            settings_id: SettingsId::from_hash("settings"),
            diagnostics_detail_level: DetailLevel::Full,
        });

        let file_a = FileId(1);
        let file_b = FileId(2);

        let source_a: Arc<str> = Arc::from(
            "Процедура Тест()\n\
             Р = Справочники.Контрагенты.ПолучитьОбъект().Ф1();\n\
             КонецПроцедуры",
        );
        let offset = source_a.find("Ф1").expect("Ф1 offset") as u32;

        host.apply_change(Change::SetFile {
            file_id: file_a,
            text: source_a.clone(),
            version: 1,
            path: Arc::from("test.bsl"),
        });

        host.apply_change(Change::SetFile {
            file_id: file_b,
            text: Arc::from(
                "Функция Ф1() Экспорт\n\
                 Возврат 1;\n\
                 КонецФункции",
            ),
            version: 1,
            path: Arc::from("Catalogs/Контрагенты/Ext/ObjectModule.bsl"),
        });

        let analysis = host.analysis();
        let ty = analysis
            .type_at_byte_offset(file_a, offset)
            .unwrap()
            .unwrap();
        assert_eq!(ty.type_name(), "Число");
        drop(analysis);

        host.apply_change(Change::SetFile {
            file_id: file_b,
            text: Arc::from(
                "Функция Ф1() Экспорт\n\
                 Возврат \"x\";\n\
                 КонецФункции",
            ),
            version: 2,
            path: Arc::from("Catalogs/Контрагенты/Ext/ObjectModule.bsl"),
        });

        let analysis = host.analysis();
        let ty = analysis
            .type_at_byte_offset(file_a, offset)
            .unwrap()
            .unwrap();
        assert_eq!(ty.type_name(), "Строка");
    }

    #[test]
    fn type_index_uses_open_file_return_overlay_across_files_record_set_module() {
        let repository = Arc::new(InMemoryTypeRepository::new()) as Arc<dyn TypeRepository>;
        let platform_signatures_loaded = repository.platform_docs_loaded();
        let mut signature_index = SignatureIndex::new();
        signature_index.add_platform_method(
            bsl_shared::domain::TypeId::new("РегистрНакопленияМенеджер.РегистрНакопления"),
            bsl_shared::domain::signature_index::MethodSignature::new(
                "СоздатьНаборЗаписей".to_string(),
                Some("РегистрНакопленияМенеджер.РегистрНакопления".to_string()),
                vec![],
                Some("РегистрНакопленияНаборЗаписей.РегистрНакопления".to_string()),
                None,
                None,
                bsl_shared::domain::signature_index::SignatureSource::Platform,
                None,
                Default::default(),
            ),
        );
        let deps = Arc::new(SemanticDeps {
            signature_index,
            resolver: Some(Arc::new(TypeResolver::new(repository.clone()))),
            repository,
            platform_signatures_loaded,
        });

        let mut host = AnalysisHostV2::default();
        host.apply_change(Change::SetDepsSnapshot {
            deps_id: DepsSnapshotId::from_hash("deps"),
            deps,
        });
        host.apply_change(Change::SetSettingsSnapshot {
            settings_id: SettingsId::from_hash("settings"),
            diagnostics_detail_level: DetailLevel::Full,
        });

        let file_a = FileId(1);
        let file_b = FileId(2);

        let source_a: Arc<str> = Arc::from(
            "Процедура Тест()\n\
             Р = РегистрыНакопления.РегистрНакопления.СоздатьНаборЗаписей().Ф1();\n\
             КонецПроцедуры",
        );
        let offset = source_a.find("Ф1").expect("Ф1 offset") as u32;

        host.apply_change(Change::SetFile {
            file_id: file_a,
            text: source_a.clone(),
            version: 1,
            path: Arc::from("test.bsl"),
        });

        host.apply_change(Change::SetFile {
            file_id: file_b,
            text: Arc::from(
                "Функция Ф1() Экспорт\n\
                 Возврат 1;\n\
                 КонецФункции",
            ),
            version: 1,
            path: Arc::from("AccumulationRegisters/РегистрНакопления/Ext/RecordSetModule.bsl"),
        });

        let analysis = host.analysis();
        let ty = analysis
            .type_at_byte_offset(file_a, offset)
            .unwrap()
            .unwrap();
        assert_eq!(ty.type_name(), "Число");
        drop(analysis);

        host.apply_change(Change::SetFile {
            file_id: file_b,
            text: Arc::from(
                "Функция Ф1() Экспорт\n\
                 Возврат \"x\";\n\
                 КонецФункции",
            ),
            version: 2,
            path: Arc::from("AccumulationRegisters/РегистрНакопления/Ext/RecordSetModule.bsl"),
        });

        let analysis = host.analysis();
        let ty = analysis
            .type_at_byte_offset(file_a, offset)
            .unwrap()
            .unwrap();
        assert_eq!(ty.type_name(), "Строка");
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
    fn signature_index_weak_return_type_produces_inferredweak_without_losing_type() {
        use bsl_shared::domain::signature_index::{
            ContextRequirements, MethodSignature, SignatureSource,
        };
        use bsl_shared::ParameterInfo;
        use bsl_shared::domain::types::Certainty;

        let repository = Arc::new(InMemoryTypeRepository::new()) as Arc<dyn TypeRepository>;
        let platform_signatures_loaded = repository.platform_docs_loaded();

        let mut signature_index = SignatureIndex::new();
        let mut sig = MethodSignature::new(
            "Ф1".to_string(),
            None,
            Vec::<ParameterInfo>::new(),
            Some("Строка".to_string()),
            None,
            None,
            SignatureSource::Configuration,
            None,
            ContextRequirements::Universal,
        );
        sig.return_is_weak = true;
        signature_index.add_global_function(bsl_shared::domain::TypeId::new("Ф1"), sig);

        let deps = Arc::new(SemanticDeps {
            signature_index,
            resolver: Some(Arc::new(TypeResolver::new(repository.clone()))),
            repository,
            platform_signatures_loaded,
        });

        let mut host = AnalysisHostV2::default();
        host.apply_change(Change::SetDepsSnapshot {
            deps_id: DepsSnapshotId::from_hash("deps"),
            deps,
        });
        host.apply_change(Change::SetSettingsSnapshot {
            settings_id: SettingsId::from_hash("settings"),
            diagnostics_detail_level: DetailLevel::Full,
        });

        let file_id = FileId(1);
        let source: Arc<str> = Arc::from(
            "Процедура Тест()\n\
             Р = Ф1();\n\
             КонецПроцедуры",
        );
        let offset = source.find("Ф1").expect("Ф1 offset") as u32;
        host.apply_change(Change::SetFile {
            file_id,
            text: source,
            version: 1,
            path: Arc::from("test.bsl"),
        });

        let analysis = host.analysis();
        let ty = analysis
            .type_at_byte_offset(file_id, offset)
            .unwrap()
            .unwrap();
        assert_eq!(ty.type_name(), "Строка");
        assert_eq!(ty.certainty, Certainty::InferredWeak);
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

        let diagnostics = host
            .analysis()
            .semantic_diagnostics(file_id)
            .unwrap()
            .unwrap();

        assert!(
            diagnostics
                .iter()
                .all(|d| !d.message.contains("может быть Null")),
            "diagnostics unexpectedly contain flow-sensitive null-safety: {:?}",
            diagnostics
        );
    }

    #[test]
    fn semantic_diagnostics_flow_sensitive_includes_null_safety() {
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

        let diagnostics = host
            .analysis()
            .semantic_diagnostics_flow_sensitive(file_id)
            .unwrap()
            .unwrap();

        assert!(
            diagnostics
                .iter()
                .any(|d| d.message.contains("может быть Null")),
            "flow-sensitive diagnostics should contain null-safety warning: {:?}",
            diagnostics
        );
    }

    #[test]
    fn flow_type_at_byte_offset_does_not_depend_on_first_entry_node() {
        let mut host = AnalysisHostV2::default();
        let file_id = FileId(1);

        host.apply_change(Change::SetFile {
            file_id,
            text: Arc::from(
                "Procedure First()\n\
                 a = 1;\n\
                 EndProcedure\n\
                 \n\
                 Procedure Second()\n\
                 x = 0;\n\
                 Если ТипЗнч(x) = Тип(\"Строка\") Тогда\n\
                 y = x;\n\
                 КонецЕсли;\n\
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

        let analysis = host.analysis();
        let code = analysis.file_text(file_id).unwrap().unwrap();
        let byte_offset = code
            .find("y = x")
            .map(|idx| (idx + "y = ".len()) as u32)
            .expect("offset for 'x' in 'y = x'");

        let flow_type = analysis
            .flow_type_at_byte_offset(file_id, byte_offset)
            .unwrap()
            .expect("flow type at offset");

        match flow_type.result {
            bsl_shared::domain::types::ResolutionResult::Concrete(
                bsl_shared::domain::types::ConcreteType::Platform(pt),
            ) => assert_eq!(pt.name, "Строка"),
            other => panic!("Expected Строка, got: {:?}", other),
        }
    }

    #[test]
    fn cancellable_propagates_panics() {
        let result = std::panic::catch_unwind(|| {
            let _: Cancellable<()> = cancellable(|| panic!("test panic"));
        });
        assert!(result.is_err());
    }
}
