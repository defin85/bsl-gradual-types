//! Интеграционные тесты для HBK Recovery компоненты
//!
//! Проверяет комплексное восстановление .hbk файлов с различными опциями

use bsl_backend::data::loaders::hbk_recovery::{
    auto_recover_directory, recover_hbk_file, HbkRecovery, RecoveryOptions, RecoveryResult,
};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

// ============================================================================
// Helper функции для создания тестовых файлов
// ============================================================================

/// Создаёт минимальный валидный пустой ZIP архив
/// Структура: Local File Header (пустой файл) + EOCD
fn create_minimal_empty_zip() -> Vec<u8> {
    let mut zip = Vec::new();

    // Local File Header for empty file
    zip.extend_from_slice(&[
        0x50, 0x4B, 0x03, 0x04, // Local file header signature
        0x14, 0x00, // Version needed to extract (2.0)
        0x00, 0x00, // General purpose bit flag
        0x00, 0x00, // Compression method (stored)
        0x00, 0x00, // File modification time
        0x00, 0x00, // File modification date
        0x00, 0x00, 0x00, 0x00, // CRC-32
        0x00, 0x00, 0x00, 0x00, // Compressed size
        0x00, 0x00, 0x00, 0x00, // Uncompressed size
        0x00, 0x00, // Filename length (0 = no filename)
        0x00, 0x00, // Extra field length
    ]);

    // EOCD (End of Central Directory)
    zip.extend_from_slice(&[
        0x50, 0x4B, 0x05, 0x06, // EOCD signature
        0x00, 0x00, // Number of this disk
        0x00, 0x00, // Number of the disk with the start of the central directory
        0x00, 0x00, // Total number of entries in the central directory on this disk
        0x00, 0x00, // Total number of entries in the central directory
        0x00, 0x00, 0x00, 0x00, // Size of the central directory
        0x00, 0x00, 0x00, 0x00, // Offset of the central directory
        0x00, 0x00, // ZIP comment length
    ]);

    zip
}

/// Создаёт тестовый .hbk файл с "мусором" и пустым ZIP архивом
fn create_test_hbk_file(junk_size: usize) -> Vec<u8> {
    let mut data = vec![0xFF; junk_size];

    // Добавляем минимальный валидный ZIP
    data.extend_from_slice(&create_minimal_empty_zip());

    data
}

// ============================================================================
// Unit-тесты для signature модуля (через HbkRecovery API)
// ============================================================================

#[test]
fn test_find_signature_at_beginning() {
    let temp_dir = tempdir().unwrap();
    let test_file_path = temp_dir.path().join("test.hbk");

    // Создаём файл с signature в самом начале (только ZIP signature, не нужна полная распаковка)
    let mut file = File::create(&test_file_path).unwrap();
    file.write_all(&create_minimal_empty_zip()).unwrap();
    drop(file);

    // Пытаемся восстановить (с auto_extract = false чтобы не распаковывать)
    let mut recovery = HbkRecovery::with_options(RecoveryOptions {
        cleanup_temp: true,
        auto_extract: false,
        max_file_size: 10 * 1024 * 1024,
    });
    let result = recovery
        .recover(&test_file_path, Some(temp_dir.path()))
        .unwrap();

    assert_eq!(result.signature_offset, 0, "Signature должна быть найдена в позиции 0");
}

#[test]
fn test_find_signature_in_middle() {
    let temp_dir = tempdir().unwrap();
    let test_file_path = temp_dir.path().join("test.hbk");

    // Создаём файл с signature в середине
    let hbk_data = create_test_hbk_file(5000);
    let mut file = File::create(&test_file_path).unwrap();
    file.write_all(&hbk_data).unwrap();
    drop(file);

    // Пытаемся восстановить
    let mut recovery = HbkRecovery::with_options(RecoveryOptions {
        cleanup_temp: true,
        auto_extract: false,
        max_file_size: 10 * 1024 * 1024,
    });
    let result = recovery
        .recover(&test_file_path, Some(temp_dir.path()))
        .unwrap();

    assert_eq!(
        result.signature_offset, 5000,
        "Signature должна быть найдена на offset 5000"
    );
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
    file.write_all(&create_minimal_empty_zip()).unwrap();
    drop(file);

    // Пытаемся восстановить
    let mut recovery = HbkRecovery::with_options(RecoveryOptions {
        cleanup_temp: true,
        auto_extract: false,
        max_file_size: 10 * 1024 * 1024,
    });
    let result = recovery
        .recover(&test_file_path, Some(temp_dir.path()))
        .unwrap();

    assert_eq!(
        result.signature_offset, signature_pos,
        "Signature должна быть найдена на границе chunk"
    );
}

#[test]
fn test_signature_not_found() {
    let temp_dir = tempdir().unwrap();
    let test_file_path = temp_dir.path().join("test.hbk");

    // Создаём файл БЕЗ ZIP signature
    let mut file = File::create(&test_file_path).unwrap();
    file.write_all(&vec![0xFF; 10000]).unwrap();
    drop(file);

    // Пытаемся восстановить
    let mut recovery = HbkRecovery::new();
    let result = recovery.recover(&test_file_path, Some(temp_dir.path()));

    assert!(result.is_err(), "Should fail without ZIP signature");
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("ZIP signature"),
        "Error message должно упоминать ZIP signature"
    );
}

#[test]
fn test_empty_file() {
    let temp_dir = tempdir().unwrap();
    let test_file_path = temp_dir.path().join("empty.hbk");

    // Создаём пустой файл
    File::create(&test_file_path).unwrap();

    // Пытаемся восстановить
    let mut recovery = HbkRecovery::new();
    let result = recovery.recover(&test_file_path, Some(temp_dir.path()));

    assert!(result.is_err(), "Should fail on empty file");
}

#[test]
fn test_file_too_small() {
    let temp_dir = tempdir().unwrap();
    let test_file_path = temp_dir.path().join("small.hbk");

    // Создаём файл меньше 4 байт
    let mut file = File::create(&test_file_path).unwrap();
    file.write_all(&[0xFF, 0xFF]).unwrap();
    drop(file);

    // Пытаемся восстановить
    let mut recovery = HbkRecovery::new();
    let result = recovery.recover(&test_file_path, Some(temp_dir.path()));

    assert!(result.is_err(), "Should fail on file smaller than signature");
}

// ============================================================================
// Integration тесты с различными опциями
// ============================================================================

#[test]
fn test_recovery_with_cleanup_enabled() {
    let temp_dir = tempdir().unwrap();
    let hbk_path = temp_dir.path().join("test.hbk");

    // Создаём тестовый .hbk файл
    let hbk_data = create_test_hbk_file(1000);
    let mut file = File::create(&hbk_path).unwrap();
    file.write_all(&hbk_data).unwrap();
    drop(file);

    // Выполняем восстановление с cleanup = true
    let mut recovery = HbkRecovery::with_options(RecoveryOptions {
        cleanup_temp: true,
        auto_extract: true,
        max_file_size: 10 * 1024 * 1024,
    });

    let result = recovery
        .recover(&hbk_path, Some(temp_dir.path()))
        .unwrap();

    // Проверяем что результаты существуют
    assert!(result.extracted_dir.is_some());
    let extracted = result.extracted_dir.unwrap();
    assert!(
        extracted.exists(),
        "Extracted directory должна существовать"
    );

    // Проверяем что временный ZIP был удалён (cleanup = true)
    assert!(
        !result.repaired_zip_path.exists(),
        "Временный ZIP должен быть удалён при cleanup = true"
    );
}

#[test]
fn test_recovery_without_cleanup() {
    let temp_dir = tempdir().unwrap();
    let hbk_path = temp_dir.path().join("test.hbk");

    // Создаём тестовый .hbk файл
    let hbk_data = create_test_hbk_file(1000);
    let mut file = File::create(&hbk_path).unwrap();
    file.write_all(&hbk_data).unwrap();
    drop(file);

    // Выполняем восстановление с cleanup = false
    let mut recovery = HbkRecovery::with_options(RecoveryOptions {
        cleanup_temp: false,
        auto_extract: true,
        max_file_size: 10 * 1024 * 1024,
    });

    let result = recovery
        .recover(&hbk_path, Some(temp_dir.path()))
        .unwrap();

    // Проверяем что временный ZIP НЕ был удалён
    assert!(
        result.repaired_zip_path.exists(),
        "Временный ZIP должен существовать при cleanup = false"
    );

    // Проверяем что extracted директория также существует
    assert!(result.extracted_dir.is_some());
}

#[test]
fn test_recovery_without_extraction() {
    let temp_dir = tempdir().unwrap();
    let hbk_path = temp_dir.path().join("test.hbk");

    // Создаём тестовый .hbk файл
    let hbk_data = create_test_hbk_file(1000);
    let mut file = File::create(&hbk_path).unwrap();
    file.write_all(&hbk_data).unwrap();
    drop(file);

    // Выполняем восстановление с auto_extract = false
    let mut recovery = HbkRecovery::with_options(RecoveryOptions {
        cleanup_temp: false,
        auto_extract: false,
        max_file_size: 10 * 1024 * 1024,
    });

    let result = recovery
        .recover(&hbk_path, Some(temp_dir.path()))
        .unwrap();

    // Проверяем что ZIP существует
    assert!(result.repaired_zip_path.exists());

    // Проверяем что extracted_dir == None (т.к. auto_extract = false)
    assert!(
        result.extracted_dir.is_none(),
        "extracted_dir должен быть None при auto_extract = false"
    );
}

#[test]
fn test_recover_multiple_files_in_directory() {
    let temp_dir = tempdir().unwrap();

    // Создаём несколько тестовых .hbk файлов
    for i in 0..3 {
        let hbk_path = temp_dir.path().join(format!("test{}.hbk", i));
        let hbk_data = create_test_hbk_file(1000 + i * 500);
        let mut file = File::create(&hbk_path).unwrap();
        file.write_all(&hbk_data).unwrap();
    }

    // Используем auto_recover_directory
    let results = auto_recover_directory(temp_dir.path()).unwrap();

    // Проверяем что восстановлены все 3 файла
    assert_eq!(
        results.len(),
        3,
        "Должны быть восстановлены все 3 файла"
    );

    // Проверяем каждый результат
    for result in &results {
        assert!(result.extracted_dir.is_some());
        assert!(result.extracted_dir.as_ref().unwrap().exists());
    }
}

#[test]
fn test_max_file_size_limit() {
    let temp_dir = tempdir().unwrap();
    let hbk_path = temp_dir.path().join("large.hbk");

    // Создаём файл
    let hbk_data = create_test_hbk_file(1000);
    let mut file = File::create(&hbk_path).unwrap();
    file.write_all(&hbk_data).unwrap();
    drop(file);

    // Пытаемся восстановить с очень маленьким max_file_size
    let mut recovery = HbkRecovery::with_options(RecoveryOptions {
        cleanup_temp: true,
        auto_extract: true,
        max_file_size: 100, // Очень маленький лимит
    });

    let result = recovery.recover(&hbk_path, Some(temp_dir.path()));

    assert!(result.is_err(), "Should fail when file exceeds max_file_size");
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("слишком большой"),
        "Error message должно упоминать размер файла"
    );
}

#[test]
fn test_auto_recover_directory_with_non_hbk_files() {
    let temp_dir = tempdir().unwrap();

    // Создаём смешанные файлы
    // .hbk файл
    let hbk_path = temp_dir.path().join("valid.hbk");
    let hbk_data = create_test_hbk_file(1000);
    let mut file = File::create(&hbk_path).unwrap();
    file.write_all(&hbk_data).unwrap();

    // .txt файл (должен быть пропущен)
    let txt_path = temp_dir.path().join("test.txt");
    let mut file = File::create(&txt_path).unwrap();
    file.write_all(b"Hello world").unwrap();

    // .bin файл без signature (должен быть пропущен с ошибкой)
    let bin_path = temp_dir.path().join("nosig.bin");
    let mut file = File::create(&bin_path).unwrap();
    file.write_all(&vec![0xFF; 1000]).unwrap();

    // Используем auto_recover_directory
    let results = auto_recover_directory(temp_dir.path()).unwrap();

    // Проверяем что восстановлен только 1 файл
    assert_eq!(
        results.len(),
        1,
        "Должен быть восстановлен только 1 .hbk файл"
    );
}

#[test]
fn test_auto_recover_empty_directory() {
    let temp_dir = tempdir().unwrap();

    // Директория пуста
    let results = auto_recover_directory(temp_dir.path()).unwrap();

    assert_eq!(results.len(), 0, "Пустая директория должна вернуть пустой результат");
}

#[test]
fn test_auto_recover_nonexistent_directory() {
    let nonexistent = Path::new("/tmp/nonexistent_hbk_test_dir_12345");

    let result = auto_recover_directory(nonexistent);

    assert!(
        result.is_err(),
        "Should fail when directory doesn't exist"
    );
}

#[test]
fn test_file_not_found() {
    let temp_dir = tempdir().unwrap();
    let nonexistent_path = temp_dir.path().join("nonexistent.hbk");

    let mut recovery = HbkRecovery::new();
    let result = recovery.recover(&nonexistent_path, Some(temp_dir.path()));

    assert!(result.is_err(), "Should fail when file doesn't exist");
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("не существует"),
        "Error message должно упоминать отсутствие файла"
    );
}

#[test]
fn test_custom_output_directory() {
    let temp_dir = tempdir().unwrap();
    let hbk_dir = temp_dir.path().join("hbk_files");
    let output_dir = temp_dir.path().join("output");
    fs::create_dir(&hbk_dir).unwrap();
    fs::create_dir(&output_dir).unwrap();

    // Создаём .hbk файл в hbk_dir
    let hbk_path = hbk_dir.join("test.hbk");
    let hbk_data = create_test_hbk_file(1000);
    let mut file = File::create(&hbk_path).unwrap();
    file.write_all(&hbk_data).unwrap();
    drop(file);

    // Восстанавливаем в output_dir
    let mut recovery = HbkRecovery::new();
    let result = recovery.recover(&hbk_path, Some(&output_dir)).unwrap();

    // Проверяем что результаты в output_dir, а не в hbk_dir
    assert!(
        result.extracted_dir.as_ref().unwrap().starts_with(&output_dir),
        "Extracted dir должна быть в output_dir"
    );
}

#[test]
fn test_recovery_result_contains_correct_info() {
    let temp_dir = tempdir().unwrap();
    let hbk_path = temp_dir.path().join("test.hbk");

    // Создаём файл с известными параметрами
    let signature_offset = 2000;
    let mut file = File::create(&hbk_path).unwrap();
    file.write_all(&vec![0xFF; signature_offset]).unwrap();
    file.write_all(&create_minimal_empty_zip()).unwrap();
    drop(file);

    // Восстанавливаем
    let mut recovery = HbkRecovery::with_options(RecoveryOptions {
        cleanup_temp: true,
        auto_extract: false,
        max_file_size: 10 * 1024 * 1024,
    });
    let result = recovery
        .recover(&hbk_path, Some(temp_dir.path()))
        .unwrap();

    // Проверяем информацию в результате
    assert_eq!(
        result.signature_offset, signature_offset,
        "signature_offset должен совпадать"
    );
    assert!(
        result.recovered_size > 0,
        "recovered_size должен быть больше 0"
    );
    assert!(
        result.repaired_zip_path.exists(),
        "repaired_zip_path должен существовать"
    );
}

// ============================================================================
// Тесты поведения при различных путях
// ============================================================================

#[test]
fn test_relative_paths() {
    let temp_dir = tempdir().unwrap();
    let hbk_path = temp_dir.path().join("test.hbk");

    let hbk_data = create_test_hbk_file(1000);
    let mut file = File::create(&hbk_path).unwrap();
    file.write_all(&hbk_data).unwrap();
    drop(file);

    // Пытаемся с абсолютным путём (должно работать)
    let mut recovery = HbkRecovery::new();
    let result = recovery.recover(&hbk_path, Some(temp_dir.path()));

    assert!(result.is_ok(), "Should work with absolute paths");
}

#[test]
fn test_recovery_options_default() {
    let options = RecoveryOptions::default();

    assert!(options.cleanup_temp, "cleanup_temp должен быть true по умолчанию");
    assert!(
        options.auto_extract,
        "auto_extract должен быть true по умолчанию"
    );
    assert_eq!(
        options.max_file_size, 500 * 1024 * 1024,
        "max_file_size должен быть 500 MB по умолчанию"
    );
}
