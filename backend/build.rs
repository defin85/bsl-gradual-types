use chrono::Local;

fn main() {
    // Генерируем timestamp сборки
    let build_time = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    println!("cargo:rustc-env=BUILD_TIMESTAMP={}", build_time);

    // ВАЖНО: Всегда перекомпилировать для свежего timestamp
    // Удаляем rerun-if-changed чтобы build.rs запускался КАЖДЫЙ раз
}
