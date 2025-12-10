//! Тесты для SymbolTable

use crate::domain::types::TypeResolution;
use crate::ir::{FunctionSignature, ScopeId, ScopeKind, Span, SymbolTable};

#[test]
fn test_symbol_table_creation() {
    let table = SymbolTable::new();

    assert_eq!(table.scopes.len(), 1); // Только root scope
    assert_eq!(table.root_scope, ScopeId(0));
    assert!(table.global_functions.is_empty());
}

#[test]
fn test_scope_hierarchy() {
    let mut table = SymbolTable::new();

    let child1 = table.create_scope(table.root_scope);
    let child2 = table.create_scope(table.root_scope);
    let grandchild = table.create_scope(child1);

    assert_eq!(table.scopes.len(), 4); // root + 2 children + 1 grandchild

    // Проверяем родительские связи
    assert_eq!(table.scopes[&child1].parent, Some(ScopeId(0)));
    assert_eq!(table.scopes[&grandchild].parent, Some(child1));

    // Проверяем дочерние связи
    let root = &table.scopes[&table.root_scope];
    assert_eq!(root.children.len(), 2);
    assert!(root.children.contains(&child1));
    assert!(root.children.contains(&child2));
}

#[test]
fn test_lookup_variable_found() {
    let mut table = SymbolTable::new();

    table.register_variable(
        table.root_scope,
        "x".to_string(),
        TypeResolution::explicit("Число"),
        Span::stub(),
    );

    let hint = table.lookup_variable(table.root_scope, "x");
    assert!(hint.is_some());

    assert_eq!(hint.unwrap().type_name(), "Число");
}

#[test]
fn test_lookup_variable_not_found() {
    let table = SymbolTable::new();

    let hint = table.lookup_variable(table.root_scope, "nonexistent");
    assert!(hint.is_none());
}

#[test]
fn test_lookup_variable_in_hierarchy_finds_in_current_scope() {
    let mut table = SymbolTable::new();

    table.register_variable(
        table.root_scope,
        "x".to_string(),
        TypeResolution::explicit("Строка"),
        Span::stub(),
    );

    let result = table.lookup_variable_in_hierarchy(table.root_scope, "x");
    assert!(result.is_some());

    let (scope_id, hint) = result.unwrap();
    assert_eq!(scope_id, table.root_scope);
    assert_eq!(hint.type_name(), "Строка");
}

#[test]
fn test_lookup_variable_in_hierarchy_finds_in_parent_scope() {
    let mut table = SymbolTable::new();

    // Регистрируем в root scope
    table.register_variable(
        table.root_scope,
        "global_var".to_string(),
        TypeResolution::explicit("Число"),
        Span::stub(),
    );

    // Создаём child scope
    let child = table.create_scope(table.root_scope);

    // Ищем из child scope
    let result = table.lookup_variable_in_hierarchy(child, "global_var");
    assert!(result.is_some());

    let (found_scope, hint) = result.unwrap();
    assert_eq!(found_scope, table.root_scope);
    assert_eq!(hint.type_name(), "Число");
}

#[test]
fn test_lookup_variable_in_hierarchy_shadow_local_over_parent() {
    let mut table = SymbolTable::new();

    // Регистрируем в root scope
    table.register_variable(
        table.root_scope,
        "x".to_string(),
        TypeResolution::explicit("Число"),
        Span::stub(),
    );

    // Создаём child scope
    let child = table.create_scope(table.root_scope);

    // Регистрируем с тем же именем в child scope
    table.register_variable(
        child,
        "x".to_string(),
        TypeResolution::explicit("Строка"),
        Span::stub(),
    );

    // Ищем из child scope - должны найти локальную переменную
    let result = table.lookup_variable_in_hierarchy(child, "x");
    assert!(result.is_some());

    let (found_scope, hint) = result.unwrap();
    assert_eq!(found_scope, child);
    assert_eq!(hint.type_name(), "Строка");
}

#[test]
fn test_lookup_variable_in_hierarchy_deeply_nested() {
    let mut table = SymbolTable::new();

    // Создаём иерархию scope: root -> level1 -> level2 -> level3
    let level1 = table.create_scope(table.root_scope);
    let level2 = table.create_scope(level1);
    let level3 = table.create_scope(level2);

    // Регистрируем переменную в level1
    table.register_variable(
        level1,
        "mid_var".to_string(),
        TypeResolution::explicit("Булево"),
        Span::stub(),
    );

    // Ищем из level3
    let result = table.lookup_variable_in_hierarchy(level3, "mid_var");
    assert!(result.is_some());

    let (found_scope, _) = result.unwrap();
    assert_eq!(found_scope, level1);
}

#[test]
fn test_lookup_variable_in_hierarchy_not_found() {
    let mut table = SymbolTable::new();
    let child = table.create_scope(table.root_scope);

    let result = table.lookup_variable_in_hierarchy(child, "nonexistent");
    assert!(result.is_none());
}

#[test]
fn test_update_variable_type_checked_success() {
    let mut table = SymbolTable::new();

    table.register_variable(
        table.root_scope,
        "x".to_string(),
        TypeResolution::explicit("Число"),
        Span::stub(),
    );

    // Обновляем тип
    let result = table.update_variable_type_checked(
        table.root_scope,
        "x",
        TypeResolution::explicit("Строка"),
    );

    assert!(result.is_ok());

    // Проверяем что тип обновлён
    let hint = table.lookup_variable(table.root_scope, "x");
    assert!(hint.is_some());
    assert_eq!(hint.unwrap().type_name(), "Строка");
}

#[test]
fn test_update_variable_type_checked_invalid_scope() {
    let mut table = SymbolTable::new();

    let invalid_scope = ScopeId(9999);

    let result = table.update_variable_type_checked(
        invalid_scope,
        "x",
        TypeResolution::explicit("Число"),
    );

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Scope"));
}

#[test]
fn test_update_variable_type_checked_nonexistent_variable() {
    let mut table = SymbolTable::new();

    let result = table.update_variable_type_checked(
        table.root_scope,
        "nonexistent",
        TypeResolution::explicit("Число"),
    );

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not found"));
}

#[test]
fn test_has_variable_true() {
    let mut table = SymbolTable::new();

    table.register_variable(
        table.root_scope,
        "x".to_string(),
        TypeResolution::explicit("Число"),
        Span::stub(),
    );

    assert!(table.has_variable(table.root_scope, "x"));
}

#[test]
fn test_has_variable_false() {
    let table = SymbolTable::new();

    assert!(!table.has_variable(table.root_scope, "nonexistent"));
}

#[test]
fn test_has_variable_different_scope() {
    let mut table = SymbolTable::new();
    let child = table.create_scope(table.root_scope);

    table.register_variable(
        table.root_scope,
        "x".to_string(),
        TypeResolution::explicit("Число"),
        Span::stub(),
    );

    // has_variable проверяет только конкретный scope, не иерархию
    assert!(table.has_variable(table.root_scope, "x"));
    assert!(!table.has_variable(child, "x"));
}

#[test]
fn test_find_function_found() {
    let mut table = SymbolTable::new();

    // Phase 3: return_type теперь Option<TypeResolution>
    let sig = FunctionSignature {
        name: "МояФункция".to_string(),
        params: vec![],
        return_type: Some(TypeResolution::explicit("Число")),
        is_export: false,
    };

    table.register_function(sig.clone());

    let found = table.find_function("МояФункция");
    assert!(found.is_some());
    assert_eq!(found.unwrap().name, "МояФункция");
    // Phase 3: сравниваем type_name()
    assert_eq!(
        found.unwrap().return_type.as_ref().map(|r| r.type_name()),
        Some("Число".to_string())
    );
}

#[test]
fn test_find_function_not_found() {
    let table = SymbolTable::new();

    let found = table.find_function("NonExistentFunction");
    assert!(found.is_none());
}

#[test]
fn test_find_function_case_insensitive() {
    let mut table = SymbolTable::new();

    // Phase 3: return_type теперь Option<TypeResolution>
    let sig = FunctionSignature {
        name: "МояФункция".to_string(),
        params: vec![],
        return_type: Some(TypeResolution::explicit("Число")),
        is_export: false,
    };

    table.register_function(sig);

    // Проверяем что функции регистрируются с оригинальным именем
    let found = table.find_function("МояФункция");
    assert!(found.is_some());
}

#[test]
fn test_find_procedure_found() {
    let mut table = SymbolTable::new();

    let sig = FunctionSignature {
        name: "МояПроцедура".to_string(),
        params: vec![],
        return_type: None,
        is_export: false,
    };

    table.register_procedure(sig.clone());

    let found = table.find_procedure("МояПроцедура");
    assert!(found.is_some());
    assert_eq!(found.unwrap().name, "МояПроцедура");
    assert_eq!(found.unwrap().return_type, None);
}

#[test]
fn test_find_procedure_not_found() {
    let table = SymbolTable::new();

    let found = table.find_procedure("NonExistentProcedure");
    assert!(found.is_none());
}

#[test]
fn test_get_parent_scope_root() {
    let table = SymbolTable::new();

    let parent = table.get_parent_scope(table.root_scope);
    assert_eq!(parent, None);
}

#[test]
fn test_get_parent_scope_child() {
    let mut table = SymbolTable::new();
    let child = table.create_scope(table.root_scope);

    let parent = table.get_parent_scope(child);
    assert_eq!(parent, Some(table.root_scope));
}

#[test]
fn test_get_parent_scope_grandchild() {
    let mut table = SymbolTable::new();
    let child = table.create_scope(table.root_scope);
    let grandchild = table.create_scope(child);

    let parent = table.get_parent_scope(grandchild);
    assert_eq!(parent, Some(child));
}

#[test]
fn test_get_parent_scope_invalid() {
    let table = SymbolTable::new();

    let parent = table.get_parent_scope(ScopeId(9999));
    assert_eq!(parent, None);
}

#[test]
fn test_variables_in_scope_empty() {
    let table = SymbolTable::new();

    let vars = table.variables_in_scope(table.root_scope);
    assert!(vars.is_some());

    let count: usize = vars.unwrap().count();
    assert_eq!(count, 0);
}

#[test]
fn test_variables_in_scope_single() {
    let mut table = SymbolTable::new();

    table.register_variable(
        table.root_scope,
        "x".to_string(),
        TypeResolution::explicit("Число"),
        Span::stub(),
    );

    let vars = table.variables_in_scope(table.root_scope);
    assert!(vars.is_some());

    let collected: Vec<_> = vars.unwrap().collect();
    assert_eq!(collected.len(), 1);
    assert_eq!(collected[0].0, "x");
}

#[test]
fn test_variables_in_scope_multiple() {
    let mut table = SymbolTable::new();

    table.register_variable(
        table.root_scope,
        "x".to_string(),
        TypeResolution::explicit("Число"),
        Span::stub(),
    );

    table.register_variable(
        table.root_scope,
        "y".to_string(),
        TypeResolution::explicit("Строка"),
        Span::stub(),
    );

    table.register_variable(
        table.root_scope,
        "z".to_string(),
        TypeResolution::explicit("Булево"),
        Span::stub(),
    );

    let vars = table.variables_in_scope(table.root_scope);
    assert!(vars.is_some());

    let collected: Vec<_> = vars.unwrap().collect();
    assert_eq!(collected.len(), 3);
}

#[test]
fn test_variables_in_scope_does_not_include_parent_scope() {
    let mut table = SymbolTable::new();

    // Регистрируем в root scope
    table.register_variable(
        table.root_scope,
        "global".to_string(),
        TypeResolution::explicit("Число"),
        Span::stub(),
    );

    // Создаём child scope и регистрируем переменную там
    let child = table.create_scope(table.root_scope);
    table.register_variable(
        child,
        "local".to_string(),
        TypeResolution::explicit("Строка"),
        Span::stub(),
    );

    // Проверяем что в child scope только одна переменная
    let vars = table.variables_in_scope(child);
    assert!(vars.is_some());

    let collected: Vec<_> = vars.unwrap().collect();
    assert_eq!(collected.len(), 1);
    assert_eq!(collected[0].0, "local");
}

#[test]
fn test_variables_in_scope_invalid_scope() {
    let table = SymbolTable::new();

    let vars = table.variables_in_scope(ScopeId(9999));
    assert!(vars.is_none());
}

// =========================================================================
// Тесты для find_enclosing_function_scope (Phase 3: BSL Function Scope)
// =========================================================================

#[test]
fn test_find_enclosing_function_scope_from_deeply_nested_block() {
    let mut table = SymbolTable::new();
    let func_scope = table.create_scope_with_kind(table.root_scope, ScopeKind::Function);
    let block1 = table.create_scope(func_scope); // Block
    let block2 = table.create_scope(block1); // Block
    let block3 = table.create_scope(block2); // Block

    assert_eq!(table.find_enclosing_function_scope(block3), func_scope);
    assert_eq!(table.find_enclosing_function_scope(block2), func_scope);
    assert_eq!(table.find_enclosing_function_scope(block1), func_scope);
}

#[test]
fn test_find_enclosing_function_scope_from_function_returns_self() {
    let mut table = SymbolTable::new();
    let func_scope = table.create_scope_with_kind(table.root_scope, ScopeKind::Function);

    assert_eq!(table.find_enclosing_function_scope(func_scope), func_scope);
}

#[test]
fn test_find_enclosing_function_scope_from_global_returns_global() {
    let table = SymbolTable::new();

    assert_eq!(
        table.find_enclosing_function_scope(table.root_scope),
        table.root_scope
    );
}

#[test]
fn test_find_enclosing_function_scope_nested_functions() {
    let mut table = SymbolTable::new();
    // Внешняя функция
    let outer_func = table.create_scope_with_kind(table.root_scope, ScopeKind::Function);
    // Внутренняя функция (вложенная)
    let inner_func = table.create_scope_with_kind(outer_func, ScopeKind::Function);
    // Блок внутри внутренней функции
    let block = table.create_scope(inner_func);

    // Из блока должны найти inner_func, не outer_func
    assert_eq!(table.find_enclosing_function_scope(block), inner_func);
    assert_eq!(table.find_enclosing_function_scope(inner_func), inner_func);
    assert_eq!(table.find_enclosing_function_scope(outer_func), outer_func);
}

#[test]
fn test_scope_kind_default_is_global() {
    // Проверяем что ScopeKind::default() = Global (для serde совместимости)
    assert_eq!(ScopeKind::default(), ScopeKind::Global);
}

#[test]
fn test_create_scope_uses_block_kind() {
    let mut table = SymbolTable::new();
    let child = table.create_scope(table.root_scope);

    let scope = table.scopes.get(&child).unwrap();
    assert_eq!(scope.kind, ScopeKind::Block);
}

#[test]
fn test_root_scope_has_global_kind() {
    let table = SymbolTable::new();

    let root = table.scopes.get(&table.root_scope).unwrap();
    assert_eq!(root.kind, ScopeKind::Global);
}
