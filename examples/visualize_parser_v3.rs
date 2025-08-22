//! Визуализация данных из парсера синтакс-помощника версии 3
//!
//! Создаёт интерактивный HTML отчёт с иерархией типов, индексами и фасетами

use anyhow::Result;
use bsl_gradual_types::adapters::syntax_helper_parser::{
    OptimizationSettings, SyntaxHelperParser, SyntaxNode, TypeInfo,
};
use bsl_gradual_types::core::types::FacetKind;
use indicatif::{ProgressBar, ProgressStyle};
use std::fs;
use std::path::Path;

fn main() -> Result<()> {
    println!("🎨 Визуализация данных парсера v3...\n");

    // Путь к распакованному синтакс-помощнику
    // Пробуем несколько возможных путей
    let syntax_helper_path = if Path::new("examples/syntax_helper/rebuilt.shcntx_ru").exists() {
        Path::new("examples/syntax_helper/rebuilt.shcntx_ru")
    } else if Path::new("examples/syntax_helper/rebuilt.shlang_ru").exists() {
        Path::new("examples/syntax_helper/rebuilt.shlang_ru")
    } else {
        Path::new("data/syntax_helper/extracted")
    };

    // Создаём парсер с настройками
    let settings = OptimizationSettings {
        show_progress: true,
        ..Default::default()
    };
    let mut parser = SyntaxHelperParser::with_settings(settings);

    // Парсим данные
    if syntax_helper_path.exists() {
        println!("📂 Парсинг из: {}", syntax_helper_path.display());
        parser.parse_directory(syntax_helper_path)?;
    } else {
        println!("⚠️  Путь не найден, создаём демо-данные...");
        create_demo_data(&mut parser)?;
    }

    // Собираем статистику
    let stats = collect_statistics(&parser);

    // Генерируем HTML визуализацию с прогресс-баром
    println!("\n📝 Генерация HTML отчёта...");
    let pb = ProgressBar::new(5); // 5 основных этапов
    pb.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} {msg}")?
            .progress_chars("##-"),
    );

    pb.set_message("Генерация заголовка и статистики");
    let mut html = String::new();
    // Начало HTML документа
    html.push_str(&generate_html_header());
    html.push_str(&generate_stats_html(&stats));
    pb.inc(1);

    pb.set_message("Генерация дерева типов");
    html.push_str(&generate_tree_html(&parser));
    pb.inc(1);

    pb.set_message("Генерация таблицы типов");
    html.push_str(&generate_types_table_html(&parser));
    pb.inc(1);

    pb.set_message("Генерация информации об индексах и фасетах");
    html.push_str(&generate_indices_and_facets_html(&parser, &stats));
    pb.inc(1);

    pb.set_message("Добавление JavaScript и завершение");
    html.push_str(&generate_html_footer(&parser));
    pb.inc(1);

    pb.finish_with_message("HTML отчёт сгенерирован");

    // Сохраняем файл
    let output_path = "type_hierarchy_v3_visualization.html";
    fs::write(output_path, html)?;

    println!("\n✅ Визуализация создана: {}", output_path);
    println!("\n📊 Статистика:");
    println!("   • Всего узлов: {}", stats.total_nodes);
    println!("   • Типов: {}", stats.types_count);
    println!("   • Категорий: {}", stats.categories_count);
    println!("   • Методов: {}", stats.methods_count);
    println!("   • Свойств: {}", stats.properties_count);
    println!("\n📑 Индексы:");
    println!("   • По русским именам: {}", stats.russian_index_size);
    println!("   • По английским именам: {}", stats.english_index_size);
    println!("   • По категориям: {}", stats.category_index_size);
    println!("   • По фасетам: {}", stats.facet_index_size);

    // Открываем в браузере
    #[cfg(windows)]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", output_path])
            .spawn()
            .ok();
    }

    Ok(())
}

struct Statistics {
    total_nodes: usize,
    types_count: usize,
    categories_count: usize,
    methods_count: usize,
    properties_count: usize,
    russian_index_size: usize,
    english_index_size: usize,
    category_index_size: usize,
    facet_index_size: usize,
}

fn collect_statistics(parser: &SyntaxHelperParser) -> Statistics {
    let mut stats = Statistics {
        total_nodes: 0,
        types_count: 0,
        categories_count: 0,
        methods_count: 0,
        properties_count: 0,
        russian_index_size: 0,
        english_index_size: 0,
        category_index_size: 0,
        facet_index_size: 0,
    };

    // Подсчёт узлов
    let database = parser.export_database();
    let mut global_functions_count = 0;
    for (_, node) in database.nodes.iter() {
        stats.total_nodes += 1;
        match node {
            SyntaxNode::Category(_) => stats.categories_count += 1,
            SyntaxNode::Type(_) => stats.types_count += 1,
            SyntaxNode::Method(_) => stats.methods_count += 1,
            SyntaxNode::Property(_) => stats.properties_count += 1,
            SyntaxNode::GlobalFunction(_) => global_functions_count += 1,
            _ => {}
        }
    }

    // Добавляем глобальные функции в методы для общей статистики
    println!("📊 Найдено глобальных функций: {}", global_functions_count);

    // Размеры индексов
    let index = parser.export_index();
    stats.russian_index_size = index.by_russian.len();
    stats.english_index_size = index.by_english.len();
    stats.category_index_size = index.by_category.len();
    stats.facet_index_size = index.by_facet.len();

    stats
}

fn generate_tree(parser: &SyntaxHelperParser) -> (String, String) {
    use bsl_gradual_types::adapters::syntax_helper_parser::CategoryInfo;
    let mut html = String::new();

    let database = parser.export_database();

    // Discovery-based: находим корневые категории динамически из данных парсера
    // Корневые категории - это те, что не содержат "/" в catalog_path или только "Global context"
    let mut root_categories: Vec<(&String, &CategoryInfo)> = Vec::new();
    let mut sub_categories: std::collections::HashMap<String, Vec<(&String, &CategoryInfo)>> =
        std::collections::HashMap::new();

    for (catalog_id, cat_info) in &database.categories {
        // Проверяем, является ли это корневой категорией
        // Корневая категория: catalog_id без "/" или "Global context"
        let is_root = !catalog_id.contains('/') || catalog_id == "Global context";

        if is_root {
            root_categories.push((catalog_id, cat_info));
        } else {
            // Это подкатегория - находим родителя
            // Пример: для "catalog234/catalog236" родитель - "catalog234"
            if let Some(slash_pos) = catalog_id.rfind('/') {
                let parent_id = &catalog_id[..slash_pos];
                // Берём последнюю часть после последнего слеша как ID родителя
                let parent_catalog = if parent_id.contains('/') {
                    parent_id.split('/').next_back().unwrap_or(parent_id)
                } else {
                    parent_id
                };
                sub_categories
                    .entry(parent_catalog.to_string())
                    .or_default()
                    .push((catalog_id, cat_info));
            }
        }
    }

    // Сортируем корневые категории по имени
    root_categories.sort_by(|a, b| a.1.name.cmp(&b.1.name));

    // Собираем типы по catalog_id категории, а не по имени!
    // Это важно, так как имена могут дублироваться
    let mut types_by_catalog_id: std::collections::HashMap<String, Vec<&TypeInfo>> =
        std::collections::HashMap::new();

    for (_, node) in database.nodes.iter() {
        if let SyntaxNode::Type(type_info) = node {
            // Определяем catalog_id из пути типа
            // Путь может быть:
            // - "objects/catalog234/Array.html" -> catalog234
            // - "objects/catalog234/catalog236/ValueTable.html" -> catalog234/catalog236
            // - "objects/catalog234/catalog236/ValueTableRow.html" -> catalog234/catalog236
            let catalog_id = if type_info.identity.catalog_path.contains("Global context") {
                "Global context".to_string()
            } else {
                let path = &type_info.identity.catalog_path;
                if let Some(objects_pos) = path.find("objects/") {
                    let after_objects = &path[objects_pos + 8..]; // Пропускаем "objects/"

                    // Подсчитываем количество catalog в пути
                    let catalog_count = after_objects.matches("catalog").count();

                    if catalog_count == 0 {
                        "unknown".to_string()
                    } else if catalog_count == 1 {
                        // Простой случай: objects/catalog234/Array.html
                        if let Some(slash_pos) = after_objects.find('/') {
                            after_objects[..slash_pos].to_string()
                        } else if let Some(dot_pos) = after_objects.find('.') {
                            after_objects[..dot_pos].to_string()
                        } else {
                            after_objects.to_string()
                        }
                    } else {
                        // Сложный случай: objects/catalog234/catalog236/ValueTable.html
                        // Нужно взять путь до последнего catalog включительно
                        let parts: Vec<&str> = after_objects.split('/').collect();
                        let mut catalog_parts = Vec::new();

                        for part in parts {
                            if part.starts_with("catalog") && !part.ends_with(".html") {
                                catalog_parts.push(part);
                            } else if part.starts_with("catalog") && part.ends_with(".html") {
                                // Это файл категории, не типа
                                let clean_part = part.trim_end_matches(".html");
                                catalog_parts.push(clean_part);
                                break;
                            } else if !catalog_parts.is_empty() {
                                // Мы нашли все catalog части
                                break;
                            }
                        }

                        catalog_parts.join("/")
                    }
                } else {
                    "unknown".to_string()
                }
            };

            types_by_catalog_id
                .entry(catalog_id)
                .or_default()
                .push(type_info);
        }
    }

    // Генерируем HTML для корневых категорий - БЕЗ ТИПОВ для упрощения
    for (cat_id, cat_info) in root_categories {
        // Считаем количество элементов в категории
        let type_count = types_by_catalog_id
            .get(cat_id)
            .map(|v| v.len())
            .unwrap_or(0);
        let subcat_count = sub_categories.get(cat_id).map(|v| v.len()).unwrap_or(0);

        // Корневая категория со счетчиком и data-атрибутами
        html.push_str(&format!(
            r#"<div class="tree-node root-category" data-category-id="{}" data-category-name="{}">
                <div class="tree-node-header">
                    <span class="icon icon-category">📁</span> {} 
                    <span style="color: #999; font-size: 0.9em;">({} подкатегорий, {} типов)</span>
                </div>
                <div class="tree-children">"#,
            cat_id, cat_info.name, cat_info.name, subcat_count, type_count
        ));

        // Добавляем только подкатегории БЕЗ их типов
        if let Some(subcats) = sub_categories.get(cat_id) {
            for (subcat_id, subcat_info) in subcats {
                // Для подкатегорий используем полный ID (например, catalog234/catalog236)
                let subcat_type_count = types_by_catalog_id
                    .get(subcat_id.as_str())
                    .map(|v| v.len())
                    .unwrap_or(0);

                html.push_str(&format!(
                    r#"
                    <div class="tree-node" data-category-id="{}" data-category-name="{}">
                        <div class="tree-node-header">
                            <span class="icon icon-category">📂</span> {} 
                            <span style="color: #999; font-size: 0.9em;">({} типов)</span>
                        </div>
                    </div>"#,
                    subcat_id, subcat_info.name, subcat_info.name, subcat_type_count
                ));
            }
        }

        // Закрываем корневую категорию
        html.push_str("\n                </div>\n            </div>\n");
    }

    // Добавляем типы без категории
    if let Some(uncategorized) = types_by_catalog_id.get("unknown") {
        if !uncategorized.is_empty() {
            html.push_str(
                r#"<div class="tree-node">
                <div class="tree-node-header">
                    <span class="icon icon-category">❓</span> Без категории
                </div>
                <div class="tree-children">"#,
            );

            for type_info in uncategorized {
                html.push_str(&format!(
                    r#"<div class="tree-node">
                        <div class="tree-node-header">
                            <span class="icon icon-type">📄</span> {} / {}
                        </div>
                    </div>"#,
                    type_info.identity.russian_name, type_info.identity.english_name
                ));
            }

            html.push_str("</div></div>");
        }
    }

    // Генерируем JSON с данными о типах для JavaScript
    let mut types_data_json = String::from("const categoryTypes = {\n");

    for (catalog_id, types) in &types_by_catalog_id {
        if !types.is_empty() {
            types_data_json.push_str(&format!("    \"{}\": [\n", catalog_id));
            for type_info in types {
                types_data_json.push_str(&format!(
                    "        {{russian: \"{}\", english: \"{}\", path: \"{}\"}},\n",
                    type_info.identity.russian_name.replace("\"", "\\\""),
                    type_info.identity.english_name.replace("\"", "\\\""),
                    type_info
                        .identity
                        .catalog_path
                        .replace("\\", "/")
                        .replace("\"", "\\\"")
                ));
            }
            types_data_json.push_str("    ],\n");
        }
    }

    types_data_json.push_str("};\n");

    (html, types_data_json)
}

fn generate_global_functions_table(parser: &SyntaxHelperParser) -> String {
    let mut html = String::new();
    let database = parser.export_database();

    // Собираем глобальные функции и группируем по категориям
    let mut categories: std::collections::HashMap<String, Vec<&SyntaxNode>> =
        std::collections::HashMap::new();
    let mut no_category = Vec::new();

    for node in database.nodes.values() {
        if let SyntaxNode::GlobalFunction(func) = node {
            match &func.category {
                Some(cat) => {
                    categories.entry(cat.clone()).or_default().push(node);
                }
                None => {
                    no_category.push(node);
                }
            }
        }
    }

    // Сортируем категории по количеству функций
    let mut sorted_categories: Vec<_> = categories.into_iter().collect();
    sorted_categories.sort_by(|a, b| b.1.len().cmp(&a.1.len()));

    html.push_str(
        r#"
    <style>
        .category-header {
            background-color: #f0f0f0;
            font-weight: bold;
            padding: 10px;
            cursor: pointer;
        }
        .category-header:hover {
            background-color: #e0e0e0;
        }
        .category-content {
            display: none;
        }
        .category-content.expanded {
            display: table-row-group;
        }
    </style>
    <table>
        <thead>
            <tr>
                <th>Имя функции</th>
                <th>Английское имя</th>
                <th>Полиморфная</th>
                <th>Чистая</th>
                <th>Параметры</th>
                <th>Контексты</th>
            </tr>
        </thead>
        <tbody>"#,
    );

    // Выводим функции по категориям
    for (idx, (category, functions)) in sorted_categories.iter().enumerate() {
        // Заголовок категории
        html.push_str(&format!(
            r#"
            <tr class="category-header" onclick="toggleCategory('cat-{}')">
                <td colspan="6">📁 {} ({} функций)</td>
            </tr>
            <tbody id="cat-{}" class="category-content {}">
        "#,
            idx,
            category,
            functions.len(),
            idx,
            if idx < 3 { "expanded" } else { "" }
        ));

        // Сортируем функции внутри категории
        let mut sorted_functions = functions.clone();
        sorted_functions.sort_by(|a, b| {
            if let (SyntaxNode::GlobalFunction(fa), SyntaxNode::GlobalFunction(fb)) = (a, b) {
                fa.name.cmp(&fb.name)
            } else {
                std::cmp::Ordering::Equal
            }
        });

        // Показываем первые 20 функций в категории
        for func_node in sorted_functions.iter().take(20) {
            if let SyntaxNode::GlobalFunction(func) = func_node {
                let english = func.english_name.as_deref().unwrap_or("-");
                let polymorphic = if func.polymorphic { "✅" } else { "❌" };
                let pure = if func.pure { "✅" } else { "❌" };
                let params_count = func.parameters.len();
                let contexts = if func.contexts.is_empty() {
                    "Все".to_string()
                } else {
                    format!("{} контекстов", func.contexts.len())
                };

                html.push_str(&format!(
                    r#"
                <tr>
                    <td><strong>{}</strong></td>
                    <td>{}</td>
                    <td style="text-align: center">{}</td>
                    <td style="text-align: center">{}</td>
                    <td>{}</td>
                    <td>{}</td>
                </tr>"#,
                    func.name, english, polymorphic, pure, params_count, contexts
                ));
            }
        }

        // Если в категории больше 20 функций, показываем сколько ещё
        if sorted_functions.len() > 20 {
            html.push_str(&format!(
                r#"
                <tr>
                    <td colspan="6" style="text-align: center; font-style: italic">
                        ... и ещё {} функций в этой категории
                    </td>
                </tr>"#,
                sorted_functions.len() - 20
            ));
        }

        html.push_str("</tbody>");
    }

    // Подсчитываем общую статистику
    let mut total = 0;
    let mut polymorphic_count = 0;
    let mut pure_count = 0;

    for (_, functions) in &sorted_categories {
        for func_node in functions {
            if let SyntaxNode::GlobalFunction(func) = func_node {
                total += 1;
                if func.polymorphic {
                    polymorphic_count += 1;
                }
                if func.pure {
                    pure_count += 1;
                }
            }
        }
    }

    // Добавляем функции без категории
    total += no_category.len();

    html.push_str(&format!(
        r#"
        </tbody>
    </table>
    
    <div style="margin-top: 20px;">
        <h4>Статистика:</h4>
        <ul>
            <li>Всего глобальных функций: <strong>{}</strong></li>
            <li>Категорий: <strong>{}</strong></li>
            <li>Полиморфных функций: <strong>{}</strong></li>
            <li>Чистых функций: <strong>{}</strong></li>
        </ul>
    </div>
    
    <script>
    function toggleCategory(id) {{
        var element = document.getElementById(id);
        if (element) {{
            element.classList.toggle('expanded');
        }}
    }}
    </script>"#,
        total,
        sorted_categories.len(),
        polymorphic_count,
        pure_count
    ));

    html
}

fn generate_facets_info(parser: &SyntaxHelperParser) -> String {
    let mut html = String::new();

    // Собираем статистику по фасетам
    let mut facet_stats: std::collections::HashMap<FacetKind, Vec<String>> =
        std::collections::HashMap::new();

    let database = parser.export_database();
    for (_, node) in database.nodes.iter() {
        if let SyntaxNode::Type(type_info) = node {
            for facet in &type_info.metadata.available_facets {
                facet_stats
                    .entry(*facet)
                    .or_default()
                    .push(type_info.identity.russian_name.clone());
            }
        }
    }

    // Генерируем HTML
    for (facet, types) in facet_stats.iter() {
        let (icon, name, description) = match facet {
            FacetKind::Collection => ("📚", "Collection", "Коллекции и итерируемые типы"),
            FacetKind::Manager => ("👔", "Manager", "Менеджеры объектов конфигурации"),
            FacetKind::Singleton => ("🔮", "Singleton", "Глобальные объекты"),
            FacetKind::Constructor => ("🏗️", "Constructor", "Конструируемые типы"),
            _ => ("📦", "Other", "Другие типы"),
        };

        html.push_str(&format!(
            r#"<div style="margin: 20px 0;">
                <h4>{} {} ({} типов)</h4>
                <p style="color: #666;">{}</p>
                <div style="margin-top: 10px;">
                    Примеры: {}</p>
                </div>
            </div>"#,
            icon,
            name,
            types.len(),
            description,
            types
                .iter()
                .take(3)
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    html
}

fn create_demo_data(parser: &mut SyntaxHelperParser) -> Result<()> {
    // Создаём временную структуру для демонстрации
    use tempfile::TempDir;

    let dir = TempDir::new()?;
    let base = dir.path();

    // Создаём демо-структуру
    let objects_dir = base.join("objects");
    fs::create_dir(&objects_dir)?;

    // Коллекции
    let collections_dir = objects_dir.join("catalog_collections");
    fs::create_dir(&collections_dir)?;

    fs::write(
        collections_dir.join("ValueTable.html"),
        r#"<html>
<h1 class="V8SH_pagetitle">ТаблицаЗначений (ValueTable)</h1>
<p>Объект для хранения табличных данных. Для объекта доступен обход коллекции посредством оператора Для каждого.</p>
</html>"#,
    )?;

    fs::write(
        collections_dir.join("Array.html"),
        r#"<html>
<h1 class="V8SH_pagetitle">Массив (Array)</h1>
<p>Упорядоченная коллекция значений. Для объекта доступен обход коллекции посредством оператора Для каждого.</p>
</html>"#,
    )?;

    fs::write(
        collections_dir.join("Map.html"),
        r#"<html>
<h1 class="V8SH_pagetitle">Соответствие (Map)</h1>
<p>Коллекция пар ключ-значение. Для объекта доступен обход коллекции посредством оператора Для каждого.</p>
</html>"#,
    )?;

    // Глобальные объекты
    let globals_dir = objects_dir.join("catalog_globals");
    fs::create_dir(&globals_dir)?;

    fs::write(
        globals_dir.join("XMLWriter.html"),
        r#"<html>
<h1 class="V8SH_pagetitle">ЗаписьXML (XMLWriter)</h1>
<p>Объект для записи XML документов.</p>
</html>"#,
    )?;

    // Парсим созданную структуру
    parser.parse_directory(base)?;

    Ok(())
}

// Новые функции для генерации HTML по частям с прогресс-баром

fn generate_html_header() -> String {
    r#"<!DOCTYPE html>
<html lang="ru">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>BSL Parser V3 - Визуализация</title>
    <style>
        * {
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }
        
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            min-height: 100vh;
            padding: 20px;
        }
        
        .container {
            max-width: 1600px;
            margin: 0 auto;
        }
        
        /* Шапка */
        .header {
            background: white;
            border-radius: 20px;
            padding: 40px;
            margin-bottom: 30px;
            box-shadow: 0 20px 60px rgba(0,0,0,0.2);
        }
        
        .header h1 {
            font-size: 2.5em;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            -webkit-background-clip: text;
            -webkit-text-fill-color: transparent;
            margin-bottom: 10px;
        }
        
        .header p {
            color: #666;
            font-size: 1.1em;
        }
        
        /* Статистика */
        .stats-grid {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
            gap: 20px;
            margin-bottom: 30px;
        }
        
        .stat-card {
            background: white;
            border-radius: 15px;
            padding: 25px;
            box-shadow: 0 10px 30px rgba(0,0,0,0.1);
            transition: transform 0.3s;
        }
        
        .stat-card:hover {
            transform: translateY(-5px);
        }
        
        .stat-value {
            font-size: 2.5em;
            font-weight: bold;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            -webkit-background-clip: text;
            -webkit-text-fill-color: transparent;
        }
        
        .stat-label {
            color: #666;
            margin-top: 5px;
        }
        
        /* Основной контент */
        .main-content {
            display: grid;
            grid-template-columns: minmax(400px, 1fr) minmax(600px, 2fr);
            gap: 30px;
        }
        
        .sidebar {
            background: white;
            border-radius: 15px;
            padding: 25px;
            box-shadow: 0 10px 30px rgba(0,0,0,0.1);
            max-height: calc(100vh - 100px);
            overflow-y: auto;
            min-width: 400px;
        }
        
        .content-area {
            background: white;
            border-radius: 15px;
            padding: 25px;
            box-shadow: 0 10px 30px rgba(0,0,0,0.1);
            overflow-x: auto;
        }
        
        /* Поиск */
        .search-box {
            width: 100%;
            padding: 12px;
            border: 2px solid #e0e0e0;
            border-radius: 8px;
            font-size: 14px;
            margin-bottom: 20px;
        }
        
        .search-box:focus {
            outline: none;
            border-color: #667eea;
        }
        
        /* Дерево типов */
        .tree-node {
            margin: 5px 0;
        }
        
        .tree-node-header {
            padding: 10px;
            cursor: pointer;
            border-radius: 6px;
            transition: all 0.2s;
        }
        
        .tree-node-header:hover {
            background: #f5f5f5;
        }
        
        .tree-node-header.selected {
            background: linear-gradient(135deg, #667eea20 0%, #764ba220 100%);
            border-left: 3px solid #667eea;
        }
        
        .tree-children {
            margin-left: 20px;
            display: none;  /* Скрываем по умолчанию */
        }
        
        .tree-children.expanded {
            display: block;  /* Показываем только если явно развернуто */
        }
        
        /* Иконки для типов */
        .icon {
            display: inline-block;
            margin-right: 8px;
        }
        
        .icon-category { color: #667eea; }
        .icon-type { color: #764ba2; }
        .icon-method { color: #28a745; }
        .icon-property { color: #17a2b8; }
        
        /* Вкладки */
        .tabs {
            display: flex;
            gap: 10px;
            margin-bottom: 20px;
            border-bottom: 2px solid #e0e0e0;
        }
        
        .tab {
            padding: 10px 20px;
            cursor: pointer;
            border-radius: 8px 8px 0 0;
            transition: all 0.2s;
        }
        
        .tab:hover {
            background: #f5f5f5;
        }
        
        .tab.active {
            background: linear-gradient(135deg, #667eea20 0%, #764ba220 100%);
            border-bottom: 3px solid #667eea;
        }
        
        .tab-content {
            display: none;
        }
        
        .tab-content.active {
            display: block;
        }
        
        /* Таблицы */
        table {
            width: 100%;
            border-collapse: collapse;
            table-layout: fixed;
        }
        
        th, td {
            padding: 12px;
            text-align: left;
            border-bottom: 1px solid #e0e0e0;
            overflow: hidden;
            text-overflow: ellipsis;
            white-space: nowrap;
        }
        
        /* Ширина колонок в таблице типов */
        th:nth-child(1), td:nth-child(1) { width: 25%; }  /* Русское имя */
        th:nth-child(2), td:nth-child(2) { width: 25%; }  /* Английское имя */
        th:nth-child(3), td:nth-child(3) { width: 30%; }  /* Категория */
        th:nth-child(4), td:nth-child(4) { width: 20%; }  /* Фасеты */
        
        th {
            background: #f5f5f5;
            font-weight: 600;
        }
        
        tr:hover {
            background: #f9f9f9;
        }
        
        /* Бейджи для фасетов */
        .facet-badge {
            display: inline-block;
            padding: 4px 8px;
            border-radius: 4px;
            font-size: 0.85em;
            margin: 2px;
        }
        
        .facet-collection { background: #e3f2fd; color: #1976d2; }
        .facet-manager { background: #f3e5f5; color: #7b1fa2; }
        .facet-singleton { background: #fff3e0; color: #f57c00; }
        .facet-constructor { background: #e8f5e9; color: #388e3c; }
    </style>
</head>
<body>
    <div class="container">
        <!-- Шапка -->
        <div class="header">
            <h1>🚀 BSL Parser V3 - Визуализация</h1>
            <p>Discovery-based парсер с двуязычной поддержкой и системой фасетов</p>
        </div>"#
        .to_string()
}

fn generate_stats_html(stats: &Statistics) -> String {
    format!(
        r#"
        <!-- Статистика -->
        <div class="stats-grid">
            <div class="stat-card">
                <div class="stat-value">{}</div>
                <div class="stat-label">Всего узлов</div>
            </div>
            <div class="stat-card">
                <div class="stat-value">{}</div>
                <div class="stat-label">Типов</div>
            </div>
            <div class="stat-card">
                <div class="stat-value">{}</div>
                <div class="stat-label">Категорий</div>
            </div>
            <div class="stat-card">
                <div class="stat-value">{}</div>
                <div class="stat-label">Русских имён</div>
            </div>
            <div class="stat-card">
                <div class="stat-value">{}</div>
                <div class="stat-label">Английских имён</div>
            </div>
            <div class="stat-card">
                <div class="stat-value">{}</div>
                <div class="stat-label">Фасетов</div>
            </div>
        </div>
        
        <!-- Основной контент -->
        <div class="main-content">
            <!-- Левая панель с деревом -->
            <div class="sidebar">
                <h2>📚 Иерархия типов</h2>
                <input type="text" class="search-box" placeholder="Поиск типов..." id="searchBox">
                <div id="tree">"#,
        stats.total_nodes,
        stats.types_count,
        stats.categories_count,
        stats.russian_index_size,
        stats.english_index_size,
        stats.facet_index_size
    )
}

fn generate_tree_html(parser: &SyntaxHelperParser) -> String {
    let mut html = String::new();

    // Генерируем дерево для левой панели и данные о типах
    let (tree_html, _types_data) = generate_tree(parser);
    html.push_str(&tree_html);

    // Закрываем div#tree и div.sidebar, открываем правую панель
    html.push_str(
        r#"
                </div>
            </div>
            
            <!-- Правая панель с деталями -->
            <div class="content-area">
                <h2>📋 Детальная информация</h2>
                
                <!-- Вкладки -->
                <div class="tabs">
                    <div class="tab active" data-tab="types">Типы</div>
                    <div class="tab" data-tab="indices">Индексы</div>
                    <div class="tab" data-tab="facets">Фасеты</div>
                    <div class="tab" data-tab="functions">Функции</div>
                </div>
                
                <!-- Содержимое вкладок -->
                <div class="tab-content active" id="types">
                    <h3>Список типов</h3>
                    <table style="display: none;">
                        <thead>
                            <tr>
                                <th>Русское имя</th>
                                <th>Английское имя</th>
                                <th>Категория</th>
                                <th>Фасеты</th>
                            </tr>
                        </thead>
                        <tbody>"#,
    );

    html
}

fn generate_types_table_html(_parser: &SyntaxHelperParser) -> String {
    // Вместо вывода всех типов показываем приглашение
    r#"
                        </tbody>
                    </table>
                    <div id="type-details" style="padding: 40px; text-align: center; color: #666;">
                        <p style="font-size: 1.2em;">👈 Выберите категорию или тип в дереве слева</p>
                        <p>Здесь будет отображаться детальная информация о выбранном элементе</p>
                    </div>
                </div>"#.to_string()
}

fn generate_indices_and_facets_html(parser: &SyntaxHelperParser, stats: &Statistics) -> String {
    let mut html = String::new();

    // Индексы
    html.push_str(&format!(
        r#"
                <div class="tab-content" id="indices">
                    <h3>Индексы для поиска</h3>
                    <p>Система индексов обеспечивает O(1) поиск типов по различным критериям:</p>
                    <ul>
                        <li>✅ По русским именам: <strong>{} записей</strong></li>
                        <li>✅ По английским именам: <strong>{} записей</strong></li>
                        <li>✅ По категориям: <strong>{} категорий</strong></li>
                        <li>✅ По фасетам: <strong>{} типов фасетов</strong></li>
                    </ul>
                </div>
                
                <div class="tab-content" id="facets">
                    <h3>Система фасетов</h3>
                    <p>Фасеты определяют различные представления одного типа:</p>"#,
        stats.russian_index_size,
        stats.english_index_size,
        stats.category_index_size,
        stats.facet_index_size
    ));

    // Используем существующую функцию generate_facets_info
    html.push_str(&generate_facets_info(parser));

    html.push_str(
        r#"
                </div>
                
                <div class="tab-content" id="functions">
                    <h3>🔧 Глобальные функции</h3>
                    <p>Встроенные функции BSL, доступные глобально:</p>"#,
    );

    // Генерируем таблицу глобальных функций
    html.push_str(&generate_global_functions_table(parser));

    html.push_str(
        r#"
                </div>
            </div>
        </div>
    </div>"#,
    );

    html
}

fn generate_html_footer(parser: &SyntaxHelperParser) -> String {
    // Генерируем данные о типах
    let (_, types_data) = generate_tree(parser);

    let script_content = format!(
        r#"
    <script>
        // Данные о типах по категориям
        {}
        
        // Поиск
        document.getElementById('searchBox').addEventListener('input', function(e) {{
            const searchTerm = e.target.value.toLowerCase();
            const nodes = document.querySelectorAll('.tree-node');
            
            nodes.forEach(node => {{
                const text = node.textContent.toLowerCase();
                if (searchTerm === '' || text.includes(searchTerm)) {{
                    node.style.display = 'block';
                }} else {{
                    node.style.display = 'none';
                }}
            }});
        }});
        
        // Вкладки
        document.querySelectorAll('.tab').forEach(tab => {{
            tab.addEventListener('click', function() {{
                // Убираем активный класс со всех вкладок
                document.querySelectorAll('.tab').forEach(t => t.classList.remove('active'));
                document.querySelectorAll('.tab-content').forEach(c => c.classList.remove('active'));
                
                // Добавляем активный класс текущей вкладке
                this.classList.add('active');
                const tabId = this.getAttribute('data-tab');
                document.getElementById(tabId).classList.add('active');
            }});
        }});
        
        // Раскрытие/скрытие узлов дерева и показ деталей
        document.querySelectorAll('.tree-node-header').forEach(header => {{
            header.addEventListener('click', function() {{
                const children = this.nextElementSibling;
                if (children && children.classList.contains('tree-children')) {{
                    children.classList.toggle('expanded');
                }}
                
                // Выделение выбранного узла
                document.querySelectorAll('.tree-node-header').forEach(h => h.classList.remove('selected'));
                this.classList.add('selected');
                
                // Показываем информацию о выбранном элементе
                const node = this.parentElement;
                const categoryId = node.getAttribute('data-category-id');
                const categoryName = node.getAttribute('data-category-name');
                
                if (categoryId) {{
                    showCategoryDetails(categoryId, categoryName);
                }}
            }});
        }});
        
        // Функция для показа деталей категории
        function showCategoryDetails(categoryId, categoryName) {{
            const detailsDiv = document.getElementById('type-details');
            if (!detailsDiv) return;
            
            // Получаем типы для этой категории
            const types = categoryTypes[categoryId] || [];
            
            // Показываем информацию о категории и её типы
            let html = `
                <div style="text-align: left;">
                    <h3>📁 ${{categoryName}}</h3>
                    <p><strong>ID категории:</strong> ${{categoryId}}</p>
                    <p><strong>Количество типов:</strong> ${{types.length}}</p>
            `;
            
            if (types.length > 0) {{
                html += `
                    <h4 style="margin-top: 20px;">Типы в категории:</h4>
                    <table style="width: 100%; margin-top: 10px;">
                        <thead>
                            <tr>
                                <th style="text-align: left; padding: 8px; border-bottom: 2px solid #e0e0e0;">Русское имя</th>
                                <th style="text-align: left; padding: 8px; border-bottom: 2px solid #e0e0e0;">Английское имя</th>
                            </tr>
                        </thead>
                        <tbody>
                `;
                
                for (const type of types) {{
                    html += `
                        <tr>
                            <td style="padding: 8px; border-bottom: 1px solid #e0e0e0;">${{type.russian}}</td>
                            <td style="padding: 8px; border-bottom: 1px solid #e0e0e0;">${{type.english}}</td>
                        </tr>
                    `;
                }}
                
                html += `
                        </tbody>
                    </table>
                `;
            }} else {{
                html += `
                    <p style="margin-top: 20px; color: #666;">
                        В этой категории нет типов верхнего уровня. Проверьте подкатегории.
                    </p>
                `;
            }}
            
            html += `</div>`;
            detailsDiv.innerHTML = html;
        }}
    </script>
</body>
</html>"#,
        types_data
    );

    script_content
}
