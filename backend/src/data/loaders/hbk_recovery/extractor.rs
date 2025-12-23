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
    let mut output_file =
        File::create(output_path).context(format!("Не удалось создать файл: {:?}", output_path))?;

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
    repair_zip_offsets_if_needed(zip_path)?;

    debug!("📂 Распаковываем ZIP {:?} → {:?}", zip_path, target_dir);

    // Создаём целевую директорию
    fs::create_dir_all(target_dir)
        .context(format!("Не удалось создать директорию: {:?}", target_dir))?;

    // Открываем ZIP архив
    let file = File::open(zip_path).context(format!("Не удалось открыть ZIP: {:?}", zip_path))?;

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

fn repair_zip_offsets_if_needed(zip_path: &Path) -> Result<()> {
    const CD_SIGNATURE: u32 = 0x02014b50;
    const LFH_SIGNATURE: u32 = 0x04034b50;

    let data = fs::read(zip_path)
        .context(format!("Не удалось прочитать ZIP: {:?}", zip_path))?;

    let eocd_pos = find_eocd(&data).ok_or_else(|| {
        anyhow::anyhow!("Не удалось найти EOCD в ZIP: {:?}", zip_path)
    })?;

    let cd_offset = read_u32_le(&data, eocd_pos + 16) as usize;
    let cd_size = read_u32_le(&data, eocd_pos + 12) as usize;

    if cd_offset >= data.len() || cd_offset + cd_size > data.len() {
        return Ok(());
    }
    if cd_size < 46 || cd_offset + 46 > data.len() {
        return Ok(());
    }

    let first_entry_offset = read_u32_le(&data, cd_offset + 42) as usize;
    if first_entry_offset + 4 <= data.len()
        && read_u32_le(&data, first_entry_offset) == LFH_SIGNATURE
    {
        return Ok(());
    }

    let first_pk = find_signature(&data, LFH_SIGNATURE).ok_or_else(|| {
        anyhow::anyhow!("Не удалось найти local file header в ZIP: {:?}", zip_path)
    })?;

    if first_entry_offset < first_pk {
        return Ok(());
    }

    let delta = first_entry_offset - first_pk;
    let mut patched = data;

    let mut pos = cd_offset;
    let cd_end = cd_offset + cd_size;
    while pos + 46 <= cd_end {
        if read_u32_le(&patched, pos) != CD_SIGNATURE {
            break;
        }

        let name_len = read_u16_le(&patched, pos + 28) as usize;
        let extra_len = read_u16_le(&patched, pos + 30) as usize;
        let comment_len = read_u16_le(&patched, pos + 32) as usize;

        let entry_offset = read_u32_le(&patched, pos + 42) as usize;
        if entry_offset >= delta {
            write_u32_le(&mut patched, pos + 42, (entry_offset - delta) as u32);
        }

        pos = pos
            .saturating_add(46 + name_len + extra_len + comment_len);
    }

    fs::write(zip_path, patched)
        .context(format!("Не удалось записать ZIP: {:?}", zip_path))?;

    Ok(())
}

fn find_eocd(data: &[u8]) -> Option<usize> {
    const EOCD_SIGNATURE: u32 = 0x06054b50;
    let max_comment = 65535usize;
    let min_size = 22usize;

    let start = data.len().saturating_sub(max_comment + min_size);
    let mut i = data.len().saturating_sub(min_size);
    while i >= start {
        if read_u32_le(data, i) == EOCD_SIGNATURE {
            return Some(i);
        }
        if i == 0 {
            break;
        }
        i -= 1;
    }
    None
}

fn find_signature(data: &[u8], signature: u32) -> Option<usize> {
    data.windows(4)
        .position(|w| read_u32_le(w, 0) == signature)
}

fn read_u16_le(data: &[u8], pos: usize) -> u16 {
    let bytes = [data[pos], data[pos + 1]];
    u16::from_le_bytes(bytes)
}

fn read_u32_le(data: &[u8], pos: usize) -> u32 {
    let bytes = [data[pos], data[pos + 1], data[pos + 2], data[pos + 3]];
    u32::from_le_bytes(bytes)
}

fn write_u32_le(data: &mut [u8], pos: usize, value: u32) {
    let bytes = value.to_le_bytes();
    data[pos..pos + 4].copy_from_slice(&bytes);
}
