//! Тесты для SymbolTable

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
    let span = Span::stub();

    table.register_variable(table.root_scope, "x".to_string(), span);

    let state = table.lookup_variable(table.root_scope, "x").expect("x exists");
    assert!(state.initialized);
    assert_eq!(state.declaration_span, span);
}

#[test]
fn test_lookup_variable_not_found() {
    let table = SymbolTable::new();
    assert!(table.lookup_variable(table.root_scope, "nonexistent").is_none());
}

#[test]
fn test_lookup_variable_in_hierarchy_finds_in_current_scope() {
    let mut table = SymbolTable::new();
    table.register_variable(table.root_scope, "x".to_string(), Span::stub());

    let (scope_id, _) = table
        .lookup_variable_in_hierarchy(table.root_scope, "x")
        .expect("x in root");
    assert_eq!(scope_id, table.root_scope);
}

#[test]
fn test_lookup_variable_in_hierarchy_finds_in_parent_scope() {
    let mut table = SymbolTable::new();
    table.register_variable(table.root_scope, "global_var".to_string(), Span::stub());

    let child = table.create_scope(table.root_scope);
    let (found_scope, _) = table
        .lookup_variable_in_hierarchy(child, "global_var")
        .expect("global_var in parent");
    assert_eq!(found_scope, table.root_scope);
}

#[test]
fn test_lookup_variable_in_hierarchy_shadow_local_over_parent() {
    let mut table = SymbolTable::new();

    table.register_variable(table.root_scope, "x".to_string(), Span::new(10, 11));
    let child = table.create_scope(table.root_scope);
    table.register_variable(child, "x".to_string(), Span::new(20, 21));

    let (found_scope, state) = table
        .lookup_variable_in_hierarchy(child, "x")
        .expect("x found");
    assert_eq!(found_scope, child);
    assert_eq!(state.declaration_span.start, 20);
}

#[test]
fn test_lookup_variable_in_hierarchy_deeply_nested() {
    let mut table = SymbolTable::new();

    let level1 = table.create_scope(table.root_scope);
    let level2 = table.create_scope(level1);
    let level3 = table.create_scope(level2);

    table.register_variable(level1, "mid_var".to_string(), Span::stub());

    let (found_scope, _) = table
        .lookup_variable_in_hierarchy(level3, "mid_var")
        .expect("mid_var found");
    assert_eq!(found_scope, level1);
}

#[test]
fn test_lookup_variable_in_hierarchy_not_found() {
    let mut table = SymbolTable::new();
    let child = table.create_scope(table.root_scope);
    assert!(table.lookup_variable_in_hierarchy(child, "nonexistent").is_none());
}

#[test]
fn test_mark_variable_initialized_checked_success() {
    let mut table = SymbolTable::new();
    let span = Span::stub();

    table.register_variable_declared(table.root_scope, "x".to_string(), span);
    assert!(!table
        .lookup_variable(table.root_scope, "x")
        .expect("x exists")
        .initialized);

    table
        .mark_variable_initialized_checked(table.root_scope, "x")
        .expect("mark ok");

    assert!(table
        .lookup_variable(table.root_scope, "x")
        .expect("x exists")
        .initialized);
}

#[test]
fn test_mark_variable_initialized_checked_invalid_scope() {
    let mut table = SymbolTable::new();
    let invalid_scope = ScopeId(9999);
    let result = table.mark_variable_initialized_checked(invalid_scope, "x");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Scope"));
}

#[test]
fn test_mark_variable_initialized_checked_nonexistent_variable() {
    let mut table = SymbolTable::new();
    let result = table.mark_variable_initialized_checked(table.root_scope, "nonexistent");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not found"));
}

#[test]
fn test_has_variable_true() {
    let mut table = SymbolTable::new();
    table.register_variable(table.root_scope, "x".to_string(), Span::stub());
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

    table.register_variable(table.root_scope, "x".to_string(), Span::stub());

    // has_variable проверяет только конкретный scope, не иерархию
    assert!(table.has_variable(table.root_scope, "x"));
    assert!(!table.has_variable(child, "x"));
}

#[test]
fn test_find_function_found() {
    let mut table = SymbolTable::new();

    let sig = FunctionSignature {
        name: "МояФункция".to_string(),
        params: vec![],
        is_export: false,
    };
    table.register_function(sig.clone());

    let found = table.find_function("МояФункция").expect("found");
    assert_eq!(found.name, "МояФункция");
    assert_eq!(found.params.len(), 0);
}

#[test]
fn test_find_function_not_found() {
    let table = SymbolTable::new();
    assert!(table.find_function("NonExistentFunction").is_none());
}

#[test]
fn test_find_procedure_found() {
    let mut table = SymbolTable::new();

    let sig = FunctionSignature {
        name: "МояПроцедура".to_string(),
        params: vec![],
        is_export: false,
    };
    table.register_procedure(sig.clone());

    let found = table.find_procedure("МояПроцедура").expect("found");
    assert_eq!(found.name, "МояПроцедура");
    assert_eq!(found.params.len(), 0);
}

#[test]
fn test_find_procedure_not_found() {
    let table = SymbolTable::new();
    assert!(table.find_procedure("NonExistentProcedure").is_none());
}

#[test]
fn test_get_parent_scope_root() {
    let table = SymbolTable::new();
    assert_eq!(table.get_parent_scope(table.root_scope), None);
}

#[test]
fn test_get_parent_scope_child() {
    let mut table = SymbolTable::new();
    let child = table.create_scope(table.root_scope);
    assert_eq!(table.get_parent_scope(child), Some(table.root_scope));
}

#[test]
fn test_get_parent_scope_invalid() {
    let table = SymbolTable::new();
    assert_eq!(table.get_parent_scope(ScopeId(9999)), None);
}

#[test]
fn test_variables_in_scope_empty() {
    let table = SymbolTable::new();
    let vars = table.variables_in_scope(table.root_scope).expect("scope exists");
    assert_eq!(vars.count(), 0);
}

#[test]
fn test_variables_in_scope_single() {
    let mut table = SymbolTable::new();
    table.register_variable(table.root_scope, "x".to_string(), Span::stub());

    let vars = table.variables_in_scope(table.root_scope).expect("scope exists");
    let collected: Vec<_> = vars.collect();
    assert_eq!(collected.len(), 1);
    assert_eq!(collected[0].0, "x");
}

#[test]
fn test_variables_in_scope_does_not_include_parent_scope() {
    let mut table = SymbolTable::new();

    table.register_variable(table.root_scope, "global".to_string(), Span::stub());
    let child = table.create_scope(table.root_scope);
    table.register_variable(child, "local".to_string(), Span::stub());

    let vars = table.variables_in_scope(child).expect("scope exists");
    let collected: Vec<_> = vars.collect();
    assert_eq!(collected.len(), 1);
    assert_eq!(collected[0].0, "local");
}

#[test]
fn test_variables_in_scope_invalid_scope() {
    let table = SymbolTable::new();
    assert!(table.variables_in_scope(ScopeId(9999)).is_none());
}

// =========================================================================
// Тесты для find_enclosing_function_scope (BSL Function Scope)
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
fn test_scope_kind_default_is_global() {
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
