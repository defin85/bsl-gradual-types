//! BSL parsing module

pub mod ast {
    #[derive(Debug, Clone)]
    pub struct Program {
        pub statements: Vec<Statement>,
    }

    #[derive(Debug, Clone)]
    pub enum Statement {
        Assignment {
            target: Expression,
            value: Expression,
        },
        VarDeclaration {
            name: String,
            type_hint: Option<String>,
        },
        FunctionDecl {
            name: String,
            params: Vec<String>,
            body: Vec<Statement>,
        },
        If {
            condition: Expression,
            then_body: Vec<Statement>,
            else_body: Option<Vec<Statement>>,
        },
    }

    #[derive(Debug, Clone)]
    pub enum Expression {
        Identifier(String),
        String(String),
        Number(f64),
        Boolean(bool),
        Call {
            function: Box<Expression>,
            args: Vec<Expression>,
        },
        Binary {
            left: Box<Expression>,
            operator: String,
            right: Box<Expression>,
        },
        Unary {
            operator: String,
            operand: Box<Expression>,
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
            content: &str,
            changes: &[TextChange],
        ) -> Result<super::ast::Program, String> {
            // Fallback to full parse
            self.parse(content)
        }
    }

    #[derive(Debug)]
    pub struct ParserFactory;

    impl ParserFactory {
        pub fn create() -> Box<dyn Parser> {
            Box::new(super::BslParser::new("").unwrap())
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

pub struct BslParser {
    content: String,
}

impl BslParser {
    pub fn new(content: &str) -> Result<Self, String> {
        Ok(Self {
            content: content.to_string(),
        })
    }

    pub fn parse(&self) -> Result<ast::Program, String> {
        // Простая заглушка парсера
        Ok(ast::Program { statements: vec![] })
    }
}

impl common::Parser for BslParser {
    fn parse(&self, content: &str) -> Result<ast::Program, String> {
        Ok(ast::Program { statements: vec![] })
    }
}

pub use ast::Program;
