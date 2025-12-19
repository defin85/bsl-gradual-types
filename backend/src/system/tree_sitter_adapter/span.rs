//! Функции для конвертации позиций tree-sitter в BSL Span
//!
//! Tree-sitter использует byte offsets (UTF-8), а LSP требует UTF-16 code units.
//! Этот модуль предоставляет функции для корректной конвертации.

use bsl_shared::ir::Span;
use tracing::debug;
use tree_sitter::Node;

#[derive(Debug, Clone)]
pub struct LineIndex {
    line_starts: Vec<usize>,
}

impl LineIndex {
    pub fn new(source: &str) -> Self {
        let mut line_starts = Vec::new();
        line_starts.push(0);
        for (idx, b) in source.bytes().enumerate() {
            if b == b'\n' {
                line_starts.push(idx + 1);
            }
        }
        Self { line_starts }
    }

    pub fn line_count(&self) -> usize {
        self.line_starts.len()
    }

    pub fn line_text<'a>(&self, source: &'a str, line: usize) -> &'a str {
        let (start, end) = self.line_bounds(line, source.len());
        &source[start..end]
    }

    pub fn byte_offset_to_utf16(&self, source: &str, line: usize, byte_offset: usize) -> u32 {
        let (start, end) = self.line_bounds(line, source.len());
        let mut capped = start.saturating_add(byte_offset);
        if capped > end {
            capped = end;
        }
        while capped > start && !source.is_char_boundary(capped) {
            capped -= 1;
        }
        let prefix = &source[start..capped];
        prefix.encode_utf16().count() as u32
    }

    fn line_bounds(&self, line: usize, source_len: usize) -> (usize, usize) {
        let start = self
            .line_starts
            .get(line)
            .copied()
            .unwrap_or(source_len);
        let end = self
            .line_starts
            .get(line + 1)
            .copied()
            .unwrap_or(source_len);
        (start, end)
    }
}

/// Конвертировать byte offset (UTF-8) в UTF-16 code units
///
/// LSP использует UTF-16 code units для позиций, а tree-sitter использует byte offsets (UTF-8).
/// Эта функция корректно преобразует byte offset в UTF-16 offset для кириллицы и других non-ASCII символов.
///
/// # Milestone 2.18 Task 1: КРИТИЧНОЕ ИСПРАВЛЕНИЕ
pub fn byte_offset_to_utf16(line: &str, byte_offset: usize) -> u32 {
    let mut capped = byte_offset.min(line.len());
    while capped > 0 && !line.is_char_boundary(capped) {
        capped -= 1;
    }
    line[..capped].chars().map(|c| c.len_utf16() as u32).sum()
}

/// Извлечь Span из tree-sitter Node с конвертацией в UTF-16 координаты
///
/// # Milestone 2.18 Task 1: FIX UTF-16 координаты
///
/// Tree-sitter возвращает позиции в byte offsets (UTF-8), но LSP требует UTF-16 code units.
/// Без этой конвертации диагностики будут показываться на НЕПРАВИЛЬНЫХ позициях в файлах с кириллицей!
///
/// Пример проблемы:
/// ```bsl
/// Функция Тест()  // "Функция" = 7 символов кириллицы
///     Перем Х;    // tree-sitter: column=14 (UTF-8 bytes), LSP нужно: 11 (UTF-16 code units)
/// ```
///
/// **ВАЖНО:** Этот метод делает O(n) итерацию по строкам для каждого узла.
/// Для производительности используйте `node_to_span_cached()` вместо него.
#[allow(dead_code)]
pub fn node_to_span(node: &Node, source: &str) -> Span {
    // Для обратной совместимости: индексируем строки для этого вызова
    let index = LineIndex::new(source);
    node_to_span_cached(node, source, &index)
}

/// Извлечь Span с использованием кеша строк (O(1) доступ вместо O(n))
///
/// # Performance Optimization (Milestone 2.19)
///
/// Эта версия использует предпросчитанный кеш строк для избежания O(n^2) итераций.
/// Для файла в 500 строк с 300 узлами:
/// - **Было:** ~150,000 итераций по строкам (O(n) для каждого узла)
/// - **Стало:** 500 итераций (O(1) доступ для каждого узла через кеш)
pub fn node_to_span_cached(node: &Node, source: &str, line_index: &LineIndex) -> Span {
    let start_pos = node.start_position();
    let end_pos = node.end_position();

    let line_count = line_index.line_count();
    if start_pos.row >= line_count {
        tracing::warn!(
            "Tree-sitter returned invalid start line: {} (file has {} lines)",
            start_pos.row,
            line_count
        );
    }
    if end_pos.row >= line_count {
        tracing::warn!(
            "Tree-sitter returned invalid end line: {} (file has {} lines)",
            end_pos.row,
            line_count
        );
    }

    // MILESTONE 2.18: Конвертируем byte offsets -> UTF-16 code units
    let start_column_utf16 =
        line_index.byte_offset_to_utf16(source, start_pos.row, start_pos.column);
    let end_column_utf16 = line_index.byte_offset_to_utf16(source, end_pos.row, end_pos.column);

    let span = Span::from_positions(
        (start_pos.row as u32, start_column_utf16),
        (end_pos.row as u32, end_column_utf16),
    );

    // Milestone 2.11 Task B1: DEBUG логи для Span extraction
    debug!(
        "Extracted Span (UTF-16): {}:{} - {}:{} (node kind: {}) [UTF-8 was: {}:{} - {}:{}]",
        span.start_line,
        span.start_column,
        span.end_line,
        span.end_column,
        node.kind(),
        start_pos.row,
        start_pos.column,
        end_pos.row,
        end_pos.column
    );

    span
}
