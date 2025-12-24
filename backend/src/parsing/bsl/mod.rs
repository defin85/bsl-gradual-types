//! BSL parsing module

pub mod ast {
    //! # Unified ParseError (Milestone 2.19 Phase 1)
    //!
    //! **Single Source of Truth:** ParseError и ErrorType теперь определены
    //! в `shared::domain::types` вместо `backend::parsing::bsl`.
    //!
    //! ## История
    //! - **До Milestone 2.19:** дублированные определения в `backend/src/parsing/bsl/mod.rs` (~57 строк)
    //! - **После Milestone 2.19:** единое определение в `shared/src/domain/types.rs`
    //!
    //! ## Архитектура
    //! ```text
    //! shared (Domain Layer) ← backend (Application Layer)
    //!    ↑
    //!    └─── Single Source of Truth для типов ошибок
    //! ```

    pub use bsl_shared::domain::code_location::CompilerDirective;
    pub use bsl_shared::domain::types::{ErrorType, ParseError};
    pub use bsl_shared::ir::Span;
    use serde::{Deserialize, Serialize};

    /// Результат парсинга с поддержкой частичного восстановления
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ParseResult {
        pub program: Program,
        pub syntax_errors: Vec<ParseError>,
    }

    impl ParseResult {
        /// Создать успешный результат без ошибок
        pub fn success(program: Program) -> Self {
            Self {
                program,
                syntax_errors: Vec::new(),
            }
        }

        /// Создать результат с ошибками
        pub fn with_errors(program: Program, errors: Vec<ParseError>) -> Self {
            Self {
                program,
                syntax_errors: errors,
            }
        }

        /// Проверить, есть ли синтаксические ошибки
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
            /// Процедура/функция объявлена с `Экспорт` (публичная)
            is_export: bool,
            span: Span,
        },
        ProcedureDecl {
            name: String,
            params: Vec<String>,
            body: Vec<Statement>,
            compiler_directive: Option<CompilerDirective>,
            /// Процедура объявлена с `Экспорт` (публичная)
            is_export: bool,
            span: Span,
        },
        If {
            condition: Expression,
            then_body: Vec<Statement>,
            else_body: Option<Vec<Statement>>,
            span: Span,
        },
        For {
            variable: String,
            start: Expression,
            end: Expression,
            body: Vec<Statement>,
            span: Span,
        },
        ForEach {
            variable: String,
            collection: Expression,
            body: Vec<Statement>,
            span: Span,
        },
        While {
            condition: Expression,
            body: Vec<Statement>,
            span: Span,
        },
        Return {
            value: Option<Expression>,
            span: Span,
        },
        Try {
            try_body: Vec<Statement>,
            except_body: Vec<Statement>,
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
}

pub mod common {
    #[derive(Debug, Clone)]
    pub struct Position {
        pub line: u32,
        pub column: u32,
    }

    pub trait Parser {
        fn parse(&self, content: &str) -> Result<super::ast::Program, String>;
        fn parse_incremental(
            &self,
            _content: &str,
            _changes: &[TextChange],
        ) -> Result<super::ast::Program, String> {
            // Fallback to full parse
            self.parse(_content)
        }
    }

    #[derive(Debug)]
    pub struct ParserFactory;

    impl ParserFactory {
        pub fn create() -> Box<dyn Parser> {
            match super::BslParser::new("") {
                Ok(parser) => Box::new(parser),
                Err(_) => Box::new(super::BslParser),
            }
        }
    }

    #[derive(Debug)]
    pub struct TextChange {
        pub start_byte: usize,
        pub old_end_byte: usize,
        pub new_end_byte: usize,
        pub start_position: Position,
        pub old_end_position: Position,
        pub new_end_position: Position,
    }
}

pub struct BslParser;

impl BslParser {
    pub fn new(_content: &str) -> Result<Self, String> {
        Ok(Self)
    }

    pub fn parse(&self) -> Result<ast::Program, String> {
        // Простая заглушка парсера
        Ok(ast::Program { statements: vec![] })
    }
}

impl common::Parser for BslParser {
    fn parse(&self, _content: &str) -> Result<ast::Program, String> {
        Ok(ast::Program { statements: vec![] })
    }
}

pub use ast::Program;
