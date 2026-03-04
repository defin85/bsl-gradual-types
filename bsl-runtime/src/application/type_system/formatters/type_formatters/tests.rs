use super::*;
use bsl_shared::domain::types::{ConcreteType, PlatformType, PrimitiveType};

#[test]
fn test_format_concrete() {
    let result = ResolutionResult::Concrete(ConcreteType::Platform(PlatformType {
        name: "Массив".to_string(),
    }));
    assert_eq!(format_resolution_result(&result), "Массив");
}

#[test]
fn test_format_dynamic() {
    let result = ResolutionResult::Dynamic;
    assert_eq!(format_resolution_result(&result), "Произвольный");
}

#[test]
fn test_format_primitive() {
    let result = ResolutionResult::Concrete(ConcreteType::Primitive(PrimitiveType::String));
    assert_eq!(format_resolution_result(&result), "Строка");
}
