//! BSL parser namespace in the new flat structure

pub mod ast;
pub mod common;
pub mod graph_builder;
pub mod lexer;
pub mod parser;
pub mod tree_sitter_adapter;
pub mod visitor;

pub use ast::{Expression, Program, Statement};
pub use common::{Parser, ParserFactory};
pub use graph_builder::DependencyGraphBuilder;
pub use parser::BslParser;
pub use visitor::AstVisitor;
