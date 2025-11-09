//! Тесты граничных случаев (Edge Cases) для HBK Recovery
//!
//! Проверяет поведение на необычных и экстремальных входных данных

use bsl_backend::data::loaders::hbk_recovery::{
    auto_recover_directory, auto_recover_directory_with_options, HbkRecovery, RecoveryOptions,
};
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use tempfile::tempdir;

// ============================================================================
// Edge Case: Различные позиции signature
// ============================================================================

#[test]
fn test_signature_at_byte_boundary() {
    let temp_dir = tempdir().unwrap();
    let edge_path = temp_dir.path().join("edge.hbk");

    // Создаём файл с signature на различных граничных позициях
    for boundary in &[256, 512, 1024, 4096, 65536] {
        let edge_path = temp_dir.path().join(format!("edge_{}.hbk", boundary));
        let mut file = File::create(&edge_path).unwrap();
        file.write_all(&vec![0xFF; boundary - 1]).unwrap();
        file.write_all(&[0x50, 0x4B, 0x03, 0x04]).unwrap();
        file.write_all(&vec![0x00; 100]).unwrap();
        drop(file);

        // Не пытаемся распаковывать невалидный ZIP
        let mut recovery = HbkRecovery::with_options(RecoveryOptions {
            auto_extract: false,
            cleanup_temp: false,
            max_file_size: 10 * 1024 * 1024,
        });
        let result = recovery
            .recover(&edge_path, Some(temp_dir.path()))
            .unwrap();

        assert_eq!(
            result.signature_offset, boundary - 1,
            "Should find signature at boundary {}", boundary
        );
    }
}

#[test]
fn test_signature_at_chunk_boundary() {
    let temp_dir = tempdir().unwrap();
    let chunk_path = temp_dir.path().join("chunk.hbk");

    // 64KB chunk size в коде
    let chunk_size = 64 * 1024;

    // Тестируем signature на различных chunk boundaries
    for chunk_offset in &[0, 1, 2, 3] {
        let sig_pos = chunk_size * chunk_offset - (*chunk_offset > 0) as usize;

        if sig_pos == 0 {
            continue; // Пропускаем нулевой offset, уже тестируется
        }

        let chunk_path = temp_dir.path().join(format!("chunk_{}.hbk", chunk_offset));
        let mut file = File::create(&chunk_path).unwrap();
        file.write_all(&vec![0xFF; sig_pos]).unwrap();
        file.write_all(&[0x50, 0x4B, 0x03, 0x04]).unwrap();
        file.write_all(&vec![0x00; 100]).unwrap();
        drop(file);

        // Не пытаемся распаковывать невалидный ZIP
        let mut recovery = HbkRecovery::with_options(RecoveryOptions {
            auto_extract: false,
            cleanup_temp: false,
            max_file_size: 10 * 1024 * 1024,
        });
        let result = recovery
            .recover(&chunk_path, Some(temp_dir.path()))
            .unwrap();

        assert_eq!(
            result.signature_offset, sig_pos,
            "Should handle chunk boundary at offset {}", sig_pos
        );
    }
}

// ============================================================================
// Edge Case: Минимальные и максимальные размеры
// ============================================================================

#[test]
fn test_minimal_valid_file() {
    let temp_dir = tempdir().unwrap();
    let minimal_path = temp_dir.path().join("minimal.hbk");

    // Минимальный файл: только ZIP signature
    let mut file = File::create(&minimal_path).unwrap();
    file.write_all(&[0x50, 0x4B, 0x03, 0x04]).unwrap();
    drop(file);

    // Не пытаемся распаковывать невалидный ZIP
    let mut recovery = HbkRecovery::with_options(RecoveryOptions {
        auto_extract: false,
        cleanup_temp: false,
        max_file_size: 10 * 1024 * 1024,
    });
    let result = recovery.recover(&minimal_path, Some(temp_dir.path()));

    // Должно работать (ZIP восстановлен, но не распакован)
    assert!(
        result.is_ok(),
        "Should handle file with only ZIP signature"
    );
    assert_eq!(result.unwrap().signature_offset, 0);
}

#[test]
fn test_one_byte_file() {
    let temp_dir = tempdir().unwrap();
    let one_byte_path = temp_dir.path().join("one_byte.hbk");

    let mut file = File::create(&one_byte_path).unwrap();
    file.write_all(&[0xFF]).unwrap();
    drop(file);

    // Не пытаемся распаковывать невалидный ZIP
    let mut recovery = HbkRecovery::with_options(RecoveryOptions {
        auto_extract: false,
        cleanup_temp: false,
        max_file_size: 10 * 1024 * 1024,
    });
    let result = recovery.recover(&one_byte_path, Some(temp_dir.path()));

    assert!(result.is_err(), "Should fail on 1-byte file");
}

#[test]
fn test_exactly_4_byte_file_without_signature() {
    let temp_dir = tempdir().unwrap();
    let four_byte_path = temp_dir.path().join("four_byte.hbk");

    let mut file = File::create(&four_byte_path).unwrap();
    file.write_all(&[0xFF, 0xFF, 0xFF, 0xFF]).unwrap();
    drop(file);

    // Не пытаемся распаковывать невалидный ZIP
    let mut recovery = HbkRecovery::with_options(RecoveryOptions {
        auto_extract: false,
        cleanup_temp: false,
        max_file_size: 10 * 1024 * 1024,
    });
    let result = recovery.recover(&four_byte_path, Some(temp_dir.path()));

    assert!(result.is_err(), "Should fail when 4 bytes don't match signature");
}

#[test]
fn test_exactly_4_byte_file_with_signature() {
    let temp_dir = tempdir().unwrap();
    let sig_path = temp_dir.path().join("sig_only.hbk");

    let mut file = File::create(&sig_path).unwrap();
    file.write_all(&[0x50, 0x4B, 0x03, 0x04]).unwrap();
    drop(file);

    // Не пытаемся распаковывать невалидный ZIP
    let mut recovery = HbkRecovery::with_options(RecoveryOptions {
        auto_extract: false,
        cleanup_temp: false,
        max_file_size: 10 * 1024 * 1024,
    });
    let result = recovery.recover(&sig_path, Some(temp_dir.path()));

    assert!(result.is_ok(), "Should handle file with only ZIP signature");
}

// ============================================================================
// Edge Case: Signature в неожиданных местах
// ============================================================================

#[test]
fn test_signature_repeated_pattern() {
    let temp_dir = tempdir().unwrap();
    let repeat_path = temp_dir.path().join("repeat.hbk");

    // Создаём файл где паттерн похожий на signature повторяется
    let mut file = File::create(&repeat_path).unwrap();

    // Пишем паттерны 0x50 0x4B (первые 2 байта signature)
    for _ in 0..500 {
        file.write_all(&[0x50, 0x4B]).unwrap();
    }

    // Затем настоящая signature
    let sig_pos = 1000;
    file.write_all(&vec![0xFF; sig_pos - file.metadata().unwrap().len() as usize])
        .unwrap();
    file.write_all(&[0x50, 0x4B, 0x03, 0x04]).unwrap();
    file.write_all(&vec![0x00; 100]).unwrap();
    drop(file);

    // Не пытаемся распаковывать невалидный ZIP
    let mut recovery = HbkRecovery::with_options(RecoveryOptions {
        auto_extract: false,
        cleanup_temp: false,
        max_file_size: 10 * 1024 * 1024,
    });
    let result = recovery.recover(&repeat_path, Some(temp_dir.path()));

    assert!(result.is_ok(), "Should find correct signature among patterns");

    // Должна быть найдена первая полная signature
    let offset = result.unwrap().signature_offset;
    assert!(offset <= 1004, "Should find first complete signature"); // 1000 + 4
}

#[test]
fn test_signature_in_sequence_of_bytes() {
    let temp_dir = tempdir().unwrap();
    let seq_path = temp_dir.path().join("sequence.hbk");

    // Создаём файл где signature появляется как часть последовательности
    let mut file = File::create(&seq_path).unwrap();

    // Пишем последовательность от 0x00 до 0xFF
    for i in 0..=255u8 {
        file.write_all(&[i]).unwrap();
    }
    // 0x50 находится на позиции 80, 0x4B на 75, 0x03 на 3, 0x04 на 4

    // Добавляем ещё мусора
    file.write_all(&vec![0xFF; 500]).unwrap();

    // Добавляем корректную signature
    file.write_all(&[0x50, 0x4B, 0x03, 0x04]).unwrap();
    file.write_all(&vec![0x00; 100]).unwrap();
    drop(file);

    // Не пытаемся распаковывать невалидный ZIP
    let mut recovery = HbkRecovery::with_options(RecoveryOptions {
        auto_extract: false,
        cleanup_temp: false,
        max_file_size: 10 * 1024 * 1024,
    });
    let result = recovery.recover(&seq_path, Some(temp_dir.path()));

    assert!(result.is_ok(), "Should find correct signature sequence");

    let offset = result.unwrap().signature_offset;
    assert!(
        offset > 256,
        "Should find signature after byte sequence, not in it"
    );
}

// ============================================================================
// Edge Case: Специальные символы в пути
// ============================================================================

#[test]
fn test_file_with_special_chars_in_path() {
    let temp_dir = tempdir().unwrap();

    // Тестируем различные спецсимволы в имени файла
    let special_names = vec![
        "test-file.hbk",
        "test_file.hbk",
        "test file.hbk",
        "test (1).hbk",
        "test [test].hbk",
    ];

    for name in special_names {
        let path = temp_dir.path().join(name);

        let mut file = File::create(&path).unwrap();
        file.write_all(&vec![0xFF; 1000]).unwrap();
        file.write_all(&[0x50, 0x4B, 0x03, 0x04]).unwrap();
        file.write_all(&vec![0x00; 500]).unwrap();
        drop(file);

        // Не пытаемся распаковывать невалидный ZIP
    let mut recovery = HbkRecovery::with_options(RecoveryOptions {
        auto_extract: false,
        cleanup_temp: false,
        max_file_size: 10 * 1024 * 1024,
    });
        let result = recovery.recover(&path, Some(temp_dir.path()));

        assert!(
            result.is_ok(),
            "Should handle special characters in filename: {}",
            name
        );
    }
}

#[test]
fn test_nested_directory_recovery() {
    let temp_dir = tempdir().unwrap();

    // Создаём глубокую директорную структуру
    let nested_dir = temp_dir.path().join("a").join("b").join("c").join("d");
    fs::create_dir_all(&nested_dir).unwrap();

    let hbk_path = nested_dir.join("test.hbk");
    let mut file = File::create(&hbk_path).unwrap();
    file.write_all(&vec![0xFF; 1000]).unwrap();
    file.write_all(&[0x50, 0x4B, 0x03, 0x04]).unwrap();
    file.write_all(&vec![0x00; 500]).unwrap();
    drop(file);

    // Не пытаемся распаковывать невалидный ZIP
    let mut recovery = HbkRecovery::with_options(RecoveryOptions {
        auto_extract: false,
        cleanup_temp: false,
        max_file_size: 10 * 1024 * 1024,
    });
    let result = recovery.recover(&hbk_path, Some(&nested_dir));

    assert!(
        result.is_ok(),
        "Should handle deeply nested directory structures"
    );
}

// ============================================================================
// Edge Case: Множественное восстановление
// ============================================================================

#[test]
fn test_recover_same_file_twice() {
    let temp_dir = tempdir().unwrap();
    let hbk_path = temp_dir.path().join("test.hbk");

    let mut file = File::create(&hbk_path).unwrap();
    file.write_all(&vec![0xFF; 1000]).unwrap();
    file.write_all(&[0x50, 0x4B, 0x03, 0x04]).unwrap();
    file.write_all(&vec![0x00; 500]).unwrap();
    drop(file);

    // Не пытаемся распаковывать невалидный ZIP
    let mut recovery = HbkRecovery::with_options(RecoveryOptions {
        auto_extract: false,
        cleanup_temp: false,
        max_file_size: 10 * 1024 * 1024,
    });

    // Восстанавливаем первый раз
    let result1 = recovery.recover(&hbk_path, Some(temp_dir.path()));
    assert!(result1.is_ok(), "First recovery should succeed");

    // Восстанавливаем второй раз
    let result2 = recovery.recover(&hbk_path, Some(temp_dir.path()));
    assert!(result2.is_ok(), "Second recovery should succeed");

    // Оба результата должны быть успешны
    let r1 = result1.unwrap();
    let r2 = result2.unwrap();

    assert_eq!(
        r1.signature_offset, r2.signature_offset,
        "Should find same offset both times"
    );
}

#[test]
fn test_recover_many_files_sequentially() {
    let temp_dir = tempdir().unwrap();

    // Создаём множество файлов
    for i in 0..10 {
        let hbk_path = temp_dir.path().join(format!("file_{:02}.hbk", i));
        let mut file = File::create(&hbk_path).unwrap();
        file.write_all(&vec![0xFF; 1000 + i * 100]).unwrap();
        file.write_all(&[0x50, 0x4B, 0x03, 0x04]).unwrap();
        file.write_all(&vec![0x00; 500]).unwrap();
    }

    // Используем auto_recover_directory_with_options с auto_extract = false
    let options = RecoveryOptions {
        auto_extract: false,
        cleanup_temp: false,
        max_file_size: 10 * 1024 * 1024,
    };
    let results = auto_recover_directory_with_options(temp_dir.path(), options).unwrap();

    assert_eq!(
        results.len(),
        10,
        "Should recover all 10 files"
    );

    // Проверяем что все файлы восстановлены (ZIP созданы)
    for result in &results {
        assert!(result.repaired_zip_path.exists(), "ZIP файл должен существовать");
        assert_eq!(result.extracted_dir, None, "Не должно быть распаковки при auto_extract = false");
    }
}

// ============================================================================
// Edge Case: Взаимодействие опций
// ============================================================================

#[test]
fn test_cleanup_without_extraction() {
    let temp_dir = tempdir().unwrap();
    let hbk_path = temp_dir.path().join("test.hbk");

    let mut file = File::create(&hbk_path).unwrap();
    file.write_all(&vec![0xFF; 1000]).unwrap();
    file.write_all(&[0x50, 0x4B, 0x03, 0x04]).unwrap();
    file.write_all(&vec![0x00; 500]).unwrap();
    drop(file);

    // cleanup_temp = true, но auto_extract = false
    // Cleanup не должен удалять ZIP если его не распакизали
    let mut recovery = HbkRecovery::with_options(RecoveryOptions {
        cleanup_temp: true,
        auto_extract: false,
        max_file_size: 10 * 1024 * 1024,
    });

    let result = recovery
        .recover(&hbk_path, Some(temp_dir.path()))
        .unwrap();

    // ZIP должен существовать (cleanup относится только к auto_extract)
    assert!(
        result.repaired_zip_path.exists(),
        "ZIP should exist when auto_extract = false"
    );
}

#[test]
fn test_large_max_file_size() {
    let temp_dir = tempdir().unwrap();
    let hbk_path = temp_dir.path().join("test.hbk");

    let mut file = File::create(&hbk_path).unwrap();
    file.write_all(&vec![0xFF; 1000]).unwrap();
    file.write_all(&[0x50, 0x4B, 0x03, 0x04]).unwrap();
    file.write_all(&vec![0x00; 500]).unwrap();
    drop(file);

    // Устанавливаем очень большой max_file_size (1GB), но не распаковываем невалидный ZIP
    let mut recovery = HbkRecovery::with_options(RecoveryOptions {
        cleanup_temp: true,
        auto_extract: false,  // Не распаковываем невалидный ZIP
        max_file_size: 1024 * 1024 * 1024,
    });

    let result = recovery.recover(&hbk_path, Some(temp_dir.path()));

    assert!(result.is_ok(), "Should succeed with very large max_file_size");
}

#[test]
fn test_zero_max_file_size() {
    let temp_dir = tempdir().unwrap();
    let hbk_path = temp_dir.path().join("test.hbk");

    let mut file = File::create(&hbk_path).unwrap();
    file.write_all(&vec![0xFF; 1000]).unwrap();
    file.write_all(&[0x50, 0x4B, 0x03, 0x04]).unwrap();
    file.write_all(&vec![0x00; 500]).unwrap();
    drop(file);

    // max_file_size = 0 должен отклонить любой файл
    let mut recovery = HbkRecovery::with_options(RecoveryOptions {
        cleanup_temp: true,
        auto_extract: true,
        max_file_size: 0,
    });

    let result = recovery.recover(&hbk_path, Some(temp_dir.path()));

    assert!(result.is_err(), "Should reject file when max_file_size = 0");
}

// ============================================================================
// Edge Case: Различные типы повреждений
// ============================================================================

#[test]
fn test_zip_with_null_bytes_after_signature() {
    let temp_dir = tempdir().unwrap();
    let null_path = temp_dir.path().join("nulls.hbk");

    let mut file = File::create(&null_path).unwrap();
    file.write_all(&vec![0xFF; 1000]).unwrap();
    file.write_all(&[0x50, 0x4B, 0x03, 0x04]).unwrap();
    // Большой блок нулевых байтов после signature
    file.write_all(&vec![0x00; 10000]).unwrap();
    drop(file);

    // Не пытаемся распаковывать невалидный ZIP
    let mut recovery = HbkRecovery::with_options(RecoveryOptions {
        auto_extract: false,
        cleanup_temp: false,
        max_file_size: 10 * 1024 * 1024,
    });
    let result = recovery.recover(&null_path, Some(temp_dir.path()));

    // Должно завершиться без паники
    let _ = result;
}

#[test]
fn test_zip_with_only_zeros() {
    let temp_dir = tempdir().unwrap();
    let zeros_path = temp_dir.path().join("zeros.hbk");

    let mut file = File::create(&zeros_path).unwrap();
    file.write_all(&vec![0xFF; 1000]).unwrap();
    file.write_all(&[0x50, 0x4B, 0x03, 0x04]).unwrap();
    file.write_all(&vec![0x00; 5000]).unwrap();
    drop(file);

    // Не пытаемся распаковывать невалидный ZIP
    let mut recovery = HbkRecovery::with_options(RecoveryOptions {
        auto_extract: false,
        cleanup_temp: false,
        max_file_size: 10 * 1024 * 1024,
    });
    let result = recovery.recover(&zeros_path, Some(temp_dir.path()));

    // Не должно паниковать
    let _ = result;
}

// ============================================================================
// Edge Case: Информация о результатах
// ============================================================================

#[test]
fn test_signature_offset_accuracy() {
    let temp_dir = tempdir().unwrap();

    // Тестируем точность определения offset при различных размерах mусора
    let offsets_to_test = vec![0, 1, 10, 100, 1000, 10000, 65535, 65536, 65537];

    for expected_offset in offsets_to_test {
        let path = temp_dir.path().join(format!("offset_{}.hbk", expected_offset));

        let mut file = File::create(&path).unwrap();
        file.write_all(&vec![0xFF; expected_offset]).unwrap();
        file.write_all(&[0x50, 0x4B, 0x03, 0x04]).unwrap();
        file.write_all(&vec![0x00; 100]).unwrap();
        drop(file);

        // Не пытаемся распаковывать невалидный ZIP
    let mut recovery = HbkRecovery::with_options(RecoveryOptions {
        auto_extract: false,
        cleanup_temp: false,
        max_file_size: 10 * 1024 * 1024,
    });
        let result = recovery.recover(&path, Some(temp_dir.path())).unwrap();

        assert_eq!(
            result.signature_offset, expected_offset,
            "Offset should be exact for size {}", expected_offset
        );
    }
}

#[test]
fn test_recovered_size_calculation() {
    let temp_dir = tempdir().unwrap();
    let hbk_path = temp_dir.path().join("test.hbk");

    let junk_size = 1500;
    let zip_content_size = 800;

    let mut file = File::create(&hbk_path).unwrap();
    file.write_all(&vec![0xFF; junk_size]).unwrap();
    file.write_all(&[0x50, 0x4B, 0x03, 0x04]).unwrap();
    file.write_all(&vec![0x00; zip_content_size]).unwrap();
    drop(file);

    // Не пытаемся распаковывать невалидный ZIP
    let mut recovery = HbkRecovery::with_options(RecoveryOptions {
        auto_extract: false,
        cleanup_temp: false,
        max_file_size: 10 * 1024 * 1024,
    });
    let result = recovery.recover(&hbk_path, Some(temp_dir.path())).unwrap();

    // recovered_size должен быть 4 (signature) + zip_content_size
    assert_eq!(
        result.recovered_size,
        4 + zip_content_size,
        "Recovered size should match signature + content"
    );
}
