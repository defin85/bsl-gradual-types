//! Syntax AST types used by the tree-sitter adapter.

pub use bsl_shared::domain::code_location::CompilerDirective;
pub use bsl_shared::domain::types::{ErrorType, ParseError};
pub use bsl_shared::ir::Span;
use serde::{Deserialize, Serialize};

/// Parse result with partial recovery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseResult {
    pub program: Program,
    pub syntax_errors: Vec<ParseError>,
}

impl ParseResult {
    pub fn success(program: Program) -> Self {
        Self {
            program,
            syntax_errors: Vec::new(),
        }
    }

    pub fn with_errors(program: Program, errors: Vec<ParseError>) -> Self {
        Self {
            program,
            syntax_errors: errors,
        }
    }

    pub fn has_errors(&self) -> bool {
        !self.syntax_errors.is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Program {
    pub statements: Vec<Statement>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Statement {
    Assignment {
        target: Expression,
        value: Expression,
        span: Span,
    },
    VarDeclaration {
        name: String,
        type_hint: Option<String>,
        span: Span,
    },
    FunctionDecl {
        name: String,
        params: Vec<String>,
        body: Vec<Statement>,
        compiler_directive: Option<CompilerDirective>,
        is_export: bool,
        span: Span,
    },
    ProcedureDecl {
        name: String,
        params: Vec<String>,
        body: Vec<Statement>,
        compiler_directive: Option<CompilerDirective>,
        is_export: bool,
        span: Span,
    },
    If {
        condition: Expression,
        then_body: Vec<Statement>,
        else_body: Option<Vec<Statement>>,
        /// Span заголовка (IF ... THEN), без тела веток.
        ///
        /// Tree-sitter pipeline заполняет это поле; fallback-парсеры могут оставить `None`.
        #[serde(default)]
        header_span: Option<Span>,
        /// Span тела then-ветки (между THEN и ELSE/ENDIF), без ключевых слов.
        #[serde(default)]
        then_span: Option<Span>,
        /// Span тела else/elseif-ветки (между ELSE/ELSIF и ENDIF), без ключевых слов.
        #[serde(default)]
        else_span: Option<Span>,
        span: Span,
    },
    For {
        variable: String,
        start: Expression,
        end: Expression,
        body: Vec<Statement>,
        /// Span заголовка (FOR ... DO), без тела цикла.
        #[serde(default)]
        header_span: Option<Span>,
        /// Span тела цикла (между DO и ENDDO), без ключевых слов.
        #[serde(default)]
        body_span: Option<Span>,
        span: Span,
    },
    ForEach {
        variable: String,
        collection: Expression,
        body: Vec<Statement>,
        /// Span заголовка (FOREACH ... DO), без тела цикла.
        #[serde(default)]
        header_span: Option<Span>,
        /// Span тела цикла (между DO и ENDDO), без ключевых слов.
        #[serde(default)]
        body_span: Option<Span>,
        span: Span,
    },
    While {
        condition: Expression,
        body: Vec<Statement>,
        /// Span заголовка (WHILE ... DO), без тела цикла.
        #[serde(default)]
        header_span: Option<Span>,
        /// Span тела цикла (между DO и ENDDO), без ключевых слов.
        #[serde(default)]
        body_span: Option<Span>,
        span: Span,
    },
    Return {
        value: Option<Expression>,
        span: Span,
    },
    Try {
        try_body: Vec<Statement>,
        except_body: Vec<Statement>,
        /// Span заголовка (TRY keyword). Нужен для корректного отделения header/body.
        #[serde(default)]
        header_span: Option<Span>,
        /// Span тела try (между TRY и EXCEPT), без ключевых слов.
        #[serde(default)]
        try_span: Option<Span>,
        /// Span тела except (между EXCEPT и ENDTRY), без ключевых слов.
        #[serde(default)]
        except_span: Option<Span>,
        span: Span,
    },
    Call {
        expression: Expression,
        span: Span,
    },
    Break {
        span: Span,
    },
    Continue {
        span: Span,
    },
    Goto {
        label: String,
        span: Span,
    },
    Label {
        name: String,
        span: Span,
    },
    Execute {
        code: Expression,
        span: Span,
    },
    RaiseError {
        message: Option<Expression>,
        span: Span,
    },
    AddHandler {
        event: Expression,
        handler: Expression,
        span: Span,
    },
    RemoveHandler {
        event: Expression,
        handler: Expression,
        span: Span,
    },
    Await {
        expression: Expression,
        span: Span,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Expression {
    Identifier {
        name: String,
        span: Span,
    },
    String {
        value: String,
        span: Span,
    },
    Number {
        value: f64,
        span: Span,
    },
    Boolean {
        value: bool,
        span: Span,
    },
    Date {
        value: String,
        span: Span,
    },
    Call {
        function: Box<Expression>,
        args: Vec<Expression>,
        span: Span,
    },
    Binary {
        left: Box<Expression>,
        operator: String,
        right: Box<Expression>,
        span: Span,
    },
    Unary {
        operator: String,
        operand: Box<Expression>,
        span: Span,
    },
    Ternary {
        condition: Box<Expression>,
        then_expr: Box<Expression>,
        else_expr: Box<Expression>,
        span: Span,
    },
    New {
        type_name: String,
        args: Vec<Expression>,
        span: Span,
    },
    PropertyAccess {
        object: Box<Expression>,
        property: String,
        span: Span,
    },
    IndexAccess {
        object: Box<Expression>,
        index: Box<Expression>,
        span: Span,
    },
    Await {
        expression: Box<Expression>,
        span: Span,
    },
}
