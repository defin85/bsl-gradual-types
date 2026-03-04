use super::*;
use bsl_types::types::RawDataSource;

#[test]
fn test_find_type_by_camel_alias() {
    let repo = InMemoryTypeRepository::new();

    // Загружаем тип с пробелом в имени
    let type_data = RawTypeData {
        name: "Табличная часть".to_string(),
        english_name: "TabularSection".to_string(),
        source: RawDataSource::Platform,
        ..Default::default()
    };
    repo.load_types(vec![type_data]).unwrap();

    // Все варианты должны находить один тип благодаря нормализации TypeId:
    // - Оригинальное имя с пробелом
    assert!(repo.find_type("Табличная часть").is_some());
    assert_eq!(
        repo.find_type("Табличная часть").unwrap().name,
        "Табличная часть"
    );

    // - CamelCase вариант (нормализуется к тому же ключу)
    assert!(repo.find_type("ТабличнаяЧасть").is_some());
    assert_eq!(
        repo.find_type("ТабличнаяЧасть").unwrap().name,
        "Табличная часть"
    );

    // - lowercase вариант
    assert!(repo.find_type("табличная часть").is_some());
    assert_eq!(
        repo.find_type("табличная часть").unwrap().name,
        "Табличная часть"
    );

    // - Английское имя
    assert!(repo.find_type("TabularSection").is_some());
    assert_eq!(
        repo.find_type("TabularSection").unwrap().name,
        "Табличная часть"
    );

    // - lowercase английское
    assert!(repo.find_type("tabularsection").is_some());
}

#[test]
fn test_type_index_not_overwrites_existing() {
    let repo = InMemoryTypeRepository::new();

    // Два типа с одинаковым нормализованным именем (разный регистр)
    let type1 = RawTypeData {
        name: "Тест алиас".to_string(),
        english_name: "TestAlias1".to_string(),
        source: RawDataSource::Platform,
        ..Default::default()
    };
    let type2 = RawTypeData {
        name: "ТЕСТ АЛИАС".to_string(), // Нормализуется к тому же ключу
        english_name: "TestAlias2".to_string(),
        source: RawDataSource::Platform,
        ..Default::default()
    };

    repo.load_types(vec![type1]).unwrap();
    repo.load_types(vec![type2]).unwrap();

    // Поиск должен вернуть первый загруженный тип (entry().or_insert)
    let found = repo.find_type("ТестАлиас");
    assert!(found.is_some());
    assert_eq!(found.unwrap().name, "Тест алиас");

    // Через разные варианты написания тоже возвращается первый тип
    let found = repo.find_type("тесталиас");
    assert!(found.is_some());
    assert_eq!(found.unwrap().name, "Тест алиас");
}

#[test]
fn test_find_type_with_generic_params() {
    let repo = InMemoryTypeRepository::new();

    let type_data = RawTypeData {
        name: "Массив".to_string(),
        english_name: "Array".to_string(),
        source: RawDataSource::Platform,
        ..Default::default()
    };
    repo.load_types(vec![type_data]).unwrap();

    // Поиск с generic параметрами должен работать
    let found = repo.find_type("Массив<Строка>");
    assert!(found.is_some());
    assert_eq!(found.unwrap().name, "Массив");

    // И без параметров тоже
    let found = repo.find_type("Массив");
    assert!(found.is_some());
    assert_eq!(found.unwrap().name, "Массив");
}

#[test]
fn test_find_type_by_english_name() {
    let repo = InMemoryTypeRepository::new();

    let type_data = RawTypeData {
        name: "Строка".to_string(),
        english_name: "String".to_string(),
        source: RawDataSource::Platform,
        ..Default::default()
    };
    repo.load_types(vec![type_data]).unwrap();

    // Поиск по русскому имени
    assert!(repo.find_type("Строка").is_some());

    // Поиск по английскому имени (регистронезависимо)
    assert!(repo.find_type("String").is_some());
    assert!(repo.find_type("string").is_some());
    assert!(repo.find_type("STRING").is_some());
}

#[test]
fn test_empty_repository() {
    let repo = InMemoryTypeRepository::new();

    assert!(repo.find_type("НесуществующийТип").is_none());
    assert_eq!(repo.get_all_types().len(), 0);
}

#[test]
fn test_upsert_types_updates_existing() {
    let repo = InMemoryTypeRepository::new();

    let type_data = RawTypeData {
        name: "Документ".to_string(),
        english_name: "Document".to_string(),
        description: "old".to_string(),
        source: RawDataSource::Configuration,
        ..Default::default()
    };
    repo.load_types(vec![type_data]).unwrap();

    let updated = RawTypeData {
        name: "Документ".to_string(),
        english_name: "Document".to_string(),
        description: "new".to_string(),
        source: RawDataSource::Configuration,
        ..Default::default()
    };
    repo.upsert_types(vec![updated]).unwrap();

    let found = repo.find_type("Документ").unwrap();
    assert_eq!(found.description, "new");
}

#[test]
fn test_remove_types_removes_indexed_entries() {
    let repo = InMemoryTypeRepository::new();

    let type_a = RawTypeData {
        name: "ТипА".to_string(),
        english_name: "TypeA".to_string(),
        source: RawDataSource::Configuration,
        ..Default::default()
    };
    let type_b = RawTypeData {
        name: "ТипБ".to_string(),
        english_name: "TypeB".to_string(),
        source: RawDataSource::Configuration,
        ..Default::default()
    };
    repo.load_types(vec![type_a, type_b]).unwrap();

    let removed = repo.remove_types(&["ТипА".to_string()]).unwrap();
    assert_eq!(removed, 1);
    assert!(repo.find_type("ТипА").is_none());
    assert!(repo.find_type("TypeA").is_none());
    assert!(repo.find_type("ТипБ").is_some());
}

#[test]
fn test_remove_signatures_by_name() {
    use crate::signature_index::{MethodSignature, SignatureSource};
    use bsl_types::ContextRequirements;

    let repo = InMemoryTypeRepository::new();
    let owner = "СправочникМенеджер.Контрагенты";

    let sig = MethodSignature::new(
        "Тест".to_string(),
        Some(owner.to_string()),
        vec![],
        None,
        None,
        None,
        SignatureSource::Configuration,
        None,
        ContextRequirements::default(),
    );
    repo.add_config_method_signature(owner, sig);
    assert!(repo.find_method_signature(Some(owner), "Тест").is_some());

    repo.remove_config_method_signatures(owner, &["Тест".to_string()]);
    assert!(repo.find_method_signature(Some(owner), "Тест").is_none());

    let global_sig = MethodSignature::new(
        "Глобальная".to_string(),
        None,
        vec![],
        None,
        None,
        None,
        SignatureSource::Configuration,
        None,
        ContextRequirements::default(),
    );
    repo.add_global_function_signature("Глобальная", global_sig);
    assert!(repo.find_method_signature(None, "Глобальная").is_some());

    repo.remove_global_function_signatures(&["Глобальная".to_string()]);
    assert!(repo.find_method_signature(None, "Глобальная").is_none());
}
