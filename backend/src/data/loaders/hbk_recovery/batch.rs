//! Batch операции для восстановления нескольких HBK файлов

use anyhow::{anyhow, Context, Result};
use std::fs;
use std::path::Path;
use tracing::{debug, info, warn};

use crate::data::loaders::progress::ProgressUpdateType;

use super::recovery::HbkRecovery;
use super::types::{RecoveryOptions, RecoveryResult};

/// Автоматически восстанавливает все .hbk файлы в директории
///
/// Сканирует указанную директорию в поисках файлов с расширением `.hbk`
/// и восстанавливает каждый найденный файл.
///
/// # Graceful Degradation
///
/// Функция НЕ прерывается при ошибках обработки отдельных файлов.
/// Все ошибки логируются как warnings, но обработка продолжается.
///
/// # Аргументы
///
/// * `dir` - Директория для сканирования
///
/// # Возвращает
///
/// Список успешно восстановленных файлов
///
/// # Пример
///
/// ```no_run
/// use bsl_backend::data::loaders::hbk_recovery::auto_recover_directory;
/// use std::path::Path;
///
/// let results = auto_recover_directory(Path::new("examples/syntax_helper"))?;
/// println!("Восстановлено файлов: {}", results.len());
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn auto_recover_directory(dir: &Path) -> Result<Vec<RecoveryResult>> {
    debug!("🔍 Сканируем директорию: {:?}", dir);

    if !dir.exists() {
        return Err(anyhow!("Директория не существует: {:?}", dir));
    }

    if !dir.is_dir() {
        return Err(anyhow!("Путь не является директорией: {:?}", dir));
    }

    let mut results = Vec::new();
    let mut recovery = HbkRecovery::new();

    // Ищем все .hbk файлы
    let entries =
        fs::read_dir(dir).context(format!("Не удалось прочитать директорию: {:?}", dir))?;

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                warn!("⚠️ Ошибка чтения записи в директории: {}", e);
                continue;
            }
        };

        let path = entry.path();

        // Проверяем расширение
        if path.extension().and_then(|e| e.to_str()) != Some("hbk") {
            continue;
        }

        debug!("📁 Найден .hbk файл: {:?}", path);

        // Пытаемся восстановить
        match recovery.recover(&path, Some(dir)) {
            Ok(result) => {
                info!("✅ Успешно восстановлен: {:?}", path);
                results.push(result);
            }
            Err(e) => {
                warn!("⚠️ Не удалось восстановить {:?}: {}", path, e);
                // Продолжаем обработку других файлов
            }
        }
    }

    debug!("📊 Итого восстановлено: {} файлов", results.len());
    Ok(results)
}

/// Автоматически восстанавливает все .hbk файлы в директории с заданными опциями
///
/// # Arguments
/// * `dir` - Директория для сканирования
/// * `options` - Опции восстановления
///
/// # Returns
/// Вектор с результатами восстановления для каждого файла
///
/// # Example
/// ```no_run
/// use bsl_backend::data::loaders::hbk_recovery::{auto_recover_directory_with_options, RecoveryOptions};
/// use std::path::Path;
///
/// let options = RecoveryOptions {
///     auto_extract: false,
///     ..Default::default()
/// };
/// let results = auto_recover_directory_with_options(Path::new("examples/syntax_helper"), options)?;
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn auto_recover_directory_with_options(
    dir: &Path,
    options: RecoveryOptions,
) -> Result<Vec<RecoveryResult>> {
    debug!("🔍 Сканируем директорию: {:?}", dir);

    if !dir.exists() {
        return Err(anyhow!("Директория не существует: {:?}", dir));
    }

    if !dir.is_dir() {
        return Err(anyhow!("Путь не является директорией: {:?}", dir));
    }

    let mut results = Vec::new();
    let mut recovery = HbkRecovery::with_options(options);

    // Ищем все .hbk файлы
    let entries =
        fs::read_dir(dir).context(format!("Не удалось прочитать директорию: {:?}", dir))?;

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                warn!("⚠️ Ошибка чтения записи в директории: {}", e);
                continue;
            }
        };

        let path = entry.path();

        // Проверяем расширение
        if path.extension().and_then(|e| e.to_str()) != Some("hbk") {
            continue;
        }

        debug!("📁 Найден .hbk файл: {:?}", path);

        // Пытаемся восстановить
        match recovery.recover(&path, Some(dir)) {
            Ok(result) => {
                info!("✅ Успешно восстановлен: {:?}", path);
                results.push(result);
            }
            Err(e) => {
                warn!("⚠️ Не удалось восстановить {:?}: {}", path, e);
                // Продолжаем обработку других файлов
            }
        }
    }

    debug!("📊 Итого восстановлено: {} файлов", results.len());
    Ok(results)
}

/// Автоматически восстанавливает все .hbk файлы в директории с progress callback
///
/// # Arguments
/// * `dir` - Директория для сканирования
/// * `progress_callback` - Опциональный callback для отправки прогресса
///
/// # Returns
/// Вектор с результатами восстановления для каждого файла
pub fn auto_recover_directory_with_progress<F>(
    dir: &Path,
    progress_callback: Option<F>,
) -> Result<Vec<RecoveryResult>>
where
    F: Fn(ProgressUpdateType) + Clone,
{
    debug!("🔍 Сканируем директорию: {:?}", dir);

    if !dir.exists() {
        return Err(anyhow!("Директория не существует: {:?}", dir));
    }

    if !dir.is_dir() {
        return Err(anyhow!("Путь не является директорией: {:?}", dir));
    }

    let mut results = Vec::new();
    let mut recovery = HbkRecovery::new();

    // Ищем все .hbk файлы
    let entries =
        fs::read_dir(dir).context(format!("Не удалось прочитать директорию: {:?}", dir))?;

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                warn!("⚠️ Ошибка чтения записи в директории: {}", e);
                continue;
            }
        };

        let path = entry.path();

        // Проверяем расширение
        if path.extension().and_then(|e| e.to_str()) != Some("hbk") {
            continue;
        }

        debug!("📁 Найден .hbk файл: {:?}", path);

        // Пытаемся восстановить с progress callback
        let callback_clone = progress_callback.clone();
        match recovery.recover_with_progress(&path, Some(dir), callback_clone) {
            Ok(result) => {
                info!("✅ Успешно восстановлен: {:?}", path);
                results.push(result);
            }
            Err(e) => {
                warn!("⚠️ Не удалось восстановить {:?}: {}", path, e);
                // Продолжаем обработку других файлов
            }
        }
    }

    debug!("📊 Итого восстановлено: {} файлов", results.len());
    Ok(results)
}
