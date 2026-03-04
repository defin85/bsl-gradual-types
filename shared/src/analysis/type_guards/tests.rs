use super::*;

#[test]
fn test_detect_type_check() {
    let guards = detect_type_guards("ТипЗнч(Параметр) = Тип(\"Число\")");
    assert_eq!(guards.len(), 1);
    assert!(matches!(guards[0], TypeGuard::TypeCheck { .. }));

    if let TypeGuard::TypeCheck {
        variable,
        expected_type,
    } = &guards[0]
    {
        assert_eq!(variable, "Параметр");
        assert_eq!(expected_type, "Число");
    }
}

#[test]
fn test_detect_not_undefined() {
    let guards = detect_type_guards("Параметр <> Неопределено");
    assert_eq!(guards.len(), 1);
    assert!(matches!(guards[0], TypeGuard::NotUndefined { .. }));

    if let TypeGuard::NotUndefined { variable } = &guards[0] {
        assert_eq!(variable, "Параметр");
    }
}

#[test]
fn test_detect_value_filled() {
    let guards = detect_type_guards("ЗначениеЗаполнено(Объект)");
    assert_eq!(guards.len(), 1);
    assert!(matches!(guards[0], TypeGuard::ValueFilled { .. }));

    if let TypeGuard::ValueFilled { variable } = &guards[0] {
        assert_eq!(variable, "Объект");
    }
}

#[test]
fn test_detect_is_null() {
    let guards = detect_type_guards("x = Null");
    assert_eq!(guards.len(), 1);
    assert!(matches!(guards[0], TypeGuard::IsNull { .. }));
}

#[test]
fn test_detect_not_empty_string() {
    let guards = detect_type_guards("Строка <> \"\"");
    assert_eq!(guards.len(), 1);
    assert!(matches!(guards[0], TypeGuard::NotEmptyString { .. }));
}

#[test]
fn test_detect_not_zero() {
    let guards = detect_type_guards("Число <> 0");
    assert_eq!(guards.len(), 1);
    assert!(matches!(guards[0], TypeGuard::NotZero { .. }));
}

#[test]
fn test_detect_boolean() {
    let guards = detect_type_guards("Флаг = Истина");
    assert_eq!(guards.len(), 1);
    assert!(matches!(guards[0], TypeGuard::IsTrue { .. }));

    let guards2 = detect_type_guards("Флаг = Ложь");
    assert_eq!(guards2.len(), 1);
    assert!(matches!(guards2[0], TypeGuard::IsFalse { .. }));
}

#[test]
fn test_apply_type_check_narrowing() {
    use crate::domain::types::{ConcreteType, TypeResolution};

    let current = TypeResolution::unknown(); // Any

    let guard = TypeGuard::TypeCheck {
        variable: "x".to_string(),
        expected_type: "Строка".to_string(),
    };

    let narrowed = guard.apply_narrowing(&current);

    // Проверяем, что тип сузился до Строка
    if let crate::domain::types::ResolutionResult::Concrete(ConcreteType::Platform(pt)) =
        &narrowed.result
    {
        assert_eq!(pt.name, "Строка");
    } else {
        panic!("Expected Concrete(Platform(Строка))");
    }
}

#[test]
fn test_apply_not_undefined_narrowing() {
    use crate::domain::types::{
        Certainty, ConcreteType, PlatformType, ResolutionResult, TypeResolution, WeightedType,
    };

    // Union: Строка | Неопределено
    let current = TypeResolution {
        certainty: Certainty::Inferred,
        result: ResolutionResult::Union(vec![
            WeightedType {
                type_: ConcreteType::Platform(PlatformType {
                    name: "Строка".to_string(),
                }),
                weight: 0.5,
            },
            WeightedType {
                type_: ConcreteType::Platform(PlatformType {
                    name: "Неопределено".to_string(),
                }),
                weight: 0.5,
            },
        ]),
        source: crate::domain::types::ResolutionSource::Inferred,
        metadata: Default::default(),
        active_facet: None,
        available_facets: vec![],
    };

    let guard = TypeGuard::NotUndefined {
        variable: "x".to_string(),
    };

    let narrowed = guard.apply_narrowing(&current);

    // Должен остаться только Строка
    if let ResolutionResult::Concrete(ConcreteType::Platform(pt)) = &narrowed.result {
        assert_eq!(pt.name, "Строка");
    } else {
        panic!("Expected Concrete(Platform(Строка)), got: {:?}", narrowed);
    }
}

#[test]
fn test_variable_name() {
    let guard = TypeGuard::TypeCheck {
        variable: "Параметр".to_string(),
        expected_type: "Число".to_string(),
    };
    assert_eq!(guard.variable_name(), "Параметр");

    let guard2 = TypeGuard::NotUndefined {
        variable: "Объект".to_string(),
    };
    assert_eq!(guard2.variable_name(), "Объект");
}
