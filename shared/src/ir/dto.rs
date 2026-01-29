//! DTO конвертация для IR
//!
//! Методы to_dto() и связанные преобразования для передачи данных клиентам.

use std::collections::HashMap;

use crate::api::semantic_dtos::*;

use super::program::SemanticProgram;
use super::types::{SemanticNode, SemanticNodeKind};

impl SemanticProgram {
    /// Конвертировать SemanticProgram в SemanticTreeDto для передачи клиентам
    ///
    /// # Milestone 2.12: Real-time Semantic Tree Visualization
    ///
    /// Этот метод преобразует внутреннее представление IR в DTO,
    /// пригодное для передачи через LSP и Web API.
    pub fn to_dto(
        &self,
        include_call_graph: bool,
        include_flow_sensitive: bool,
    ) -> SemanticTreeDto {
        let start_time = std::time::Instant::now();

        // Конвертируем root-level узлы (только узлы в root scope!)
        let root_nodes = self
            .nodes
            .iter()
            .filter(|node| {
                // ИСПРАВЛЕНИЕ: Фильтруем по scope_id, а не по типу узла
                // Показываем только узлы, которые находятся в root scope
                node.scope_id == self.symbols.root_scope
            })
            .map(|node| self.node_to_dto(node, 0))
            .collect();

        // Конвертируем таблицу символов
        let symbol_table = self.symbols_to_dto(include_flow_sensitive);

        // Собираем граф вызовов (если requested)
        let call_graph = if include_call_graph {
            self.extract_call_graph()
        } else {
            Vec::new()
        };

        // Вычисляем метрики
        let metrics = self.calculate_metrics();

        let analysis_time_ms = start_time.elapsed().as_millis() as u64;

        SemanticTreeDto {
            file_path: self.source_info.path.clone(),
            root_nodes,
            symbol_table,
            call_graph,
            metrics: SemanticMetricsDto {
                analysis_time_ms,
                ..metrics
            },
            timestamp: Some(chrono::Utc::now().to_rfc3339()),
        }
    }

    /// Compact версия DTO без symbol_table и call_graph для уменьшения размера ответа
    ///
    /// Используется для быстрого просмотра структуры кода без детальной информации о символах.
    pub fn to_compact_dto(&self) -> SemanticTreeDto {
        let start_time = std::time::Instant::now();

        // Конвертируем root-level узлы (только узлы в root scope!)
        let root_nodes = self
            .nodes
            .iter()
            .filter(|node| node.scope_id == self.symbols.root_scope)
            .map(|node| self.node_to_dto(node, 0))
            .collect();

        // Вычисляем метрики
        let metrics = self.calculate_metrics();
        let analysis_time_ms = start_time.elapsed().as_millis() as u64;

        SemanticTreeDto {
            file_path: self.source_info.path.clone(),
            root_nodes,
            symbol_table: HashMap::new(), // Пустая в compact режиме
            call_graph: Vec::new(),       // Пустой в compact режиме
            metrics: SemanticMetricsDto {
                analysis_time_ms,
                ..metrics
            },
            timestamp: Some(chrono::Utc::now().to_rfc3339()),
        }
    }

    /// Конвертировать узел в DTO
    fn node_to_dto(&self, node: &SemanticNode, depth: usize) -> SemanticNodeDto {
        let (kind, name, attributes) = self.extract_node_info(&node.kind);

        // Рекурсивно конвертируем children (с ограничением глубины)
        let children = if depth < 10 {
            self.get_node_children(node, depth + 1)
        } else {
            Vec::new()
        };

        SemanticNodeDto {
            kind,
            name,
            location: SourceLocationDto {
                line: node.span.start_line,
                column: node.span.start_column,
            },
            range: Some(SourceRangeDto {
                start: SourceLocationDto {
                    line: node.span.start_line,
                    column: node.span.start_column,
                },
                end: SourceLocationDto {
                    line: node.span.end_line,
                    column: node.span.end_column,
                },
            }),
            children,
            attributes,
        }
    }

    /// Извлечь информацию из SemanticNodeKind
    fn extract_node_info(
        &self,
        kind: &SemanticNodeKind,
    ) -> (String, Option<String>, HashMap<String, String>) {
        let mut attributes = HashMap::new();

        match kind {
            SemanticNodeKind::VariableDeclaration {
                name,
                type_hint,
                is_export,
                ..
            } => {
                if let Some(hint) = type_hint {
                    attributes.insert("type".to_string(), hint.clone());
                }
                attributes.insert("is_export".to_string(), is_export.to_string());
                ("Variable".to_string(), Some(name.clone()), attributes)
            }
            SemanticNodeKind::VariableAccess { name } => {
                attributes.insert("name".to_string(), name.clone());
                ("VariableAccess".to_string(), Some(name.clone()), attributes)
            }
            SemanticNodeKind::FunctionDeclaration {
                name,
                params,
                ..
            } => {
                attributes.insert("parameter_count".to_string(), params.len().to_string());
                ("Function".to_string(), Some(name.clone()), attributes)
            }
            SemanticNodeKind::ProcedureDeclaration { name, params, .. } => {
                attributes.insert("parameter_count".to_string(), params.len().to_string());
                ("Procedure".to_string(), Some(name.clone()), attributes)
            }
            SemanticNodeKind::Assignment {
                variable,
                value_node,
            } => {
                attributes.insert("variable".to_string(), variable.clone());
                if let Some(vn) = value_node {
                    attributes.insert("value_node".to_string(), vn.to_string());
                }
                // Показываем имя переменной в UI
                (
                    "Assignment".to_string(),
                    Some(format!("{} =", variable)),
                    attributes,
                )
            }
            SemanticNodeKind::IfStatement { .. } => ("IfStatement".to_string(), None, attributes),
            SemanticNodeKind::ForLoop { .. } => ("ForLoop".to_string(), None, attributes),
            SemanticNodeKind::WhileLoop { .. } => ("WhileLoop".to_string(), None, attributes),
            SemanticNodeKind::FunctionCall {
                function_name,
                object_name,
                ..
            } => {
                attributes.insert("function_name".to_string(), function_name.clone());

                // MILESTONE 3.5: Показываем полное имя вызова (объект.метод)
                let display_name = if let Some(obj_name) = object_name {
                    attributes.insert("object_name".to_string(), obj_name.clone());
                    format!("{}.{}", obj_name, function_name)
                } else {
                    function_name.clone()
                };

                ("FunctionCall".to_string(), Some(display_name), attributes)
            }
            SemanticNodeKind::Return { .. } => ("Return".to_string(), None, attributes),
            SemanticNodeKind::TryExcept { .. } => ("TryExcept".to_string(), None, attributes),
            SemanticNodeKind::Break => ("Break".to_string(), None, attributes),
            SemanticNodeKind::Continue => ("Continue".to_string(), None, attributes),
            SemanticNodeKind::ForEachLoop { variable, .. } => {
                attributes.insert("variable".to_string(), variable.clone());
                ("ForEachLoop".to_string(), None, attributes)
            }
            SemanticNodeKind::MemberAccess {
                object_node,
                object_name,
                member_name,
                access_kind,
            } => {
                // object_node — индекс узла-объекта
                if let Some(node_idx) = object_node {
                    attributes.insert("object_node".to_string(), node_idx.to_string());
                }
                // object_name теперь Option<String>
                if let Some(name) = object_name {
                    attributes.insert("object_name".to_string(), name.clone());
                }
                attributes.insert("member_name".to_string(), member_name.clone());
                attributes.insert("access_kind".to_string(), format!("{:?}", access_kind));

                let description = object_name
                    .as_ref()
                    .map(|name| format!("{}.{}", name, member_name))
                    .unwrap_or_else(|| format!("<expr>.{}", member_name));

                ("MemberAccess".to_string(), Some(description), attributes)
            }
            SemanticNodeKind::BlockScope { .. } => ("BlockScope".to_string(), None, attributes),
            SemanticNodeKind::GlobalPropertyAccess { name } => {
                attributes.insert("name".to_string(), name.clone());
                (
                    "GlobalPropertyAccess".to_string(),
                    Some(name.clone()),
                    attributes,
                )
            }
            SemanticNodeKind::NewExpression {
                type_name,
                is_dynamic,
                generic_params,
            } => {
                attributes.insert("type_name".to_string(), type_name.clone());
                attributes.insert("is_dynamic".to_string(), is_dynamic.to_string());

                if let Some(params) = generic_params {
                    attributes.insert("generic_params".to_string(), params.join(", "));
                }

                // Форматируем имя для отображения в UI
                let display_name = if *is_dynamic {
                    format!("Новый(\"{}\")", type_name)
                } else {
                    format!("Новый {}", type_name)
                };

                ("NewExpression".to_string(), Some(display_name), attributes)
            }
        }
    }

    /// Получить дочерние узлы (для построения иерархии)
    fn get_node_children(&self, parent: &SemanticNode, depth: usize) -> Vec<SemanticNodeDto> {
        use SemanticNodeKind::*;

        // Извлекаем индексы дочерних узлов в зависимости от типа родителя
        let child_indices: Vec<usize> = match &parent.kind {
            // НОВОЕ: Извлекаем body из FunctionDeclaration
            FunctionDeclaration { body, .. } => body.clone(),

            // НОВОЕ: Извлекаем body из ProcedureDeclaration
            ProcedureDeclaration { body, .. } => body.clone(),

            // Существующие узлы с индексами
            IfStatement {
                then_branch,
                else_branch,
                ..
            } => {
                let mut indices = then_branch.clone();
                if let Some(else_idx) = else_branch {
                    indices.extend(else_idx);
                }
                indices
            }
            WhileLoop { body, .. } => body.clone(),
            ForLoop { body, .. } => body.clone(),
            ForEachLoop { body, .. } => body.clone(),
            TryExcept {
                try_body,
                except_body,
            } => {
                let mut indices = try_body.clone();
                indices.extend(except_body);
                indices
            }
            BlockScope { statements, .. } => statements.clone(),

            VariableDeclaration {
                initial_value_node,
                ..
            } => initial_value_node.iter().copied().collect(),

            // MILESTONE 3.5: Assignment может содержать вложенный FunctionCall
            Assignment { value_node, .. } => value_node.iter().copied().collect(),

            // MILESTONE 5.4: FunctionCall может содержать вложенный узел (цепочки методов)
            // Например: Справочники.Контрагенты.НайтиПоКоду().ПолучитьОбъект()
            FunctionCall { object_node, .. } => object_node.iter().copied().collect(),

            // MILESTONE 5.4: MemberAccess может содержать вложенный узел (цепочки доступа)
            // Например: Справочники.Контрагенты (GlobalPropertyAccess → MemberAccess)
            MemberAccess { object_node, .. } => object_node.iter().copied().collect(),

            Return { value_node } => value_node.iter().copied().collect(),

            // Листовые узлы (нет детей): GlobalPropertyAccess, VariableDeclaration, Return, Break, Continue и др.
            _ => Vec::new(),
        };

        // Конвертируем дочерние узлы в DTO (рекурсивно)
        child_indices
            .iter()
            .filter_map(|&idx| self.nodes.get(idx))
            .map(|node| self.node_to_dto(node, depth))
            .collect()
    }

    /// Конвертировать таблицу символов в DTO
    fn symbols_to_dto(&self, _include_flow_sensitive: bool) -> HashMap<String, SymbolInfoDto> {
        let mut result = HashMap::new();

        // Обходим все scopes используя публичное API
        for (_scope_id, scope) in self.symbols.iter_all_scopes() {
            for (var_name, var_state) in &scope.variables {
                let symbol = SymbolInfoDto {
                    name: var_name.clone(),
                    kind: "Variable".to_string(),
                    resolved_type: None,
                    scope: "Local".to_string(), // TODO: различать Global/Local
                    declaration_location: SourceLocationDto {
                        line: var_state.declaration_span.start_line,
                        column: var_state.declaration_span.start_column,
                    },
                    // TODO: flow-sensitive analysis (пока не реализовано)
                    flow_variants: Vec::new(),
                    metadata: {
                        let mut meta = HashMap::new();
                        meta.insert("initialized".to_string(), var_state.initialized.to_string());
                        meta
                    },
                };

                result.insert(var_name.clone(), symbol);
            }
        }

        // Добавляем функции используя публичное API
        for (fn_name, sig) in self.symbols.iter_functions() {
            let symbol = SymbolInfoDto {
                name: fn_name.clone(),
                kind: "Function".to_string(),
                resolved_type: None,
                scope: "Global".to_string(),
                declaration_location: SourceLocationDto { line: 0, column: 0 }, // TODO: store location
                flow_variants: Vec::new(),
                metadata: {
                    let mut meta = HashMap::new();
                    meta.insert("is_export".to_string(), sig.is_export.to_string());
                    meta.insert("parameter_count".to_string(), sig.params.len().to_string());
                    meta
                },
            };

            result.insert(fn_name.clone(), symbol);
        }

        for (proc_name, sig) in self.symbols.iter_procedures() {
            let symbol = SymbolInfoDto {
                name: proc_name.clone(),
                kind: "Procedure".to_string(),
                resolved_type: None,
                scope: "Global".to_string(),
                declaration_location: SourceLocationDto { line: 0, column: 0 }, // TODO: store location
                flow_variants: Vec::new(),
                metadata: {
                    let mut meta = HashMap::new();
                    meta.insert("is_export".to_string(), sig.is_export.to_string());
                    meta.insert("parameter_count".to_string(), sig.params.len().to_string());
                    meta
                },
            };

            result.insert(proc_name.clone(), symbol);
        }

        result
    }

    /// Извлечь граф вызовов функций
    fn extract_call_graph(&self) -> Vec<CallEdgeDto> {
        // TODO: Реализовать извлечение call graph из узлов
        // Для MVP возвращаем пустой граф
        Vec::new()
    }

    /// Вычислить метрики семантического анализа
    fn calculate_metrics(&self) -> SemanticMetricsDto {
        let mut procedure_count = 0;
        let mut function_count = 0;
        let mut variable_count = 0;
        let mut parameter_count = 0;

        // Подсчёт procedures и functions
        for node in &self.nodes {
            match &node.kind {
                SemanticNodeKind::ProcedureDeclaration { params, .. } => {
                    procedure_count += 1;
                    parameter_count += params.len();
                }
                SemanticNodeKind::FunctionDeclaration { params, .. } => {
                    function_count += 1;
                    parameter_count += params.len();
                }
                SemanticNodeKind::VariableDeclaration { .. } => variable_count += 1,
                _ => {}
            }
        }

        SemanticMetricsDto {
            procedure_count,
            function_count,
            variable_count,
            parameter_count,
            // TypeResolution удалён из IR; метрики уверенности типизации больше не вычисляются здесь.
            known_types: 0,
            inferred_types: 0,
            unknown_types: 0,
            average_certainty: 0.0,
            analysis_time_ms: 0, // Will be set by caller
            node_count: self.nodes.len(),
            tree_depth: self.calculate_tree_depth(),
            call_count: 0, // TODO: calculate
        }
    }

    /// Вычислить максимальную глубину дерева
    fn calculate_tree_depth(&self) -> usize {
        // TODO: Реализовать подсчёт глубины
        1
    }
}
