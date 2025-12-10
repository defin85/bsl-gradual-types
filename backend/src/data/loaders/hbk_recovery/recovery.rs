//! Основная логика восстановления HBK файлов

use anyhow::{anyhow, Context, Result};
use std::fs::{self, File};
use std::path::Path;
use tracing::{debug, info, warn};

use crate::data::loaders::progress::ProgressUpdateType;

use super::extractor;
use super::signature;
use super::types::{RecoveryOptions, RecoveryResult};

/// Компонента для восстановления повреждённых .hbk файлов
pub struct HbkRecovery {
    options: RecoveryOptions,
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

    /// Восстановить .hbk файл с опциональной callback для отслеживания прогресса
    ///
    /// # Аргументы
    ///
    /// * `hbk_path` - Путь к исходному .hbk файлу
    /// * `output_dir` - Директория для сохранения результатов (по умолчанию: рядом с исходным)
    /// * `progress_callback` - Опциональная callback функция для отслеживания прогресса
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
    pub fn recover_with_progress<F>(
        &mut self,
        hbk_path: &Path,
        output_dir: Option<&Path>,
        progress_callback: Option<F>,
    ) -> Result<RecoveryResult>
    where
        F: Fn(ProgressUpdateType) + Clone,
    {
        info!("🔧 Начинаем восстановление: {:?}", hbk_path);

        // Проверяем существование файла
        if !hbk_path.exists() {
            return Err(anyhow!("Файл не существует: {:?}", hbk_path));
        }

        // Определяем директорию для вывода
        let output_dir = match output_dir {
            Some(dir) => dir.to_path_buf(),
            None => hbk_path
                .parent()
                .ok_or_else(|| anyhow!("Не удалось определить родительскую директорию"))?
                .to_path_buf(),
        };

        // Получаем имя файла
        let file_stem = hbk_path
            .file_stem()
            .ok_or_else(|| anyhow!("Не удалось получить имя файла"))?;

        // Проверяем кеш ПЕРЕД началом обработки
        let extract_dir_name = format!("rebuilt.{}", file_stem.to_string_lossy());
        let extract_dir = output_dir.join(&extract_dir_name);

        if extract_dir.exists() && extract_dir.is_dir() {
            match fs::read_dir(&extract_dir) {
                Ok(entries) => {
                    let entry_count = entries.count();
                    if entry_count > 0 {
                        info!(
                            "⚡ Используем кеш: {:?} ({} файлов)",
                            extract_dir, entry_count
                        );

                        // Возвращаем результат из кеша (БЕЗ распаковки)
                        return Ok(RecoveryResult {
                            repaired_zip_path: output_dir
                                .join(format!("{}_recovered.zip", file_stem.to_string_lossy())),
                            extracted_dir: Some(extract_dir),
                            signature_offset: 0, // Неизвестно из кеша
                            recovered_size: 0,   // Неизвестно из кеша
                        });
                    } else {
                        warn!(
                            "⚠️ Кеш {:?} пустой, пересоздаём",
                            extract_dir
                        );
                    }
                }
                Err(e) => {
                    warn!(
                        "⚠️ Ошибка чтения кеша {:?}: {}, пересоздаём",
                        extract_dir, e
                    );
                }
            }
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
        let mut file =
            File::open(hbk_path).context(format!("Не удалось открыть файл: {:?}", hbk_path))?;

        // Ищем ZIP signature
        let signature_offset =
            signature::find_zip_signature(&mut file, file_size).context("Поиск ZIP signature")?;

        info!("✅ ZIP signature найдена на offset: {}", signature_offset);

        // Создаём директорию если её нет
        fs::create_dir_all(&output_dir)
            .context(format!("Не удалось создать директорию: {:?}", output_dir))?;

        // Проверяем права на запись в директорию
        let test_file_path = output_dir.join(".write_test");
        fs::File::create(&test_file_path)
            .and_then(|_| fs::remove_file(&test_file_path))
            .context(format!(
                "Директория недоступна для записи: {:?}",
                output_dir
            ))?;

        // Формируем путь для восстановленного ZIP
        let repaired_zip_path =
            output_dir.join(format!("{}_recovered.zip", file_stem.to_string_lossy()));

        // Извлекаем валидный ZIP
        let recovered_size =
            extractor::extract_valid_zip(&mut file, signature_offset, &repaired_zip_path)
                .context("Извлечение ZIP архива")?;

        info!(
            "✅ ZIP архив восстановлен: {} байт → {:?}",
            recovered_size, repaired_zip_path
        );

        // Распаковываем если требуется
        let extracted_dir = if self.options.auto_extract {
            // Добавляем префикс "rebuilt." для совместимости с SyntaxHelperParser
            let extract_dir = output_dir.join(format!("rebuilt.{}", file_stem.to_string_lossy()));

            extractor::unpack_zip_with_progress(&repaired_zip_path, &extract_dir, progress_callback.clone())
                .context("Распаковка ZIP архива")?;

            info!("✅ Архив распакован в: {:?}", extract_dir);

            // Удаляем временный ZIP если требуется
            if self.options.cleanup_temp {
                fs::remove_file(&repaired_zip_path).context("Удаление временного ZIP файла")?;
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

    /// Восстановить .hbk файл (без callback)
    ///
    /// # Аргументы
    ///
    /// * `hbk_path` - Путь к исходному .hbk файлу
    /// * `output_dir` - Директория для сохранения результатов (по умолчанию: рядом с исходным)
    ///
    /// # Возвращает
    ///
    /// `RecoveryResult` с информацией о восстановленных файлах
    pub fn recover(
        &mut self,
        hbk_path: &Path,
        output_dir: Option<&Path>,
    ) -> Result<RecoveryResult> {
        self.recover_with_progress(hbk_path, output_dir, None::<fn(ProgressUpdateType)>)
    }

    /// Очистить кеш для конкретного .hbk файла
    ///
    /// Удаляет распакованную директорию кеша для указанного .hbk файла.
    ///
    /// # Аргументы
    ///
    /// * `hbk_path` - Путь к исходному .hbk файлу
    /// * `output_dir` - Директория, где хранится кеш (по умолчанию: рядом с исходным)
    ///
    /// # Возвращает
    ///
    /// `Ok(())` если кеш был удалён или не существовал
    ///
    /// # Ошибки
    ///
    /// - Ошибка при удалении директории
    ///
    /// # Пример
    ///
    /// ```no_run
    /// use bsl_backend::data::loaders::hbk_recovery::HbkRecovery;
    /// use std::path::Path;
    ///
    /// HbkRecovery::clear_cache(
    ///     Path::new("shcntx_ru.hbk"),
    ///     Some(Path::new("output"))
    /// )?;
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    pub fn clear_cache(
        hbk_path: &Path,
        output_dir: Option<&Path>,
    ) -> Result<()> {
        // Определяем директорию для вывода
        let output_dir = match output_dir {
            Some(dir) => dir.to_path_buf(),
            None => hbk_path
                .parent()
                .ok_or_else(|| anyhow!("Не удалось определить родительскую директорию"))?
                .to_path_buf(),
        };

        let file_stem = hbk_path
            .file_stem()
            .ok_or_else(|| anyhow!("Не удалось получить имя файла"))?;

        let extract_dir_name = format!("rebuilt.{}", file_stem.to_string_lossy());
        let extract_dir = output_dir.join(&extract_dir_name);

        if extract_dir.exists() {
            fs::remove_dir_all(&extract_dir)
                .context(format!("Не удалось удалить кеш: {:?}", extract_dir))?;
            info!("🗑️  Кеш удалён: {:?}", extract_dir);
        } else {
            debug!("ℹ️  Кеш не найден: {:?}", extract_dir);
        }

        Ok(())
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
