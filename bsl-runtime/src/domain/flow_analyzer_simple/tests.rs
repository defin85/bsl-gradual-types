use super::*;
use bsl_shared::domain::repository::InMemoryTypeRepository;
use bsl_shared::domain::types::{ConcreteType, ResolutionResult};

#[test]
fn test_simple_variable_assignment() {
    let repo = Arc::new(InMemoryTypeRepository::new());
    let resolver = Arc::new(TypeResolver::new(repo));
    let analyzer = SimpleFlowAnalyzer::new(resolver);

    let code = r#"
Перем x = 42;
Перем y = "текст";
    "#;

    let result = analyzer.analyze_code(code);

    assert!(result.variables.contains_key(&TypeId::new("x")));
    assert!(result.variables.contains_key(&TypeId::new("y")));

    // Проверяем тип x (Число)
    if let Some(x_type) = result.variables.get(&TypeId::new("x")) {
        if let ResolutionResult::Concrete(ConcreteType::Platform(pt)) = &x_type.result {
            assert_eq!(pt.name, "Число");
        }
    }

    // Проверяем тип y (Строка)
    if let Some(y_type) = result.variables.get(&TypeId::new("y")) {
        if let ResolutionResult::Concrete(ConcreteType::Platform(pt)) = &y_type.result {
            assert_eq!(pt.name, "Строка");
        }
    }
}

#[test]
fn test_constructor_call() {
    let repo = Arc::new(InMemoryTypeRepository::new());
    let resolver = Arc::new(TypeResolver::new(repo));
    let analyzer = SimpleFlowAnalyzer::new(resolver);

    let code = r#"
Перем массив = Новый Массив();
    "#;

    let result = analyzer.analyze_code(code);

    assert!(result.variables.contains_key(&TypeId::new("массив")));

    if let Some(arr_type) = result.variables.get(&TypeId::new("массив")) {
        if let ResolutionResult::Concrete(ConcreteType::Platform(pt)) = &arr_type.result {
            assert_eq!(pt.name, "Массив");
        }
    }
}

#[test]
fn test_scope_tracking() {
    let repo = Arc::new(InMemoryTypeRepository::new());
    let resolver = Arc::new(TypeResolver::new(repo));
    let analyzer = SimpleFlowAnalyzer::new(resolver);

    let code = r#"
Перем x = 1;
Если x > 0 Тогда
Перем y = 2;
КонецЕсли;
    "#;

    let result = analyzer.analyze_code(code);

    // Проверяем, что scope depth корректно отслеживается
    assert_eq!(result.context.get_scope_depth(), 0);
}
