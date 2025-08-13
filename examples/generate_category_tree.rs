//! Генерация интерактивной визуализации с иерархией категорий

use bsl_gradual_types::adapters::syntax_helper_parser::{
    SyntaxHelperParser, OptimizationSettings, SyntaxNode,
};
use std::path::Path;
use std::fs;
use anyhow::Result;
use std::collections::HashMap;

fn main() -> Result<()> {
    println!("🎨 Генерация визуализации с категориями...\n");
    
    let syntax_helper_path = Path::new("examples/syntax_helper/rebuilt.shcntx_ru/objects");
    
    // Создаём парсер
    let settings = OptimizationSettings {
        show_progress: true,
        ..Default::default()
    };
    let mut parser = SyntaxHelperParser::with_settings(settings);
    
    // Парсим директорию
    println!("📂 Парсинг: {}", syntax_helper_path.display());
    parser.parse_directory(syntax_helper_path)?;
    
    // Экспортируем данные
    let database = parser.export_database();
    
    // Генерируем HTML
    let html = generate_html_visualization(&database);
    
    // Сохраняем
    let output_file = "bsl_type_hierarchy.html";
    fs::write(output_file, html)?;
    
    println!("\n✅ Визуализация создана: {}", output_file);
    
    // Открываем в браузере на Windows
    #[cfg(windows)]
    {
        std::process::Command::new("cmd")
            .args(&["/C", "start", output_file])
            .spawn()
            .ok();
    }
    
    Ok(())
}

fn generate_html_visualization(database: &bsl_gradual_types::adapters::syntax_helper_parser::SyntaxHelperDatabase) -> String {
    // Группируем типы по категориям
    let mut category_types: HashMap<String, Vec<String>> = HashMap::new();
    let mut uncategorized = Vec::new();
    
    for (_, node) in &database.nodes {
        if let SyntaxNode::Type(type_info) = node {
            if type_info.identity.category_path.is_empty() {
                uncategorized.push(type_info.identity.russian_name.clone());
            } else {
                category_types.entry(type_info.identity.category_path.clone())
                    .or_default()
                    .push(type_info.identity.russian_name.clone());
            }
        }
    }
    
    // Сортируем категории и типы
    let mut sorted_categories: Vec<(String, Vec<String>)> = category_types.into_iter().collect();
    sorted_categories.sort_by(|a, b| a.0.cmp(&b.0));
    for (_, types) in &mut sorted_categories {
        types.sort();
    }
    uncategorized.sort();
    
    // Считаем статистику
    let total_types = database.nodes.values()
        .filter(|n| matches!(n, SyntaxNode::Type(_)))
        .count();
    let total_categories = database.categories.len();
    let total_methods = database.methods.len();
    let total_properties = database.properties.len();
    
    format!(r#"<!DOCTYPE html>
<html lang="ru">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>BSL Type System - Иерархия типов</title>
    <style>
        * {{
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }}
        
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            min-height: 100vh;
            padding: 20px;
        }}
        
        .container {{
            max-width: 1800px;
            margin: 0 auto;
        }}
        
        /* Шапка */
        .header {{
            background: white;
            border-radius: 20px;
            padding: 30px;
            margin-bottom: 20px;
            box-shadow: 0 20px 60px rgba(0,0,0,0.2);
        }}
        
        .header h1 {{
            font-size: 2.5em;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            -webkit-background-clip: text;
            -webkit-text-fill-color: transparent;
            margin-bottom: 10px;
        }}
        
        .header p {{
            color: #666;
            font-size: 1.1em;
        }}
        
        /* Статистика */
        .stats {{
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
            gap: 15px;
            margin-bottom: 20px;
        }}
        
        .stat-card {{
            background: white;
            border-radius: 15px;
            padding: 20px;
            text-align: center;
            box-shadow: 0 10px 30px rgba(0,0,0,0.1);
            transition: transform 0.3s;
        }}
        
        .stat-card:hover {{
            transform: translateY(-5px);
        }}
        
        .stat-value {{
            font-size: 2.5em;
            font-weight: bold;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            -webkit-background-clip: text;
            -webkit-text-fill-color: transparent;
        }}
        
        .stat-label {{
            color: #666;
            margin-top: 5px;
            font-size: 0.9em;
        }}
        
        /* Основной контент */
        .main-content {{
            display: grid;
            grid-template-columns: 400px 1fr;
            gap: 20px;
        }}
        
        /* Панель категорий */
        .categories-panel {{
            background: white;
            border-radius: 15px;
            padding: 20px;
            max-height: 80vh;
            overflow-y: auto;
            box-shadow: 0 10px 30px rgba(0,0,0,0.1);
        }}
        
        .categories-panel h2 {{
            color: #333;
            margin-bottom: 15px;
            font-size: 1.3em;
        }}
        
        /* Поиск */
        .search-box {{
            width: 100%;
            padding: 10px;
            border: 2px solid #e0e0e0;
            border-radius: 8px;
            font-size: 14px;
            margin-bottom: 15px;
        }}
        
        .search-box:focus {{
            outline: none;
            border-color: #667eea;
        }}
        
        /* Список категорий */
        .category-list {{
            max-height: calc(80vh - 100px);
            overflow-y: auto;
        }}
        
        .category-item {{
            padding: 10px;
            cursor: pointer;
            border-radius: 8px;
            margin-bottom: 5px;
            transition: all 0.2s;
            display: flex;
            justify-content: space-between;
            align-items: center;
        }}
        
        .category-item:hover {{
            background: #f5f5f5;
        }}
        
        .category-item.active {{
            background: linear-gradient(135deg, #667eea20 0%, #764ba220 100%);
            border-left: 3px solid #667eea;
        }}
        
        .category-name {{
            flex: 1;
            font-weight: 500;
        }}
        
        .category-count {{
            background: #e0e0e0;
            padding: 2px 8px;
            border-radius: 12px;
            font-size: 0.85em;
            color: #666;
        }}
        
        /* Панель типов */
        .types-panel {{
            background: white;
            border-radius: 15px;
            padding: 20px;
            box-shadow: 0 10px 30px rgba(0,0,0,0.1);
            max-height: 80vh;
            overflow-y: auto;
        }}
        
        .types-panel h2 {{
            color: #333;
            margin-bottom: 15px;
            font-size: 1.3em;
        }}
        
        .category-title {{
            font-size: 1.5em;
            color: #333;
            margin-bottom: 10px;
            padding-bottom: 10px;
            border-bottom: 2px solid #e0e0e0;
        }}
        
        .types-grid {{
            display: grid;
            grid-template-columns: repeat(auto-fill, minmax(250px, 1fr));
            gap: 10px;
            margin-top: 15px;
        }}
        
        .type-card {{
            background: #f9f9f9;
            padding: 12px;
            border-radius: 8px;
            border: 1px solid #e0e0e0;
            transition: all 0.2s;
            cursor: pointer;
        }}
        
        .type-card:hover {{
            background: #fff;
            border-color: #667eea;
            box-shadow: 0 4px 12px rgba(102, 126, 234, 0.2);
        }}
        
        .type-name {{
            font-weight: 500;
            color: #333;
            margin-bottom: 4px;
        }}
        
        .type-english {{
            font-size: 0.85em;
            color: #999;
        }}
        
        /* Выделение категорий по важности */
        .top-category {{
            background: linear-gradient(135deg, #ffd89b 0%, #19547b 100%);
            color: white;
        }}
        
        .top-category .category-count {{
            background: rgba(255,255,255,0.3);
            color: white;
        }}
        
        /* Прогресс-бар */
        .progress-bar {{
            height: 4px;
            background: #e0e0e0;
            border-radius: 2px;
            overflow: hidden;
            margin-top: 5px;
        }}
        
        .progress-fill {{
            height: 100%;
            background: linear-gradient(90deg, #667eea, #764ba2);
            transition: width 0.3s;
        }}
        
        /* Скроллбар стилизация */
        ::-webkit-scrollbar {{
            width: 8px;
        }}
        
        ::-webkit-scrollbar-track {{
            background: #f1f1f1;
            border-radius: 4px;
        }}
        
        ::-webkit-scrollbar-thumb {{
            background: #888;
            border-radius: 4px;
        }}
        
        ::-webkit-scrollbar-thumb:hover {{
            background: #555;
        }}
        
        /* Анимация появления */
        @keyframes fadeIn {{
            from {{ opacity: 0; transform: translateY(20px); }}
            to {{ opacity: 1; transform: translateY(0); }}
        }}
        
        .fade-in {{
            animation: fadeIn 0.5s ease-out;
        }}
    </style>
</head>
<body>
    <div class="container">
        <!-- Шапка -->
        <div class="header fade-in">
            <h1>🚀 BSL Type System</h1>
            <p>Интерактивная визуализация системы типов 1С:Предприятие</p>
        </div>
        
        <!-- Статистика -->
        <div class="stats fade-in">
            <div class="stat-card">
                <div class="stat-value">{}</div>
                <div class="stat-label">Категорий</div>
            </div>
            <div class="stat-card">
                <div class="stat-value">{}</div>
                <div class="stat-label">Типов</div>
            </div>
            <div class="stat-card">
                <div class="stat-value">{}</div>
                <div class="stat-label">Методов</div>
            </div>
            <div class="stat-card">
                <div class="stat-value">{}</div>
                <div class="stat-label">Свойств</div>
            </div>
        </div>
        
        <!-- Основной контент -->
        <div class="main-content fade-in">
            <!-- Панель категорий -->
            <div class="categories-panel">
                <h2>📚 Категории типов</h2>
                <input type="text" class="search-box" id="categorySearch" placeholder="Поиск категории...">
                <div class="category-list" id="categoryList">
                    {}</div>
            </div>
            
            <!-- Панель типов -->
            <div class="types-panel">
                <h2>📋 Типы в категории</h2>
                <div id="typesContent">
                    <p style="color: #999; text-align: center; margin-top: 50px;">
                        Выберите категорию для просмотра типов
                    </p>
                </div>
            </div>
        </div>
    </div>
    
    <script>
        // Данные категорий
        const categoriesData = {{
            {}
        }};
        
        // Текущая выбранная категория
        let currentCategory = null;
        
        // Инициализация
        document.addEventListener('DOMContentLoaded', function() {{
            setupCategorySearch();
            setupCategoryClicks();
        }});
        
        // Поиск категорий
        function setupCategorySearch() {{
            const searchBox = document.getElementById('categorySearch');
            searchBox.addEventListener('input', function(e) {{
                const searchTerm = e.target.value.toLowerCase();
                const categoryItems = document.querySelectorAll('.category-item');
                
                categoryItems.forEach(item => {{
                    const name = item.querySelector('.category-name').textContent.toLowerCase();
                    if (searchTerm === '' || name.includes(searchTerm)) {{
                        item.style.display = 'flex';
                    }} else {{
                        item.style.display = 'none';
                    }}
                }});
            }});
        }}
        
        // Клики по категориям
        function setupCategoryClicks() {{
            const categoryItems = document.querySelectorAll('.category-item');
            
            categoryItems.forEach(item => {{
                item.addEventListener('click', function() {{
                    // Убираем активный класс со всех
                    categoryItems.forEach(i => i.classList.remove('active'));
                    // Добавляем текущему
                    this.classList.add('active');
                    
                    // Показываем типы
                    const categoryName = this.dataset.category;
                    showTypes(categoryName);
                }});
            }});
        }}
        
        // Показать типы категории
        function showTypes(categoryName) {{
            const types = categoriesData[categoryName] || [];
            const typesContent = document.getElementById('typesContent');
            
            let html = `<div class="category-title">${{categoryName}}</div>`;
            html += `<p style="color: #666; margin-bottom: 10px;">Найдено типов: ${{types.length}}</p>`;
            
            if (types.length > 0) {{
                html += '<div class="types-grid">';
                types.forEach(type => {{
                    // Разделяем русское и английское название если есть
                    const parts = type.split(' (');
                    const russian = parts[0];
                    const english = parts[1] ? parts[1].replace(')', '') : '';
                    
                    html += `
                        <div class="type-card">
                            <div class="type-name">${{russian}}</div>
                            ${{english ? `<div class="type-english">${{english}}</div>` : ''}}
                        </div>
                    `;
                }});
                html += '</div>';
            }} else {{
                html += '<p style="color: #999; text-align: center; margin-top: 20px;">Типы не найдены</p>';
            }}
            
            typesContent.innerHTML = html;
            
            // Анимация появления
            typesContent.classList.remove('fade-in');
            void typesContent.offsetWidth; // Trigger reflow
            typesContent.classList.add('fade-in');
        }}
    </script>
</body>
</html>"#,
        total_categories,
        total_types,
        total_methods,
        total_properties,
        generate_category_list_html(&sorted_categories, &uncategorized),
        generate_categories_js_data(&sorted_categories, &uncategorized)
    )
}

fn generate_category_list_html(categories: &[(String, Vec<String>)], uncategorized: &[String]) -> String {
    let mut html = String::new();
    
    // Топ категории (с наибольшим количеством типов)
    let mut top_categories: Vec<(String, Vec<String>)> = categories.iter()
        .map(|(name, types)| (name.clone(), types.clone()))
        .collect();
    top_categories.sort_by(|a, b| b.1.len().cmp(&a.1.len()));
    
    // Находим топ-5 категорий по количеству типов
    let top_names: Vec<String> = top_categories.iter()
        .take(5)
        .filter(|(_, types)| types.len() > 50)
        .map(|(name, _)| name.clone())
        .collect();
    
    for (name, types) in categories.iter() {
        let is_top = top_names.contains(name);
        let class = if is_top { "category-item top-category" } else { "category-item" };
        
        html.push_str(&format!(
            r#"<div class="{}" data-category="{}">
                <span class="category-name">{}</span>
                <span class="category-count">{}</span>
            </div>"#,
            class,
            html_escape(name),
            html_escape(name),
            types.len()
        ));
    }
    
    // Добавляем некатегоризованные
    if !uncategorized.is_empty() {
        html.push_str(&format!(
            r#"<div class="category-item" data-category="Без категории">
                <span class="category-name">Без категории</span>
                <span class="category-count">{}</span>
            </div>"#,
            uncategorized.len()
        ));
    }
    
    html
}

fn generate_categories_js_data(categories: &[(String, Vec<String>)], uncategorized: &[String]) -> String {
    let mut js = String::new();
    
    for (name, types) in categories {
        js.push_str(&format!(
            "'{}': [{}],\n",
            js_escape(name),
            types.iter()
                .map(|t| format!("'{}'", js_escape(t)))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    
    if !uncategorized.is_empty() {
        js.push_str(&format!(
            "'Без категории': [{}],\n",
            uncategorized.iter()
                .map(|t| format!("'{}'", js_escape(t)))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    
    js
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn js_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('\'', "\\'")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}