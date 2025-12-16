//! MethodSignature - сигнатура метода с lazy resolution
//!
//! Milestone 3.15: Lazy Resolution with Arc<OnceLock>

use super::super::runtime_context::ContextRequirements;
use super::super::types::{FacetKind, ParameterInfo, TypeResolution};
use super::types::SignatureSource;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, OnceLock};

// ==================== Lazy Resolution Defaults ====================

/// Default function for serde skip - creates empty resolved_return cache
fn default_resolved_return() -> Arc<OnceLock<Option<TypeResolution>>> {
    Arc::new(OnceLock::new())
}

/// Default function for serde skip - creates empty resolved_params cache
fn default_resolved_params() -> Arc<OnceLock<Vec<(String, TypeResolution)>>> {
    Arc::new(OnceLock::new())
}

/// Сигнатура метода
///
/// Расширенная информация о методе/функции включая:
/// - Базовые параметры (имя, тип владельца, параметры)
/// - Facet информацию для методов конфигурационных объектов
/// - Требования к контексту выполнения
/// - Lazy Resolution Cache (Milestone 3.15) для отложенного резолвинга типов
///
/// # Примеры
/// ```
/// use bsl_shared::domain::signature_index::{MethodSignature, SignatureSource, ContextRequirements};
/// use bsl_shared::domain::types::{ParameterInfo, FacetKind};
///
/// // Метод Справочник.СоздатьЭлемент() -> Object, ServerOnly
/// let signature = MethodSignature::new(
///     "СоздатьЭлемент".to_string(),
///     Some("СправочникМенеджер.Номенклатура".to_string()),
///     vec![],
///     Some("СправочникОбъект.Номенклатура".to_string()),
///     SignatureSource::Platform,
///     Some(FacetKind::Object),
///     ContextRequirements::ServerOnly,
/// );
/// ```
#[derive(Debug, Serialize, Deserialize)] // БЕЗ Clone - реализуем вручную!
pub struct MethodSignature {
    pub name: String,
    pub owner_type: Option<String>, // None для глобальных функций
    pub params: Vec<ParameterInfo>,
    pub return_type: Option<String>,
    pub source: SignatureSource,

    /// Facet возвращаемого типа (для методов конфигурационных объектов)
    ///
    /// # Примеры
    /// - `СоздатьЭлемент()` -> Object
    /// - `НайтиПоКоду()` -> Reference
    /// - `Выбрать()` -> Selection
    #[serde(default)]
    pub return_facet: Option<FacetKind>,

    /// Требования к контексту выполнения
    ///
    /// Определяет где может быть вызван метод (сервер/клиент/везде)
    #[serde(default)]
    pub context_requirements: ContextRequirements,

    // ==================== Lazy Resolution Cache ====================
    /// Кэш резолвленного типа возврата (lazy, thread-safe)
    ///
    /// Заполняется при первом вызове `get_resolved_return_type()`.
    /// Arc позволяет разделять кэш между клонированными сигнатурами.
    #[serde(skip, default = "default_resolved_return")]
    resolved_return: Arc<OnceLock<Option<TypeResolution>>>,

    /// Кэш резолвленных типов параметров (lazy, thread-safe)
    ///
    /// Заполняется при первом вызове `get_resolved_params()`.
    /// Vec содержит пары (имя_параметра, TypeResolution).
    #[serde(skip, default = "default_resolved_params")]
    resolved_params: Arc<OnceLock<Vec<(String, TypeResolution)>>>,
}

// ==================== Clone Implementation ====================

impl Clone for MethodSignature {
    /// Клонирование с разделяемым кэшем
    ///
    /// ВАЖНО: Arc::clone создаёт shared reference на кэш.
    /// Это означает что клонированные сигнатуры разделяют кэш резолвленных типов.
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            owner_type: self.owner_type.clone(),
            params: self.params.clone(),
            return_type: self.return_type.clone(),
            source: self.source,
            return_facet: self.return_facet,
            context_requirements: self.context_requirements, // Copy trait
            // ВАЖНО: Arc::clone = shared cache между всеми клонами!
            resolved_return: Arc::clone(&self.resolved_return),
            resolved_params: Arc::clone(&self.resolved_params),
        }
    }
}

impl MethodSignature {
    /// Создать новую сигнатуру метода
    ///
    /// Инициализирует lazy resolution кэши как пустые.
    pub fn new(
        name: String,
        owner_type: Option<String>,
        params: Vec<ParameterInfo>,
        return_type: Option<String>,
        source: SignatureSource,
        return_facet: Option<FacetKind>,
        context_requirements: ContextRequirements,
    ) -> Self {
        Self {
            name,
            owner_type,
            params,
            return_type,
            source,
            return_facet,
            context_requirements,
            resolved_return: default_resolved_return(),
            resolved_params: default_resolved_params(),
        }
    }

    /// Получить резолвленный тип возврата (lazy, с кэшированием)
    ///
    /// При первом вызове выполняет резолвинг через переданную функцию.
    /// Последующие вызовы возвращают закэшированное значение.
    ///
    /// # Arguments
    /// * `resolve_fn` - Функция резолвинга: принимает строку типа, возвращает TypeResolution
    ///
    /// # Returns
    /// * `Some(&TypeResolution)` - если return_type определён
    /// * `None` - если return_type = None (процедура без возвращаемого значения)
    ///
    /// # Example
    /// ```ignore
    /// let resolved = method.get_resolved_return_type(|type_str| {
    ///     resolver.resolve_expression_sync(type_str)
    /// });
    /// ```
    pub fn get_resolved_return_type<F>(&self, resolve_fn: F) -> Option<&TypeResolution>
    where
        F: FnOnce(&str) -> TypeResolution,
    {
        self.resolved_return
            .get_or_init(|| self.return_type.as_ref().map(|rt| resolve_fn(rt)))
            .as_ref()
    }

    /// Получить резолвленные типы параметров (lazy, с кэшированием)
    ///
    /// При первом вызове выполняет резолвинг всех параметров.
    /// Последующие вызовы возвращают закэшированное значение.
    ///
    /// # Arguments
    /// * `resolve_fn` - Функция резолвинга: принимает строку типа, возвращает TypeResolution
    ///
    /// # Returns
    /// Слайс пар (имя_параметра, TypeResolution)
    ///
    /// # Example
    /// ```ignore
    /// let params = method.get_resolved_params(|type_str| {
    ///     resolver.resolve_expression_sync(type_str)
    /// });
    /// for (name, resolution) in params {
    ///     println!("{}: {:?}", name, resolution);
    /// }
    /// ```
    pub fn get_resolved_params<F>(&self, resolve_fn: F) -> &[(String, TypeResolution)]
    where
        F: Fn(&str) -> TypeResolution,
    {
        self.resolved_params.get_or_init(|| {
            self.params
                .iter()
                .map(|p| {
                    let resolved = p
                        .type_name
                        .as_ref()
                        .map(|t| resolve_fn(t))
                        .unwrap_or_else(TypeResolution::unknown);
                    (p.name.clone(), resolved)
                })
                .collect()
        })
    }

    /// Проверить, закэширован ли тип возврата
    pub fn has_cached_return_type(&self) -> bool {
        self.resolved_return.get().is_some()
    }

    /// Проверить, закэшированы ли типы параметров
    pub fn has_cached_params(&self) -> bool {
        self.resolved_params.get().is_some()
    }

    /// Сбросить кэш резолвленных типов
    ///
    /// Создаёт новые пустые кэши. Полезно при изменении TypeResolver.
    /// ВНИМАНИЕ: Это НЕ влияет на уже склонированные сигнатуры!
    pub fn reset_cache(&mut self) {
        self.resolved_return = default_resolved_return();
        self.resolved_params = default_resolved_params();
    }
}
