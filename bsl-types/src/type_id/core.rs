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
/// use bsl_types::TypeId;
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
    /// use bsl_types::TypeId;
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
    /// use bsl_types::TypeId;
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
    /// use bsl_types::TypeId;
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
    /// use bsl_types::TypeId;
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
#[path = "core/tests.rs"]
mod tests;
