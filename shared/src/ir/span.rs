//! Span и SourceInfo - позиционная информация для IR

use serde::{Deserialize, Serialize};

/// Позиция в исходном коде
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Span {
    pub start_line: u32,
    pub start_column: u32,
    pub end_line: u32,
    pub end_column: u32,
}

impl Span {
    /// Проверить, содержит ли span указанную позицию
    pub fn contains(&self, line: u32, column: u32) -> bool {
        if line < self.start_line || line > self.end_line {
            return false;
        }
        if line == self.start_line && column < self.start_column {
            return false;
        }
        if line == self.end_line && column > self.end_column {
            return false;
        }
        true
    }

    /// Создать stub span (для тестов и временного использования)
    pub fn stub() -> Self {
        Self {
            start_line: 0,
            start_column: 0,
            end_line: 0,
            end_column: 0,
        }
    }

    /// Создать span из координат
    pub fn new(start_line: u32, start_column: u32, end_line: u32, end_column: u32) -> Self {
        Self {
            start_line,
            start_column,
            end_line,
            end_column,
        }
    }

    /// Создать span из tree-sitter позиций (для совместимости с backend)
    pub fn from_positions(start: (u32, u32), end: (u32, u32)) -> Self {
        Self {
            start_line: start.0,
            start_column: start.1,
            end_line: end.0,
            end_column: end.1,
        }
    }
}

/// Информация об исходном файле
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceInfo {
    pub path: String,
    pub content_hash: u64,
}
