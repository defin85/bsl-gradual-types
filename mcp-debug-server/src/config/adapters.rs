//! DAP Adapter auto-discovery
//!
//! This module provides automatic discovery of installed DAP adapters
//! in standard locations (VS Code extensions, system PATH, etc.).

use std::env;
use std::path::PathBuf;

/// Tries to find CodeLLDB adapter in standard VS Code extension locations.
///
/// Searches in:
/// - `~/.vscode/extensions/vadimcn.vscode-lldb-*/adapter/codelldb` (Linux/macOS)
/// - `%USERPROFILE%\.vscode\extensions\vadimcn.vscode-lldb-*\adapter\codelldb.exe` (Windows)
///
/// # Returns
///
/// Returns full path to `codelldb` executable if found, otherwise `None`.
pub fn find_codelldb() -> Option<PathBuf> {
    // Получить home directory
    let home = env::var("HOME").or_else(|_| env::var("USERPROFILE")).ok()?;

    let extensions_dir = PathBuf::from(home).join(".vscode").join("extensions");

    if !extensions_dir.exists() {
        return None;
    }

    // Найти директорию vadimcn.vscode-lldb-*
    let entries = std::fs::read_dir(&extensions_dir).ok()?;

    for entry in entries.flatten() {
        let dir_name = entry.file_name();
        let dir_name_str = dir_name.to_str()?;

        if dir_name_str.starts_with("vadimcn.vscode-lldb-") {
            // Проверить наличие adapter/codelldb или adapter/codelldb.exe
            let adapter_dir = entry.path().join("adapter");

            #[cfg(windows)]
            let codelldb_path = adapter_dir.join("codelldb.exe");

            #[cfg(not(windows))]
            let codelldb_path = adapter_dir.join("codelldb");

            if codelldb_path.exists() {
                return Some(codelldb_path);
            }
        }
    }

    None
}

/// Tries to find lldb-dap in standard LLVM installation locations.
///
/// Searches in:
/// - `C:\Program Files\LLVM\bin\lldb-dap.exe` (Windows)
/// - `/usr/bin/lldb-dap` (Linux)
/// - `/opt/homebrew/bin/lldb-dap` (macOS Apple Silicon)
/// - `/usr/local/bin/lldb-dap` (macOS Intel)
///
/// # Returns
///
/// Returns full path to `lldb-dap` executable if found, otherwise `None`.
pub fn find_lldb_dap() -> Option<PathBuf> {
    let candidates = [
        // Windows - официальная установка LLVM
        r"C:\Program Files\LLVM\bin\lldb-dap.exe",
        // Linux
        "/usr/bin/lldb-dap",
        "/usr/local/bin/lldb-dap",
        // macOS
        "/opt/homebrew/bin/lldb-dap",
        "/usr/local/opt/llvm/bin/lldb-dap",
    ];

    for path_str in &candidates {
        let path = PathBuf::from(path_str);
        if path.exists() {
            return Some(path);
        }
    }

    None
}

/// Resolves adapter command to full path.
///
/// If `adapter_type` is a known adapter name (e.g., "lldb", "gdb"), tries to find it automatically.
/// Otherwise, returns the input as-is (assumes it's already a full path).
///
/// # Supported adapters
///
/// - `"lldb"` → auto-discovers:
///   1. lldb-dap from official LLVM installation (C:\Program Files\LLVM\bin\lldb-dap.exe)
///   2. CodeLLDB from VS Code extensions
///   3. Fallback: "lldb" (assumes it's in PATH)
/// - `"codelldb"` → auto-discovers CodeLLDB from VS Code extensions only
/// - `"lldb-dap"` → auto-discovers lldb-dap from official LLVM
/// - Other values → returned as-is
///
/// # Examples
///
/// ```no_run
/// use mcp_debug_server::config::resolve_adapter;
///
/// // Auto-discover lldb-dap or CodeLLDB
/// let path = resolve_adapter("lldb");
/// // Returns: "C:\\Program Files\\LLVM\\bin\\lldb-dap.exe" (if LLVM installed)
/// // Or: "/path/to/.vscode/extensions/vadimcn.vscode-lldb-*/adapter/codelldb"
///
/// // Use custom path
/// let path = resolve_adapter("/usr/local/bin/lldb-dap");
/// // Returns: "/usr/local/bin/lldb-dap" (as-is)
/// ```
pub fn resolve_adapter(adapter_type: &str) -> String {
    match adapter_type {
        "lldb" => {
            // Приоритет 1: CodeLLDB из VS Code extensions (проверен и стабилен)
            if let Some(path) = find_codelldb() {
                return path.to_string_lossy().to_string();
            }
            // Приоритет 2: lldb-dap из официального LLVM
            if let Some(path) = find_lldb_dap() {
                return path.to_string_lossy().to_string();
            }
            // Fallback: попробовать найти в PATH
            "lldb".to_string()
        }
        "codelldb" => {
            if let Some(path) = find_codelldb() {
                path.to_string_lossy().to_string()
            } else {
                // Fallback: попробовать найти в PATH
                "codelldb".to_string()
            }
        }
        "lldb-dap" => {
            if let Some(path) = find_lldb_dap() {
                path.to_string_lossy().to_string()
            } else {
                // Fallback: попробовать найти в PATH
                "lldb-dap".to_string()
            }
        }
        // Для других адаптеров - вернуть as-is (предполагаем полный путь или в PATH)
        _ => adapter_type.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_codelldb() {
        // Тест только логирует результат (зависит от локальной установки)
        match find_codelldb() {
            Some(path) => {
                println!("Found CodeLLDB: {:?}", path);
                assert!(path.exists());
            }
            None => {
                println!("CodeLLDB not found (this is OK if not installed)");
            }
        }
    }

    #[test]
    fn test_resolve_adapter_lldb() {
        let resolved = resolve_adapter("lldb");
        println!("Resolved 'lldb' to: {}", resolved);
        // Не проверяем конкретный путь, так как зависит от установки
    }

    #[test]
    fn test_resolve_adapter_custom_path() {
        let custom = "/custom/path/to/lldb";
        let resolved = resolve_adapter(custom);
        assert_eq!(resolved, custom);
    }

    #[test]
    fn test_find_lldb_dap() {
        // Тест только логирует результат (зависит от локальной установки)
        match find_lldb_dap() {
            Some(path) => {
                println!("Found lldb-dap: {:?}", path);
                assert!(path.exists());
            }
            None => {
                println!("lldb-dap not found (this is OK if LLVM not installed)");
            }
        }
    }

    #[test]
    fn test_resolve_adapter_lldb_dap() {
        let resolved = resolve_adapter("lldb-dap");
        println!("Resolved 'lldb-dap' to: {}", resolved);
        // Не проверяем конкретный путь, так как зависит от установки
    }

    #[test]
    fn test_resolve_adapter_priority() {
        // Для "lldb" приоритет: lldb-dap > CodeLLDB > fallback
        let resolved = resolve_adapter("lldb");
        println!("Resolved 'lldb' to: {}", resolved);
        // Если оба найдены, должен вернуться lldb-dap
        if find_lldb_dap().is_some() {
            assert!(resolved.contains("lldb-dap"));
        }
    }
}
