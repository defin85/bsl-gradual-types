//! Граф зависимостей типов для анализа связей между элементами кода

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, RwLock};

/// Узел в графе зависимостей
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DependencyNode {
    /// Переменная (глобальная или локальная)
    Variable { 
        name: String,
        scope: Scope,
    },
    /// Функция или процедура
    Function {
        name: String,
        exported: bool,
    },
    /// Параметр функции
    Parameter {
        function: String,
        name: String,
    },
    /// Возвращаемое значение функции
    ReturnValue {
        function: String,
    },
    /// Поле объекта
    Field {
        object: String,
        field: String,
    },
    /// Метод объекта
    Method {
        object: String,
        method: String,
    },
}

/// Область видимости
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Scope {
    Global,
    Module(String),
    Function(String),
    Local { function: String, block_id: usize },
}

/// Тип зависимости между узлами
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DependencyType {
    /// Прямое присваивание: target = source
    Assignment,
    /// Передача в качестве параметра
    Parameter(usize), // индекс параметра
    /// Возврат из функции
    Return,
    /// Доступ к полю
    FieldAccess,
    /// Вызов метода
    MethodCall,
    /// Использование в выражении
    Expression,
    /// Условная зависимость (в if/case)
    Conditional,
}

/// Ребро графа зависимостей
#[derive(Debug, Clone)]
pub struct DependencyEdge {
    pub from: DependencyNode,
    pub to: DependencyNode,
    pub dep_type: DependencyType,
    /// Позиция в исходном коде
    pub location: Option<SourceLocation>,
}

/// Позиция в исходном коде
#[derive(Debug, Clone)]
pub struct SourceLocation {
    pub file: String,
    pub line: usize,
    pub column: usize,
}

/// Граф зависимостей типов
pub struct TypeDependencyGraph {
    /// Узлы графа
    nodes: Arc<RwLock<HashSet<DependencyNode>>>,
    /// Рёбра графа (от узла к списку исходящих рёбер)
    edges: Arc<RwLock<HashMap<DependencyNode, Vec<DependencyEdge>>>>,
    /// Обратные рёбра (от узла к списку входящих рёбер)
    reverse_edges: Arc<RwLock<HashMap<DependencyNode, Vec<DependencyEdge>>>>,
}

impl TypeDependencyGraph {
    /// Создание нового графа
    pub fn new() -> Self {
        Self {
            nodes: Arc::new(RwLock::new(HashSet::new())),
            edges: Arc::new(RwLock::new(HashMap::new())),
            reverse_edges: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Добавление узла в граф
    pub fn add_node(&self, node: DependencyNode) {
        let mut nodes = self.nodes.write().unwrap();
        nodes.insert(node);
    }
    
    /// Добавление ребра в граф
    pub fn add_edge(&self, edge: DependencyEdge) {
        // Добавляем узлы если их ещё нет
        {
            let mut nodes = self.nodes.write().unwrap();
            nodes.insert(edge.from.clone());
            nodes.insert(edge.to.clone());
        }
        
        // Добавляем прямое ребро
        {
            let mut edges = self.edges.write().unwrap();
            edges.entry(edge.from.clone())
                .or_default()
                .push(edge.clone());
        }
        
        // Добавляем обратное ребро
        {
            let mut reverse = self.reverse_edges.write().unwrap();
            reverse.entry(edge.to.clone())
                .or_default()
                .push(edge);
        }
    }
    
    /// Получение всех узлов, от которых зависит данный узел
    pub fn get_dependencies(&self, node: &DependencyNode) -> Vec<DependencyNode> {
        let edges = self.edges.read().unwrap();
        edges.get(node)
            .map(|deps| deps.iter().map(|e| e.to.clone()).collect())
            .unwrap_or_default()
    }
    
    /// Получение всех узлов, которые зависят от данного узла
    pub fn get_dependents(&self, node: &DependencyNode) -> Vec<DependencyNode> {
        let reverse = self.reverse_edges.read().unwrap();
        reverse.get(node)
            .map(|deps| deps.iter().map(|e| e.from.clone()).collect())
            .unwrap_or_default()
    }
    
    /// Поиск пути между двумя узлами (BFS)
    pub fn find_path(&self, from: &DependencyNode, to: &DependencyNode) -> Option<Vec<DependencyNode>> {
        let edges = self.edges.read().unwrap();
        
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        let mut parents: HashMap<DependencyNode, DependencyNode> = HashMap::new();
        
        queue.push_back(from.clone());
        visited.insert(from.clone());
        
        while let Some(current) = queue.pop_front() {
            if current == *to {
                // Восстанавливаем путь
                let mut path = vec![current.clone()];
                let mut node = current;
                
                while let Some(parent) = parents.get(&node) {
                    path.push(parent.clone());
                    node = parent.clone();
                }
                
                path.reverse();
                return Some(path);
            }
            
            if let Some(neighbors) = edges.get(&current) {
                for edge in neighbors {
                    if !visited.contains(&edge.to) {
                        visited.insert(edge.to.clone());
                        parents.insert(edge.to.clone(), current.clone());
                        queue.push_back(edge.to.clone());
                    }
                }
            }
        }
        
        None
    }
    
    /// Обнаружение циклов в графе
    pub fn find_cycles(&self) -> Vec<Vec<DependencyNode>> {
        let nodes = self.nodes.read().unwrap();
        let edges = self.edges.read().unwrap();
        
        let mut cycles = Vec::new();
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        let mut path = Vec::new();
        
        for node in nodes.iter() {
            if !visited.contains(node) {
                self.dfs_find_cycles(
                    node,
                    &edges,
                    &mut visited,
                    &mut rec_stack,
                    &mut path,
                    &mut cycles,
                );
            }
        }
        
        cycles
    }
    
    /// DFS для поиска циклов
    fn dfs_find_cycles(
        &self,
        node: &DependencyNode,
        edges: &HashMap<DependencyNode, Vec<DependencyEdge>>,
        visited: &mut HashSet<DependencyNode>,
        rec_stack: &mut HashSet<DependencyNode>,
        path: &mut Vec<DependencyNode>,
        cycles: &mut Vec<Vec<DependencyNode>>,
    ) {
        visited.insert(node.clone());
        rec_stack.insert(node.clone());
        path.push(node.clone());
        
        if let Some(neighbors) = edges.get(node) {
            for edge in neighbors {
                if !visited.contains(&edge.to) {
                    self.dfs_find_cycles(
                        &edge.to,
                        edges,
                        visited,
                        rec_stack,
                        path,
                        cycles,
                    );
                } else if rec_stack.contains(&edge.to) {
                    // Нашли цикл
                    if let Some(start_idx) = path.iter().position(|n| n == &edge.to) {
                        cycles.push(path[start_idx..].to_vec());
                    }
                }
            }
        }
        
        path.pop();
        rec_stack.remove(node);
    }
    
    /// Топологическая сортировка (возвращает None если есть циклы)
    pub fn topological_sort(&self) -> Option<Vec<DependencyNode>> {
        let nodes = self.nodes.read().unwrap();
        let edges = self.edges.read().unwrap();
        let reverse_edges = self.reverse_edges.read().unwrap();
        
        // Подсчитываем входящие рёбра для каждого узла
        let mut in_degree: HashMap<DependencyNode, usize> = HashMap::new();
        for node in nodes.iter() {
            in_degree.insert(
                node.clone(),
                reverse_edges.get(node).map(|v| v.len()).unwrap_or(0),
            );
        }
        
        // Находим узлы без входящих рёбер
        let mut queue: VecDeque<DependencyNode> = in_degree
            .iter()
            .filter(|(_, &degree)| degree == 0)
            .map(|(node, _)| node.clone())
            .collect();
        
        let mut sorted = Vec::new();
        
        while let Some(node) = queue.pop_front() {
            sorted.push(node.clone());
            
            if let Some(neighbors) = edges.get(&node) {
                for edge in neighbors {
                    let degree = in_degree.get_mut(&edge.to).unwrap();
                    *degree -= 1;
                    
                    if *degree == 0 {
                        queue.push_back(edge.to.clone());
                    }
                }
            }
        }
        
        // Если отсортировали все узлы, значит циклов нет
        if sorted.len() == nodes.len() {
            Some(sorted)
        } else {
            None
        }
    }
    
    /// Получение всех функций, которые вызывает данная функция
    pub fn get_called_functions(&self, function_name: &str) -> Vec<String> {
        let function_node = DependencyNode::Function {
            name: function_name.to_string(),
            exported: false, // не важно для поиска
        };
        
        let edges = self.edges.read().unwrap();
        edges.get(&function_node)
            .map(|deps| {
                deps.iter()
                    .filter_map(|e| match &e.to {
                        DependencyNode::Function { name, .. } => Some(name.clone()),
                        _ => None,
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
    
    /// Анализ достижимости: какие узлы достижимы из данного
    pub fn get_reachable_nodes(&self, start: &DependencyNode) -> HashSet<DependencyNode> {
        let edges = self.edges.read().unwrap();
        
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        
        queue.push_back(start.clone());
        visited.insert(start.clone());
        
        while let Some(current) = queue.pop_front() {
            if let Some(neighbors) = edges.get(&current) {
                for edge in neighbors {
                    if !visited.contains(&edge.to) {
                        visited.insert(edge.to.clone());
                        queue.push_back(edge.to.clone());
                    }
                }
            }
        }
        
        visited
    }
}

impl Default for TypeDependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_add_nodes_and_edges() {
        let graph = TypeDependencyGraph::new();
        
        let var_a = DependencyNode::Variable {
            name: "А".to_string(),
            scope: Scope::Global,
        };
        
        let var_b = DependencyNode::Variable {
            name: "Б".to_string(),
            scope: Scope::Global,
        };
        
        graph.add_edge(DependencyEdge {
            from: var_a.clone(),
            to: var_b.clone(),
            dep_type: DependencyType::Assignment,
            location: None,
        });
        
        let deps = graph.get_dependencies(&var_a);
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0], var_b);
        
        let dependents = graph.get_dependents(&var_b);
        assert_eq!(dependents.len(), 1);
        assert_eq!(dependents[0], var_a);
    }
    
    #[test]
    fn test_find_path() {
        let graph = TypeDependencyGraph::new();
        
        let var_a = DependencyNode::Variable {
            name: "А".to_string(),
            scope: Scope::Global,
        };
        
        let var_b = DependencyNode::Variable {
            name: "Б".to_string(),
            scope: Scope::Global,
        };
        
        let var_c = DependencyNode::Variable {
            name: "В".to_string(),
            scope: Scope::Global,
        };
        
        // А -> Б -> В
        graph.add_edge(DependencyEdge {
            from: var_a.clone(),
            to: var_b.clone(),
            dep_type: DependencyType::Assignment,
            location: None,
        });
        
        graph.add_edge(DependencyEdge {
            from: var_b.clone(),
            to: var_c.clone(),
            dep_type: DependencyType::Assignment,
            location: None,
        });
        
        let path = graph.find_path(&var_a, &var_c);
        assert!(path.is_some());
        
        let path = path.unwrap();
        assert_eq!(path.len(), 3);
        assert_eq!(path[0], var_a);
        assert_eq!(path[1], var_b);
        assert_eq!(path[2], var_c);
    }
    
    #[test]
    fn test_detect_cycle() {
        let graph = TypeDependencyGraph::new();
        
        let var_a = DependencyNode::Variable {
            name: "А".to_string(),
            scope: Scope::Global,
        };
        
        let var_b = DependencyNode::Variable {
            name: "Б".to_string(),
            scope: Scope::Global,
        };
        
        let var_c = DependencyNode::Variable {
            name: "В".to_string(),
            scope: Scope::Global,
        };
        
        // Создаём цикл: А -> Б -> В -> А
        graph.add_edge(DependencyEdge {
            from: var_a.clone(),
            to: var_b.clone(),
            dep_type: DependencyType::Assignment,
            location: None,
        });
        
        graph.add_edge(DependencyEdge {
            from: var_b.clone(),
            to: var_c.clone(),
            dep_type: DependencyType::Assignment,
            location: None,
        });
        
        graph.add_edge(DependencyEdge {
            from: var_c.clone(),
            to: var_a.clone(),
            dep_type: DependencyType::Assignment,
            location: None,
        });
        
        let cycles = graph.find_cycles();
        assert!(!cycles.is_empty());
        
        // Топологическая сортировка должна вернуть None при наличии циклов
        let sorted = graph.topological_sort();
        assert!(sorted.is_none());
    }
}