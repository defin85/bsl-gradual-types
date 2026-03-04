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
/// use bsl_types::type_id::normalization::normalize;
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
/// use bsl_types::type_id::normalization::camel_to_spaced;
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
/// use bsl_types::type_id::normalization::spaced_to_camel;
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
#[path = "normalization/tests.rs"]
mod tests;
