//! Generic-специфичная логика для SymbolTable

use super::{ScopeId, SymbolTable};
use crate::domain::types::{Certainty, ConcreteType, ResolutionResult, SpecialType, TypeResolution};
use crate::ir::span::Span;
use crate::ir::types::VariableState;

impl SymbolTable {
    /// Инициализировать переменную как Generic тип с неизвестными параметрами
    ///
    /// # Примеры
    ///
    /// ```
    /// # use bsl_shared::ir::SymbolTable;
    /// # use bsl_shared::domain::types::{TypeResolution, Certainty, ResolutionResult};
    /// let mut table = SymbolTable::new();
    /// table.initialize_as_generic(table.root_scope, "МассивСтрок".to_string(), "Массив".to_string(), 1);
    ///
    /// let resolution = table.get_variable_type(table.root_scope, "МассивСтрок");
    /// assert!(resolution.is_some());
    /// let res = resolution.unwrap();
    /// assert_eq!(res.type_name(), "Массив<Неопределено>");
    /// assert!(matches!(res.certainty, Certainty::InferredWeak));
    /// ```
    pub fn initialize_as_generic(
        &mut self,
        scope_id: ScopeId,
        var_name: String,
        base_type: String,
        type_param_count: usize,
    ) {
        // Создаём пустые параметры (неизвестные типы = "?")
        let type_params: Vec<&str> = vec!["?"; type_param_count];

        // Используем TypeResolution::generic() с InferredWeak (параметры неизвестны)
        let resolution = TypeResolution::generic(&base_type, &type_params, Certainty::InferredWeak);

        // Регистрируем или обновляем переменную
        if let Some(scope) = self.scopes.get_mut(&scope_id) {
            // Если переменная уже зарегистрирована, получаем её span и initialized
            let (span, initialized) = scope
                .variables
                .get(&var_name)
                .map(|vs| (vs.declaration_span, vs.initialized))
                .unwrap_or_else(|| (Span::stub(), true));

            scope.variables.insert(
                var_name,
                VariableState::new(resolution, span, initialized),
            );
        }
    }

    /// Обновить Generic параметр переменной
    ///
    /// # Примеры
    ///
    /// ```
    /// # use bsl_shared::ir::SymbolTable;
    /// # use bsl_shared::domain::types::{TypeResolution, Certainty, ResolutionResult};
    /// let mut table = SymbolTable::new();
    /// table.initialize_as_generic(table.root_scope, "МассивСтрок".to_string(), "Массив".to_string(), 1);
    /// table.update_generic_param(table.root_scope, "МассивСтрок", 0, "Строка".to_string());
    ///
    /// let resolution = table.get_variable_type(table.root_scope, "МассивСтрок");
    /// assert!(resolution.is_some());
    /// let res = resolution.unwrap();
    /// assert_eq!(res.type_name(), "Массив<Строка>");
    /// assert!(matches!(res.certainty, Certainty::Known));
    /// ```
    pub fn update_generic_param(
        &mut self,
        scope_id: ScopeId,
        var_name: &str,
        param_index: usize,
        param_type: String,
    ) -> bool {
        if let Some(scope) = self.scopes.get_mut(&scope_id) {
            if let Some(var_state) = scope.variables.get_mut(var_name) {
                match &mut var_state.resolution.result {
                    ResolutionResult::Generic(gen) => {
                        // Получаем новый ConcreteType для параметра
                        let new_param = TypeResolution::string_to_concrete(&param_type);

                        // Обновляем конкретный параметр (расширяем вектор, если нужно)
                        if param_index < gen.type_params.len() {
                            gen.type_params[param_index] = new_param;
                        } else {
                            gen.type_params.push(new_param);
                        }

                        // Вычисляем новую уверенность
                        // Если ВСЕ параметры известны (не Undefined), то certainty = Known
                        let all_known = gen.type_params.iter().all(|p| {
                            !matches!(p, ConcreteType::Special(SpecialType::Undefined))
                        });
                        var_state.resolution.certainty = if all_known {
                            Certainty::Known
                        } else {
                            Certainty::InferredWeak
                        };

                        return true;
                    }
                    _ => {
                        // Не Generic тип — попробуем конвертировать
                        // (например, если раньше был Inferred, теперь становится Generic)
                        tracing::warn!(
                            "update_generic_param: {} не Generic тип, пропускаем",
                            var_name
                        );
                    }
                }
            }
        }

        false
    }
}
