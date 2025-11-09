//! Тесты обработки ошибок для HBK Recovery
//!
//! Проверяет корректное обрабатывание различных ошибочных ситуаций

use bsl_backend::data::loaders::hbk_recovery::{HbkRecovery, RecoveryOptions};
use std::fs::{self, File};
use std::io::Write;
use tempfile::tempdir;

// ============================================================================
// Тесты на ошибки файловой системы
// ============================================================================

#[test]
fn test_file_not_found_error() {
    let temp_dir = tempdir().unwrap();
    let nonexistent_path = temp_dir.path().join("nonexistent.hbk");

    let mut recovery = HbkRecovery::new();
    let result = recovery.recover(&nonexistent_path, Some(temp_dir.path()));

    assert!(result.is_err(), "Should fail when file doesn't exist");

    let err = result.unwrap_err();
    let err_str = err.to_string();

    assert!(
        err_str.contains("не существует") || err_str.contains("No such file"),
        "Error message should mention file not found"
    );
}

#[test]
fn test_empty_file_error() {
    let temp_dir = tempdir().unwrap();
    let empty_path = temp_dir.path().join("empty.hbk");

    // Создаём пустой файл
    File::create(&empty_path).unwrap();

    let mut recovery = HbkRecovery::new();
    let result = recovery.recover(&empty_path, Some(temp_dir.path()));

    assert!(result.is_err(), "Should fail on empty file");
}

#[test]
fn test_file_smaller_than_signature() {
    let temp_dir = tempdir().unwrap();
    let small_path = temp_dir.path().join("small.hbk");

    // Создаём файл меньше 4 байт (размер ZIP signature)
    let mut file = File::create(&small_path).unwrap();
    file.write_all(&[0xFF, 0xFF]).unwrap();
    drop(file);

    let mut recovery = HbkRecovery::new();
    let result = recovery.recover(&small_path, Some(temp_dir.path()));

    assert!(result.is_err(), "Should fail when file is smaller than signature");
}

// ============================================================================
// Тесты на отсутствие ZIP signature
// ============================================================================

#[test]
fn test_file_without_zip_signature() {
    let temp_dir = tempdir().unwrap();
    let no_sig_path = temp_dir.path().join("no_signature.bin");

    // Создаём файл с "мусором" но без ZIP signature
    let mut file = File::create(&no_sig_path).unwrap();
    file.write_all(&vec![0xFF; 10000]).unwrap();
    drop(file);

    let mut recovery = HbkRecovery::new();
    let result = recovery.recover(&no_sig_path, Some(temp_dir.path()));

    assert!(result.is_err(), "Should fail without ZIP signature");

    let err = result.unwrap_err();
    let err_str = err.to_string();

    assert!(
        err_str.contains("ZIP signature") || err_str.contains("не найдена"),
        "Error message should mention ZIP signature not found"
    );
}

#[test]
fn test_file_with_similar_but_wrong_signature() {
    let temp_dir = tempdir().unwrap();
    let wrong_sig_path = temp_dir.path().join("wrong_sig.bin");

    // Создаём файл с похожей но неправильной signature
    let mut file = File::create(&wrong_sig_path).unwrap();
    file.write_all(&vec![0xFF; 1000]).unwrap();
    file.write_all(&[0x50, 0x4B, 0x03, 0x03]).unwrap(); // Неправильный 4-й байт
    file.write_all(&vec![0x00; 100]).unwrap();
    drop(file);

    let mut recovery = HbkRecovery::new();
    let result = recovery.recover(&wrong_sig_path, Some(temp_dir.path()));

    assert!(
        result.is_err(),
        "Should fail with incorrect ZIP signature"
    );
}

// ============================================================================
// Тесты лимитов файлов
// ============================================================================

#[test]
fn test_max_file_size_exceeded() {
    let temp_dir = tempdir().unwrap();
    let hbk_path = temp_dir.path().join("large.hbk");

    // Создаём файл размером 2KB
    let mut file = File::create(&hbk_path).unwrap();
    file.write_all(&vec![0xFF; 1000]).unwrap();
    file.write_all(&[0x50, 0x4B, 0x03, 0x04]).unwrap();
    file.write_all(&vec![0x00; 500]).unwrap();
    drop(file);

    // Пытаемся с max_file_size = 100 байт
    let mut recovery = HbkRecovery::with_options(RecoveryOptions {
        cleanup_temp: true,
        auto_extract: true,
        max_file_size: 100,
    });

    let result = recovery.recover(&hbk_path, Some(temp_dir.path()));

    assert!(result.is_err(), "Should fail when file exceeds max_file_size");

    let err = result.unwrap_err();
    let err_str = err.to_string();

    assert!(
        err_str.contains("слишком большой") || err_str.contains("too large"),
        "Error message should mention file size limit"
    );
}

#[test]
fn test_file_size_exactly_at_limit() {
    let temp_dir = tempdir().unwrap();
    let hbk_path = temp_dir.path().join("at_limit.hbk");

    let junk_size = 1000;
    let mut file = File::create(&hbk_path).unwrap();
    file.write_all(&vec![0xFF; junk_size]).unwrap();
    file.write_all(&[0x50, 0x4B, 0x03, 0x04]).unwrap();
    file.write_all(&vec![0x00; 500]).unwrap();
    drop(file);

    let file_size = fs::metadata(&hbk_path).unwrap().len() as usize;

    // Пытаемся с max_file_size = точный размер файла
    let mut recovery = HbkRecovery::with_options(RecoveryOptions {
        cleanup_temp: true,
        auto_extract: true,
        max_file_size: file_size,
    });

    let result = recovery.recover(&hbk_path, Some(temp_dir.path()));

    assert!(result.is_ok(), "Should succeed when file is at exact size limit");
}

#[test]
fn test_file_size_just_below_limit() {
    let temp_dir = tempdir().unwrap();
    let hbk_path = temp_dir.path().join("below_limit.hbk");

    let junk_size = 1000;
    let mut file = File::create(&hbk_path).unwrap();
    file.write_all(&vec![0xFF; junk_size]).unwrap();
    file.write_all(&[0x50, 0x4B, 0x03, 0x04]).unwrap();
    file.write_all(&vec![0x00; 500]).unwrap();
    drop(file);

    let file_size = fs::metadata(&hbk_path).unwrap().len() as usize;

    // Пытаемся с max_file_size = размер + 1 байт
    let mut recovery = HbkRecovery::with_options(RecoveryOptions {
        cleanup_temp: true,
        auto_extract: true,
        max_file_size: file_size + 1,
    });

    let result = recovery.recover(&hbk_path, Some(temp_dir.path()));

    assert!(
        result.is_ok(),
        "Should succeed when file is just below limit"
    );
}

// ============================================================================
// Тесты на разрешения и доступ к файловой системе
// ============================================================================

#[test]
fn test_file_read_error_simulation() {
    let temp_dir = tempdir().unwrap();
    let hbk_path = temp_dir.path().join("test.hbk");

    // Создаём файл
    let mut file = File::create(&hbk_path).unwrap();
    file.write_all(&vec![0xFF; 1000]).unwrap();
    file.write_all(&[0x50, 0x4B, 0x03, 0x04]).unwrap();
    file.write_all(&vec![0x00; 500]).unwrap();
    drop(file);

    // На Unix-like системах можем изменить permissions
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o000);
        fs::set_permissions(&hbk_path, perms).unwrap();

        let mut recovery = HbkRecovery::new();
        let result = recovery.recover(&hbk_path, Some(temp_dir.path()));

        // Восстанавливаем permissions для cleanup
        let perms = std::fs::Permissions::from_mode(0o644);
        fs::set_permissions(&hbk_path, perms).ok();

        assert!(
            result.is_err(),
            "Should fail when file is not readable"
        );
    }

    // На Windows тест пропускаем (нет легального способа сделать файл нечитаемым)
    #[cfg(windows)]
    {
        // Просто проверяем что файл нормально читается
        let mut recovery = HbkRecovery::new();
        let result = recovery.recover(&hbk_path, Some(temp_dir.path()));
        assert!(result.is_ok());
    }
}

#[test]
fn test_directory_not_found_for_creation() {
    let temp_dir = tempdir().unwrap();
    let hbk_path = temp_dir.path().join("test.hbk");

    // Создаём валидный .hbk файл
    let mut file = File::create(&hbk_path).unwrap();
    file.write_all(&vec![0xFF; 1000]).unwrap();
    file.write_all(&[0x50, 0x4B, 0x03, 0x04]).unwrap();
    file.write_all(&vec![0x00; 500]).unwrap();
    drop(file);

    // Пытаемся вывести результаты в несуществующую директорию
    // (HbkRecovery должна создать её, так что это должно работать)
    let nonexistent_output = temp_dir.path().join("nonexistent").join("deep").join("dir");

    let mut recovery = HbkRecovery::new();
    let result = recovery.recover(&hbk_path, Some(&nonexistent_output));

    // Должно успешно создать директории
    assert!(result.is_ok(), "Should create nested directories automatically");
}

// ============================================================================
// Тесты с поддельными/повреждёнными данными
// ============================================================================

#[test]
fn test_corrupted_data_after_signature() {
    let temp_dir = tempdir().unwrap();
    let corrupted_path = temp_dir.path().join("corrupted.hbk");

    // Создаём файл с ZIP signature но с повреждёнными данными после
    let mut file = File::create(&corrupted_path).unwrap();
    file.write_all(&vec![0xFF; 1000]).unwrap();
    file.write_all(&[0x50, 0x4B, 0x03, 0x04]).unwrap();
    // Вместо валидного ZIP пишем мусор
    file.write_all(&vec![0xFF; 500]).unwrap();
    drop(file);

    let mut recovery = HbkRecovery::new();
    let result = recovery.recover(&corrupted_path, Some(temp_dir.path()));

    // Может быть успех (если распаковка не проверяет интегрность)
    // или ошибка (если проверяет). Важно что не crash
    let _ = result; // Просто проверяем что не паникует
}

#[test]
fn test_multiple_zip_signatures() {
    let temp_dir = tempdir().unwrap();
    let multi_sig_path = temp_dir.path().join("multi_sig.hbk");

    // Создаём файл с несколькими ZIP signatures
    let mut file = File::create(&multi_sig_path).unwrap();
    file.write_all(&vec![0xFF; 1000]).unwrap();
    file.write_all(&[0x50, 0x4B, 0x03, 0x04]).unwrap(); // Первая signature
    file.write_all(&vec![0x00; 500]).unwrap();
    file.write_all(&[0x50, 0x4B, 0x03, 0x04]).unwrap(); // Вторая signature
    file.write_all(&vec![0x00; 500]).unwrap();
    drop(file);

    let mut recovery = HbkRecovery::new();
    let result = recovery.recover(&multi_sig_path, Some(temp_dir.path()));

    // Должно найти первую signature
    assert!(result.is_ok(), "Should handle multiple signatures");

    let result = result.unwrap();
    assert_eq!(
        result.signature_offset, 1000,
        "Should find first signature at offset 1000"
    );
}

// ============================================================================
// Тесты на graceful degradation при обработке множества файлов
// ============================================================================

#[test]
fn test_auto_recover_with_mixed_valid_invalid_files() {
    let temp_dir = tempdir().unwrap();

    // Валидный .hbk файл
    let valid_hbk = temp_dir.path().join("valid.hbk");
    let mut file = File::create(&valid_hbk).unwrap();
    file.write_all(&vec![0xFF; 1000]).unwrap();
    file.write_all(&[0x50, 0x4B, 0x03, 0x04]).unwrap();
    file.write_all(&vec![0x00; 500]).unwrap();

    // Невалидный .hbk файл (без signature)
    let invalid_hbk = temp_dir.path().join("invalid.hbk");
    let mut file = File::create(&invalid_hbk).unwrap();
    file.write_all(&vec![0xFF; 1000]).unwrap();

    // Используем auto_recover_directory
    let results = bsl_backend::data::loaders::hbk_recovery::auto_recover_directory(temp_dir.path())
        .unwrap();

    // Должен восстановить только валидный файл
    assert_eq!(
        results.len(),
        1,
        "Should recover only valid file and gracefully skip invalid"
    );
}

// ============================================================================
// Тесты на edge cases при поиске signature
// ============================================================================

#[test]
fn test_signature_at_very_large_offset() {
    let temp_dir = tempdir().unwrap();
    let large_offset_path = temp_dir.path().join("large_offset.hbk");

    // Создаём файл с signature на offset 1MB
    let offset = 1024 * 1024;
    let mut file = File::create(&large_offset_path).unwrap();
    file.write_all(&vec![0xFF; offset]).unwrap();
    file.write_all(&[0x50, 0x4B, 0x03, 0x04]).unwrap();
    file.write_all(&vec![0x00; 500]).unwrap();
    drop(file);

    let mut recovery = HbkRecovery::new();
    let result = recovery.recover(&large_offset_path, Some(temp_dir.path()));

    assert!(result.is_ok(), "Should handle large offsets");

    let result = result.unwrap();
    assert_eq!(
        result.signature_offset, offset,
        "Should find signature at large offset"
    );
}

#[test]
fn test_signature_search_performance_large_file() {
    let temp_dir = tempdir().unwrap();
    let large_path = temp_dir.path().join("large.hbk");

    // Создаём файл размером 10 MB с signature в конце
    let junk_size = 10 * 1024 * 1024 - 1000;
    let mut file = File::create(&large_path).unwrap();

    // Пишем большой блок мусора
    let chunk = vec![0xFF; 1024 * 1024]; // 1 MB chunks
    for _ in 0..10 {
        file.write_all(&chunk).unwrap();
    }

    file.write_all(&[0x50, 0x4B, 0x03, 0x04]).unwrap();
    file.write_all(&vec![0x00; 500]).unwrap();
    drop(file);

    // Проверяем что файл создан корректно
    let file_size = fs::metadata(&large_path).unwrap().len();
    assert!(file_size > 10_000_000, "File should be large");

    let mut recovery = HbkRecovery::new();
    let result = recovery.recover(&large_path, Some(temp_dir.path()));

    assert!(result.is_ok(), "Should handle large files");
}
