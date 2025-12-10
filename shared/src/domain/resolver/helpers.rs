//! Domain Layer: Type Resolver Helper Functions
//!
//! Вспомогательные функции для резолюции типов

use crate::domain::types::{ConcreteType, GenericType, WeightedType};

/// Case-insensitive сравнение строк (работает с кириллицей и латиницей)
pub fn names_equal_ignore_case(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.chars()
        .zip(b.chars())
        .all(|(ca, cb)| ca.to_lowercase().eq(cb.to_lowercase()))
}

/// Проверка совместимости типов для валидации параметров
///
/// Поддерживает gradual typing: Unknown, Dynamic, Произвольный совместимы со всем
///
/// # Параметры
/// - `expected` - ожидаемый тип параметра (из сигнатуры)
/// - `actual` - фактический тип аргумента
///
/// # Возвращает
/// `true` если типы совместимы, `false` иначе
pub fn is_type_compatible(expected: &str, actual: &str) -> bool {
    // Gradual typing: Unknown, Dynamic, Произвольный совместимы со всем
    if actual == "Unknown"
        || actual == "Dynamic"
        || actual == "Произвольный"
        || actual == "Неопределено"
    {
        return true;
    }

    // Если ожидается Произвольный - любой тип подходит
    if expected == "Произвольный" || expected == "Dynamic" {
        return true;
    }

    // BUGFIX: Поддержка union типов в expected
    // "Число | Строка" должен принимать "Строка" или "Число"
    if expected.contains(" | ") {
        // Разбиваем union на части и проверяем что actual входит в одну из них
        return expected
            .split(" | ")
            .any(|variant| names_equal_ignore_case(variant.trim(), actual));
    }

    // Case-insensitive сравнение (кириллица + латиница)
    names_equal_ignore_case(expected, actual)
}

/// Форматирование Union типа для отображения
pub fn format_union_type(union_types: &[WeightedType]) -> String {
    union_types
        .iter()
        .map(|wt| {
            let type_name = match &wt.type_ {
                ConcreteType::Primitive(p) => format!("{:?}", p),
                ConcreteType::Platform(pt) => pt.name.clone(),
                ConcreteType::Configuration(ct) => ct.name.clone(),
                ConcreteType::Special(st) => format!("{:?}", st),
                ConcreteType::GlobalFunction(gf) => gf.name.clone(),
                ConcreteType::TabularRow(tr) => tr.get_full_name(),
            };

            // Если вес не равен 1.0, показываем его
            if (wt.weight - 1.0).abs() > 0.01 {
                format!("{}({:.0}%)", type_name, wt.weight * 100.0)
            } else {
                type_name
            }
        })
        .collect::<Vec<_>>()
        .join(" | ")
}

/// Форматирование Intersection типа для отображения
pub fn format_intersection_type(intersection_types: &[ConcreteType]) -> String {
    intersection_types
        .iter()
        .map(|ct| match ct {
            ConcreteType::Primitive(p) => format!("{:?}", p),
            ConcreteType::Platform(pt) => pt.name.clone(),
            ConcreteType::Configuration(ct) => ct.name.clone(),
            ConcreteType::Special(st) => format!("{:?}", st),
            ConcreteType::GlobalFunction(gf) => gf.name.clone(),
            ConcreteType::TabularRow(tr) => tr.get_full_name(),
        })
        .collect::<Vec<_>>()
        .join(" & ")
}

/// Форматирование Generic типа для отображения
pub fn format_generic_type(generic: &GenericType) -> String {
    let params = generic
        .type_params
        .iter()
        .map(|ct| match ct {
            ConcreteType::Primitive(p) => format!("{:?}", p),
            ConcreteType::Platform(pt) => pt.name.clone(),
            ConcreteType::Configuration(ct) => ct.name.clone(),
            ConcreteType::Special(st) => format!("{:?}", st),
            ConcreteType::TabularRow(tr) => tr.get_full_name(),
            ConcreteType::GlobalFunction(gf) => gf.name.clone(),
        })
        .collect::<Vec<_>>()
        .join(", ");

    format!("{}<{}>", generic.base_type, params)
}

/// Форматирование Nullable типа для отображения
pub fn format_nullable_type(base_type: &ConcreteType) -> String {
    let type_name = match base_type {
        ConcreteType::Primitive(p) => format!("{:?}", p),
        ConcreteType::Platform(pt) => pt.name.clone(),
        ConcreteType::Configuration(ct) => ct.name.clone(),
        ConcreteType::Special(st) => format!("{:?}", st),
        ConcreteType::TabularRow(tr) => tr.get_full_name(),
        ConcreteType::GlobalFunction(gf) => gf.name.clone(),
    };

    format!("{}?", type_name)
}
