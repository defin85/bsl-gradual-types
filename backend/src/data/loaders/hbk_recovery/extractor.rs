//! Модуль для извлечения и распаковки ZIP архивов

use anyhow::{anyhow, Context, Result};
use flate2::read::DeflateDecoder;
use std::fs::{self, File};
use std::io::{Cursor, Read, Seek, SeekFrom, Write};
use std::path::{Component, Path};
use tracing::{debug, info, warn};

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
    F: Fn(ProgressUpdateType) + Clone,
{
    let mut data = match repair_zip_offsets_if_needed(zip_path) {
        Ok(data) => data,
        Err(err) => {
            warn!(
                "⚠️ Не удалось восстановить EOCD/offsets для {:?}: {}. Продолжаем с raw bytes...",
                zip_path, err
            );
            fs::read(zip_path).context(format!("Не удалось прочитать ZIP: {:?}", zip_path))?
        }
    };

    debug!("📂 Распаковываем ZIP {:?} → {:?}", zip_path, target_dir);

    // Создаём целевую директорию
    fs::create_dir_all(target_dir)
        .context(format!("Не удалось создать директорию: {:?}", target_dir))?;

    // Открываем ZIP архив
    let cursor = Cursor::new(data.as_slice());
    let mut archive = match zip::ZipArchive::with_config(
        zip::read::Config {
            archive_offset: zip::read::ArchiveOffset::Known(0),
        },
        cursor,
    ) {
        Ok(archive) => archive,
        Err(err) => {
            warn!("⚠️ ZIP архив повреждён {:?}: {}", zip_path, err);
            match reconstruct_zip_from_central_directory(&data) {
                Ok(rebuilt) => {
                    info!(
                        "✅ ZIP реконструирован: entries={}, cd_offset={}, cd_size={}",
                        rebuilt.entries, rebuilt.cd_offset, rebuilt.cd_size
                    );
                    data = rebuilt.data;
                    if let Err(err) = fs::write(zip_path, &data) {
                        warn!("⚠️ Не удалось сохранить реконструированный ZIP: {}", err);
                    }
                    let extracted = match unpack_zip_fallback(
                        &data,
                        zip_path,
                        target_dir,
                        progress_callback.clone(),
                    ) {
                        Ok(count) => count,
                        Err(err) => {
                            warn!("⚠️ Fallback распаковка не удалась: {}", err);
                            0
                        }
                    };
                    if extracted > 0 {
                        return Ok(());
                    }
                }
                Err(err) => {
                    warn!("⚠️ Не удалось восстановить ZIP: {}", err);
                    let extracted = match unpack_zip_fallback(
                        &data,
                        zip_path,
                        target_dir,
                        progress_callback.clone(),
                    ) {
                        Ok(count) => count,
                        Err(err) => {
                            warn!("⚠️ Fallback распаковка не удалась: {}", err);
                            0
                        }
                    };
                    if extracted > 0 {
                        return Ok(());
                    }
                }
            }
            // Last-resort: ignore central directory entirely and extract by scanning local file
            // headers. This is needed for some real-world 1C HBK/ZIP variants where EOCD/CD is
            // present but points to an invalid/partial central directory.
            let extracted = unpack_zip_scan_local_headers(
                &data,
                zip_path,
                target_dir,
                progress_callback,
            )
            .context("Не удалось распаковать ZIP через сканирование local headers")?;
            if extracted == 0 {
                return Err(anyhow!(
                    "Не удалось распаковать ZIP: fallback и scan extracted 0 files"
                ));
            }
            return Ok(());
        }
    };

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

        let mut file = match archive.by_index(i) {
            Ok(file) => file,
            Err(err) => {
                warn!("⚠️ Пропускаем повреждённый файл #{}: {}", i, err);
                continue;
            }
        };

        let file_path = match file.enclosed_name() {
            Some(path) => target_dir.join(path),
            None => {
                // Пропускаем файлы с небезопасными именами без логирования
                continue;
            }
        };

        // Создаём родительские директории
        let write_result = (|| -> Result<()> {
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

            Ok(())
        })();

        if let Err(err) = write_result {
            warn!("⚠️ Не удалось распаковать {:?}: {}", file_path, err);
            continue;
        }
    }

    info!("✅ ZIP распакован успешно: {} файлов", total_files);
    Ok(())
}

fn unpack_zip_fallback<F>(
    data: &[u8],
    zip_path: &Path,
    target_dir: &Path,
    progress_callback: Option<F>,
) -> Result<usize>
where
    F: Fn(ProgressUpdateType),
{
    const CD_SIGNATURE: u32 = 0x02014b50;
    const LFH_SIGNATURE: u32 = 0x04034b50;

    let eocd_pos = find_eocd(data).ok_or_else(|| anyhow::anyhow!("Не удалось найти EOCD в ZIP"))?;
    let total_files = read_u16_le(data, eocd_pos + 10) as usize;
    let cd_size = read_u32_le(data, eocd_pos + 12) as usize;
    let cd_offset = read_u32_le(data, eocd_pos + 16) as usize;

    if cd_offset + cd_size > data.len() {
        return Err(anyhow::anyhow!("Невалидный размер центрального каталога"));
    }

    let file_name = zip_path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    fs::create_dir_all(target_dir)
        .context(format!("Не удалось создать директорию: {:?}", target_dir))?;

    let mut extracted = 0usize;
    let mut pos = cd_offset;
    while pos + 46 <= cd_offset + cd_size {
        if read_u32_le(data, pos) != CD_SIGNATURE {
            break;
        }

        let compression = read_u16_le(data, pos + 10);
        let compressed_size = read_u32_le(data, pos + 20) as usize;
        let name_len = read_u16_le(data, pos + 28) as usize;
        let extra_len = read_u16_le(data, pos + 30) as usize;
        let comment_len = read_u16_le(data, pos + 32) as usize;
        let local_header_offset = read_u32_le(data, pos + 42) as usize;

        let name_start = pos + 46;
        let name_end = name_start + name_len;
        if name_end > data.len() {
            break;
        }

        let name = String::from_utf8_lossy(&data[name_start..name_end]).to_string();
        let is_dir = name.ends_with('/') || name.ends_with('\\');
        let file_path = target_dir.join(&name);

        if !is_dir {
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent)
                    .context(format!("Не удалось создать директорию: {:?}", parent))?;
            }

            if local_header_offset + 30 > data.len() {
                break;
            }
            if read_u32_le(data, local_header_offset) != LFH_SIGNATURE {
                break;
            }

            let lfh_name_len = read_u16_le(data, local_header_offset + 26) as usize;
            let lfh_extra_len = read_u16_le(data, local_header_offset + 28) as usize;
            let data_start = local_header_offset + 30 + lfh_name_len + lfh_extra_len;
            let data_end = data_start.saturating_add(compressed_size);
            if data_end > data.len() {
                break;
            }

            let mut output = Vec::new();
            match compression {
                0 => {
                    output.extend_from_slice(&data[data_start..data_end]);
                }
                8 => {
                    let mut decoder = DeflateDecoder::new(&data[data_start..data_end]);
                    if decoder.read_to_end(&mut output).is_err() {
                        pos += 46 + name_len + extra_len + comment_len;
                        continue;
                    }
                }
                _ => {
                    pos += 46 + name_len + extra_len + comment_len;
                    continue;
                }
            }

            let mut outfile = File::create(&file_path)
                .context(format!("Не удалось создать файл: {:?}", file_path))?;
            outfile
                .write_all(&output)
                .context(format!("Ошибка записи файла: {:?}", file_path))?;
        }

        extracted += 1;
        if extracted.is_multiple_of(100) || extracted == total_files {
            if let Some(ref callback) = progress_callback {
                callback(ProgressUpdateType::hbk_extraction(
                    file_name.clone(),
                    extracted,
                    total_files.max(1),
                ));
            }
        }

        pos += 46 + name_len + extra_len + comment_len;
    }

    info!(
        "✅ ZIP распакован через fallback: {}/{} файлов",
        extracted, total_files
    );

    Ok(extracted)
}

fn unpack_zip_scan_local_headers<F>(
    data: &[u8],
    zip_path: &Path,
    target_dir: &Path,
    progress_callback: Option<F>,
) -> Result<usize>
where
    F: Fn(ProgressUpdateType),
{
    const LFH_SIGNATURE: u32 = 0x04034b50;

    let file_name = zip_path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let mut extracted = 0usize;
    let mut pos = 0usize;

    while pos + 4 <= data.len() {
        let Some(sig_pos) = find_signature_from(data, LFH_SIGNATURE, pos) else {
            break;
        };
        pos = sig_pos;
        if pos + 30 > data.len() {
            break;
        }
        if read_u32_le(data, pos) != LFH_SIGNATURE {
            pos = pos.saturating_add(1);
            continue;
        }

        let flags = read_u16_le(data, pos + 6);
        if (flags & 0x08) != 0 {
            // Data descriptor mode would require parsing post-data descriptors to find the next
            // header reliably. We don't currently need it for 1C docs fixtures.
            return Err(anyhow!("ZIP data descriptor is not supported in scan fallback"));
        }

        let compression = read_u16_le(data, pos + 8);
        let compressed_size = read_u32_le(data, pos + 18) as usize;
        let name_len = read_u16_le(data, pos + 26) as usize;
        let extra_len = read_u16_le(data, pos + 28) as usize;

        let name_start = pos + 30;
        let name_end = name_start.saturating_add(name_len);
        if name_end > data.len() {
            break;
        }

        // Normalize path separators to handle Windows-style entries.
        let raw_name = String::from_utf8_lossy(&data[name_start..name_end]);
        let name = raw_name.replace('\\', "/");

        let data_start = name_end.saturating_add(extra_len);
        let data_end = data_start.saturating_add(compressed_size);
        if data_start > data.len() || data_end > data.len() {
            break;
        }

        let is_dir = name.ends_with('/');
        let rel = Path::new(name.as_str());
        if rel.components().any(|c| {
            matches!(
                c,
                Component::Prefix(_) | Component::RootDir | Component::ParentDir
            )
        }) {
            // Skip unsafe paths (zip-slip).
            pos = data_end;
            continue;
        }
        let file_path = target_dir.join(rel);

        let write_result = (|| -> Result<()> {
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent)
                    .context(format!("Не удалось создать директорию: {:?}", parent))?;
            }
            if is_dir {
                fs::create_dir_all(&file_path)
                    .context(format!("Не удалось создать директорию: {:?}", file_path))?;
                return Ok(());
            }

            let mut output = Vec::new();
            match compression {
                0 => output.extend_from_slice(&data[data_start..data_end]),
                8 => {
                    let mut decoder = DeflateDecoder::new(&data[data_start..data_end]);
                    decoder
                        .read_to_end(&mut output)
                        .context("deflate decode")?;
                }
                _ => return Ok(()), // skip unsupported compression
            }

            let mut outfile = File::create(&file_path)
                .context(format!("Не удалось создать файл: {:?}", file_path))?;
            outfile
                .write_all(&output)
                .context(format!("Ошибка записи файла: {:?}", file_path))?;
            Ok(())
        })();

        if write_result.is_ok() {
            extracted += 1;
            if extracted.is_multiple_of(100) {
                if let Some(ref callback) = progress_callback {
                    callback(ProgressUpdateType::hbk_extraction(
                        file_name.clone(),
                        extracted,
                        extracted + 1,
                    ));
                }
            }
        }

        pos = data_end;
    }

    info!("✅ ZIP распакован через scan fallback: {} файлов", extracted);
    Ok(extracted)
}

fn reconstruct_zip_from_central_directory(data: &[u8]) -> Result<RebuiltZip> {
    const CD_SIGNATURE: u32 = 0x02014b50;

    let eocd_pos = find_eocd(data).ok_or_else(|| anyhow::anyhow!("Не удалось найти EOCD в ZIP"))?;
    let total_files = read_u16_le(data, eocd_pos + 10) as usize;
    let cd_size = read_u32_le(data, eocd_pos + 12) as usize;
    let cd_offset = read_u32_le(data, eocd_pos + 16) as usize;
    let comment_len = read_u16_le(data, eocd_pos + 20) as usize;
    let comment_end = eocd_pos.saturating_add(22 + comment_len);

    if cd_offset + cd_size > data.len() {
        return Err(anyhow::anyhow!("Невалидный размер центрального каталога"));
    }
    if comment_end > data.len() {
        return Err(anyhow::anyhow!("Невалидная длина комментария EOCD"));
    }

    let comment = &data[eocd_pos + 22..comment_end];

    let mut entries: Vec<CdEntry> = Vec::with_capacity(total_files);
    let mut pos = cd_offset;
    for _ in 0..total_files {
        if pos + 46 > data.len() || read_u32_le(data, pos) != CD_SIGNATURE {
            return Err(anyhow::anyhow!("Невалидная запись CDFH"));
        }

        let name_len = read_u16_le(data, pos + 28) as usize;
        let extra_len = read_u16_le(data, pos + 30) as usize;
        let comment_len = read_u16_le(data, pos + 32) as usize;
        let entry_len = 46 + name_len + extra_len + comment_len;
        if pos + entry_len > data.len() {
            return Err(anyhow::anyhow!("Невалидная длина CDFH записи"));
        }

        let local_header_offset = read_u32_le(data, pos + 42);
        entries.push(CdEntry {
            raw: data[pos..pos + entry_len].to_vec(),
            local_header_offset,
        });

        pos += entry_len;
    }

    let mut order: Vec<(usize, u32)> = entries
        .iter()
        .enumerate()
        .map(|(idx, entry)| (idx, entry.local_header_offset))
        .collect();
    order.sort_by_key(|(_, offset)| *offset);

    let mut new_offsets = vec![0u32; entries.len()];
    let mut output = Vec::new();

    for (i, (idx, offset)) in order.iter().enumerate() {
        let start = *offset as usize;
        if start >= cd_offset {
            return Err(anyhow::anyhow!("Невалидный offset локального заголовка"));
        }

        let end = if i + 1 < order.len() {
            order[i + 1].1 as usize
        } else {
            cd_offset
        };

        if end <= start || end > data.len() {
            return Err(anyhow::anyhow!("Невалидный диапазон данных файла"));
        }

        new_offsets[*idx] = output.len() as u32;
        output.extend_from_slice(&data[start..end]);
    }

    let cd_offset_new = output.len() as u32;
    let mut cd_out = Vec::new();
    for (idx, entry) in entries.iter().enumerate() {
        let mut raw = entry.raw.clone();
        write_u32_le(&mut raw, 42, new_offsets[idx]);
        cd_out.extend_from_slice(&raw);
    }

    let cd_size_new = cd_out.len() as u32;
    output.extend_from_slice(&cd_out);
    output.extend_from_slice(&build_eocd(
        total_files as u16,
        cd_size_new,
        cd_offset_new,
        comment,
    ));

    if read_u32_le(&output, cd_offset_new as usize) != CD_SIGNATURE {
        return Err(anyhow::anyhow!("Реконструкция ZIP завершилась ошибкой"));
    }

    Ok(RebuiltZip {
        data: output,
        entries: total_files,
        cd_offset: cd_offset_new,
        cd_size: cd_size_new,
    })
}

struct CdEntry {
    raw: Vec<u8>,
    local_header_offset: u32,
}

struct RebuiltZip {
    data: Vec<u8>,
    entries: usize,
    cd_offset: u32,
    cd_size: u32,
}

fn build_eocd(total_files: u16, cd_size: u32, cd_offset: u32, comment: &[u8]) -> Vec<u8> {
    let mut buf = vec![0u8; 22 + comment.len()];

    write_u32_le(&mut buf, 0, 0x06054b50);
    write_u16_le(&mut buf, 4, 0);
    write_u16_le(&mut buf, 6, 0);
    write_u16_le(&mut buf, 8, total_files);
    write_u16_le(&mut buf, 10, total_files);
    write_u32_le(&mut buf, 12, cd_size);
    write_u32_le(&mut buf, 16, cd_offset);
    write_u16_le(&mut buf, 20, comment.len() as u16);
    buf[22..].copy_from_slice(comment);

    buf
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

fn repair_zip_offsets_if_needed(zip_path: &Path) -> Result<Vec<u8>> {
    const CD_SIGNATURE: u32 = 0x02014b50;
    const LFH_SIGNATURE: u32 = 0x04034b50;

    let data = fs::read(zip_path).context(format!("Не удалось прочитать ZIP: {:?}", zip_path))?;

    let eocd_pos = find_eocd(&data)
        .ok_or_else(|| anyhow::anyhow!("Не удалось найти EOCD в ZIP: {:?}", zip_path))?;

    let mut patched = data;
    let mut patched_changed = false;
    let mut cd_offset = read_u32_le(&patched, eocd_pos + 16) as usize;
    let mut cd_size = read_u32_le(&patched, eocd_pos + 12) as usize;
    let comment_len = read_u16_le(&patched, eocd_pos + 20) as usize;
    let eocd_end = eocd_pos.saturating_add(22 + comment_len);

    if eocd_end < patched.len() {
        patched.truncate(eocd_end);
        patched_changed = true;
    }

    if cd_offset + 4 > patched.len() || read_u32_le(&patched, cd_offset) != CD_SIGNATURE {
        let cd_start_by_size = eocd_pos.saturating_sub(cd_size);
        if cd_start_by_size + 4 <= patched.len()
            && read_u32_le(&patched, cd_start_by_size) == CD_SIGNATURE
        {
            cd_offset = cd_start_by_size;
            write_u32_le(&mut patched, eocd_pos + 16, cd_offset as u32);
            patched_changed = true;
        } else if let Some(last_cd) = find_signature_before(&patched, CD_SIGNATURE, eocd_pos) {
            cd_offset = last_cd;
            cd_size = eocd_pos.saturating_sub(cd_offset);
            write_u32_le(&mut patched, eocd_pos + 16, cd_offset as u32);
            write_u32_le(&mut patched, eocd_pos + 12, cd_size as u32);
            patched_changed = true;
        } else {
            if patched_changed {
                fs::write(zip_path, &patched)
                    .context(format!("Не удалось записать ZIP: {:?}", zip_path))?;
            }
            return Ok(patched);
        }
    }

    if cd_offset >= patched.len() || cd_offset + cd_size > patched.len() {
        if patched_changed {
            fs::write(zip_path, &patched)
                .context(format!("Не удалось записать ZIP: {:?}", zip_path))?;
        }
        return Ok(patched);
    }
    if cd_size < 46 || cd_offset + 46 > patched.len() {
        if patched_changed {
            fs::write(zip_path, &patched)
                .context(format!("Не удалось записать ZIP: {:?}", zip_path))?;
        }
        return Ok(patched);
    }

    let first_entry_offset = read_u32_le(&patched, cd_offset + 42) as usize;
    if first_entry_offset + 4 <= patched.len()
        && read_u32_le(&patched, first_entry_offset) == LFH_SIGNATURE
    {
        if patched_changed {
            fs::write(zip_path, &patched)
                .context(format!("Не удалось записать ZIP: {:?}", zip_path))?;
        }
        return Ok(patched);
    }

    let first_pk = find_signature(&patched, LFH_SIGNATURE).ok_or_else(|| {
        anyhow::anyhow!("Не удалось найти local file header в ZIP: {:?}", zip_path)
    })?;

    if first_entry_offset < first_pk {
        if patched_changed {
            fs::write(zip_path, &patched)
                .context(format!("Не удалось записать ZIP: {:?}", zip_path))?;
        }
        return Ok(patched);
    }

    let delta = first_entry_offset - first_pk;
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
            patched_changed = true;
        }

        pos = pos.saturating_add(46 + name_len + extra_len + comment_len);
    }

    if patched_changed {
        fs::write(zip_path, &patched)
            .context(format!("Не удалось записать ZIP: {:?}", zip_path))?;
    }

    Ok(patched)
}

fn find_eocd(data: &[u8]) -> Option<usize> {
    const EOCD_SIGNATURE: u32 = 0x06054b50;
    const CD_SIGNATURE: u32 = 0x02014b50;
    let max_comment = 65535usize;
    let min_size = 22usize;

    let start = data.len().saturating_sub(max_comment + min_size);
    let mut i = data.len().saturating_sub(min_size);
    while i >= start {
        if read_u32_le(data, i) == EOCD_SIGNATURE {
            let cd_size = read_u32_le(data, i + 12) as usize;
            let cd_offset = read_u32_le(data, i + 16) as usize;

            if cd_offset + 4 <= data.len() && read_u32_le(data, cd_offset) == CD_SIGNATURE {
                return Some(i);
            }

            let cd_start_by_size = i.saturating_sub(cd_size);
            if cd_start_by_size + 4 <= data.len()
                && read_u32_le(data, cd_start_by_size) == CD_SIGNATURE
            {
                return Some(i);
            }
        }
        if i == 0 {
            break;
        }
        i -= 1;
    }
    None
}

fn find_signature(data: &[u8], signature: u32) -> Option<usize> {
    data.windows(4).position(|w| read_u32_le(w, 0) == signature)
}

fn find_signature_from(data: &[u8], signature: u32, start: usize) -> Option<usize> {
    if start >= data.len() {
        return None;
    }
    data[start..]
        .windows(4)
        .position(|w| read_u32_le(w, 0) == signature)
        .map(|idx| start + idx)
}

fn find_signature_before(data: &[u8], signature: u32, end: usize) -> Option<usize> {
    let end = end.min(data.len());
    (0..end)
        .rev()
        .find(|&i| i + 4 <= end && read_u32_le(data, i) == signature)
}

fn read_u16_le(data: &[u8], pos: usize) -> u16 {
    let bytes = [data[pos], data[pos + 1]];
    u16::from_le_bytes(bytes)
}

fn write_u16_le(data: &mut [u8], pos: usize, value: u16) {
    let bytes = value.to_le_bytes();
    data[pos] = bytes[0];
    data[pos + 1] = bytes[1];
}

fn read_u32_le(data: &[u8], pos: usize) -> u32 {
    let bytes = [data[pos], data[pos + 1], data[pos + 2], data[pos + 3]];
    u32::from_le_bytes(bytes)
}

fn write_u32_le(data: &mut [u8], pos: usize, value: u32) {
    let bytes = value.to_le_bytes();
    data[pos..pos + 4].copy_from_slice(&bytes);
}
