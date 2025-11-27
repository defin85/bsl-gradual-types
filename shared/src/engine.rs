//! The core analysis engine, independent of any specific adapter (backend, CLI, etc.).
//!
//! Phase 3: Simplified - no Infrastructure dependencies
//! Infrastructure initialization moved to SystemCoordinator (backend)
//!
//! Milestone 2.8: Added IR-based analysis flow

use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tracing::{info, warn};

use crate::domain::repository::{InMemoryTypeRepository, TypeRepository};
use crate::domain::resolver::TypeResolver;
use crate::domain::types::TypeResolution;
use crate::ir::SemanticProgram;
use crate::parsing::Parser;

/// A simplified, self-contained analysis result for the CLI.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CliAnalysisResult {
    pub file_path: String,
    pub type_resolutions: Vec<(String, TypeResolution)>,
    pub analysis_duration_ms: u128,
}

/// Milestone 2.8: Результат IR-based анализа (Task 4.2)
#[derive(Debug)]
pub struct IrAnalysisResult {
    /// Семантическая IR программы
    pub ir: SemanticProgram,
    /// Типы, разрешенные для каждого узла (key = node index)
    pub type_resolutions: HashMap<usize, TypeResolution>,
    /// Время парсинга (мс)
    pub parse_duration_ms: u128,
    /// Время анализа типов (мс)
    pub analysis_duration_ms: u128,
}

/// The core analysis engine.
/// It orchestrates parsing and type resolution.
///
/// Phase 3: Simplified - no Infrastructure dependencies, receives pre-initialized components
pub struct AnalysisEngine {
    resolver: Arc<TypeResolver>,
    repository: Arc<dyn TypeRepository>,
}

impl AnalysisEngine {
    /// Creates a new analysis engine with pre-initialized components (Phase 3)
    ///
    /// No I/O operations, no Infrastructure - pure Domain orchestration
    pub fn new(resolver: Arc<TypeResolver>, repository: Arc<dyn TypeRepository>) -> Self {
        info!("✨ AnalysisEngine: создан с готовыми компонентами (Phase 3)");
        Self {
            resolver,
            repository,
        }
    }

    /// DEPRECATED: Old initialization method with Infrastructure dependencies
    ///
    /// This method will be removed in future versions. Use `new()` instead.
    /// Infrastructure initialization is now in SystemCoordinator (backend).
    #[deprecated(
        since = "0.4.3",
        note = "Use AnalysisEngine::new() with pre-initialized components. Infrastructure moved to SystemCoordinator."
    )]
    #[allow(dead_code)]
    pub async fn new_with_init(
        _syntax_helper_path: Option<&Path>,
        _config_path: Option<&Path>,
    ) -> Result<Self> {
        warn!("⚠️ AnalysisEngine::new_with_init() is deprecated!");
        warn!("⚠️ Infrastructure initialization moved to SystemCoordinator");
        warn!("⚠️ Use AnalysisEngine::new() with pre-initialized components");

        // Stub implementation for backward compatibility
        let repository = Arc::new(InMemoryTypeRepository::new());
        let resolver = Arc::new(TypeResolver::new(repository.clone()));

        Ok(Self {
            resolver,
            repository,
        })
    }

    /// Получить resolver для TypeSystemService
    pub fn get_resolver(&self) -> Arc<TypeResolver> {
        self.resolver.clone()
    }

    /// Получить repository для TypeInferenceService
    pub fn get_repository(&self) -> Arc<dyn TypeRepository> {
        self.repository.clone()
    }

    /// Найти конструктор для указанного типа
    ///
    /// # Arguments
    /// * `type_name` - Имя типа (регистронезависимо)
    ///
    /// # Returns
    /// * `Some(ConstructorSignature)` - если конструктор найден
    /// * `None` - если конструктор не найден
    pub fn find_constructor(
        &self,
        type_name: &str,
    ) -> Option<crate::domain::signature_index::ConstructorSignature> {
        self.repository.find_constructor(type_name)
    }

    /// Резолвит тип по имени (Domain Use Case для hover/completion)
    ///
    /// Принимает имя типа (например: "Число", "Массив", "Справочники.Контрагенты")
    /// и возвращает полный TypeResolution с метаданными.
    ///
    /// Это основной entry point для Application Layer (TypeSystemService)
    /// после маппинга AST Expression → имя типа.
    pub fn resolve_type(&self, type_name: &str) -> TypeResolution {
        self.resolver.resolve_expression_sync(type_name)
    }

    /// Milestone 2.8: Парсинг и анализ через IR (Task 4.2)
    ///
    /// Новый flow: Code → Parser → IR → Type Analysis
    ///
    /// # Arguments
    /// * `parser` - реализация Parser trait (может быть ParserCoordinator или MockParser)
    /// * `content` - исходный код BSL
    /// * `file_path` - путь к файлу для диагностики
    ///
    /// # Returns
    /// SemanticProgram с типами, разрешенными для всех узлов
    pub fn parse_and_analyze(
        &self,
        parser: &dyn Parser,
        content: &str,
        file_path: &str,
    ) -> Result<IrAnalysisResult> {
        let start_time = std::time::Instant::now();

        // 1. Парсинг → IR
        let ir = parser.parse_to_ir(content, file_path)?;
        let parse_time = start_time.elapsed();

        info!(
            "🎯 AnalysisEngine: IR получен ({} nodes, {} symbols)",
            ir.nodes.len(),
            ir.symbols.scopes.len()
        );

        // 2. Анализ типов через IR
        let analysis_start = std::time::Instant::now();
        let type_resolutions = self.analyze_ir(&ir)?;
        let analysis_time = analysis_start.elapsed();

        Ok(IrAnalysisResult {
            ir,
            type_resolutions,
            parse_duration_ms: parse_time.as_millis(),
            analysis_duration_ms: analysis_time.as_millis(),
        })
    }

    /// Milestone 2.8: Анализ типов по IR (Task 4.2)
    ///
    /// Обходит SemanticProgram и разрешает типы для каждого узла
    fn analyze_ir(&self, ir: &SemanticProgram) -> Result<HashMap<usize, TypeResolution>> {
        use crate::ir::FlowContext;

        let mut resolutions = HashMap::new();

        // Создаем visitor для резолюции типов
        let mut type_resolver_visitor = IrTypeResolverVisitor {
            resolver: self.resolver.clone(),
            resolutions: &mut resolutions,
        };

        // Обходим IR с контекстом потока
        let mut context = FlowContext::new(ir.symbols.root_scope);

        for (idx, node) in ir.nodes.iter().enumerate() {
            type_resolver_visitor.visit_node_indexed(node, &mut context, idx);
        }

        info!(
            "🎯 AnalysisEngine: Разрешено {} типов из {} узлов",
            resolutions.len(),
            ir.nodes.len()
        );

        Ok(resolutions)
    }

    /// Analyzes a single BSL file.
    pub async fn analyze_file<P: AsRef<Path>>(&self, path: P) -> Result<CliAnalysisResult> {
        let start_time = std::time::Instant::now();
        let path_str = path.as_ref().display().to_string();

        let _content = std::fs::read_to_string(path.as_ref())
            .map_err(|e| anyhow::anyhow!("Failed to read file {}: {}", path_str, e))?;

        // MOCK IMPLEMENTATION
        let mut resolutions_map = HashMap::new();
        resolutions_map.insert(
            "ПеременнаяА".to_string(),
            self.resolver.resolve_expression_sync("Строка"),
        );
        resolutions_map.insert(
            "ПеременнаяБ".to_string(),
            self.resolver
                .resolve_expression_sync("Справочники.Контрагенты"),
        );
        let resolutions: Vec<(String, TypeResolution)> = resolutions_map.into_iter().collect();

        let result = CliAnalysisResult {
            file_path: path_str,
            type_resolutions: resolutions,
            analysis_duration_ms: start_time.elapsed().as_millis(),
        };

        Ok(result)
    }
}

// === MILESTONE 2.8: IR Type Resolver Visitor (Task 4.3) ===

/// Visitor для резолюции типов в IR
///
/// Обходит SemanticProgram и вызывает TypeResolver для каждого узла,
/// сохраняя результаты по индексу узла
struct IrTypeResolverVisitor<'a> {
    resolver: Arc<TypeResolver>,
    resolutions: &'a mut HashMap<usize, TypeResolution>,
}

impl<'a> IrTypeResolverVisitor<'a> {
    /// Обработка узла с сохранением индекса для маппинга
    fn visit_node_indexed(
        &mut self,
        node: &crate::ir::SemanticNode,
        context: &mut crate::ir::FlowContext,
        index: usize,
    ) {
        use crate::ir::SemanticNodeKind;

        let resolution = match &node.kind {
            SemanticNodeKind::VariableDeclaration {
                name, type_hint, ..
            } => {
                // Phase 3: type_hint теперь Option<TypeResolution>
                if let Some(hint_resolution) = type_hint {
                    // Резолвим тип через resolver для дополнительной информации
                    let res = self.resolver.resolve_expression_sync(&hint_resolution.type_name());
                    // Обновляем тип переменной в контексте
                    context.update_variable_type(name.clone(), hint_resolution.clone());
                    res
                } else {
                    // Без hint - Unknown
                    TypeResolution::unknown()
                }
            }

            SemanticNodeKind::Assignment {
                variable,
                value_type,
                ..
            } => {
                // Phase 3: value_type уже TypeResolution
                // Обновляем тип переменной в контексте
                context.update_variable_type(variable.clone(), value_type.clone());
                // Phase 3: Возвращаем value_type напрямую (уже TypeResolution)
                value_type.clone()
            }

            SemanticNodeKind::MemberAccess {
                object_type,
                member_name,
                ..
            } => {
                // Phase 3: object_type теперь TypeResolution
                // Резолвим доступ к члену: object_type.member_name
                let full_path = format!("{}.{}", object_type.type_name(), member_name);
                self.resolver.resolve_expression_sync(&full_path)
            }

            SemanticNodeKind::FunctionCall { function_name, .. } => {
                // Поиск функции в символах или встроенных
                self.resolver.resolve_expression_sync(function_name)
            }

            SemanticNodeKind::IfStatement { condition_type, .. }
            | SemanticNodeKind::WhileLoop { condition_type, .. } => {
                // Phase 3: condition_type теперь TypeResolution
                // Условия должны быть Boolean
                self.resolver.resolve_expression_sync(&condition_type.type_name())
            }

            SemanticNodeKind::ForLoop { range_type, .. } => {
                // Phase 3: range_type теперь TypeResolution
                self.resolver.resolve_expression_sync(&range_type.type_name())
            }

            _ => {
                // Остальные узлы пока не требуют резолюции
                return;
            }
        };

        self.resolutions.insert(index, resolution);
    }
}

#[cfg(test)]
mod ir_analysis_tests {
    use super::*;
    use crate::parsing::MockParser;

    #[test]
    fn test_parse_and_analyze_with_mock() {
        let repo = Arc::new(InMemoryTypeRepository::new());
        let resolver = Arc::new(TypeResolver::new(repo.clone()));
        let engine = AnalysisEngine::new(resolver, repo);

        let parser = MockParser;
        let result = engine.parse_and_analyze(&parser, "", "test.bsl");

        assert!(result.is_ok());
        let analysis = result.unwrap();
        assert_eq!(analysis.ir.nodes.len(), 0);
        assert_eq!(analysis.type_resolutions.len(), 0);
    }

    #[test]
    fn test_ir_analysis_result_structure() {
        let repo = Arc::new(InMemoryTypeRepository::new());
        let resolver = Arc::new(TypeResolver::new(repo.clone()));
        let engine = AnalysisEngine::new(resolver, repo);

        let parser = MockParser;
        let result = engine.parse_and_analyze(&parser, "", "test.bsl").unwrap();

        // Проверяем структуру результата
        assert_eq!(result.ir.source_info.path, "test.bsl");
    }

    /// Milestone 2.3: Интеграционный тест для Union Types в IR анализе
    #[test]
    fn test_union_type_in_ir_analysis() {
        use crate::domain::types::{Certainty, ResolutionResult};
        use crate::ir::{SemanticNode, SemanticNodeKind, SemanticProgram, SourceInfo, Span};

        let repo = Arc::new(InMemoryTypeRepository::new());
        let resolver = Arc::new(TypeResolver::new(repo.clone()));
        let engine = AnalysisEngine::new(resolver.clone(), repo);

        // Создаем IR с Union type hint
        let symbols = crate::ir::SymbolTable::default();
        let root_scope = symbols.root_scope;

        let ir = SemanticProgram {
            nodes: vec![SemanticNode {
                kind: SemanticNodeKind::VariableDeclaration {
                    name: "МояПеременная".to_string(),
                    // Phase 3: type_hint теперь Option<TypeResolution>
                    type_hint: Some(TypeResolution::explicit("Строка | Число")),
                    is_export: false,
                    initial_value_type: None,
                },
                span: Span {
                    start_line: 1,
                    start_column: 1,
                    end_line: 1,
                    end_column: 30,
                },
                scope_id: root_scope,
            }],
            symbols,
            cfg: None,
            source_info: SourceInfo {
                path: "test_union.bsl".to_string(),
                content_hash: 0,
            },
        };

        // Анализируем IR
        let result = engine.analyze_ir(&ir);
        assert!(result.is_ok());

        let resolutions = result.unwrap();
        assert_eq!(resolutions.len(), 1);

        // Проверяем, что разрешился Union тип
        let resolution = resolutions.get(&0).unwrap();
        assert!(matches!(resolution.certainty, Certainty::Known));

        match &resolution.result {
            ResolutionResult::Union(types) => {
                assert_eq!(types.len(), 2, "Union должен содержать 2 типа");
            }
            _ => panic!("Ожидался Union тип, получен {:?}", resolution.result),
        }
    }

    /// Milestone 2.3: Тест для assignment compatibility с Union в IR
    #[test]
    fn test_union_assignment_in_ir() {
        let repo = Arc::new(InMemoryTypeRepository::new());
        let resolver = Arc::new(TypeResolver::new(repo.clone()));

        // Проверяем совместимость через resolver напрямую
        let string_type = resolver.resolve_expression_sync("Строка");
        let union_type = resolver.resolve_expression_sync("Строка | Число");

        // String должен быть совместим с Union(String | Number)
        assert!(
            resolver.is_assignment_compatible(&string_type, &union_type),
            "String должен быть совместим с Union(String | Number)"
        );
    }
}
