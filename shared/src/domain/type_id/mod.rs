//! Типобезопасные идентификаторы типов BSL.
//!
//! Этот модуль предоставляет унифицированную инфраструктуру для работы
//! с идентификаторами типов, обеспечивая:
//!
//! - Регистронезависимое сравнение и поиск
//! - Нормализацию имён для HashMap ключей
//! - Конвертацию между CamelCase и форматом с пробелами
//!
//! # Примеры
//!
//! ```
//! use bsl_shared::domain::type_id::TypeId;
//!
//! // Создание TypeId
//! let id1 = TypeId::new("ТаблицаЗначений");
//! let id2 = TypeId::new("таблицазначений");
//!
//! // Регистронезависимое сравнение
//! assert_eq!(id1, id2);
//!
//! // Использование как ключ HashMap
//! use std::collections::HashMap;
//! let mut types = HashMap::new();
//! types.insert(id1.clone(), "some data");
//! assert!(types.contains_key(&id2));
//! ```

pub mod normalization;
mod type_id;

pub use normalization::{camel_to_spaced, normalize, spaced_to_camel};
pub use type_id::TypeId;
