//! Типобезопасный идентификатор типа в системе BSL.
//!
//! `TypeId` обеспечивает:
//! - Нормализацию имён для регистронезависимого поиска
//! - Единый формат хранения для HashMap ключей
//! - Сохранение читаемого отображаемого имени

use serde::{Deserialize, Serialize};

use super::normalization::{camel_to_spaced, normalize};

/// Типобезопасный идентификатор типа в системе BSL.
///
/// Обеспечивает нормализацию имён для регистронезависимого поиска
/// и единый формат хранения для HashMap ключей.
///
/// # Примеры
///
/// ```
/// use bsl_shared::domain::type_id::TypeId;
///
/// // Создание из обычного имени
/// let id1 = TypeId::new("ТаблицаЗначений");
/// let id2 = TypeId::new("таблицазначений");
///
/// // Оба TypeId равны (регистронезависимое сравнение)
/// assert_eq!(id1, id2);
///
/// // Display сохраняет оригинальный формат
/// assert_eq!(id1.display(), "ТаблицаЗначений");
/// ```
#[derive(Debug, Clone, Eq, Serialize, Deserialize)]
pub struct TypeId {
    /// Нормализованное имя для lookup (lowercase, без пробелов)
    normalized: String,
    /// Отображаемое имя (оригинальное или восстановленное)
    display: String,
}

impl TypeId {
    /// Создать TypeId из имени типа (любой формат).
    ///
    /// Сохраняет оригинальное имя как display, нормализует для lookup.
    ///
    /// # Примеры
    /// ```
    /// use bsl_shared::domain::type_id::TypeId;
    ///
    /// let id = TypeId::new("ТаблицаЗначений");
    /// assert_eq!(id.normalized(), "таблицазначений");
    /// assert_eq!(id.display(), "ТаблицаЗначений");
    /// ```
    pub fn new(name: &str) -> Self {
        Self {
            normalized: normalize(name),
            display: name.to_string(),
        }
    }

    /// Создать TypeId из CamelCase имени.
    ///
    /// Конвертирует CamelCase в читаемый формат с пробелами для display.
    ///
    /// # Примеры
    /// ```
    /// use bsl_shared::domain::type_id::TypeId;
    ///
    /// let id = TypeId::from_camel_case("ТабличнаяЧасть");
    /// assert_eq!(id.display(), "Табличная часть");
    /// assert_eq!(id.normalized(), "табличнаячасть");
    /// ```
    pub fn from_camel_case(name: &str) -> Self {
        let display = camel_to_spaced(name);
        Self {
            normalized: normalize(&display),
            display,
        }
    }

    /// Получить нормализованное имя (для HashMap key).
    ///
    /// Всегда lowercase, без пробелов.
    pub fn normalized(&self) -> &str {
        &self.normalized
    }

    /// Получить отображаемое имя.
    ///
    /// Сохраняет оригинальный формат или конвертированный из CamelCase.
    pub fn display(&self) -> &str {
        &self.display
    }

    /// Извлечь базовый тип без точки.
    ///
    /// Для фасетных типов возвращает TypeId базового типа.
    ///
    /// # Примеры
    /// ```
    /// use bsl_shared::domain::type_id::TypeId;
    ///
    /// let id = TypeId::new("СправочникМенеджер.Контрагенты");
    /// let base = id.base_type().unwrap();
    /// assert_eq!(base.display(), "СправочникМенеджер");
    ///
    /// let simple = TypeId::new("Массив");
    /// assert!(simple.base_type().is_none());
    /// ```
    pub fn base_type(&self) -> Option<TypeId> {
        let mut parts = self.display.splitn(2, '.');
        let base = parts.next()?;

        // Если есть второй элемент — значит была точка
        if parts.next().is_some() {
            Some(TypeId::new(base))
        } else {
            None
        }
    }

    /// Убрать generic параметры из имени типа.
    ///
    /// # Примеры
    /// ```
    /// use bsl_shared::domain::type_id::TypeId;
    ///
    /// let id = TypeId::new("Массив<Строка>");
    /// let without = id.without_generic_params();
    /// assert_eq!(without.display(), "Массив");
    ///
    /// let simple = TypeId::new("Число");
    /// let same = simple.without_generic_params();
    /// assert_eq!(same.display(), "Число");
    /// ```
    pub fn without_generic_params(&self) -> TypeId {
        if let Some(pos) = self.display.find('<') {
            TypeId::new(&self.display[..pos])
        } else {
            self.clone()
        }
    }
}

impl PartialEq for TypeId {
    fn eq(&self, other: &Self) -> bool {
        self.normalized == other.normalized
    }
}

impl std::hash::Hash for TypeId {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.normalized.hash(state);
    }
}

impl std::fmt::Display for TypeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display)
    }
}

#[cfg(test)]
mod tests {
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
}
