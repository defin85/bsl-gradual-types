//! BSL parsing module

pub mod ast {
    #[derive(Debug, Clone)]
    pub struct Program {
        pub statements: Vec<Statement>,
    }

    #[derive(Debug, Clone)]
    pub struct Expression;

    #[derive(Debug, Clone)]
    pub struct Statement;
}

pub mod common {
    #[derive(Debug, Clone)]
    pub struct Position {
        pub line: u32,
        pub column: u32,
    }

    pub trait Parser {
        fn parse(&self, content: &str) -> Result<super::ast::Program, String>;
    }

    pub struct ParserFactory;

    #[derive(Debug)]
    pub struct TextChange;
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

pub use ast::Program;
