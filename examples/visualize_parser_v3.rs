//! Визуализация данных из парсера синтакс-помощника версии 3
//! 
//! Создаёт интерактивный HTML отчёт с иерархией типов, индексами и фасетами

use bsl_gradual_types::adapters::syntax_helper_parser::{
    SyntaxHelperParser, SyntaxNode, TypeInfo, OptimizationSettings,
};
use bsl_gradual_types::core::types::FacetKind;
use std::fs;
use std::path::Path;
use anyhow::Result;

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
    
    // Генерируем HTML визуализацию
    let html = generate_visualization(&parser, &stats);
    
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
            .args(&["/C", "start", output_path])
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
    for (_, node) in database.nodes.iter() {
        stats.total_nodes += 1;
        match node {
            SyntaxNode::Category(_) => stats.categories_count += 1,
            SyntaxNode::Type(_) => stats.types_count += 1,
            SyntaxNode::Method(_) => stats.methods_count += 1,
            SyntaxNode::Property(_) => stats.properties_count += 1,
            _ => {}
        }
    }
    
    // Размеры индексов
    let index = parser.export_index();
    stats.russian_index_size = index.by_russian.len();
    stats.english_index_size = index.by_english.len();
    stats.category_index_size = index.by_category.len();
    stats.facet_index_size = index.by_facet.len();
    
    stats
}

fn generate_visualization(parser: &SyntaxHelperParser, stats: &Statistics) -> String {
    let mut html = String::from(r#"<!DOCTYPE html>
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
            grid-template-columns: 1fr 2fr;
            gap: 30px;
        }
        
        .sidebar {
            background: white;
            border-radius: 15px;
            padding: 25px;
            box-shadow: 0 10px 30px rgba(0,0,0,0.1);
            max-height: 800px;
            overflow-y: auto;
        }
        
        .content-area {
            background: white;
            border-radius: 15px;
            padding: 25px;
            box-shadow: 0 10px 30px rgba(0,0,0,0.1);
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
            display: none;
        }
        
        .tree-children.expanded {
            display: block;
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
        }
        
        th, td {
            padding: 12px;
            text-align: left;
            border-bottom: 1px solid #e0e0e0;
        }
        
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
        </div>
        
        <!-- Статистика -->
        <div class="stats-grid">
            <div class="stat-card">
                <div class="stat-value">"#);
    
    html.push_str(&stats.total_nodes.to_string());
    html.push_str(r#"</div>
                <div class="stat-label">Всего узлов</div>
            </div>
            <div class="stat-card">
                <div class="stat-value">"#);
    
    html.push_str(&stats.types_count.to_string());
    html.push_str(r#"</div>
                <div class="stat-label">Типов</div>
            </div>
            <div class="stat-card">
                <div class="stat-value">"#);
    
    html.push_str(&stats.categories_count.to_string());
    html.push_str(r#"</div>
                <div class="stat-label">Категорий</div>
            </div>
            <div class="stat-card">
                <div class="stat-value">"#);
    
    html.push_str(&stats.russian_index_size.to_string());
    html.push_str(r#"</div>
                <div class="stat-label">Русских имён</div>
            </div>
            <div class="stat-card">
                <div class="stat-value">"#);
    
    html.push_str(&stats.english_index_size.to_string());
    html.push_str(r#"</div>
                <div class="stat-label">Английских имён</div>
            </div>
            <div class="stat-card">
                <div class="stat-value">"#);
    
    html.push_str(&stats.facet_index_size.to_string());
    html.push_str(r#"</div>
                <div class="stat-label">Фасетов</div>
            </div>
        </div>
        
        <!-- Основной контент -->
        <div class="main-content">
            <!-- Левая панель с деревом -->
            <div class="sidebar">
                <h2>📚 Иерархия типов</h2>
                <input type="text" class="search-box" placeholder="Поиск типов..." id="searchBox">
                <div id="tree">"#);
    
    // Генерируем дерево типов
    html.push_str(&generate_tree(parser));
    
    html.push_str(r#"</div>
            </div>
            
            <!-- Правая панель с деталями -->
            <div class="content-area">
                <h2>📋 Детальная информация</h2>
                
                <!-- Вкладки -->
                <div class="tabs">
                    <div class="tab active" data-tab="types">Типы</div>
                    <div class="tab" data-tab="indices">Индексы</div>
                    <div class="tab" data-tab="facets">Фасеты</div>
                </div>
                
                <!-- Содержимое вкладок -->
                <div class="tab-content active" id="types">
                    <h3>Список типов</h3>
                    <table>
                        <thead>
                            <tr>
                                <th>Русское имя</th>
                                <th>Английское имя</th>
                                <th>Категория</th>
                                <th>Фасеты</th>
                            </tr>
                        </thead>
                        <tbody>"#);
    
    // Генерируем таблицу типов
    html.push_str(&generate_types_table(parser));
    
    html.push_str(r#"</tbody>
                    </table>
                </div>
                
                <div class="tab-content" id="indices">
                    <h3>Индексы для поиска</h3>
                    <p>Система индексов обеспечивает O(1) поиск типов по различным критериям:</p>
                    <ul>
                        <li>✅ По русским именам: <strong>"#);
    html.push_str(&stats.russian_index_size.to_string());
    html.push_str(r#" записей</strong></li>
                        <li>✅ По английским именам: <strong>"#);
    html.push_str(&stats.english_index_size.to_string());
    html.push_str(r#" записей</strong></li>
                        <li>✅ По категориям: <strong>"#);
    html.push_str(&stats.category_index_size.to_string());
    html.push_str(r#" категорий</strong></li>
                        <li>✅ По фасетам: <strong>"#);
    html.push_str(&stats.facet_index_size.to_string());
    html.push_str(r#" типов фасетов</strong></li>
                    </ul>
                </div>
                
                <div class="tab-content" id="facets">
                    <h3>Система фасетов</h3>
                    <p>Фасеты определяют различные представления одного типа:</p>"#);
    
    // Генерируем информацию о фасетах
    html.push_str(&generate_facets_info(parser));
    
    html.push_str(r#"
                </div>
            </div>
        </div>
    </div>
    
    <script>
        // Поиск
        document.getElementById('searchBox').addEventListener('input', function(e) {
            const searchTerm = e.target.value.toLowerCase();
            const nodes = document.querySelectorAll('.tree-node');
            
            nodes.forEach(node => {
                const text = node.textContent.toLowerCase();
                if (searchTerm === '' || text.includes(searchTerm)) {
                    node.style.display = 'block';
                } else {
                    node.style.display = 'none';
                }
            });
        });
        
        // Вкладки
        document.querySelectorAll('.tab').forEach(tab => {
            tab.addEventListener('click', function() {
                // Убираем активный класс со всех вкладок
                document.querySelectorAll('.tab').forEach(t => t.classList.remove('active'));
                document.querySelectorAll('.tab-content').forEach(c => c.classList.remove('active'));
                
                // Добавляем активный класс текущей вкладке
                this.classList.add('active');
                const tabId = this.getAttribute('data-tab');
                document.getElementById(tabId).classList.add('active');
            });
        });
        
        // Раскрытие/скрытие узлов дерева
        document.querySelectorAll('.tree-node-header').forEach(header => {
            header.addEventListener('click', function() {
                const children = this.nextElementSibling;
                if (children && children.classList.contains('tree-children')) {
                    children.classList.toggle('expanded');
                }
                
                // Выделение выбранного узла
                document.querySelectorAll('.tree-node-header').forEach(h => h.classList.remove('selected'));
                this.classList.add('selected');
            });
        });
    </script>
</body>
</html>"#);
    
    html
}

fn generate_tree(parser: &SyntaxHelperParser) -> String {
    let mut html = String::new();
    
    // Группируем типы по категориям
    let mut types_by_category: std::collections::HashMap<String, Vec<&TypeInfo>> = std::collections::HashMap::new();
    
    let database = parser.export_database();
    for (_, node) in database.nodes.iter() {
        if let SyntaxNode::Type(type_info) = node {
            let category = if type_info.identity.category_path.is_empty() {
                "Без категории".to_string()
            } else {
                type_info.identity.category_path.clone()
            };
            types_by_category.entry(category).or_default().push(type_info);
        }
    }
    
    // Генерируем HTML для каждой категории
    for (category, types) in types_by_category.iter() {
        html.push_str(&format!(
            r#"<div class="tree-node">
                <div class="tree-node-header">
                    <span class="icon icon-category">📁</span> {}
                </div>
                <div class="tree-children">"#,
            category
        ));
        
        for type_info in types {
            html.push_str(&format!(
                r#"<div class="tree-node">
                    <div class="tree-node-header">
                        <span class="icon icon-type">📄</span> {} / {}
                    </div>
                </div>"#,
                type_info.identity.russian_name,
                type_info.identity.english_name
            ));
        }
        
        html.push_str("</div></div>");
    }
    
    html
}

fn generate_types_table(parser: &SyntaxHelperParser) -> String {
    let mut html = String::new();
    
    let database = parser.export_database();
    for (_, node) in database.nodes.iter() {
        if let SyntaxNode::Type(type_info) = node {
            html.push_str(&format!(
                r#"<tr>
                    <td>{}</td>
                    <td>{}</td>
                    <td>{}</td>
                    <td>"#,
                type_info.identity.russian_name,
                type_info.identity.english_name,
                type_info.identity.category_path
            ));
            
            // Добавляем бейджи фасетов
            for facet in &type_info.metadata.available_facets {
                let (class, name) = match facet {
                    FacetKind::Collection => ("facet-collection", "Collection"),
                    FacetKind::Manager => ("facet-manager", "Manager"),
                    FacetKind::Singleton => ("facet-singleton", "Singleton"),
                    FacetKind::Constructor => ("facet-constructor", "Constructor"),
                    _ => ("", "Other"),
                };
                html.push_str(&format!(
                    r#"<span class="facet-badge {}">{}</span>"#,
                    class, name
                ));
            }
            
            html.push_str("</td></tr>");
        }
    }
    
    html
}

fn generate_facets_info(parser: &SyntaxHelperParser) -> String {
    let mut html = String::new();
    
    // Собираем статистику по фасетам
    let mut facet_stats: std::collections::HashMap<FacetKind, Vec<String>> = std::collections::HashMap::new();
    
    let database = parser.export_database();
    for (_, node) in database.nodes.iter() {
        if let SyntaxNode::Type(type_info) = node {
            for facet in &type_info.metadata.available_facets {
                facet_stats.entry(*facet).or_default().push(type_info.identity.russian_name.clone());
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
            icon, name, types.len(), description,
            types.iter().take(3).map(|s| s.as_str()).collect::<Vec<_>>().join(", ")
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
</html>"#
    )?;
    
    fs::write(
        collections_dir.join("Array.html"),
        r#"<html>
<h1 class="V8SH_pagetitle">Массив (Array)</h1>
<p>Упорядоченная коллекция значений. Для объекта доступен обход коллекции посредством оператора Для каждого.</p>
</html>"#
    )?;
    
    fs::write(
        collections_dir.join("Map.html"),
        r#"<html>
<h1 class="V8SH_pagetitle">Соответствие (Map)</h1>
<p>Коллекция пар ключ-значение. Для объекта доступен обход коллекции посредством оператора Для каждого.</p>
</html>"#
    )?;
    
    // Глобальные объекты
    let globals_dir = objects_dir.join("catalog_globals");
    fs::create_dir(&globals_dir)?;
    
    fs::write(
        globals_dir.join("XMLWriter.html"),
        r#"<html>
<h1 class="V8SH_pagetitle">ЗаписьXML (XMLWriter)</h1>
<p>Объект для записи XML документов.</p>
</html>"#
    )?;
    
    // Парсим созданную структуру
    parser.parse_directory(base)?;
    
    Ok(())
}