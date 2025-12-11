//! Source code position tracking
//!
//! NOTE: For new code, prefer using `bsl_shared::ir::Span` which is the canonical
//! span type in the project. This module contains LSP-specific position types
//! that include offset information needed for incremental parsing.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Canonical Span type (re-export from shared)
///
/// Используйте этот тип для большинства случаев работы с позициями.
/// LspPosition/LspSpan нужны только когда требуется offset для
/// инкрементального парсинга.
pub use bsl_shared::ir::Span as IrSpan;

/// Position in source code with offset
///
/// This type includes byte offset for incremental parsing.
/// For simple line/column spans, use `bsl_shared::ir::Span` instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Position {
    pub line: usize,
    pub column: usize,
    pub offset: usize,
}

impl Position {
    pub fn new(line: usize, column: usize, offset: usize) -> Self {
        Self {
            line,
            column,
            offset,
        }
    }

    pub fn zero() -> Self {
        Self::new(0, 0, 0)
    }
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}

/// Span in source code (LSP-specific with offset support)
///
/// This type uses Position which includes byte offsets for incremental parsing.
/// For simple line/column spans, use `bsl_shared::ir::Span` instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Span {
    pub start: Position,
    pub end: Position,
}

impl Span {
    pub fn new(start: Position, end: Position) -> Self {
        Self { start, end }
    }

    pub fn zero() -> Self {
        Self::new(Position::zero(), Position::zero())
    }

    /// Convert to IrSpan (loses offset information)
    pub fn to_ir_span(&self) -> IrSpan {
        IrSpan::new(
            self.start.line as u32,
            self.start.column as u32,
            self.end.line as u32,
            self.end.column as u32,
        )
    }
}
