//! BSL parsing module

pub mod ast {
    pub use bsl_syntax::ast::*;
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
