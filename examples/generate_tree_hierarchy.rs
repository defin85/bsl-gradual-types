//! Генератор HTML с древовидной иерархией как в синтакс-помощнике
//! 
//! Создаёт интерактивное дерево типов и функций

use bsl_gradual_types::adapters::syntax_helper_parser::{SyntaxHelperParser, SyntaxHelperDatabase, KeywordCategory};
use std::fs;
use std::collections::BTreeMap;

fn main() -> anyhow::Result<()> {
    println!("🌳 Генерация древовидной иерархии типов...");
    
    // Загружаем данные
    let json_path = "examples/syntax_helper/syntax_database.json";
    let database = if std::path::Path::new(json_path).exists() {
        SyntaxHelperParser::load_from_file(json_path)?
    } else {
        println!("❌ База данных не найдена. Запустите: cargo run --example syntax_helper_parser_demo");
        return Ok(());
    };
    
    // Группируем функции по категориям (первая буква для примера)
    let mut functions_by_category: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
    
    for (name, func) in &database.global_functions {
        let category = match name.chars().next() {
            Some('А'..='Я') => name.chars().next().unwrap().to_string(),
            Some('A'..='Z') => name.chars().next().unwrap().to_string(),
            _ => "Другие".to_string(),
        };
        
        let english = func.english_name.as_deref().unwrap_or("");
        functions_by_category.entry(category).or_default().push((name.clone(), english.to_string()));
    }
    
    // Генерируем HTML с древовидной структурой
    let html = format!(r#"<!DOCTYPE html>
<html lang="ru">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>BSL Type Hierarchy - Древовидная структура</title>
    <style>
        * {{
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }}
        
        body {{
            font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
            background: #f5f5f5;
            display: flex;
            height: 100vh;
        }}
        
        /* Левая панель с деревом */
        .sidebar {{
            width: 400px;
            background: white;
            border-right: 1px solid #e0e0e0;
            overflow-y: auto;
            overflow-x: hidden;
        }}
        
        .sidebar-header {{
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: white;
            padding: 20px;
            position: sticky;
            top: 0;
            z-index: 10;
        }}
        
        .sidebar-header h1 {{
            font-size: 1.5em;
            margin-bottom: 10px;
        }}
        
        .search-box {{
            width: 100%;
            padding: 10px;
            border: none;
            border-radius: 5px;
            font-size: 14px;
            margin-top: 10px;
        }}
        
        /* Древовидная структура */
        .tree {{
            padding: 10px;
        }}
        
        .tree-node {{
            user-select: none;
        }}
        
        .tree-node-header {{
            display: flex;
            align-items: center;
            padding: 8px 5px;
            cursor: pointer;
            border-radius: 4px;
            transition: all 0.2s;
        }}
        
        .tree-node-header:hover {{
            background: #f0f0f0;
        }}
        
        .tree-node-header.selected {{
            background: #e3f2fd;
            color: #1976d2;
        }}
        
        .tree-icon {{
            width: 20px;
            height: 20px;
            margin-right: 8px;
            display: inline-flex;
            align-items: center;
            justify-content: center;
            transition: transform 0.2s;
        }}
        
        .tree-icon.collapsed {{
            transform: rotate(-90deg);
        }}
        
        .tree-label {{
            flex: 1;
            font-size: 14px;
        }}
        
        .tree-count {{
            color: #999;
            font-size: 12px;
            margin-left: 10px;
        }}
        
        .tree-children {{
            margin-left: 20px;
            overflow: hidden;
            transition: max-height 0.3s ease-out;
        }}
        
        .tree-children.collapsed {{
            max-height: 0;
        }}
        
        .tree-leaf {{
            padding: 6px 5px 6px 28px;
            cursor: pointer;
            border-radius: 4px;
            transition: all 0.2s;
            font-size: 13px;
            display: flex;
            align-items: center;
        }}
        
        .tree-leaf:hover {{
            background: #f0f0f0;
        }}
        
        .tree-leaf.selected {{
            background: #e3f2fd;
            color: #1976d2;
        }}
        
        .tree-leaf-icon {{
            width: 16px;
            height: 16px;
            margin-right: 8px;
        }}
        
        .tree-leaf-name {{
            flex: 1;
        }}
        
        .tree-leaf-english {{
            color: #999;
            font-size: 11px;
            margin-left: 10px;
        }}
        
        /* Категории иконки */
        .icon-folder {{ color: #fbc02d; }}
        .icon-function {{ color: #4caf50; }}
        .icon-keyword {{ color: #2196f3; }}
        .icon-object {{ color: #ff9800; }}
        .icon-enum {{ color: #9c27b0; }}
        .icon-method {{ color: #00bcd4; }}
        .icon-property {{ color: #ff5722; }}
        
        /* Правая панель с деталями */
        .content {{
            flex: 1;
            padding: 20px;
            overflow-y: auto;
            background: white;
        }}
        
        .content-header {{
            border-bottom: 2px solid #e0e0e0;
            padding-bottom: 20px;
            margin-bottom: 20px;
        }}
        
        .content-title {{
            font-size: 2em;
            color: #333;
            margin-bottom: 10px;
        }}
        
        .content-subtitle {{
            color: #666;
            font-size: 1.1em;
        }}
        
        .detail-section {{
            margin-bottom: 30px;
        }}
        
        .detail-section h3 {{
            color: #333;
            margin-bottom: 15px;
            font-size: 1.3em;
        }}
        
        .code-block {{
            background: #f5f5f5;
            border: 1px solid #e0e0e0;
            border-radius: 5px;
            padding: 15px;
            font-family: 'Consolas', 'Monaco', monospace;
            font-size: 13px;
            overflow-x: auto;
        }}
        
        .parameter-list {{
            list-style: none;
            padding: 0;
        }}
        
        .parameter-item {{
            padding: 10px;
            border-bottom: 1px solid #f0f0f0;
            display: flex;
            align-items: center;
        }}
        
        .parameter-name {{
            font-weight: bold;
            color: #333;
            min-width: 150px;
        }}
        
        .parameter-type {{
            color: #4caf50;
            margin-left: 10px;
        }}
        
        .parameter-optional {{
            color: #999;
            font-size: 12px;
            margin-left: 10px;
        }}
        
        /* Tabs */
        .tabs {{
            display: flex;
            border-bottom: 2px solid #e0e0e0;
            margin-bottom: 20px;
        }}
        
        .tab {{
            padding: 10px 20px;
            cursor: pointer;
            border-bottom: 3px solid transparent;
            transition: all 0.3s;
            color: #666;
        }}
        
        .tab:hover {{
            color: #333;
        }}
        
        .tab.active {{
            color: #1976d2;
            border-bottom-color: #1976d2;
        }}
        
        .tab-content {{
            display: none;
        }}
        
        .tab-content.active {{
            display: block;
        }}
        
        /* Welcome screen */
        .welcome {{
            text-align: center;
            padding: 50px;
            color: #999;
        }}
        
        .welcome h2 {{
            font-size: 2em;
            margin-bottom: 20px;
            color: #666;
        }}
        
        .welcome p {{
            font-size: 1.2em;
            line-height: 1.6;
        }}
        
        /* Statistics */
        .stats-grid {{
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
            gap: 20px;
            margin-top: 30px;
        }}
        
        .stat-card {{
            background: #f5f5f5;
            padding: 20px;
            border-radius: 8px;
            text-align: center;
        }}
        
        .stat-value {{
            font-size: 2em;
            font-weight: bold;
            color: #1976d2;
        }}
        
        .stat-label {{
            color: #666;
            margin-top: 5px;
        }}
    </style>
</head>
<body>
    <!-- Левая панель с деревом -->
    <div class="sidebar">
        <div class="sidebar-header">
            <h1>📚 BSL Type System</h1>
            <div>Иерархия типов и функций</div>
            <input type="text" class="search-box" placeholder="🔍 Поиск..." onkeyup="filterTree(this.value)">
        </div>
        
        <div class="tree" id="tree">
            <!-- Глобальные функции -->
            <div class="tree-node">
                <div class="tree-node-header" onclick="toggleNode(this)">
                    <span class="tree-icon">▼</span>
                    <span class="tree-label">📦 Глобальные функции</span>
                    <span class="tree-count">{functions_count}</span>
                </div>
                <div class="tree-children" id="functions-tree">
                    {functions_tree}
                </div>
            </div>
            
            <!-- Ключевые слова -->
            <div class="tree-node">
                <div class="tree-node-header" onclick="toggleNode(this)">
                    <span class="tree-icon">▼</span>
                    <span class="tree-label">🔤 Ключевые слова</span>
                    <span class="tree-count">{keywords_count}</span>
                </div>
                <div class="tree-children" id="keywords-tree">
                    {keywords_tree}
                </div>
            </div>
            
            <!-- Глобальные объекты -->
            <div class="tree-node">
                <div class="tree-node-header" onclick="toggleNode(this)">
                    <span class="tree-icon">▼</span>
                    <span class="tree-label">🏢 Глобальные объекты</span>
                    <span class="tree-count">{objects_count}</span>
                </div>
                <div class="tree-children" id="objects-tree">
                    <div class="tree-node">
                        <div class="tree-node-header" onclick="toggleNode(this)">
                            <span class="tree-icon">▼</span>
                            <span class="tree-label">Справочники</span>
                        </div>
                        <div class="tree-children">
                            <div class="tree-leaf" onclick="selectItem(this, 'catalog', 'Контрагенты')">
                                <span class="tree-leaf-name">Контрагенты</span>
                                <span class="tree-leaf-english">Counterparties</span>
                            </div>
                            <div class="tree-leaf" onclick="selectItem(this, 'catalog', 'Номенклатура')">
                                <span class="tree-leaf-name">Номенклатура</span>
                                <span class="tree-leaf-english">Products</span>
                            </div>
                        </div>
                    </div>
                    <div class="tree-node">
                        <div class="tree-node-header" onclick="toggleNode(this)">
                            <span class="tree-icon">▼</span>
                            <span class="tree-label">Документы</span>
                        </div>
                        <div class="tree-children">
                            <div class="tree-leaf" onclick="selectItem(this, 'document', 'ПоступлениеТоваров')">
                                <span class="tree-leaf-name">ПоступлениеТоваров</span>
                                <span class="tree-leaf-english">GoodsReceipt</span>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
            
            <!-- Системные перечисления -->
            <div class="tree-node">
                <div class="tree-node-header" onclick="toggleNode(this)">
                    <span class="tree-icon">▼</span>
                    <span class="tree-label">📝 Системные перечисления</span>
                    <span class="tree-count">{enums_count}</span>
                </div>
                <div class="tree-children" id="enums-tree">
                    <div class="tree-leaf" onclick="selectItem(this, 'enum', 'ВидДвиженияНакопления')">
                        <span class="tree-leaf-name">ВидДвиженияНакопления</span>
                    </div>
                    <div class="tree-leaf" onclick="selectItem(this, 'enum', 'ВидСчета')">
                        <span class="tree-leaf-name">ВидСчета</span>
                    </div>
                </div>
            </div>
        </div>
    </div>
    
    <!-- Правая панель с деталями -->
    <div class="content" id="content">
        <div class="welcome">
            <h2>👋 Добро пожаловать в BSL Type System Explorer</h2>
            <p>Выберите элемент в дереве слева для просмотра подробной информации</p>
            
            <div class="stats-grid">
                <div class="stat-card">
                    <div class="stat-value">{functions_total}</div>
                    <div class="stat-label">Функций</div>
                </div>
                <div class="stat-card">
                    <div class="stat-value">{keywords_total}</div>
                    <div class="stat-label">Ключевых слов</div>
                </div>
                <div class="stat-card">
                    <div class="stat-value">6</div>
                    <div class="stat-label">Фасетов</div>
                </div>
                <div class="stat-card">
                    <div class="stat-value">3</div>
                    <div class="stat-label">Типов TypeRef</div>
                </div>
            </div>
        </div>
    </div>
    
    <script>
        // Данные функций и ключевых слов
        const functionsData = {{}};
        const keywordsData = {{}};
        
        // Переключение узла дерева
        function toggleNode(header) {{
            const icon = header.querySelector('.tree-icon');
            const children = header.nextElementSibling;
            
            if (children) {{
                const isCollapsed = children.classList.toggle('collapsed');
                icon.textContent = isCollapsed ? '▶' : '▼';
                icon.classList.toggle('collapsed', isCollapsed);
            }}
        }}
        
        // Выбор элемента
        function selectItem(element, type, name) {{
            // Снимаем выделение с других элементов
            document.querySelectorAll('.tree-leaf.selected, .tree-node-header.selected').forEach(el => {{
                el.classList.remove('selected');
            }});
            
            // Выделяем текущий элемент
            element.classList.add('selected');
            
            // Показываем детали
            showDetails(type, name);
        }}
        
        // Показ деталей элемента
        function showDetails(type, name) {{
            const content = document.getElementById('content');
            
            let html = '';
            
            switch(type) {{
                case 'function':
                    html = `
                        <div class="content-header">
                            <h2 class="content-title">📦 ${{name}}</h2>
                            <div class="content-subtitle">Глобальная функция</div>
                        </div>
                        <div class="tabs">
                            <div class="tab active" onclick="switchTab(this, 'description')">Описание</div>
                            <div class="tab" onclick="switchTab(this, 'syntax')">Синтаксис</div>
                            <div class="tab" onclick="switchTab(this, 'examples')">Примеры</div>
                        </div>
                        <div class="tab-content active" id="description">
                            <div class="detail-section">
                                <h3>Описание</h3>
                                <p>Функция ${{name}} используется для...</p>
                            </div>
                        </div>
                        <div class="tab-content" id="syntax">
                            <div class="detail-section">
                                <h3>Синтаксис</h3>
                                <div class="code-block">${{name}}(Параметр1, Параметр2)</div>
                            </div>
                        </div>
                        <div class="tab-content" id="examples">
                            <div class="detail-section">
                                <h3>Примеры использования</h3>
                                <div class="code-block">// Пример 1\n${{name}}("Значение");</div>
                            </div>
                        </div>
                    `;
                    break;
                    
                case 'keyword':
                    html = `
                        <div class="content-header">
                            <h2 class="content-title">🔤 ${{name}}</h2>
                            <div class="content-subtitle">Ключевое слово</div>
                        </div>
                        <div class="detail-section">
                            <h3>Использование</h3>
                            <div class="code-block">${{name}} Условие Тогда\n    // код\nКонец${{name}};</div>
                        </div>
                    `;
                    break;
                    
                case 'catalog':
                    html = `
                        <div class="content-header">
                            <h2 class="content-title">📁 Справочник.${{name}}</h2>
                            <div class="content-subtitle">Объект метаданных</div>
                        </div>
                        <div class="detail-section">
                            <h3>Фасеты</h3>
                            <ul class="parameter-list">
                                <li class="parameter-item">
                                    <span class="parameter-name">Manager</span>
                                    <span class="parameter-type">Справочники.${{name}}</span>
                                </li>
                                <li class="parameter-item">
                                    <span class="parameter-name">Object</span>
                                    <span class="parameter-type">СправочникОбъект.${{name}}</span>
                                </li>
                                <li class="parameter-item">
                                    <span class="parameter-name">Reference</span>
                                    <span class="parameter-type">СправочникСсылка.${{name}}</span>
                                </li>
                            </ul>
                        </div>
                    `;
                    break;
            }}
            
            content.innerHTML = html;
        }}
        
        // Переключение вкладок
        function switchTab(tab, contentId) {{
            // Деактивируем все вкладки
            document.querySelectorAll('.tab').forEach(t => t.classList.remove('active'));
            document.querySelectorAll('.tab-content').forEach(c => c.classList.remove('active'));
            
            // Активируем выбранную вкладку
            tab.classList.add('active');
            const content = document.getElementById(contentId);
            if (content) content.classList.add('active');
        }}
        
        // Фильтрация дерева
        function filterTree(query) {{
            query = query.toLowerCase();
            const allLeaves = document.querySelectorAll('.tree-leaf');
            
            allLeaves.forEach(leaf => {{
                const text = leaf.textContent.toLowerCase();
                if (query === '' || text.includes(query)) {{
                    leaf.style.display = '';
                }} else {{
                    leaf.style.display = 'none';
                }}
            }});
        }}
        
        // Инициализация при загрузке
        window.addEventListener('load', () => {{
            // Можно добавить анимации загрузки
        }});
    </script>
</body>
</html>"#,
        functions_count = database.global_functions.len(),
        functions_tree = generate_functions_tree(&functions_by_category),
        keywords_count = database.keywords.len(),
        keywords_tree = generate_keywords_tree(&database),
        objects_count = database.global_objects.len(),
        enums_count = database.system_enums.len(),
        functions_total = database.global_functions.len(),
        keywords_total = database.keywords.len()
    );
    
    // Сохраняем файл
    let output_path = "type_hierarchy_tree.html";
    fs::write(output_path, html)?;
    
    println!("✅ Древовидная иерархия создана: {}", output_path);
    println!("🌳 Структура:");
    println!("   ├─ Глобальные функции: {}", database.global_functions.len());
    println!("   ├─ Ключевые слова: {}", database.keywords.len());
    println!("   ├─ Глобальные объекты: {}", database.global_objects.len());
    println!("   └─ Системные перечисления: {}", database.system_enums.len());
    
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

fn generate_functions_tree(categories: &BTreeMap<String, Vec<(String, String)>>) -> String {
    let mut html = String::new();
    
    for (category, functions) in categories.iter().take(10) {
        html.push_str(&format!(r#"
            <div class="tree-node">
                <div class="tree-node-header" onclick="toggleNode(this)">
                    <span class="tree-icon">▶</span>
                    <span class="tree-label">{}</span>
                    <span class="tree-count">{}</span>
                </div>
                <div class="tree-children collapsed">
        "#, category, functions.len()));
        
        for (name, english) in functions.iter().take(20) {
            let english_part = if english.is_empty() {
                String::new()
            } else {
                format!(r#"<span class="tree-leaf-english">{}</span>"#, english)
            };
            
            html.push_str(&format!(r#"
                    <div class="tree-leaf" onclick="selectItem(this, 'function', '{}')">
                        <span class="tree-leaf-name">{}</span>
                        {}
                    </div>
            "#, name, name, english_part));
        }
        
        if functions.len() > 20 {
            html.push_str(&format!(r#"
                    <div class="tree-leaf" style="color: #999;">
                        <span class="tree-leaf-name">... ещё {}</span>
                    </div>
            "#, functions.len() - 20));
        }
        
        html.push_str("</div></div>");
    }
    
    html
}

fn generate_keywords_tree(database: &SyntaxHelperDatabase) -> String {
    let mut by_category: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    
    for keyword in &database.keywords {
        let category = match keyword.category {
            KeywordCategory::Structure => "Управляющие конструкции",
            KeywordCategory::Definition => "Определения",
            KeywordCategory::Root => "Базовые",
            KeywordCategory::Operator => "Операторы",
            KeywordCategory::Instruction => "Инструкции",
            KeywordCategory::Other => "Другие",
        };
        by_category.entry(category).or_default().push(&keyword.russian);
    }
    
    let mut html = String::new();
    
    for (category, keywords) in by_category {
        html.push_str(&format!(r#"
            <div class="tree-node">
                <div class="tree-node-header" onclick="toggleNode(this)">
                    <span class="tree-icon">▶</span>
                    <span class="tree-label">{}</span>
                    <span class="tree-count">{}</span>
                </div>
                <div class="tree-children collapsed">
        "#, category, keywords.len()));
        
        for keyword in keywords {
            html.push_str(&format!(r#"
                    <div class="tree-leaf" onclick="selectItem(this, 'keyword', '{}')">
                        <span class="tree-leaf-name">{}</span>
                    </div>
            "#, keyword, keyword));
        }
        
        html.push_str("</div></div>");
    }
    
    html
}