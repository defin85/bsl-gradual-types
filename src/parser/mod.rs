//! BSL Parser module
//! 
//! Парсер для языка 1С:Предприятие BSL (Built-in Script Language)
//! Использует nom для построения AST (Abstract Syntax Tree)

pub mod lexer;
pub mod ast;
pub mod parser;
pub mod visitor;
pub mod graph_builder;

pub use ast::{Program, Statement, Expression};
pub use parser::BslParser;
pub use visitor::AstVisitor;
pub use graph_builder::DependencyGraphBuilder;