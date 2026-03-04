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
