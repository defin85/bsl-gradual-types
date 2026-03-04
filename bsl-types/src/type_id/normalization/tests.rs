use super::*;

#[test]
fn test_normalize_lowercase() {
    assert_eq!(normalize("ТаблицаЗначений"), "таблицазначений");
    assert_eq!(normalize("Массив"), "массив");
}

#[test]
fn test_normalize_removes_spaces() {
    assert_eq!(normalize("Табличная часть"), "табличнаячасть");
    assert_eq!(normalize("Таблица значений"), "таблицазначений");
}

#[test]
fn test_normalize_preserves_dots() {
    assert_eq!(
        normalize("СправочникМенеджер.Контрагенты"),
        "справочникменеджер.контрагенты"
    );
}

#[test]
fn test_normalize_preserves_angle_brackets() {
    assert_eq!(normalize("Массив<Строка>"), "массив<строка>");
    assert_eq!(
        normalize("Соответствие<Строка, Число>"),
        "соответствие<строка,число>"
    );
}

#[test]
fn test_camel_to_spaced_basic() {
    assert_eq!(camel_to_spaced("ТабличнаяЧасть"), "Табличная часть");
    assert_eq!(camel_to_spaced("ТаблицаЗначений"), "Таблица значений");
}

#[test]
fn test_camel_to_spaced_single_word() {
    assert_eq!(camel_to_spaced("Массив"), "Массив");
    assert_eq!(camel_to_spaced("Строка"), "Строка");
}

#[test]
fn test_camel_to_spaced_abbreviations() {
    // Последовательные заглавные буквы не разбиваются
    assert_eq!(camel_to_spaced("HTTPКлиент"), "HTTPКлиент");
}

#[test]
fn test_spaced_to_camel_basic() {
    assert_eq!(spaced_to_camel("Табличная часть"), "ТабличнаяЧасть");
    assert_eq!(spaced_to_camel("Таблица значений"), "ТаблицаЗначений");
}

#[test]
fn test_spaced_to_camel_single_word() {
    assert_eq!(spaced_to_camel("Массив"), "Массив");
}

#[test]
fn test_roundtrip_camel_spaced() {
    let original = "ТабличнаяЧасть";
    let spaced = camel_to_spaced(original);
    let back = spaced_to_camel(&spaced);
    assert_eq!(back, original);
}

#[test]
fn test_normalize_idempotent() {
    let name = "Табличная часть";
    let normalized = normalize(name);
    assert_eq!(normalize(&normalized), normalized);
}

#[test]
fn test_normalize_empty_string() {
    assert_eq!(normalize(""), "");
}

#[test]
fn test_normalize_only_spaces() {
    assert_eq!(normalize("   "), "");
}

#[test]
fn test_camel_to_spaced_empty() {
    assert_eq!(camel_to_spaced(""), "");
}

#[test]
fn test_spaced_to_camel_empty() {
    assert_eq!(spaced_to_camel(""), "");
}

#[test]
fn test_normalize_multiple_spaces() {
    assert_eq!(normalize("Табличная    часть"), "табличнаячасть");
}
