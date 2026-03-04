//! HBK Recovery Module
//!
//! Компонента для восстановления повреждённых .hbk файлов синтаксис-помощника 1С.
//!
//! ## Проблема
//!
//! Файлы синтаксис-помощника 1С (shcntx*.hbk) имеют специфический формат:
//! - Первые ~1600 байт: бинарный заголовок (назначение неизвестно)
//! - Остальное: валидный ZIP архив
//!
//! Стандартные ZIP-библиотеки не могут открыть такие файлы из-за "мусора" в начале.
//!
//! ## Решение
//!
//! 1. **Поиск ZIP signature**: находим начало валидного ZIP архива (0x50 0x4B 0x03 0x04)
//! 2. **Извлечение**: копируем данные с найденного offset до конца файла
//! 3. **Распаковка**: стандартная распаковка восстановленного ZIP
//!
//! ## Архитектура
//!
//! ```text
//! HbkRecovery
//!   ├── signature::find_zip_signature()  → offset
//!   ├── extractor::extract_valid_zip()   → recovered.zip
//!   └── extractor::unpack_zip()          → extracted_dir/
//! ```
//!
//! ## Пример использования
//!
//! ```no_run
//! use bsl_runtime::data::loaders::hbk_recovery::recover_hbk_file;
//! use std::path::Path;
//!
//! let result = recover_hbk_file(
//!     Path::new("shcntx_ru.hbk"),
//!     Some(Path::new("output"))
//! )?;
//!
//! println!("Восстановлен ZIP: {:?}", result.repaired_zip_path);
//! println!("Распаковано в: {:?}", result.extracted_dir);
//! # Ok::<(), anyhow::Error>(())
//! ```

// Внутренние модули
mod batch;
mod extractor;
mod recovery;
pub(crate) mod signature;
mod types;

// Публичный API
pub use batch::{
    auto_recover_directory, auto_recover_directory_with_options,
    auto_recover_directory_with_progress,
};
pub use recovery::{recover_hbk_file, HbkRecovery};
pub use types::{RecoveryOptions, RecoveryResult};

#[cfg(test)]
mod tests;
