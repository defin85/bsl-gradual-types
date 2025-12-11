//! Type Narrowing
//!
//! Сужение типов на основе flow-sensitive анализа

use super::type_resolver::TypeResolver;
use crate::domain::types::TypeResolution;

impl TypeResolver {
    /// Сужение типа на основе условия (flow-sensitive анализ)
    /// Например: Если ТипЗнч(x) = Тип("Строка"), то x: Строка
    ///
    /// Milestone 3.7: Интеграция с NarrowingEngine
    pub fn narrow_type(&self, current: &TypeResolution, type_check: &str) -> TypeResolution {
        use crate::analysis::type_guards::detect_type_guards;

        // Обнаруживаем type guards в условии
        let guards = detect_type_guards(type_check);

        if guards.is_empty() {
            // Fallback: пробуем найти тип напрямую
            if let Some(raw_type) = self.repository.find_type(type_check) {
                return self.create_resolution_from_raw(&raw_type);
            }
            return current.clone();
        }

        // Применяем первый найденный guard
        if let Some(guard) = guards.first() {
            guard.apply_narrowing(current)
        } else {
            current.clone()
        }
    }

    /// Type narrowing для Nullable - убрать null из типа после проверки
    pub fn narrow_nullable(&self, nullable_resolution: &TypeResolution) -> TypeResolution {
        use super::strategies::NullableStrategy;
        NullableStrategy::narrow(nullable_resolution)
    }
}
