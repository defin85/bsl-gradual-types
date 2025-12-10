//! DTO конвертация для IR
//!
//! Методы to_dto() и связанные преобразования для передачи данных клиентам.

use std::collections::HashMap;

use crate::api::semantic_dtos::*;
use crate::domain::types::{Certainty, ResolutionResult, TypeResolution};

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
                // Phase 3: type_hint теперь Option<TypeResolution>
                if let Some(hint) = type_hint {
                    attributes.insert("type".to_string(), hint.type_name());
                }
                attributes.insert("is_export".to_string(), is_export.to_string());
                ("Variable".to_string(), Some(name.clone()), attributes)
            }
            SemanticNodeKind::FunctionDeclaration {
                name,
                params,
                return_type,
                ..
            } => {
                attributes.insert("parameter_count".to_string(), params.len().to_string());
                // Phase 3: return_type теперь Option<TypeResolution>
                if let Some(ret) = return_type {
                    attributes.insert("return_type".to_string(), ret.type_name());
                }
                ("Function".to_string(), Some(name.clone()), attributes)
            }
            SemanticNodeKind::ProcedureDeclaration { name, params, .. } => {
                attributes.insert("parameter_count".to_string(), params.len().to_string());
                ("Procedure".to_string(), Some(name.clone()), attributes)
            }
            SemanticNodeKind::Assignment {
                variable,
                value_type,
                value_node,
            } => {
                attributes.insert("variable".to_string(), variable.clone());
                // Phase 3: value_type теперь TypeResolution, используем type_name()
                attributes.insert("value_type".to_string(), value_type.type_name());
                if let Some(vn) = value_node {
                    attributes.insert("value_node".to_string(), vn.to_string());
                }
                // Показываем имя переменной в UI
                (
                    "Assignment".to_string(),
                    Some(format!("{} = {}", variable, value_type.type_name())),
                    attributes,
                )
            }
            SemanticNodeKind::IfStatement { .. } => ("IfStatement".to_string(), None, attributes),
            SemanticNodeKind::ForLoop { .. } => ("ForLoop".to_string(), None, attributes),
            SemanticNodeKind::WhileLoop { .. } => ("WhileLoop".to_string(), None, attributes),
            SemanticNodeKind::FunctionCall {
                function_name,
                object_name,
                arg_types,
                ..
            } => {
                attributes.insert("function_name".to_string(), function_name.clone());
                attributes.insert("arg_count".to_string(), arg_types.len().to_string());

                // MILESTONE 3.5: Показываем полное имя вызова (объект.метод)
                let display_name = if let Some(obj_name) = object_name {
                    attributes.insert("object_name".to_string(), obj_name.clone());
                    format!("{}.{}", obj_name, function_name)
                } else {
                    function_name.clone()
                };

                (
                    "FunctionCall".to_string(),
                    Some(display_name),
                    attributes,
                )
            }
            SemanticNodeKind::Return { .. } => ("Return".to_string(), None, attributes),
            SemanticNodeKind::TryExcept { .. } => ("TryExcept".to_string(), None, attributes),
            SemanticNodeKind::Break => ("Break".to_string(), None, attributes),
            SemanticNodeKind::Continue => ("Continue".to_string(), None, attributes),
            SemanticNodeKind::ForEachLoop {
                variable,
                collection_type,
                ..
            } => {
                attributes.insert("variable".to_string(), variable.clone());
                // Phase 3: collection_type теперь TypeResolution
                attributes.insert("collection_type".to_string(), collection_type.type_name());
                ("ForEachLoop".to_string(), None, attributes)
            }
            SemanticNodeKind::MemberAccess {
                object_node,
                object_name,
                object_type,
                member_name,
                access_kind,
                result_type,
            } => {
                // object_node — индекс узла-объекта
                if let Some(node_idx) = object_node {
                    attributes.insert("object_node".to_string(), node_idx.to_string());
                }
                // object_name теперь Option<String>
                if let Some(name) = object_name {
                    attributes.insert("object_name".to_string(), name.clone());
                }
                // Phase 3: object_type теперь TypeResolution
                attributes.insert("object_type".to_string(), object_type.type_name());
                attributes.insert("member_name".to_string(), member_name.clone());
                attributes.insert("access_kind".to_string(), format!("{:?}", access_kind));
                // НОВОЕ: result_type — тип результата доступа
                attributes.insert("result_type".to_string(), result_type.type_name());

                let description = object_name
                    .as_ref()
                    .map(|name| format!("{}.{}", name, member_name))
                    .unwrap_or_else(|| format!("<expr>.{}", member_name));

                ("MemberAccess".to_string(), Some(description), attributes)
            }
            SemanticNodeKind::BlockScope { .. } => ("BlockScope".to_string(), None, attributes),
            SemanticNodeKind::GlobalPropertyAccess { name, result_type } => {
                attributes.insert("name".to_string(), name.clone());
                attributes.insert("result_type".to_string(), result_type.type_name());
                (
                    "GlobalPropertyAccess".to_string(),
                    Some(name.clone()),
                    attributes,
                )
            }
            SemanticNodeKind::NewExpression {
                type_name,
                arg_types,
                is_dynamic,
                result_type,
                generic_params,
            } => {
                attributes.insert("type_name".to_string(), type_name.clone());
                attributes.insert("arg_count".to_string(), arg_types.len().to_string());
                attributes.insert("is_dynamic".to_string(), is_dynamic.to_string());
                // Phase 3: result_type теперь TypeResolution
                attributes.insert("result_type".to_string(), result_type.type_name());

                if let Some(params) = generic_params {
                    attributes.insert("generic_params".to_string(), params.join(", "));
                }

                // Форматируем имя для отображения в UI
                let display_name = if *is_dynamic {
                    format!("Новый(\"{}\")", type_name)
                } else if arg_types.is_empty() {
                    format!("Новый {}", type_name)
                } else {
                    format!("Новый {}({} args)", type_name, arg_types.len())
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

            // MILESTONE 3.5: Assignment может содержать вложенный FunctionCall
            Assignment { value_node, .. } => value_node.iter().copied().collect(),

            // MILESTONE 5.4: FunctionCall может содержать вложенный узел (цепочки методов)
            // Например: Справочники.Контрагенты.НайтиПоКоду().ПолучитьОбъект()
            FunctionCall { object_node, .. } => object_node.iter().copied().collect(),

            // MILESTONE 5.4: MemberAccess может содержать вложенный узел (цепочки доступа)
            // Например: Справочники.Контрагенты (GlobalPropertyAccess → MemberAccess)
            MemberAccess { object_node, .. } => object_node.iter().copied().collect(),

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
                    resolved_type: self.type_resolution_to_dto(&var_state.resolution),
                    scope: "Local".to_string(), // TODO: различать Global/Local
                    declaration_location: SourceLocationDto {
                        line: var_state.declaration_span.start_line,
                        column: var_state.declaration_span.start_column,
                    },
                    // TODO: flow-sensitive analysis (пока не реализовано)
                    flow_variants: Vec::new(),
                    metadata: HashMap::new(),
                };

                result.insert(var_name.clone(), symbol);
            }
        }

        // Добавляем функции используя публичное API
        // Phase 3: return_type теперь Option<TypeResolution>
        for (fn_name, sig) in self.symbols.iter_functions() {
            let symbol = SymbolInfoDto {
                name: fn_name.clone(),
                kind: if sig.return_type.is_some() {
                    "Function".to_string()
                } else {
                    "Procedure".to_string()
                },
                resolved_type: sig.return_type.as_ref().map(|rt| TypeResolutionDto {
                    name: rt.type_name(), // Phase 3: используем type_name()
                    category: "Unknown".to_string(),
                    certainty: "Inferred".to_string(),
                    certainty_percent: 50,
                    active_facet: None,
                    methods: Vec::new(),
                    properties: Vec::new(),
                    is_union: None,
                    union_components: Vec::new(),
                }),
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

        result
    }

    /// Конвертировать TypeResolution в TypeResolutionDto
    fn type_resolution_to_dto(&self, resolution: &TypeResolution) -> Option<TypeResolutionDto> {
        if matches!(resolution.certainty, Certainty::Unknown) {
            return None;
        }

        let type_name = resolution.type_name();

        let (category, certainty_str, certainty_percent) = match &resolution.certainty {
            Certainty::Known => ("Platform".to_string(), "Known".to_string(), 100u8),
            Certainty::Inferred(conf) => {
                let percent = (*conf * 100.0) as u8;
                (
                    if matches!(resolution.result, ResolutionResult::Generic(_)) {
                        "Generic".to_string()
                    } else {
                        "Inferred".to_string()
                    },
                    if *conf > 0.8 {
                        "Known".to_string()
                    } else {
                        "Inferred".to_string()
                    },
                    percent,
                )
            }
            Certainty::Unknown => return None,
        };

        Some(TypeResolutionDto {
            name: type_name,
            category,
            certainty: certainty_str,
            certainty_percent,
            active_facet: None,
            methods: Vec::new(),
            properties: Vec::new(),
            is_union: None,
            union_components: Vec::new(),
        })
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
        let mut known_types = 0;
        let mut inferred_types = 0;
        let mut unknown_types = 0;

        // Подсчёт procedures и functions
        for node in &self.nodes {
            match &node.kind {
                SemanticNodeKind::ProcedureDeclaration { .. } => procedure_count += 1,
                SemanticNodeKind::FunctionDeclaration { .. } => function_count += 1,
                SemanticNodeKind::VariableDeclaration { .. } => variable_count += 1,
                _ => {}
            }
        }

        // Подсчёт типов
        for scope in self.symbols.scopes.values() {
            for var_state in scope.variables.values() {
                match &var_state.resolution.certainty {
                    Certainty::Known => known_types += 1,
                    Certainty::Inferred(conf) => {
                        if *conf > 0.8 {
                            known_types += 1;
                        } else {
                            inferred_types += 1;
                        }
                    }
                    Certainty::Unknown => unknown_types += 1,
                }
            }
        }

        let total_types = known_types + inferred_types + unknown_types;
        let average_certainty = if total_types > 0 {
            (known_types as f32 + inferred_types as f32 * 0.75) / total_types as f32
        } else {
            0.0
        };

        SemanticMetricsDto {
            procedure_count,
            function_count,
            variable_count,
            parameter_count: 0, // TODO: calculate
            known_types,
            inferred_types,
            unknown_types,
            average_certainty,
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
