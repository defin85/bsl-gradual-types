use super::*;
use std::collections::HashMap;

#[test]
fn test_new_preserves_display() {
    let id = TypeId::new("ТаблицаЗначений");
    assert_eq!(id.display(), "ТаблицаЗначений");
    assert_eq!(id.normalized(), "таблицазначений");
}

#[test]
fn test_from_camel_case_converts_display() {
    let id = TypeId::from_camel_case("ТабличнаяЧасть");
    assert_eq!(id.display(), "Табличная часть");
    assert_eq!(id.normalized(), "табличнаячасть");
}

#[test]
fn test_eq_case_insensitive() {
    let id1 = TypeId::new("ТаблицаЗначений");
    let id2 = TypeId::new("таблицазначений");
    assert_eq!(id1, id2);
}

#[test]
fn test_eq_with_spaces() {
    let id1 = TypeId::new("Табличная часть");
    let id2 = TypeId::new("ТабличнаяЧасть");
    assert_eq!(id1, id2);
}

#[test]
fn test_hash_consistency() {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let id1 = TypeId::new("ТаблицаЗначений");
    let id2 = TypeId::new("таблицазначений");

    let mut h1 = DefaultHasher::new();
    let mut h2 = DefaultHasher::new();
    id1.hash(&mut h1);
    id2.hash(&mut h2);

    assert_eq!(h1.finish(), h2.finish());
}

#[test]
fn test_hashmap_lookup() {
    let mut map = HashMap::new();
    let id1 = TypeId::new("ТаблицаЗначений");
    map.insert(id1, "value");

    let lookup_key = TypeId::new("таблицазначений");
    assert_eq!(map.get(&lookup_key), Some(&"value"));
}

#[test]
fn test_base_type_with_dot() {
    let id = TypeId::new("СправочникМенеджер.Контрагенты");
    let base = id.base_type().unwrap();
    assert_eq!(base.display(), "СправочникМенеджер");
}

#[test]
fn test_base_type_without_dot() {
    let id = TypeId::new("Массив");
    assert!(id.base_type().is_none());
}

#[test]
fn test_without_generic_params() {
    let id = TypeId::new("Массив<Строка>");
    let without = id.without_generic_params();
    assert_eq!(without.display(), "Массив");
    assert_eq!(without.normalized(), "массив");
}

#[test]
fn test_without_generic_params_nested() {
    let id = TypeId::new("Соответствие<Строка, Массив<Число>>");
    let without = id.without_generic_params();
    assert_eq!(without.display(), "Соответствие");
}

#[test]
fn test_without_generic_params_no_generics() {
    let id = TypeId::new("Число");
    let without = id.without_generic_params();
    assert_eq!(without.display(), "Число");
}

#[test]
fn test_display_trait() {
    let id = TypeId::new("ТаблицаЗначений");
    assert_eq!(format!("{}", id), "ТаблицаЗначений");
}

#[test]
fn test_faceted_type() {
    let id = TypeId::new("ДокументСсылка.ЗаказНаряды");
    assert_eq!(id.normalized(), "документссылка.заказнаряды");

    let base = id.base_type().unwrap();
    assert_eq!(base.normalized(), "документссылка");
}

#[test]
fn test_clone() {
    let id1 = TypeId::new("Тест");
    let id2 = id1.clone();
    assert_eq!(id1, id2);
    assert_eq!(id1.display(), id2.display());
}

#[test]
fn test_debug() {
    let id = TypeId::new("Тест");
    let debug = format!("{:?}", id);
    assert!(debug.contains("TypeId"));
    assert!(debug.contains("тест"));
}
