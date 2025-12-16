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

// Внутренние модули
mod batch;
mod extractor;
mod recovery;
pub(crate) mod signature;
mod types;

// Публичный API
pub use batch::{
    auto_recover_directory, auto_recover_directory_with_options,
    auto_recover_directory_with_progress,
};
pub use recovery::{recover_hbk_file, HbkRecovery};
pub use types::{RecoveryOptions, RecoveryResult};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::loaders::progress::ProgressUpdateType;
    use std::fs::{self, File};
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_signature_search() {
        let temp_dir = tempdir().unwrap();
        let test_file_path = temp_dir.path().join("test.hbk");

        // Создаём тестовый файл с ZIP signature на offset 100
        let mut file = File::create(&test_file_path).unwrap();

        // Пишем 100 байт "мусора"
        file.write_all(&[0xFF; 100]).unwrap();

        // Пишем ZIP signature
        file.write_all(&[0x50, 0x4B, 0x03, 0x04]).unwrap();

        // Пишем ещё немного данных
        file.write_all(&[0x00; 100]).unwrap();

        drop(file);

        // Тестируем поиск
        let mut file = File::open(&test_file_path).unwrap();
        let offset = signature::find_zip_signature(&mut file, 1000).unwrap();

        assert_eq!(
            offset, 100,
            "ZIP signature должна быть найдена на offset 100"
        );
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
        file.write_all(&[0x00; 100]).unwrap();
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
        let junk = vec![0xFF; offset_expected];
        file.write_all(&junk).unwrap();
        file.write_all(&[0x50, 0x4B, 0x03, 0x04]).unwrap();
        let padding = vec![0x00; 500];
        file.write_all(&padding).unwrap();
        drop(file);

        // Тестируем поиск
        let mut file = File::open(&test_file_path).unwrap();
        let offset = signature::find_zip_signature(&mut file, 10600).unwrap();

        assert_eq!(
            offset, offset_expected,
            "Signature должна быть найдена на offset {}",
            offset_expected
        );
    }

    #[test]
    fn test_find_signature_near_chunk_boundary() {
        let temp_dir = tempdir().unwrap();
        let test_file_path = temp_dir.path().join("test.hbk");

        // Создаём файл где signature на границе chunk (64KB - 2)
        const CHUNK_SIZE: usize = 64 * 1024;
        const SIGNATURE_POS: usize = CHUNK_SIZE - 2;

        let mut file = File::create(&test_file_path).unwrap();
        let junk = vec![0xFF; SIGNATURE_POS];
        file.write_all(&junk).unwrap();
        file.write_all(&[0x50, 0x4B, 0x03, 0x04]).unwrap();
        file.write_all(&[0x00; 100]).unwrap();
        drop(file);

        // Тестируем поиск
        let mut file = File::open(&test_file_path).unwrap();
        let offset = signature::find_zip_signature(&mut file, SIGNATURE_POS + 104).unwrap();

        assert_eq!(
            offset, SIGNATURE_POS,
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
        let empty_zip = [
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
        file.write_all(&[0xFF; 1000]).unwrap();
        file.write_all(&[0x50, 0x4B, 0x05, 0x06]).unwrap();
        file.write_all(&empty_zip[4..]).unwrap();
        drop(file);

        // Открываем и извлекаем
        let mut source_file = File::open(&source_path).unwrap();
        let size = extractor::extract_valid_zip(&mut source_file, 1000, &output_path).unwrap();

        assert!(output_path.exists(), "Output ZIP должен существовать");
        assert_eq!(size, empty_zip.len(), "Размер должен совпадать с ZIP");
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
        assert_eq!(
            size,
            empty_zip.len() as usize,
            "Размер должен быть размер ZIP"
        );
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
        let result = recovery
            .recover(&test_file_path, Some(temp_dir.path()))
            .unwrap();

        // Проверяем что опции работают
        assert!(
            result.repaired_zip_path.exists(),
            "ZIP должен существовать (cleanup_temp = false)"
        );
        assert!(
            result.extracted_dir.is_none(),
            "Директория не должна быть распакована (auto_extract = false)"
        );
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
        let result = recovery
            .recover(&test_file_path, Some(temp_dir.path()))
            .unwrap();

        // Проверяем свойства результата
        assert_eq!(
            result.signature_offset, junk_offset,
            "signature_offset должен совпадать"
        );
        assert_eq!(
            result.recovered_size, 304,
            "recovered_size должен быть 4 (signature) + 300"
        );
        assert!(
            result.repaired_zip_path.exists(),
            "repaired_zip_path должен существовать"
        );
        assert!(
            result.extracted_dir.is_none(),
            "extracted_dir должна быть None при auto_extract = false"
        );
    }

    #[test]
    fn test_recover_with_progress_callback() {
        let temp_dir = tempdir().unwrap();
        let test_file_path = temp_dir.path().join("test.hbk");

        let junk_offset = 1500;

        // Используем auto_extract = false чтобы не нужно было распаковывать
        let mut file = File::create(&test_file_path).unwrap();
        file.write_all(&vec![0xFF; junk_offset]).unwrap();
        file.write_all(&[0x50, 0x4B, 0x03, 0x04]).unwrap(); // ZIP signature
        file.write_all(&vec![0x00; 500]).unwrap();
        drop(file);

        // Восстанавливаем с progress callback
        let mut recovery = HbkRecovery::with_options(RecoveryOptions {
            cleanup_temp: true,
            auto_extract: false,
            max_file_size: 10 * 1024 * 1024,
        });

        let callback_invocations = std::cell::RefCell::new(0);
        let result = recovery
            .recover_with_progress(
                &test_file_path,
                Some(temp_dir.path()),
                Some(|_update| {
                    *callback_invocations.borrow_mut() += 1;
                }),
            )
            .unwrap();

        // Проверяем что callback не вызывался (т.к. auto_extract = false)
        assert_eq!(
            *callback_invocations.borrow(),
            0,
            "Callback не должен быть вызван при auto_extract = false"
        );

        // Проверяем результат
        assert_eq!(result.signature_offset, junk_offset);
        assert!(result.repaired_zip_path.exists());
    }

    #[test]
    fn test_progress_update_type_serialization() {
        // Тест сериализации HbkExtraction
        let update = ProgressUpdateType::hbk_extraction("shcntx_ru", 500, 1000);

        if let ProgressUpdateType::HbkExtraction {
            file_name,
            extracted_files,
            total_files,
            percentage,
        } = update
        {
            assert_eq!(file_name, "shcntx_ru");
            assert_eq!(extracted_files, 500);
            assert_eq!(total_files, 1000);
            assert_eq!(percentage, 50); // 500/1000 = 50%
        } else {
            panic!("Expected HbkExtraction variant");
        }

        // Тест граничного случая: 0 файлов
        let update_empty = ProgressUpdateType::hbk_extraction("test", 0, 0);
        if let ProgressUpdateType::HbkExtraction { percentage, .. } = update_empty {
            assert_eq!(percentage, 0);
        } else {
            panic!("Expected HbkExtraction variant");
        }
    }

    #[test]
    fn test_cache_reuse_on_second_recovery() {
        let temp_dir = tempdir().unwrap();
        let test_file_path = temp_dir.path().join("test.hbk");

        // Создаём тестовый .hbk файл с ZIP signature
        let junk_offset = 1000;
        let mut file = File::create(&test_file_path).unwrap();
        file.write_all(&vec![0xFF; junk_offset]).unwrap();
        file.write_all(&[0x50, 0x4B, 0x03, 0x04]).unwrap(); // ZIP signature
        file.write_all(&vec![0x00; 500]).unwrap();
        drop(file);

        // Первый вызов - распаковка из .hbk
        let mut recovery = HbkRecovery::with_options(RecoveryOptions {
            cleanup_temp: false,
            auto_extract: false, // НЕ распаковываем для упрощения теста
            max_file_size: 10 * 1024 * 1024,
        });
        let result1 = recovery
            .recover(&test_file_path, Some(temp_dir.path()))
            .unwrap();

        assert!(result1.extracted_dir.is_none(), "extract = false -> None");
        // Первый вызов должен найти реальный offset
        assert_eq!(
            result1.signature_offset, junk_offset,
            "Первый вызов должен найти signature"
        );

        // Вручную создаём кеш директорию
        let extract_dir = temp_dir.path().join("rebuilt.test");
        fs::create_dir_all(&extract_dir).unwrap();

        // Добавляем несколько файлов в кеш
        for i in 0..5 {
            File::create(extract_dir.join(format!("file_{}.txt", i))).unwrap();
        }

        // Второй вызов - должен использовать кеш
        let mut recovery = HbkRecovery::new();
        let result2 = recovery
            .recover(&test_file_path, Some(temp_dir.path()))
            .unwrap();

        // При использовании кеша signature_offset = 0 (не искали в файле)
        assert_eq!(
            result2.signature_offset, 0,
            "Второй вызов должен использовать кеш (signature_offset = 0)"
        );

        // Проверяем что та же директория возвращена
        assert_eq!(
            result2.extracted_dir.as_ref().map(|p| p.to_string_lossy()),
            Some(extract_dir.to_string_lossy()),
            "Должна быть возвращена та же директория кеша"
        );
    }

    #[test]
    fn test_cache_not_used_when_empty() {
        let temp_dir = tempdir().unwrap();
        let test_file_path = temp_dir.path().join("test.hbk");

        // Создаём тестовый .hbk файл
        let junk_offset = 1000;
        let mut file = File::create(&test_file_path).unwrap();
        file.write_all(&vec![0xFF; junk_offset]).unwrap();
        file.write_all(&[0x50, 0x4B, 0x03, 0x04]).unwrap(); // ZIP signature
        file.write_all(&vec![0x00; 500]).unwrap();
        drop(file);

        // Создаём пустую директорию кеша (без файлов)
        let extract_dir = temp_dir.path().join("rebuilt.test");
        fs::create_dir_all(&extract_dir).unwrap();

        // Попытка восстановления - должна пропустить пустой кеш
        let mut recovery = HbkRecovery::with_options(RecoveryOptions {
            cleanup_temp: false,
            auto_extract: false,
            max_file_size: 10 * 1024 * 1024,
        });

        let result = recovery.recover(&test_file_path, Some(temp_dir.path()));

        // Результат должен быть Ok, т.к. находится ZIP signature
        assert!(
            result.is_ok(),
            "Восстановление должно пройти (кеш пустой, используется обычный путь)"
        );
    }

    #[test]
    fn test_clear_cache() {
        let temp_dir = tempdir().unwrap();
        let test_file_path = temp_dir.path().join("test.hbk");

        // Создаём тестовый .hbk файл
        let mut file = File::create(&test_file_path).unwrap();
        file.write_all(&vec![0xFF; 100]).unwrap();
        file.write_all(&[0x50, 0x4B, 0x03, 0x04]).unwrap();
        file.write_all(&vec![0x00; 100]).unwrap();
        drop(file);

        // Создаём директорию кеша
        let extract_dir = temp_dir.path().join("rebuilt.test");
        fs::create_dir_all(&extract_dir).unwrap();
        File::create(extract_dir.join("file.txt")).unwrap();

        assert!(extract_dir.exists(), "Кеш должен существовать");

        // Очищаем кеш
        let result = HbkRecovery::clear_cache(&test_file_path, Some(temp_dir.path()));

        assert!(result.is_ok(), "clear_cache должен вернуть Ok");
        assert!(!extract_dir.exists(), "Кеш должен быть удалён");
    }

    #[test]
    fn test_clear_cache_nonexistent() {
        let temp_dir = tempdir().unwrap();
        let test_file_path = temp_dir.path().join("nonexistent.hbk");

        // Пытаемся очистить кеш для несуществующего файла
        let result = HbkRecovery::clear_cache(&test_file_path, Some(temp_dir.path()));

        // Должен вернуть Ok, т.к. кеша не было
        assert!(
            result.is_ok(),
            "clear_cache должен успешно обработать несуществующий кеш"
        );
    }
}
