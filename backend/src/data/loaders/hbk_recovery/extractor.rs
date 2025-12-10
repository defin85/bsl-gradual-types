//! Модуль для извлечения и распаковки ZIP архивов

use anyhow::{Context, Result};
use std::fs::{self, File};
use std::io::{Seek, SeekFrom};
use std::path::Path;
use tracing::{debug, info};

use crate::data::loaders::progress::ProgressUpdateType;

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
pub fn extract_valid_zip(
    source_file: &mut File,
    offset: usize,
    output_path: &Path,
) -> Result<usize> {
    debug!("📦 Извлекаем ZIP с offset {} в {:?}", offset, output_path);

    // Перемещаемся к началу ZIP данных
    source_file
        .seek(SeekFrom::Start(offset as u64))
        .context("Не удалось переместиться к offset")?;

    // Создаём выходной файл
    let mut output_file = File::create(output_path)
        .context(format!("Не удалось создать файл: {:?}", output_path))?;

    // Копируем данные
    let bytes_copied =
        std::io::copy(source_file, &mut output_file).context("Ошибка копирования данных")?;

    debug!("✅ Скопировано {} байт", bytes_copied);

    Ok(bytes_copied as usize)
}

/// Распаковывает ZIP архив в указанную директорию с опциональным progress callback
///
/// Использует библиотеку `zip` для распаковки.
///
/// # Аргументы
///
/// * `zip_path` - Путь к ZIP архиву
/// * `target_dir` - Директория для распаковки
/// * `progress_callback` - Опциональная callback функция для отслеживания прогресса
///
/// # Безопасность
///
/// Проверяет, что все файлы в архиве распаковываются внутри target_dir
/// (защита от zip-slip атак).
pub fn unpack_zip_with_progress<F>(
    zip_path: &Path,
    target_dir: &Path,
    progress_callback: Option<F>,
) -> Result<()>
where
    F: Fn(ProgressUpdateType),
{
    debug!("📂 Распаковываем ZIP {:?} → {:?}", zip_path, target_dir);

    // Создаём целевую директорию
    fs::create_dir_all(target_dir)
        .context(format!("Не удалось создать директорию: {:?}", target_dir))?;

    // Открываем ZIP архив
    let file =
        File::open(zip_path).context(format!("Не удалось открыть ZIP: {:?}", zip_path))?;

    let mut archive = zip::ZipArchive::new(file).context("Не удалось прочитать ZIP архив")?;

    let total_files = archive.len();
    info!("📊 Файлов в архиве: {}", total_files);

    // Извлекаем имя архива для progress сообщений
    let file_name = zip_path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    // Распаковываем каждый файл
    for i in 0..total_files {
        // Логируем прогресс каждые 1000 файлов или в конце
        if i % 1000 == 0 || i == total_files - 1 {
            info!(
                "📦 Обработано {}/{} файлов ({}%)",
                i + 1,
                total_files,
                (i + 1) * 100 / total_files
            );
        }

        // Отправляем UI прогресс каждые 100 файлов
        if i % 100 == 0 || i == total_files - 1 {
            if let Some(ref callback) = progress_callback {
                callback(ProgressUpdateType::hbk_extraction(
                    file_name.clone(),
                    i + 1,
                    total_files,
                ));
            }
        }

        let mut file = archive
            .by_index(i)
            .context(format!("Не удалось прочитать файл #{} из архива", i))?;

        let file_path = match file.enclosed_name() {
            Some(path) => target_dir.join(path),
            None => {
                // Пропускаем файлы с небезопасными именами без логирования
                continue;
            }
        };

        // Создаём родительские директории
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)
                .context(format!("Не удалось создать директорию: {:?}", parent))?;
        }

        if file.is_dir() {
            fs::create_dir_all(&file_path)
                .context(format!("Не удалось создать директорию: {:?}", file_path))?;
        } else {
            let mut outfile = File::create(&file_path)
                .context(format!("Не удалось создать файл: {:?}", file_path))?;

            std::io::copy(&mut file, &mut outfile)
                .context(format!("Ошибка записи файла: {:?}", file_path))?;
        }
    }

    info!("✅ ZIP распакован успешно: {} файлов", total_files);
    Ok(())
}

/// Распаковывает ZIP архив в указанную директорию (без callback)
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
#[allow(dead_code)]
pub fn unpack_zip(zip_path: &Path, target_dir: &Path) -> Result<()> {
    unpack_zip_with_progress(zip_path, target_dir, None::<fn(ProgressUpdateType)>)
}
