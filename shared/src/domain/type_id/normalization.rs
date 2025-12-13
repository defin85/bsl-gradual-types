//! Функции нормализации имён типов BSL.
//!
//! Централизованная логика нормализации для регистронезависимого поиска
//! и конвертации между форматами CamelCase и "с пробелами".

/// Нормализовать имя типа для ключа HashMap.
///
/// # Правила
/// 1. Lowercase
/// 2. Убрать пробелы ("Табличная часть" -> "табличнаячасть")
/// 3. НЕ трогать точки и угловые скобки (для фасетных типов и generics)
///
/// # Примеры
/// ```
/// use bsl_shared::domain::type_id::normalization::normalize;
///
/// assert_eq!(normalize("ТаблицаЗначений"), "таблицазначений");
/// assert_eq!(normalize("Табличная часть"), "табличнаячасть");
/// assert_eq!(normalize("СправочникМенеджер.Контрагенты"), "справочникменеджер.контрагенты");
/// assert_eq!(normalize("Массив<Строка>"), "массив<строка>");
/// ```
pub fn normalize(name: &str) -> String {
    let mut result = String::with_capacity(name.len());
    for c in name.chars() {
        if c != ' ' {
            for lower in c.to_lowercase() {
                result.push(lower);
            }
        }
    }
    result
}

/// Конвертировать CamelCase в формат с пробелами.
///
/// Добавляет пробел перед каждой заглавной буквой (кроме первой),
/// если предыдущий символ был строчным.
///
/// # Примеры
/// ```
/// use bsl_shared::domain::type_id::normalization::camel_to_spaced;
///
/// assert_eq!(camel_to_spaced("ТабличнаяЧасть"), "Табличная часть");
/// assert_eq!(camel_to_spaced("ТаблицаЗначений"), "Таблица значений");
/// assert_eq!(camel_to_spaced("Массив"), "Массив");
/// assert_eq!(camel_to_spaced("HTTPКлиент"), "HTTPКлиент"); // Аббревиатуры сохраняются
/// ```
pub fn camel_to_spaced(name: &str) -> String {
    let mut result = String::with_capacity(name.len() + 4);
    let mut prev_lower = false;

    for (i, c) in name.chars().enumerate() {
        if i > 0 && c.is_uppercase() && prev_lower {
            result.push(' ');
            for lower in c.to_lowercase() {
                result.push(lower);
            }
        } else {
            result.push(c);
        }
        prev_lower = c.is_lowercase();
    }

    result
}

/// Конвертировать формат с пробелами в CamelCase.
///
/// Убирает пробелы и делает первую букву каждого слова заглавной.
///
/// # Примеры
/// ```
/// use bsl_shared::domain::type_id::normalization::spaced_to_camel;
///
/// assert_eq!(spaced_to_camel("Табличная часть"), "ТабличнаяЧасть");
/// assert_eq!(spaced_to_camel("Таблица значений"), "ТаблицаЗначений");
/// assert_eq!(spaced_to_camel("Массив"), "Массив");
/// ```
pub fn spaced_to_camel(name: &str) -> String {
    let mut result = String::with_capacity(name.len());
    let mut capitalize_next = true;

    for c in name.chars() {
        if c == ' ' {
            capitalize_next = true;
        } else if capitalize_next {
            for upper in c.to_uppercase() {
                result.push(upper);
            }
            capitalize_next = false;
        } else {
            result.push(c);
        }
    }

    result
}

#[cfg(test)]
mod tests {
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
}
