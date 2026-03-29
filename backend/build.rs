use chrono::Local;
use std::path::Path;
use std::process::Command;

fn rerun_if_changed(path: &str) {
    assert!(
        Path::new(path).exists(),
        "backend/build.rs rerun-if-changed target is missing: {path}"
    );
    println!("cargo:rerun-if-changed={}", path);
}

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

    // Обновляем build metadata при реальных изменениях backend source tree, но
    // не dirty-им пакет на каждом no-op build ссылкой на отсутствующий путь.
    rerun_if_changed("build.rs");
    rerun_if_changed("src");
    println!("cargo:rerun-if-env-changed=FORCE_REBUILD");
}
