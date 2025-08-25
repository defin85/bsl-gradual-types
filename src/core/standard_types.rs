//! Стандартные типы BSL

use crate::domain::types::{
    Certainty, ConcreteType, PlatformType, PrimitiveType as CorePrimitiveType, ResolutionResult,
    ResolutionSource, SpecialType, TypeResolution,
};

/// Создание примитивного типа
pub fn primitive_type(prim: CorePrimitiveType) -> TypeResolution {
    TypeResolution {
        certainty: Certainty::Known,
        result: ResolutionResult::Concrete(ConcreteType::Primitive(prim)),
        source: ResolutionSource::Static,
        metadata: Default::default(),
        active_facet: None,
        available_facets: vec![],
    }
}

/// Создание специального типа
pub fn special_type(spec: SpecialType) -> TypeResolution {
    TypeResolution {
        certainty: Certainty::Known,
        result: ResolutionResult::Concrete(ConcreteType::Special(spec)),
        source: ResolutionSource::Static,
        metadata: Default::default(),
        active_facet: None,
        available_facets: vec![],
    }
}

/// Создание платформенного типа (коллекции)
pub fn platform_type(name: &str) -> TypeResolution {
    let platform = PlatformType {
        name: name.to_string(),
        methods: Vec::new(),
        properties: Vec::new(),
    };

    TypeResolution {
        certainty: Certainty::Known,
        result: ResolutionResult::Concrete(ConcreteType::Platform(platform)),
        source: ResolutionSource::Static,
        metadata: Default::default(),
        active_facet: None,
        available_facets: vec![],
    }
}

/// Проверка, является ли тип числовым
pub fn is_number(type_res: &TypeResolution) -> bool {
    matches!(
        &type_res.result,
        ResolutionResult::Concrete(ConcreteType::Primitive(CorePrimitiveType::Number))
    )
}

/// Проверка, является ли тип строковым
pub fn is_string(type_res: &TypeResolution) -> bool {
    matches!(
        &type_res.result,
        ResolutionResult::Concrete(ConcreteType::Primitive(CorePrimitiveType::String))
    )
}

/// Проверка, является ли тип булевым
pub fn is_boolean(type_res: &TypeResolution) -> bool {
    matches!(
        &type_res.result,
        ResolutionResult::Concrete(ConcreteType::Primitive(CorePrimitiveType::Boolean))
    )
}

/// Проверка, является ли тип датой
pub fn is_date(type_res: &TypeResolution) -> bool {
    matches!(
        &type_res.result,
        ResolutionResult::Concrete(ConcreteType::Primitive(CorePrimitiveType::Date))
    )
}

/// Проверка, является ли тип неопределенным
pub fn is_undefined(type_res: &TypeResolution) -> bool {
    matches!(
        &type_res.result,
        ResolutionResult::Concrete(ConcreteType::Special(SpecialType::Undefined))
    )
}

/// Проверка, является ли тип null
pub fn is_null(type_res: &TypeResolution) -> bool {
    matches!(
        &type_res.result,
        ResolutionResult::Concrete(ConcreteType::Special(SpecialType::Null))
    )
}

/// Проверка, является ли тип массивом
pub fn is_array(type_res: &TypeResolution) -> bool {
    if let ResolutionResult::Concrete(ConcreteType::Platform(platform)) = &type_res.result {
        platform.name == "Массив" || platform.name == "Array"
    } else {
        false
    }
}

/// Проверка, является ли тип структурой
pub fn is_structure(type_res: &TypeResolution) -> bool {
    if let ResolutionResult::Concrete(ConcreteType::Platform(platform)) = &type_res.result {
        platform.name == "Структура" || platform.name == "Structure"
    } else {
        false
    }
}

/// Проверка, является ли тип соответствием
pub fn is_map(type_res: &TypeResolution) -> bool {
    if let ResolutionResult::Concrete(ConcreteType::Platform(platform)) = &type_res.result {
        platform.name == "Соответствие" || platform.name == "Map"
    } else {
        false
    }
}
