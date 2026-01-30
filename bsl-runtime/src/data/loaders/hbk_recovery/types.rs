//! Типы для HBK Recovery модуля

use std::path::PathBuf;

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
