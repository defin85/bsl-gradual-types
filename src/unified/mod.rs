//! Единая архитектура системы типов BSL
//!
//! Практичная реализация слоистой архитектуры для системы типов
//! с акцентом на единый источник данных и чистые интерфейсы.

// Временное связывание: подключаем исходные модули из каталога `architecture/`
// через реэкспорт, чтобы избежать дублирующихся определений модулей.

pub use crate::architecture::application;
pub mod data;
pub use crate::architecture::domain;
pub use crate::architecture::presentation;
pub use crate::architecture::system;