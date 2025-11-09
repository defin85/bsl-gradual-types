//! Parser Coordinator - координация Tree-sitter парсера
//!
//! Milestone 2.8 Task 7: Regex fallback удалён, используется только Tree-sitter

use anyhow::Result;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tracing::{debug, error, warn};
use tree_sitter::{InputEdit, Parser, Point};

use crate::parsing::ParseResult;
use crate::system::tree_cache::{hash_content, TreeCache};
use crate::system::tree_sitter_adapter::TreeSitterAdapter;
use bsl_shared::domain::repository::TypeRepository;
use bsl_shared::parsing::Parser as ParserTrait; // Milestone 2.8: Parser trait

/// Текстовое изменение для инкрементального парсинга (из LSP)
#[derive(Debug, Clone)]
pub struct TextEdit {
    /// Начальная позиция изменения
    pub start_line: u32,
    pub start_column: u32,
    /// Конечная позиция в старом тексте
    pub old_end_line: u32,
    pub old_end_column: u32,
    /// Конечная позиция в новом тексте
    pub new_end_line: u32,
    pub new_end_column: u32,
    /// Новый текст (вставленный/замененный)
    pub new_text: String,
}

/// Координатор Tree-sitter парсера (Milestone 2.8: без regex fallback)
pub struct ParserCoordinator {
    tree_sitter: TreeSitterParser,
    tree_cache: TreeCache,
    repository: Arc<dyn TypeRepository>,
}

/// TreeSitter парсер с tree-sitter-bsl
pub struct TreeSitterParser {
    parser: Mutex<Parser>,
}

impl ParserCoordinator {
    /// Создаёт координатор с указанным TypeRepository.
    ///
    /// Используйте этот метод когда у вас есть готовый TypeRepository
    /// с загруженными платформенными типами.
    ///
    /// # Примеры
    ///
    /// ```no_run
    /// use bsl_backend::system::ParserCoordinator;
    /// use bsl_shared::domain::repository::InMemoryTypeRepository;
    /// use std::sync::Arc;
    ///
    /// let repo = Arc::new(InMemoryTypeRepository::new());
    /// // Загрузите типы в repo...
    /// let parser = ParserCoordinator::new(repo);
    /// ```
    pub fn new(repository: Arc<dyn TypeRepository>) -> Self {
        Self {
            tree_sitter: TreeSitterParser::new(),
            tree_cache: TreeCache::new(),
            repository,
        }
    }

    /// Создаёт координатор с ПУСТЫМ TypeRepository (для обратной совместимости).
    ///
    /// **ВНИМАНИЕ:** Этот метод создаёт пустой репозиторий БЕЗ платформенных типов.
    /// Используйте только для тестов или простых сценариев без необходимости
    /// разрешения платформенных типов.
    ///
    /// Для production кода используйте [`ParserCoordinator::new`] с загруженными типами.
    ///
    /// # Milestone 2.8 Note
    ///
    /// Название "with_fallback" — исторический артефакт. Regex fallback был удалён
    /// в Milestone 2.8. Метод просто создаёт ParserCoordinator с пустым
    /// InMemoryTypeRepository.
    ///
    /// # Примеры
    ///
    /// ```no_run
    /// use bsl_backend::system::ParserCoordinator;
    ///
    /// // Для простых тестов или парсинга без типов
    /// let parser = ParserCoordinator::with_fallback();
    /// let ir = parser.parse_to_ir("Перем x: Число;", "test.bsl");
    /// ```
    pub fn with_fallback() -> Self {
        use bsl_shared::domain::repository::InMemoryTypeRepository;
        let empty_repo = Arc::new(InMemoryTypeRepository::new()) as Arc<dyn TypeRepository>;
        Self::new(empty_repo)
    }

    /// Парсинг через Tree-sitter с поддержкой ParseResult (Milestone 2.7 Task 3)
    pub fn parse(&self, content: &str) -> Result<ParseResult, String> {
        match self.tree_sitter.parse(content) {
            Ok(result) => {
                if result.has_errors() {
                    warn!(
                        "TreeSitter parsing completed with {} syntax errors",
                        result.syntax_errors.len()
                    );
                } else {
                    debug!("TreeSitter parsing successful");
                }
                Ok(result)
            }
            Err(tree_sitter_error) => {
                error!("TreeSitter parsing failed: {}", tree_sitter_error);
                Err(format!("Tree-sitter parsing failed: {}", tree_sitter_error))
            }
        }
    }

    /// Инкрементальный парсинг для LSP textDocument/didChange с ParseResult (Milestone 2.7 Task 3)
    pub fn parse_incremental(
        &self,
        file_path: PathBuf,
        new_content: String,
        edits: Vec<TextEdit>,
    ) -> Result<ParseResult, String> {
        let new_hash = hash_content(&new_content);

        // Попытка получить старое дерево из кеша
        if let Some((old_tree, old_source, old_hash)) = self.tree_cache.get(&file_path) {
            // Проверяем, нужно ли обновление
            if old_hash == new_hash {
                debug!("Content unchanged, using cached tree");
                return TreeSitterAdapter::convert_tree(&old_tree, &new_content);
            }

            // Применяем инкрементальное обновление
            debug!("Applying {} edits incrementally", edits.len());

            match self.tree_sitter.parse_incremental(
                &new_content,
                Some(&old_tree),
                edits,
                &old_source,
            ) {
                Ok((new_tree, program)) => {
                    // Кешируем новое дерево
                    self.tree_cache
                        .update(&file_path, new_tree, new_content, new_hash);
                    return Ok(program);
                }
                Err(e) => {
                    warn!(
                        "Incremental parsing failed: {}, falling back to full parse",
                        e
                    );
                }
            }
        }

        // Fallback: полный парсинг (Milestone 2.8: только Tree-sitter)
        debug!("Full parse for file: {:?}", file_path);
        match self.tree_sitter.parse(&new_content) {
            Ok(program) => {
                // Кешируем дерево для будущих инкрементальных обновлений
                // TODO: получить tree из парсера
                Ok(program)
            }
            Err(e) => {
                error!("TreeSitter parsing failed: {}", e);
                Err(format!("Tree-sitter parsing failed: {}", e))
            }
        }
    }

    /// Парсинг с конвертацией в IR (Milestone 2.8)
    ///
    /// # Примеры
    ///
    /// ```no_run
    /// use bsl_backend::system::ParserCoordinator;
    ///
    /// let parser = ParserCoordinator::with_fallback();
    /// let ir = parser.parse_to_ir("Перем x: Число;", "test.bsl");
    /// match ir {
    ///     Ok(program) => println!("IR получен с {} узлами", program.nodes.len()),
    ///     Err(e) => eprintln!("Ошибка парсинга: {}", e),
    /// }
    /// ```
    pub fn parse_to_ir(
        &self,
        content: &str,
        file_path: &str,
    ) -> Result<bsl_shared::ir::SemanticProgram, String> {
        use crate::application::ast_to_ir::AstToIrConverter;

        // 1. Парсинг в AST (tree-sitter) с поддержкой ParseResult
        let parse_result = self.parse(content)?;

        // Логируем синтаксические ошибки, если они есть
        if parse_result.has_errors() {
            warn!("⚠️ Обнаружены синтаксические ошибки при парсинге:");
            for error in &parse_result.syntax_errors {
                warn!(
                    "  - [{}:{}-{}:{}] {}",
                    error.span.start_line,
                    error.span.start_column,
                    error.span.end_line,
                    error.span.end_column,
                    error.message
                );
            }
        }

        // 2. Конвертация AST → IR (извлекаем program из ParseResult)
        AstToIrConverter::convert(
            parse_result.program,
            content.to_string(),
            file_path.to_string(),
            self.repository.clone(), // ✅ Передаём TypeRepository для Generic inference
        )
        .map_err(|e| format!("AST to IR conversion failed: {}", e))
    }

    /// Загрузка платформенных типов (упрощенная)
    pub async fn load_platform_types(&self, repository: &Arc<dyn TypeRepository>) -> Result<()> {
        debug!("Loading platform types via simple parser coordination");

        // Загружаем базовые платформенные типы
        let platform_types = crate::data::loaders::load_all_platform_types();

        debug!("Loaded {} platform types", platform_types.len());

        // Загружаем типы в репозиторий
        repository
            .load_types(platform_types)
            .map_err(|e| anyhow::anyhow!("Failed to load platform types: {}", e))?;

        let stats = repository.get_stats();
        debug!(
            "TypeRepository stats after platform types load: {} types total",
            stats.total_types
        );

        Ok(())
    }
}

impl TreeSitterParser {
    fn new() -> Self {
        let mut parser = Parser::new();

        // Инициализация BSL грамматики от alkoleft
        parser
            .set_language(&tree_sitter_bsl::LANGUAGE.into())
            .expect("Failed to load BSL grammar");

        debug!("Tree-sitter-bsl parser initialized");

        Self {
            parser: Mutex::new(parser),
        }
    }

    fn parse(&self, content: &str) -> Result<ParseResult, String> {
        // Парсинг с использованием tree-sitter-bsl
        let mut parser = self
            .parser
            .lock()
            .map_err(|e| format!("Failed to lock parser: {}", e))?;

        let tree = parser
            .parse(content, None)
            .ok_or_else(|| "Tree-sitter parsing failed".to_string())?;

        let root_node = tree.root_node();

        debug!(
            "Tree-sitter parsed: {} nodes, {} bytes",
            root_node.descendant_count(),
            content.len()
        );

        // Конвертация tree-sitter AST → ParseResult через TreeSitterAdapter
        TreeSitterAdapter::convert_tree(&tree, content)
    }

    /// Инкрементальный парсинг с использованием старого дерева (Milestone 2.7 Task 3)
    fn parse_incremental(
        &self,
        new_content: &str,
        old_tree: Option<&tree_sitter::Tree>,
        edits: Vec<TextEdit>,
        old_source: &str,
    ) -> Result<(tree_sitter::Tree, ParseResult), String> {
        let mut parser = self
            .parser
            .lock()
            .map_err(|e| format!("Failed to lock parser: {}", e))?;

        // Применяем редактирования к старому дереву
        if let Some(mut tree) = old_tree.cloned() {
            for edit in edits {
                let input_edit = Self::text_edit_to_input_edit(&edit, old_source, new_content)?;
                tree.edit(&input_edit);
                debug!("Applied edit: {:?}", input_edit);
            }

            // Парсим с использованием отредактированного дерева
            let new_tree = parser
                .parse(new_content, Some(&tree))
                .ok_or_else(|| "Incremental parsing failed".to_string())?;

            debug!(
                "Incremental parse: {} nodes, {} bytes",
                new_tree.root_node().descendant_count(),
                new_content.len()
            );

            let program = TreeSitterAdapter::convert_tree(&new_tree, new_content)?;
            Ok((new_tree, program))
        } else {
            // Нет старого дерева — полный парсинг
            let tree = parser
                .parse(new_content, None)
                .ok_or_else(|| "Tree-sitter parsing failed".to_string())?;

            let program = TreeSitterAdapter::convert_tree(&tree, new_content)?;
            Ok((tree, program))
        }
    }

    /// Конвертировать TextEdit → tree-sitter InputEdit
    fn text_edit_to_input_edit(
        edit: &TextEdit,
        old_source: &str,
        new_source: &str,
    ) -> Result<InputEdit, String> {
        // Вычисляем байтовые офсеты из позиций (line, column)
        let start_byte = Self::position_to_byte(old_source, edit.start_line, edit.start_column)?;
        let old_end_byte =
            Self::position_to_byte(old_source, edit.old_end_line, edit.old_end_column)?;
        let new_end_byte =
            Self::position_to_byte(new_source, edit.new_end_line, edit.new_end_column)?;

        Ok(InputEdit {
            start_byte,
            old_end_byte,
            new_end_byte,
            start_position: Point::new(edit.start_line as usize, edit.start_column as usize),
            old_end_position: Point::new(edit.old_end_line as usize, edit.old_end_column as usize),
            new_end_position: Point::new(edit.new_end_line as usize, edit.new_end_column as usize),
        })
    }

    /// Конвертировать (line, column) → byte offset
    fn position_to_byte(source: &str, line: u32, column: u32) -> Result<usize, String> {
        let mut current_line = 0u32;
        let mut byte_offset = 0usize;

        for (idx, ch) in source.char_indices() {
            if current_line == line {
                // Нашли нужную строку, считаем колонки
                if column == 0 {
                    return Ok(byte_offset);
                }

                let mut current_column = 0u32;
                for (col_idx, _) in source[byte_offset..].char_indices() {
                    if current_column == column {
                        return Ok(byte_offset + col_idx);
                    }
                    current_column += 1;
                }

                return Ok(byte_offset + source[byte_offset..].len());
            }

            if ch == '\n' {
                current_line += 1;
                byte_offset = idx + 1;
            }
        }

        // Если не нашли строку, возвращаем конец файла
        Ok(source.len())
    }
}

// === Milestone 2.8 Task 7: RegexParser удалён ===
// Regex fallback legacy был удалён, используется только Tree-sitter

// === COMPARISON WITH COMPLEX PARSING ===

/// Milestone 2.8: Упрощённая архитектура парсера
///
/// Previous Complex (UnifiedParserCoordinator):
/// - Strategy pattern с 3+ парсерами
/// - TreeSitterStrategy + SyntaxHelperStrategy + RegexFallback
/// - Parser selection logic
/// - Configuration-guided discovery
/// - ~300+ LOC
///
/// Current Simple (ParserCoordinator):
/// - Tree-sitter только (regex legacy удалён)
/// - Инкрементальный парсинг с кешированием
/// - Parser trait для инверсии зависимостей
/// - ~200 LOC
///
/// Milestone 2.8: Полный переход на Tree-sitter, regex удалён
#[cfg(test)]
mod comparison_notes {
    //! Сравнение архитектур парсинга
    //!
    //! Old Complex (UnifiedParserCoordinator):
    //! - Strategy pattern с 3+ парсерами
    //! - TreeSitterStrategy + SyntaxHelperStrategy + RegexFallback
    //! - Parser selection logic
    //! - ~300+ LOC
    //!
    //! Current (ParserCoordinator после Milestone 2.8):
    //! - Только Tree-sitter (regex legacy удалён)
    //! - Parser trait для инверсии зависимостей
    //! - Инкрементальный парсинг
    //! - ~200 LOC
    //!
    //! Результат: Упрощение архитектуры + качество анализа
}

// === MILESTONE 2.8: Parser trait implementation ===

/// Реализация Parser trait для ParserCoordinator (Milestone 2.8: Task 3.2)
///
/// Это позволяет:
/// - CLI использовать ParserCoordinator через trait без зависимости от backend
/// - Легко тестировать с mock parser
/// - Инверсия зависимостей: shared не зависит от backend
impl ParserTrait for ParserCoordinator {
    fn parse_to_ir(
        &self,
        content: &str,
        file_path: &str,
    ) -> Result<bsl_shared::ir::SemanticProgram> {
        // Используем существующий метод parse_to_ir
        self.parse_to_ir(content, file_path)
            .map_err(|e| anyhow::anyhow!("ParserCoordinator::parse_to_ir failed: {}", e))
    }

    fn parser_name(&self) -> &'static str {
        "TreeSitter" // Milestone 2.8: Regex fallback удалён
    }

    fn supports_incremental(&self) -> bool {
        true // ParserCoordinator поддерживает инкрементальный парсинг
    }
}

#[cfg(test)]
mod parser_trait_tests {
    use super::*;

    #[test]
    fn test_parser_trait_parse_to_ir() {
        let parser = ParserCoordinator::with_fallback();
        let code = "Перем x: Число;";

        let result = ParserTrait::parse_to_ir(&parser, code, "test.bsl");

        assert!(result.is_ok());
        let program = result.unwrap();
        assert_eq!(program.nodes.len(), 1); // One variable declaration
    }

    #[test]
    fn test_parser_trait_parser_name() {
        let parser = ParserCoordinator::with_fallback();
        assert_eq!(ParserTrait::parser_name(&parser), "TreeSitter"); // Milestone 2.8: regex удалён
    }

    #[test]
    fn test_parser_trait_supports_incremental() {
        let parser = ParserCoordinator::with_fallback();
        assert!(ParserTrait::supports_incremental(&parser));
    }
}
