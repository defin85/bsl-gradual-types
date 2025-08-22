use super::ast::Query;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Пакетный запрос в 1С - может содержать несколько связанных или независимых запросов
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BatchQuery {
    /// Отдельные запросы в пакете
    pub queries: Vec<QueryPart>,
    /// Временные таблицы, созданные в пакете (имя -> индекс запроса, где создана)
    pub temp_tables: HashMap<String, usize>,
    /// Является ли пакет связанным (использует временные таблицы между запросами)
    pub is_connected: bool,
}

/// Часть пакетного запроса
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueryPart {
    /// Сам запрос
    pub query: Query,
    /// Индекс в пакете (0-based)
    pub index: usize,
    /// Временные таблицы, которые создаёт этот запрос
    pub creates_temp_tables: Vec<String>,
    /// Временные таблицы, которые использует этот запрос
    pub uses_temp_tables: Vec<String>,
    /// Уничтожает ли временную таблицу после использования
    pub drops_temp_table: Option<String>,
}

impl BatchQuery {
    /// Создаёт новый пакетный запрос из списка запросов
    pub fn from_queries(queries: Vec<Query>) -> Self {
        let mut temp_tables = HashMap::new();
        let mut query_parts = Vec::new();
        let mut is_connected = false;

        for (index, query) in queries.into_iter().enumerate() {
            let mut creates_temp_tables = Vec::new();
            let mut uses_temp_tables = Vec::new();
            let drops_temp_table = None; // TODO: парсить УНИЧТОЖИТЬ

            // Проверяем, создаёт ли запрос временную таблицу
            if let Some(temp_table) = &query.select_clause.into_temp_table {
                creates_temp_tables.push(temp_table.clone());
                temp_tables.insert(temp_table.clone(), index);
            }

            // Проверяем, использует ли запрос временные таблицы
            // Это упрощённая проверка - нужно анализировать FROM и JOIN
            for source in &query.from_clause.sources {
                if let super::ast::TableReference::Table(name) = &source.table {
                    if temp_tables.contains_key(name) {
                        uses_temp_tables.push(name.clone());
                        is_connected = true;
                    }
                }
            }

            query_parts.push(QueryPart {
                query,
                index,
                creates_temp_tables,
                uses_temp_tables,
                drops_temp_table,
            });
        }

        BatchQuery {
            queries: query_parts,
            temp_tables,
            is_connected,
        }
    }

    /// Проверяет, можно ли выполнить запросы параллельно
    pub fn can_parallelize(&self) -> bool {
        !self.is_connected && self.queries.len() > 1
    }

    /// Возвращает группы запросов, которые можно выполнить параллельно
    pub fn get_parallel_groups(&self) -> Vec<Vec<usize>> {
        if !self.is_connected {
            // Все запросы независимы - каждый в своей группе
            self.queries.iter().map(|q| vec![q.index]).collect()
        } else {
            // Анализируем зависимости для группировки
            let mut groups = Vec::new();
            let mut current_group = Vec::new();
            let mut available_tables = HashMap::new();

            for query_part in &self.queries {
                // Если запрос использует временные таблицы, проверяем их доступность
                let can_execute = query_part
                    .uses_temp_tables
                    .iter()
                    .all(|table| available_tables.contains_key(table));

                if can_execute || query_part.uses_temp_tables.is_empty() {
                    current_group.push(query_part.index);

                    // Добавляем созданные таблицы в доступные
                    for table in &query_part.creates_temp_tables {
                        available_tables.insert(table.clone(), query_part.index);
                    }
                } else {
                    // Начинаем новую группу
                    if !current_group.is_empty() {
                        groups.push(current_group);
                        current_group = Vec::new();
                    }

                    current_group.push(query_part.index);
                    for table in &query_part.creates_temp_tables {
                        available_tables.insert(table.clone(), query_part.index);
                    }
                }
            }

            if !current_group.is_empty() {
                groups.push(current_group);
            }

            groups
        }
    }

    /// Возвращает порядок выполнения с учётом зависимостей
    pub fn get_execution_order(&self) -> Vec<usize> {
        if !self.is_connected {
            // Независимые запросы - выполняем по порядку
            (0..self.queries.len()).collect()
        } else {
            // Топологическая сортировка по зависимостям
            let mut visited = vec![false; self.queries.len()];
            let mut order = Vec::new();

            fn visit(
                index: usize,
                queries: &[QueryPart],
                temp_tables: &HashMap<String, usize>,
                visited: &mut [bool],
                order: &mut Vec<usize>,
            ) {
                if visited[index] {
                    return;
                }

                visited[index] = true;

                // Сначала посещаем зависимости
                for table in &queries[index].uses_temp_tables {
                    if let Some(&dep_index) = temp_tables.get(table) {
                        if dep_index < index && !visited[dep_index] {
                            visit(dep_index, queries, temp_tables, visited, order);
                        }
                    }
                }

                order.push(index);
            }

            for i in 0..self.queries.len() {
                visit(
                    i,
                    &self.queries,
                    &self.temp_tables,
                    &mut visited,
                    &mut order,
                );
            }

            order
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_independent_queries() {
        // Создаём два независимых запроса
        let query1 = Query {
            select_clause: super::super::ast::SelectClause {
                distinct: false,
                top: None,
                allowed: false,
                fields: vec![],
                into_temp_table: None,
            },
            from_clause: super::super::ast::FromClause { sources: vec![] },
            where_clause: None,
            group_by_clause: None,
            having_clause: None,
            order_by_clause: None,
            totals_clause: None,
            union_clause: None,
        };

        let query2 = query1.clone();

        let batch = BatchQuery::from_queries(vec![query1, query2]);

        assert!(!batch.is_connected);
        assert!(batch.can_parallelize());
        assert_eq!(batch.get_execution_order(), vec![0, 1]);
    }

    #[test]
    fn test_connected_queries() {
        // Создаём связанные запросы с временной таблицей
        let query1 = Query {
            select_clause: super::super::ast::SelectClause {
                distinct: false,
                top: None,
                allowed: false,
                fields: vec![],
                into_temp_table: Some("TempTable".to_string()),
            },
            from_clause: super::super::ast::FromClause { sources: vec![] },
            where_clause: None,
            group_by_clause: None,
            having_clause: None,
            order_by_clause: None,
            totals_clause: None,
            union_clause: None,
        };

        let query2 = Query {
            select_clause: super::super::ast::SelectClause {
                distinct: false,
                top: None,
                allowed: false,
                fields: vec![],
                into_temp_table: None,
            },
            from_clause: super::super::ast::FromClause {
                sources: vec![super::super::ast::TableSource {
                    table: super::super::ast::TableReference::Table("TempTable".to_string()),
                    alias: None,
                    joins: vec![],
                }],
            },
            where_clause: None,
            group_by_clause: None,
            having_clause: None,
            order_by_clause: None,
            totals_clause: None,
            union_clause: None,
        };

        let batch = BatchQuery::from_queries(vec![query1, query2]);

        assert!(batch.is_connected);
        assert!(!batch.can_parallelize());
        assert_eq!(batch.get_execution_order(), vec![0, 1]);
        assert_eq!(batch.temp_tables.get("TempTable"), Some(&0));
    }
}
