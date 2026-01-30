//! Модуль для поиска ZIP signature в файле

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
pub fn find_zip_signature(file: &mut File, max_search_size: usize) -> Result<usize> {
    debug!("🔎 Ищем ZIP signature (максимум: {} байт)", max_search_size);

    const OVERLAP_SIZE: usize = ZIP_SIGNATURE.len() - 1; // 3 байта для signature размером 4

    let mut buffer = vec![0u8; CHUNK_SIZE];
    let mut total_read = 0;
    let mut overlap_buffer = Vec::new();
    let mut chunk_start_offset = 0; // Отслеживаем начало текущего чанка

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
        if let Some(pos) = search_data
            .windows(ZIP_SIGNATURE.len())
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

    Err(anyhow!(
        "ZIP signature не найдена в первых {} байтах",
        total_read
    ))
}
