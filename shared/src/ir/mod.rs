//! Intermediate Representation (IR) — семантический слой между Syntax и Domain
//!
//! IR не зависит от конкретного парсера (tree-sitter, regex, etc.)
//! и представляет программу в терминах, удобных для type analysis.
//!
//! # Архитектура
//!
//! ```text
//! Source Code
//!     ↓
//! AST (Syntax, backend) ← tree-sitter-bsl
//!     ↓
//! IR (Semantics, shared) ← AstToIrConverter (backend)
//!     ↓
//! Types (Domain, shared) ← AnalysisEngine, TypeResolver
//! ```
//!
//! # Основные компоненты
//!
//! - [`SemanticProgram`] — корневая структура IR
//! - [`SemanticNode`] — узлы программы (переменные, функции, control flow)
//! - [`SymbolTable`] — таблица символов с scope hierarchy
//! - [`ControlFlowGraph`] — граф потока управления (для flow-sensitive анализа)

// Модули IR
mod cfg;
mod dto;
mod program;
mod span;
mod symbol_table;
mod types;
pub mod visitor;

#[cfg(test)]
mod tests;

// Re-exports: Span и SourceInfo
pub use span::{SourceInfo, Span};

// Re-exports: основные типы
pub use types::{
    FunctionSignature, MemberAccessKind, Parameter, SemanticNode, SemanticNodeKind, VariableState,
};

// Re-exports: таблица символов
pub use symbol_table::{Scope, ScopeId, ScopeKind, SymbolTable};

// Re-exports: Control Flow Graph
pub use cfg::{
    CfgEdge, CfgNode, CfgNodeAtByteOffsetBias, CfgNodeId, CfgNodeKind, ControlFlowGraph, EdgeKind,
};

// Re-exports: SemanticProgram
pub use program::SemanticProgram;

// Re-exports: visitor pattern
pub use visitor::{walk_program, FlowContext, SemanticVisitor};
