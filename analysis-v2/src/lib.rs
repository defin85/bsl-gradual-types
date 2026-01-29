use std::collections::HashMap;
use std::sync::Arc;

use salsa::Setter;

pub use bsl_line_index::{byte_offset_to_utf16, utf16_to_byte_offset, LineIndex};

pub mod ast_to_ir;
pub use ast_to_ir::AstToIrConverter;

mod type_inference_v2;

use bsl_diagnostics::{SemanticTypeHints, SemanticValidationVisitor};
use bsl_shared::domain::repository::{InMemoryTypeRepository, TypeRepository};
use bsl_shared::domain::resolver::TypeResolver;
use bsl_shared::domain::signature_index::SignatureIndex;
use bsl_shared::domain::types::{DiagnosticSeverity, ParseError, TypeDiagnostic};
use bsl_shared::domain::types::TypeResolution;
use bsl_shared::domain::validators::TypeValidator;
use bsl_shared::domain::TypeMetadataLookup;
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
    let type_index = type_index(db, file, deps, settings).0;

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
        (a.span.start, a.span.end, severity_key(a.severity), &a.message).cmp(&(
            b.span.start,
            b.span.end,
            severity_key(b.severity),
            &b.message,
        ))
    });

    SemanticDiagnosticsSnapshot(Arc::new(diagnostics))
}

fn populate_assignment_value_hints(
    program: &SemanticProgram,
    type_index: &type_inference_v2::TypeIndex,
    out: &mut SemanticTypeHints,
) {
    use bsl_shared::ir::SemanticNodeKind;

    for node in &program.nodes {
        let SemanticNodeKind::Assignment { value_node, .. } = &node.kind else {
            continue;
        };
        let Some(value_node_idx) = value_node else {
            continue;
        };
        let Some(value_node) = program.nodes.get(*value_node_idx) else {
            continue;
        };
        if let Some(resolution) = type_index_resolution_for_span(type_index, value_node.span) {
            out.assignment_value_type_by_span.insert(node.span, resolution);
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
            Statement::While { condition, body, .. } => {
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
            Statement::Return { value, .. } => {
                if let Some(value) = value {
                    visit_expression(value, type_index, out);
                }
            }
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
            Statement::RaiseError { message, .. } => {
                if let Some(message) = message {
                    visit_expression(message, type_index, out);
                }
            }
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
                        out.call_receiver_type_by_span.insert(key_span, receiver_type);
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
    let _deps_id = deps.id(db);
    let _settings_id = settings.id(db);
    let deps_data = deps.data(db).0.clone();
    let parsed = parse_result(db, file, settings).0;
    TypeIndexSnapshot(Arc::new(type_inference_v2::build_type_index_with_path(
        &parsed.program,
        file.path(db).as_ref(),
        deps_data,
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
            Change::RemoveFile { file_id } => {
                self.files.remove(&file_id);
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
        cancellable(|| semantic_diagnostics(&self.db, file, self.deps, self.settings).0).map(Some)
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
        cancellable(|| type_index(&self.db, file, self.deps, self.settings).0.clone())
            .map(|index| index.type_at_byte_offset(byte_offset))
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
    fn cancellable_propagates_panics() {
        let result = std::panic::catch_unwind(|| {
            let _: Cancellable<()> = cancellable(|| panic!("test panic"));
        });
        assert!(result.is_err());
    }
}
