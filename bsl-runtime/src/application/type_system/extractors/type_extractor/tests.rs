use super::*;

#[test]
fn test_extract_var_name() {
    assert_eq!(
        extract_var_name("Перем Счетчик: Число;"),
        Some("Счетчик".to_string())
    );
    assert_eq!(
        extract_var_name("Перем МояПеременная;"),
        Some("МояПеременная;".to_string())
    );
}

#[test]
fn test_extract_type_from_var_declaration() {
    assert_eq!(
        extract_type_from_var_declaration("Перем Счетчик: Число;"),
        Some("Число".to_string())
    );
    assert_eq!(
        extract_type_from_var_declaration("Перем МояПеременная;"),
        None
    );
}

#[test]
fn test_extract_function_name() {
    assert_eq!(
        extract_function_name("Функция Тест()"),
        Some("Тест()".to_string())
    );
    assert_eq!(
        extract_function_name("Процедура Обработать(Параметр)"),
        Some("Обработать(Параметр)".to_string())
    );
}

#[test]
fn test_extract_return_type() {
    assert_eq!(
        extract_return_type("    Возврат Число;"),
        Some("Число".to_string())
    );
    assert_eq!(extract_return_type("    КонецФункции"), None);
}
