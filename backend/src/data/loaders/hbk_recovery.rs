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
//! use bsl_backend::data::loaders::hbk_recovery::recover_hbk_file;
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

use anyhow::{anyhow, Context, Result};
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

/// Компонента для восстановления повреждённых .hbk файлов
pub struct HbkRecovery {
    options: RecoveryOptions,
}

/// Опции для процесса восстановления
#[derive(Debug, Clone)]
pub struct RecoveryOptions {
    /// Автоматически удалять временные файлы после распаковки
    pub cleanup_temp: bool,
    /// Автоматически распаковывать восстановленный ZIP
    pub auto_extract: bool,
    /// Максимальный размер файла для обработки (защита от огромных файлов)
    pub max_file_size: usize,
}

impl Default for RecoveryOptions {
    fn default() -> Self {
        Self {
            cleanup_temp: true,
            auto_extract: true,
            max_file_size: 500 * 1024 * 1024, // 500 MB
        }
    }
}

/// Результат восстановления .hbk файла
#[derive(Debug)]
pub struct RecoveryResult {
    /// Путь к восстановленному ZIP файлу
    pub repaired_zip_path: PathBuf,
    /// Путь к распакованной директории (если auto_extract = true)
    pub extracted_dir: Option<PathBuf>,
    /// Смещение, где был найден ZIP signature
    pub signature_offset: usize,
    /// Размер восстановленного ZIP архива
    pub recovered_size: usize,
}

impl HbkRecovery {
    /// Создать новый экземпляр с опциями по умолчанию
    pub fn new() -> Self {
        Self::with_options(RecoveryOptions::default())
    }

    /// Создать новый экземпляр с пользовательскими опциями
    pub fn with_options(options: RecoveryOptions) -> Self {
        Self { options }
    }

    /// Восстановить .hbk файл
    ///
    /// # Аргументы
    ///
    /// * `hbk_path` - Путь к исходному .hbk файлу
    /// * `output_dir` - Директория для сохранения результатов (по умолчанию: рядом с исходным)
    ///
    /// # Возвращает
    ///
    /// `RecoveryResult` с информацией о восстановленных файлах
    ///
    /// # Ошибки
    ///
    /// - Файл не существует
    /// - Файл слишком большой (> max_file_size)
    /// - ZIP signature не найдена
    /// - Ошибка распаковки
    pub fn recover(&mut self, hbk_path: &Path, output_dir: Option<&Path>) -> Result<RecoveryResult> {
        info!("🔧 Начинаем восстановление: {:?}", hbk_path);

        // Проверяем существование файла
        if !hbk_path.exists() {
            return Err(anyhow!("Файл не существует: {:?}", hbk_path));
        }

        // Проверяем размер файла
        let file_size = fs::metadata(hbk_path)
            .context("Не удалось получить метаданные файла")?
            .len() as usize;

        if file_size > self.options.max_file_size {
            return Err(anyhow!(
                "Файл слишком большой: {} байт (max: {} байт)",
                file_size,
                self.options.max_file_size
            ));
        }

        debug!("📊 Размер файла: {} байт", file_size);

        // Открываем файл
        let mut file = File::open(hbk_path)
            .context(format!("Не удалось открыть файл: {:?}", hbk_path))?;

        // Ищем ZIP signature
        let signature_offset = signature::find_zip_signature(&mut file, file_size)
            .context("Поиск ZIP signature")?;

        info!("✅ ZIP signature найдена на offset: {}", signature_offset);

        // Определяем директорию для вывода
        let output_dir = match output_dir {
            Some(dir) => dir.to_path_buf(),
            None => hbk_path.parent()
                .ok_or_else(|| anyhow!("Не удалось определить родительскую директорию"))?
                .to_path_buf(),
        };

        // Создаём директорию если её нет
        fs::create_dir_all(&output_dir)
            .context(format!("Не удалось создать директорию: {:?}", output_dir))?;

        // Проверяем права на запись в директорию
        let test_file_path = output_dir.join(".write_test");
        fs::File::create(&test_file_path)
            .and_then(|_| fs::remove_file(&test_file_path))
            .context(format!("Директория недоступна для записи: {:?}", output_dir))?;

        // Формируем путь для восстановленного ZIP
        let file_stem = hbk_path.file_stem()
            .ok_or_else(|| anyhow!("Не удалось получить имя файла"))?;
        let repaired_zip_path = output_dir.join(format!("{}_recovered.zip", file_stem.to_string_lossy()));

        // Извлекаем валидный ZIP
        let recovered_size = extractor::extract_valid_zip(&mut file, signature_offset, &repaired_zip_path)
            .context("Извлечение ZIP архива")?;

        info!("✅ ZIP архив восстановлен: {} байт → {:?}", recovered_size, repaired_zip_path);

        // Распаковываем если требуется
        let extracted_dir = if self.options.auto_extract {
            // Добавляем префикс "rebuilt." для совместимости с SyntaxHelperParser
            let extract_dir = output_dir.join(format!("rebuilt.{}", file_stem.to_string_lossy()));

            extractor::unpack_zip(&repaired_zip_path, &extract_dir)
                .context("Распаковка ZIP архива")?;

            info!("✅ Архив распакован в: {:?}", extract_dir);

            // Удаляем временный ZIP если требуется
            if self.options.cleanup_temp {
                fs::remove_file(&repaired_zip_path)
                    .context("Удаление временного ZIP файла")?;
                debug!("🗑️ Временный ZIP удалён");
            }

            Some(extract_dir)
        } else {
            None
        };

        Ok(RecoveryResult {
            repaired_zip_path,
            extracted_dir,
            signature_offset,
            recovered_size,
        })
    }
}

impl Default for HbkRecovery {
    fn default() -> Self {
        Self::new()
    }
}

/// Утилитарная функция для быстрого восстановления .hbk файла
///
/// Эквивалентно вызову `HbkRecovery::new().recover(hbk_path, output_dir)`
///
/// # Пример
///
/// ```no_run
/// use bsl_backend::data::loaders::hbk_recovery::recover_hbk_file;
/// use std::path::Path;
///
/// let result = recover_hbk_file(
///     Path::new("shcntx_ru.hbk"),
///     Some(Path::new("output"))
/// )?;
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn recover_hbk_file(hbk_path: &Path, output_dir: Option<&Path>) -> Result<RecoveryResult> {
    HbkRecovery::new().recover(hbk_path, output_dir)
}

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
    let entries = fs::read_dir(dir)
        .context(format!("Не удалось прочитать директорию: {:?}", dir))?;

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
    let entries = fs::read_dir(dir)
        .context(format!("Не удалось прочитать директорию: {:?}", dir))?;

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

// ============================================================================
// Приватные модули
// ============================================================================

/// Модуль для поиска ZIP signature в файле
mod signature {
    use anyhow::{anyhow, Result};
    use std::fs::File;
    use std::io::{Read, Seek};
    use tracing::debug;

    /// ZIP signature: "PK\x03\x04"
    const ZIP_SIGNATURE: &[u8] = &[0x50, 0x4B, 0x03, 0x04];

    /// Размер чанка для чтения (64 KB)
    const CHUNK_SIZE: usize = 64 * 1024;

    /// Находит смещение ZIP signature в файле
    ///
    /// Читает файл чанками по 64 KB и ищет паттерн [0x50, 0x4B, 0x03, 0x04]
    /// с использованием sliding window.
    ///
    /// # Аргументы
    ///
    /// * `file` - Открытый файл для поиска
    /// * `max_search_size` - Максимальный размер для поиска (обычно = размер файла)
    ///
    /// # Возвращает
    ///
    /// Смещение (offset) начала ZIP signature
    ///
    /// # Ошибки
    ///
    /// - ZIP signature не найдена
    /// - Ошибка чтения файла
    pub(super) fn find_zip_signature(file: &mut File, max_search_size: usize) -> Result<usize> {
        debug!("🔎 Ищем ZIP signature (максимум: {} байт)", max_search_size);

        const OVERLAP_SIZE: usize = ZIP_SIGNATURE.len() - 1; // 3 байта для signature размером 4

        let mut buffer = vec![0u8; CHUNK_SIZE];
        let mut total_read = 0;
        let mut overlap_buffer = Vec::new();
        let mut chunk_start_offset = 0;  // Отслеживаем начало текущего чанка

        // Сбрасываем позицию в начало файла
        file.rewind()?;

        while total_read < max_search_size {
            // Читаем очередной чанк
            let bytes_read = file.read(&mut buffer)?;
            if bytes_read == 0 {
                break; // EOF
            }

            // Объединяем overlap с новыми данными для поиска через границы чанков
            let search_data = if overlap_buffer.is_empty() {
                &buffer[..bytes_read]
            } else {
                overlap_buffer.extend_from_slice(&buffer[..bytes_read]);
                &overlap_buffer[..]
            };

            // Ищем signature через sliding window
            if let Some(pos) = search_data.windows(ZIP_SIGNATURE.len())
                .position(|window| window == ZIP_SIGNATURE)
            {
                // Нашли! Вычисляем абсолютный offset
                let absolute_offset = if overlap_buffer.is_empty() {
                    // Signature в текущем чанке без overlap
                    chunk_start_offset + pos
                } else {
                    // Signature в overlap+chunk области
                    // Начало overlap = chunk_start_offset - OVERLAP_SIZE
                    chunk_start_offset - OVERLAP_SIZE + pos
                };

                debug!("✅ ZIP signature найдена на offset: {}", absolute_offset);
                return Ok(absolute_offset);
            }

            total_read += bytes_read;

            // Сохраняем последние 3 байта для overlap
            // На случай если signature находится на границе чанков
            overlap_buffer.clear();
            if bytes_read >= OVERLAP_SIZE {
                let overlap_start = bytes_read - OVERLAP_SIZE;
                overlap_buffer.extend_from_slice(&buffer[overlap_start..bytes_read]);
            }

            // Обновляем начало следующего чанка
            chunk_start_offset = total_read;
        }

        Err(anyhow!("ZIP signature не найдена в первых {} байтах", total_read))
    }
}

/// Модуль для извлечения и распаковки ZIP архивов
mod extractor {
    use anyhow::{Context, Result};
    use std::fs::{self, File};
    use std::io::{Seek, SeekFrom};
    use std::path::Path;
    use tracing::{debug, info};

    /// Извлекает валидный ZIP начиная с указанного offset
    ///
    /// Копирует данные от offset до конца файла в новый файл.
    ///
    /// # Аргументы
    ///
    /// * `source_file` - Исходный файл (открытый)
    /// * `offset` - Смещение начала ZIP данных
    /// * `output_path` - Путь для сохранения восстановленного ZIP
    ///
    /// # Возвращает
    ///
    /// Размер восстановленного ZIP архива в байтах
    pub(super) fn extract_valid_zip(
        source_file: &mut File,
        offset: usize,
        output_path: &Path,
    ) -> Result<usize> {
        debug!("📦 Извлекаем ZIP с offset {} в {:?}", offset, output_path);

        // Перемещаемся к началу ZIP данных
        source_file.seek(SeekFrom::Start(offset as u64))
            .context("Не удалось переместиться к offset")?;

        // Создаём выходной файл
        let mut output_file = File::create(output_path)
            .context(format!("Не удалось создать файл: {:?}", output_path))?;

        // Копируем данные
        let bytes_copied = std::io::copy(source_file, &mut output_file)
            .context("Ошибка копирования данных")?;

        debug!("✅ Скопировано {} байт", bytes_copied);

        Ok(bytes_copied as usize)
    }

    /// Распаковывает ZIP архив в указанную директорию
    ///
    /// Использует библиотеку `zip` для распаковки.
    ///
    /// # Аргументы
    ///
    /// * `zip_path` - Путь к ZIP архиву
    /// * `target_dir` - Директория для распаковки
    ///
    /// # Безопасность
    ///
    /// Проверяет, что все файлы в архиве распаковываются внутри target_dir
    /// (защита от zip-slip атак).
    pub(super) fn unpack_zip(zip_path: &Path, target_dir: &Path) -> Result<()> {
        debug!("📂 Распаковываем ZIP {:?} → {:?}", zip_path, target_dir);

        // Создаём целевую директорию
        fs::create_dir_all(target_dir)
            .context(format!("Не удалось создать директорию: {:?}", target_dir))?;

        // Открываем ZIP архив
        let file = File::open(zip_path)
            .context(format!("Не удалось открыть ZIP: {:?}", zip_path))?;

        let mut archive = zip::ZipArchive::new(file)
            .context("Не удалось прочитать ZIP архив")?;

        info!("📊 Файлов в архиве: {}", archive.len());

        // Распаковываем каждый файл
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)
                .context(format!("Не удалось прочитать файл #{} из архива", i))?;

            let file_path = match file.enclosed_name() {
                Some(path) => target_dir.join(path),
                None => {
                    debug!("⚠️ Пропускаем файл с небезопасным именем: {:?}", file.name());
                    continue;
                }
            };

            // Создаём родительские директории
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent)
                    .context(format!("Не удалось создать директорию: {:?}", parent))?;
            }

            if file.is_dir() {
                debug!("📁 Создаём директорию: {:?}", file_path);
                fs::create_dir_all(&file_path)
                    .context(format!("Не удалось создать директорию: {:?}", file_path))?;
            } else {
                debug!("📄 Извлекаем файл: {:?}", file_path);
                let mut outfile = File::create(&file_path)
                    .context(format!("Не удалось создать файл: {:?}", file_path))?;

                std::io::copy(&mut file, &mut outfile)
                    .context(format!("Ошибка записи файла: {:?}", file_path))?;
            }
        }

        info!("✅ ZIP распакован успешно");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_signature_search() {
        let temp_dir = tempdir().unwrap();
        let test_file_path = temp_dir.path().join("test.hbk");

        // Создаём тестовый файл с ZIP signature на offset 100
        let mut file = File::create(&test_file_path).unwrap();

        // Пишем 100 байт "мусора"
        file.write_all(&vec![0xFF; 100]).unwrap();

        // Пишем ZIP signature
        file.write_all(&[0x50, 0x4B, 0x03, 0x04]).unwrap();

        // Пишем ещё немного данных
        file.write_all(&vec![0x00; 100]).unwrap();

        drop(file);

        // Тестируем поиск
        let mut file = File::open(&test_file_path).unwrap();
        let offset = signature::find_zip_signature(&mut file, 1000).unwrap();

        assert_eq!(offset, 100, "ZIP signature должна быть найдена на offset 100");
    }

    #[test]
    fn test_recovery_options_default() {
        let options = RecoveryOptions::default();

        assert!(options.cleanup_temp);
        assert!(options.auto_extract);
        assert_eq!(options.max_file_size, 500 * 1024 * 1024);
    }

    #[test]
    fn test_find_signature_at_beginning() {
        let temp_dir = tempdir().unwrap();
        let test_file_path = temp_dir.path().join("test.hbk");

        // Создаём файл с signature в самом начале
        let mut file = File::create(&test_file_path).unwrap();
        file.write_all(&[0x50, 0x4B, 0x03, 0x04]).unwrap(); // ZIP signature
        file.write_all(&vec![0x00; 100]).unwrap();
        drop(file);

        // Тестируем поиск
        let mut file = File::open(&test_file_path).unwrap();
        let offset = signature::find_zip_signature(&mut file, 104).unwrap();

        assert_eq!(offset, 0, "Signature должна быть найдена в позиции 0");
    }

    #[test]
    fn test_find_signature_large_offset() {
        let temp_dir = tempdir().unwrap();
        let test_file_path = temp_dir.path().join("test.hbk");

        // Создаём файл с signature на большом offset
        let offset_expected = 10000;
        let mut file = File::create(&test_file_path).unwrap();
        file.write_all(&vec![0xFF; offset_expected]).unwrap();
        file.write_all(&[0x50, 0x4B, 0x03, 0x04]).unwrap();
        file.write_all(&vec![0x00; 500]).unwrap();
        drop(file);

        // Тестируем поиск
        let mut file = File::open(&test_file_path).unwrap();
        let offset = signature::find_zip_signature(&mut file, 10600).unwrap();

        assert_eq!(offset, offset_expected, "Signature должна быть найдена на offset {}", offset_expected);
    }

    #[test]
    fn test_find_signature_near_chunk_boundary() {
        let temp_dir = tempdir().unwrap();
        let test_file_path = temp_dir.path().join("test.hbk");

        // Создаём файл где signature на границе chunk (64KB - 2)
        let chunk_size = 64 * 1024;
        let signature_pos = chunk_size - 2;

        let mut file = File::create(&test_file_path).unwrap();
        file.write_all(&vec![0xFF; signature_pos]).unwrap();
        file.write_all(&[0x50, 0x4B, 0x03, 0x04]).unwrap();
        file.write_all(&vec![0x00; 100]).unwrap();
        drop(file);

        // Тестируем поиск
        let mut file = File::open(&test_file_path).unwrap();
        let offset = signature::find_zip_signature(&mut file, signature_pos + 104).unwrap();

        assert_eq!(offset, signature_pos, "Signature должна быть найдена на границе chunk");
    }

    #[test]
    fn test_signature_not_found() {
        let temp_dir = tempdir().unwrap();
        let test_file_path = temp_dir.path().join("test.hbk");

        // Создаём файл БЕЗ ZIP signature
        let mut file = File::create(&test_file_path).unwrap();
        file.write_all(&vec![0xFF; 10000]).unwrap();
        drop(file);

        // Тестируем поиск
        let mut file = File::open(&test_file_path).unwrap();
        let result = signature::find_zip_signature(&mut file, 10000);

        assert!(result.is_err(), "Should fail without ZIP signature");
    }

    #[test]
    fn test_empty_file_signature_search() {
        let temp_dir = tempdir().unwrap();
        let test_file_path = temp_dir.path().join("empty.hbk");

        // Создаём пустой файл
        File::create(&test_file_path).unwrap();

        // Тестируем поиск
        let mut file = File::open(&test_file_path).unwrap();
        let result = signature::find_zip_signature(&mut file, 0);

        assert!(result.is_err(), "Should fail on empty file");
    }

    #[test]
    fn test_extractor_extract_full_file() {
        let temp_dir = tempdir().unwrap();
        let source_path = temp_dir.path().join("source.hbk");
        let output_path = temp_dir.path().join("output.zip");

        // Пустой ZIP архив (EOCD = End of Central Directory)
        let empty_zip = vec![
            0x50, 0x4B, 0x05, 0x06, // EOCD signature
            0x00, 0x00, // Number of this disk
            0x00, 0x00, // Number of the disk with the start of the central directory
            0x00, 0x00, // Total number of entries in the central directory on this disk
            0x00, 0x00, // Total number of entries in the central directory
            0x00, 0x00, 0x00, 0x00, // Size of the central directory
            0x00, 0x00, 0x00, 0x00, // Offset of the central directory
            0x00, 0x00, // ZIP comment length
        ];

        // Создаём исходный файл
        let mut file = File::create(&source_path).unwrap();
        file.write_all(&vec![0xFF; 1000]).unwrap();
        file.write_all(&[0x50, 0x4B, 0x05, 0x06]).unwrap();
        file.write_all(&empty_zip[4..]).unwrap();
        drop(file);

        // Открываем и извлекаем
        let mut source_file = File::open(&source_path).unwrap();
        let size = extractor::extract_valid_zip(&mut source_file, 1000, &output_path).unwrap();

        assert!(output_path.exists(), "Output ZIP должен существовать");
        assert_eq!(size, empty_zip.len() as usize, "Размер должен совпадать с ZIP");
    }

    #[test]
    fn test_extractor_extract_from_offset() {
        let temp_dir = tempdir().unwrap();
        let source_path = temp_dir.path().join("source.hbk");
        let output_path = temp_dir.path().join("output.zip");

        // Пустой ZIP архив (EOCD = End of Central Directory)
        let empty_zip = vec![
            0x50, 0x4B, 0x05, 0x06, // EOCD signature
            0x00, 0x00, // Number of this disk
            0x00, 0x00, // Number of the disk with the start of the central directory
            0x00, 0x00, // Total number of entries in the central directory on this disk
            0x00, 0x00, // Total number of entries in the central directory
            0x00, 0x00, 0x00, 0x00, // Size of the central directory
            0x00, 0x00, 0x00, 0x00, // Offset of the central directory
            0x00, 0x00, // ZIP comment length
        ];

        // Создаём исходный файл
        let junk_size = 5000;
        let mut file = File::create(&source_path).unwrap();
        file.write_all(&vec![0xFF; junk_size]).unwrap();
        file.write_all(&empty_zip).unwrap();
        drop(file);

        // Открываем и извлекаем с offset
        let mut source_file = File::open(&source_path).unwrap();
        let size = extractor::extract_valid_zip(&mut source_file, junk_size, &output_path).unwrap();

        assert!(output_path.exists(), "Output ZIP должен существовать");
        assert_eq!(size, empty_zip.len() as usize, "Размер должен быть размер ZIP");
    }

    #[test]
    fn test_hbk_recovery_with_custom_options() {
        let temp_dir = tempdir().unwrap();
        let test_file_path = temp_dir.path().join("test.hbk");

        // Создаём файл с auto_extract = false, чтобы не нужна была распаковка
        // Просто используем ZIP signature без полного ZIP архива
        let mut file = File::create(&test_file_path).unwrap();
        file.write_all(&vec![0xFF; 1000]).unwrap();
        file.write_all(&[0x50, 0x4B, 0x03, 0x04]).unwrap(); // ZIP signature
        file.write_all(&vec![0x00; 500]).unwrap();
        drop(file);

        // Создаём HbkRecovery с пользовательскими опциями
        let options = RecoveryOptions {
            cleanup_temp: false,
            auto_extract: false, // НЕ распаковываем, поэтому не нужен корректный ZIP
            max_file_size: 1024 * 1024, // 1 MB
        };

        let mut recovery = HbkRecovery::with_options(options);
        let result = recovery.recover(&test_file_path, Some(temp_dir.path())).unwrap();

        // Проверяем что опции работают
        assert!(result.repaired_zip_path.exists(), "ZIP должен существовать (cleanup_temp = false)");
        assert!(result.extracted_dir.is_none(), "Директория не должна быть распакована (auto_extract = false)");
    }

    #[test]
    fn test_recovery_result_properties() {
        let temp_dir = tempdir().unwrap();
        let test_file_path = temp_dir.path().join("test.hbk");

        let junk_offset = 2500;

        // Используем auto_extract = false чтобы не нужно было распаковывать
        let mut file = File::create(&test_file_path).unwrap();
        file.write_all(&vec![0xFF; junk_offset]).unwrap();
        file.write_all(&[0x50, 0x4B, 0x03, 0x04]).unwrap(); // ZIP signature
        file.write_all(&vec![0x00; 300]).unwrap();
        drop(file);

        // Восстанавливаем с auto_extract = false
        let mut recovery = HbkRecovery::with_options(RecoveryOptions {
            cleanup_temp: true,
            auto_extract: false,
            max_file_size: 10 * 1024 * 1024,
        });
        let result = recovery.recover(&test_file_path, Some(temp_dir.path())).unwrap();

        // Проверяем свойства результата
        assert_eq!(result.signature_offset, junk_offset, "signature_offset должен совпадать");
        assert_eq!(result.recovered_size, 304, "recovered_size должен быть 4 (signature) + 300");
        assert!(result.repaired_zip_path.exists(), "repaired_zip_path должен существовать");
        assert!(result.extracted_dir.is_none(), "extracted_dir должна быть None при auto_extract = false");
    }
}
