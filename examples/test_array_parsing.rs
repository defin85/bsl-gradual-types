use std::path::PathBuf;

fn main() {
    // Путь к файлу Array.html
    let array_html_path =
        PathBuf::from("examples/syntax_helper/rebuilt.shcntx_ru/objects/catalog234/Array.html");

    println!("🔍 Проверяем файл: {}", array_html_path.display());

    if !array_html_path.exists() {
        println!("❌ Файл не существует!");
        return;
    }

    // Читаем HTML
    let html_content =
        std::fs::read_to_string(&array_html_path).expect("Failed to read Array.html");

    println!("✅ Файл прочитан, размер: {} байт", html_content.len());

    // Проверяем наличие секции "Методы:"
    if html_content.contains("Методы:") {
        println!("✅ Секция 'Методы:' найдена в HTML");
    } else {
        println!("❌ Секция 'Методы:' НЕ найдена в HTML");
    }

    // Проверяем наличие секции "Элементы коллекции:"
    if html_content.contains("Элементы коллекции:") {
        println!("✅ Секция 'Элементы коллекции:' найдена в HTML");
    } else {
        println!("❌ Секция 'Элементы коллекции:' НЕ найдена в HTML");
    }

    // Ищем упоминания методов
    let method_names = vec!["Добавить", "Вставить", "Найти", "Удалить", "Очистить"];
    for method in &method_names {
        if html_content.contains(method) {
            println!("  ✅ Метод '{}' упоминается в HTML", method);
        }
    }

    // Проверяем "Произвольный"
    if html_content.contains("Произвольный") {
        println!("✅ Тип элемента 'Произвольный' найден");
    }

    // Проверяем маркеры итерируемости/индексируемости
    if html_content.contains("Для каждого") {
        println!("✅ Маркер итерируемости 'Для каждого' найден");
    }
    if html_content.contains("оператора [") {
        println!("✅ Маркер индексируемости 'оператора [' найден");
    }
}
