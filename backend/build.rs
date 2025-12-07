use chrono::Local;
use std::process::Command;

fn main() {
    // Генерируем timestamp сборки
    let build_time = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    println!("cargo:rustc-env=BUILD_TIMESTAMP={}", build_time);

    // Получаем короткий git hash
    let git_hash = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());
    println!("cargo:rustc-env=GIT_HASH={}", git_hash);

    // ВАЖНО: Принудительная перекомпиляция при каждой сборке
    // Это гарантирует актуальный BUILD_TIMESTAMP
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/bin/lsp_server.rs");
    println!("cargo:rerun-if-env-changed=FORCE_REBUILD");
}
