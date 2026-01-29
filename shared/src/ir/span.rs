//! Span и SourceInfo - позиционная информация для IR

use serde::{Deserialize, Serialize};

/// Диапазон в исходном коде в **UTF-8 byte offsets** (абсолютные оффсеты в документе).
///
/// Invariants:
/// - `start <= end`
/// - `start` и `end` считаются в байтах относительно начала текста файла
/// - `end` — правая граница диапазона (exclusive), как в `start..end`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

impl Span {
    /// Проверить, содержит ли span указанный byte offset.
    ///
    /// Обратите внимание: для пустого диапазона (`start == end`) всегда возвращает `false`.
    pub fn contains(&self, byte_offset: u32) -> bool {
        self.start <= byte_offset && byte_offset < self.end
    }

    /// Создать stub span (для тестов и временного использования)
    pub fn stub() -> Self {
        Self {
            start: 0,
            end: 0,
        }
    }

    /// Создать span из byte offsets.
    pub fn new(start: u32, end: u32) -> Self {
        Self {
            start,
            end: end.max(start),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    pub fn len(&self) -> u32 {
        self.end.saturating_sub(self.start)
    }
}

impl std::fmt::Display for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}..{}", self.start, self.end)
    }
}

/// Информация об исходном файле
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceInfo {
    pub path: String,
    pub content_hash: u64,
}
