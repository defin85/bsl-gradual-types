//! Генератор HTML с правильной иерархией категорий как в синтакс-помощнике
//! 
//! Структура соответствует официальной документации 1С:
//! - Общее описание встроенного языка
//!   - Операторы и синтаксические конструкции
//!   - Определения
//!   - Выражения

use bsl_gradual_types::adapters::syntax_helper_parser::{SyntaxHelperParser, SyntaxHelperDatabase, KeywordCategory};
use std::fs;
use std::collections::BTreeMap;

fn main() -> anyhow::Result<()> {
    println!("🌳 Генерация визуализации с правильной иерархией...");
    
    // Загружаем данные
    let json_path = "examples/syntax_helper/syntax_database.json";
    let database = if std::path::Path::new(json_path).exists() {
        SyntaxHelperParser::load_from_file(json_path)?
    } else {
        println!("❌ База данных не найдена. Запустите: cargo run --example syntax_helper_parser_demo");
        return Ok(());
    };
    
    // Правильная иерархия категорий согласно синтакс-помощнику
    let proper_hierarchy = ProperHierarchy::build(&database);
    
    // Генерируем HTML
    let html = generate_html(&database, &proper_hierarchy);
    
    // Сохраняем файл
    let output_path = "type_hierarchy_proper.html";
    fs::write(output_path, html)?;
    
    println!("✅ Визуализация с правильной иерархией создана: {}", output_path);
    println!("📊 Структура:");
    println!("   └─ 📚 Общее описание встроенного языка");
    println!("      ├─ 🔧 Операторы и синтаксические конструкции: {}", proper_hierarchy.operators_and_constructs.len());
    println!("      ├─ 📝 Определения: {}", proper_hierarchy.definitions.len());
    println!("      ├─ 🔤 Выражения: {}", proper_hierarchy.expressions.len());
    println!("      └─ 📦 Глобальный контекст");
    println!("         └─ Глобальные функции: {}", database.global_functions.len());
    
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

/// Правильная иерархия категорий
struct ProperHierarchy {
    operators_and_constructs: Vec<KeywordItem>,
    definitions: Vec<KeywordItem>,
    expressions: Vec<KeywordItem>,
    instructions: Vec<KeywordItem>,
}

struct KeywordItem {
    russian: String,
    english: String,
    description: Option<String>,
}

impl ProperHierarchy {
    fn build(database: &SyntaxHelperDatabase) -> Self {
        let mut operators_and_constructs = Vec::new();
        let mut definitions = Vec::new();
        let mut expressions = Vec::new();
        let mut instructions = Vec::new();
        
        for keyword in &database.keywords {
            let item = KeywordItem {
                russian: keyword.russian.clone(),
                english: keyword.english.clone(),
                description: keyword.description.clone(),
            };
            
            match keyword.category {
                KeywordCategory::Structure | KeywordCategory::Operator => {
                    operators_and_constructs.push(item);
                }
                KeywordCategory::Definition => {
                    definitions.push(item);
                }
                KeywordCategory::Root => {
                    expressions.push(item);
                }
                KeywordCategory::Instruction => {
                    instructions.push(item);
                }
                KeywordCategory::Other => {
                    expressions.push(item); // По умолчанию в выражения
                }
            }
        }
        
        ProperHierarchy {
            operators_and_constructs,
            definitions,
            expressions,
            instructions,
        }
    }
}

fn generate_html(database: &SyntaxHelperDatabase, hierarchy: &ProperHierarchy) -> String {
    format!(r#"<!DOCTYPE html>
<html lang="ru">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>BSL Type System - Правильная иерархия</title>
    <style>
        * {{
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }}
        
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            min-height: 100vh;
            display: flex;
        }}
        
        /* Левая панель с деревом */
        .sidebar {{
            width: 450px;
            background: white;
            box-shadow: 0 0 20px rgba(0,0,0,0.1);
            overflow-y: auto;
            overflow-x: hidden;
        }}
        
        .sidebar-header {{
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: white;
            padding: 25px;
            position: sticky;
            top: 0;
            z-index: 10;
        }}
        
        .sidebar-header h1 {{
            font-size: 1.6em;
            margin-bottom: 10px;
        }}
        
        .sidebar-subtitle {{
            opacity: 0.9;
            font-size: 0.95em;
        }}
        
        .search-box {{
            width: 100%;
            padding: 12px;
            border: none;
            border-radius: 8px;
            font-size: 14px;
            margin-top: 15px;
            background: rgba(255,255,255,0.9);
        }}
        
        .search-box:focus {{
            outline: none;
            box-shadow: 0 0 0 3px rgba(255,255,255,0.3);
        }}
        
        /* Древовидная структура */
        .tree {{
            padding: 15px;
        }}
        
        .tree-node {{
            user-select: none;
            margin-bottom: 5px;
        }}
        
        .tree-node-header {{
            display: flex;
            align-items: center;
            padding: 10px;
            cursor: pointer;
            border-radius: 6px;
            transition: all 0.2s;
            background: #f8f9fa;
            margin-bottom: 5px;
        }}
        
        .tree-node-header:hover {{
            background: #e9ecef;
            transform: translateX(3px);
        }}
        
        .tree-node-header.selected {{
            background: linear-gradient(135deg, #667eea20 0%, #764ba220 100%);
            border-left: 3px solid #667eea;
        }}
        
        /* Разные уровни вложенности */
        .tree-node-level-0 > .tree-node-header {{
            font-weight: bold;
            font-size: 1.1em;
            background: linear-gradient(135deg, #f8f9fa 0%, #e9ecef 100%);
        }}
        
        .tree-node-level-1 > .tree-node-header {{
            margin-left: 20px;
            font-weight: 600;
        }}
        
        .tree-node-level-2 > .tree-node-header {{
            margin-left: 40px;
            font-size: 0.95em;
        }}
        
        .tree-icon {{
            width: 24px;
            height: 24px;
            margin-right: 10px;
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
            color: #2c3e50;
        }}
        
        .tree-count {{
            color: #6c757d;
            font-size: 0.85em;
            background: #e9ecef;
            padding: 2px 8px;
            border-radius: 12px;
            margin-left: 10px;
        }}
        
        .tree-children {{
            overflow: hidden;
            transition: max-height 0.3s ease-out, opacity 0.3s;
        }}
        
        .tree-children.collapsed {{
            max-height: 0;
            opacity: 0;
        }}
        
        .tree-leaf {{
            padding: 8px 12px;
            margin-left: 60px;
            cursor: pointer;
            border-radius: 4px;
            transition: all 0.2s;
            font-size: 0.95em;
            display: flex;
            align-items: center;
            justify-content: space-between;
        }}
        
        .tree-leaf:hover {{
            background: #f8f9fa;
            transform: translateX(3px);
        }}
        
        .tree-leaf.selected {{
            background: linear-gradient(135deg, #667eea15 0%, #764ba215 100%);
            color: #667eea;
        }}
        
        .tree-leaf-name {{
            flex: 1;
            color: #495057;
        }}
        
        .tree-leaf-english {{
            color: #6c757d;
            font-size: 0.9em;
            font-style: italic;
        }}
        
        /* Правая панель с деталями */
        .content {{
            flex: 1;
            padding: 30px;
            overflow-y: auto;
            background: white;
        }}
        
        .content-header {{
            border-bottom: 2px solid #e9ecef;
            padding-bottom: 20px;
            margin-bottom: 25px;
        }}
        
        .content-title {{
            font-size: 2.2em;
            color: #2c3e50;
            margin-bottom: 10px;
        }}
        
        .content-breadcrumb {{
            color: #6c757d;
            font-size: 0.95em;
            margin-bottom: 15px;
        }}
        
        .content-breadcrumb span {{
            color: #667eea;
        }}
        
        .detail-section {{
            margin-bottom: 30px;
            padding: 20px;
            background: #f8f9fa;
            border-radius: 8px;
        }}
        
        .detail-section h3 {{
            color: #495057;
            margin-bottom: 15px;
            font-size: 1.3em;
        }}
        
        .code-block {{
            background: #2c3e50;
            color: #ecf0f1;
            border-radius: 6px;
            padding: 20px;
            font-family: 'Consolas', 'Monaco', monospace;
            font-size: 14px;
            overflow-x: auto;
            box-shadow: 0 4px 6px rgba(0,0,0,0.1);
        }}
        
        .code-keyword {{
            color: #e74c3c;
            font-weight: bold;
        }}
        
        .code-comment {{
            color: #95a5a6;
            font-style: italic;
        }}
        
        .code-string {{
            color: #2ecc71;
        }}
        
        /* Welcome screen */
        .welcome {{
            text-align: center;
            padding: 60px 40px;
        }}
        
        .welcome h2 {{
            font-size: 2.5em;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            -webkit-background-clip: text;
            -webkit-text-fill-color: transparent;
            margin-bottom: 20px;
        }}
        
        .welcome p {{
            font-size: 1.2em;
            color: #6c757d;
            line-height: 1.6;
            max-width: 600px;
            margin: 0 auto;
        }}
        
        .feature-grid {{
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
            gap: 20px;
            margin-top: 40px;
        }}
        
        .feature-card {{
            padding: 25px;
            background: linear-gradient(135deg, #f8f9fa 0%, white 100%);
            border-radius: 10px;
            box-shadow: 0 2px 10px rgba(0,0,0,0.05);
            transition: all 0.3s;
        }}
        
        .feature-card:hover {{
            transform: translateY(-5px);
            box-shadow: 0 5px 20px rgba(0,0,0,0.1);
        }}
        
        .feature-icon {{
            font-size: 2.5em;
            margin-bottom: 15px;
        }}
        
        .feature-title {{
            font-size: 1.1em;
            font-weight: 600;
            color: #2c3e50;
            margin-bottom: 10px;
        }}
        
        .feature-desc {{
            color: #6c757d;
            font-size: 0.95em;
        }}
    </style>
</head>
<body>
    <!-- Левая панель с деревом -->
    <div class="sidebar">
        <div class="sidebar-header">
            <h1>📚 BSL Type System</h1>
            <div class="sidebar-subtitle">Правильная иерархия языковых конструкций</div>
            <input type="text" class="search-box" id="searchBox" placeholder="🔍 Поиск..." onkeyup="filterTree(this.value)">
        </div>
        
        <div class="tree" id="tree">
            <!-- Корневой узел: Общее описание встроенного языка -->
            <div class="tree-node tree-node-level-0">
                <div class="tree-node-header" onclick="toggleNode(this)">
                    <span class="tree-icon">▼</span>
                    <span class="tree-label">📚 Общее описание встроенного языка</span>
                </div>
                <div class="tree-children">
                    
                    <!-- Операторы и синтаксические конструкции -->
                    <div class="tree-node tree-node-level-1">
                        <div class="tree-node-header" onclick="toggleNode(this)">
                            <span class="tree-icon">▼</span>
                            <span class="tree-label">🔧 Операторы и синтаксические конструкции</span>
                            <span class="tree-count">{operators_count}</span>
                        </div>
                        <div class="tree-children">
                            {operators_html}
                        </div>
                    </div>
                    
                    <!-- Определения -->
                    <div class="tree-node tree-node-level-1">
                        <div class="tree-node-header" onclick="toggleNode(this)">
                            <span class="tree-icon">▼</span>
                            <span class="tree-label">📝 Определения</span>
                            <span class="tree-count">{definitions_count}</span>
                        </div>
                        <div class="tree-children">
                            {definitions_html}
                        </div>
                    </div>
                    
                    <!-- Выражения -->
                    <div class="tree-node tree-node-level-1">
                        <div class="tree-node-header" onclick="toggleNode(this)">
                            <span class="tree-icon">▼</span>
                            <span class="tree-label">🔤 Выражения</span>
                            <span class="tree-count">{expressions_count}</span>
                        </div>
                        <div class="tree-children">
                            {expressions_html}
                        </div>
                    </div>
                    
                    <!-- Инструкции препроцессора -->
                    {instructions_section}
                    
                </div>
            </div>
            
            <!-- Глобальный контекст -->
            <div class="tree-node tree-node-level-0">
                <div class="tree-node-header" onclick="toggleNode(this)">
                    <span class="tree-icon">▶</span>
                    <span class="tree-label">🌐 Глобальный контекст</span>
                </div>
                <div class="tree-children collapsed">
                    <!-- Глобальные функции -->
                    <div class="tree-node tree-node-level-1">
                        <div class="tree-node-header" onclick="toggleNode(this)">
                            <span class="tree-icon">▶</span>
                            <span class="tree-label">📦 Глобальные функции</span>
                            <span class="tree-count">{functions_count}</span>
                        </div>
                        <div class="tree-children collapsed">
                            {functions_html}
                        </div>
                    </div>
                    
                    <!-- Глобальные объекты -->
                    <div class="tree-node tree-node-level-1">
                        <div class="tree-node-header" onclick="toggleNode(this)">
                            <span class="tree-icon">▶</span>
                            <span class="tree-label">🏢 Глобальные объекты</span>
                            <span class="tree-count">{objects_count}</span>
                        </div>
                        <div class="tree-children collapsed">
                            <!-- Здесь будут объекты когда их извлечём из синтакс-помощника -->
                        </div>
                    </div>
                </div>
            </div>
        </div>
    </div>
    
    <!-- Правая панель с деталями -->
    <div class="content" id="content">
        <div class="welcome">
            <h2>Добро пожаловать!</h2>
            <p>Это правильная иерархическая структура языковых конструкций BSL, соответствующая официальной документации 1С:Предприятие</p>
            
            <div class="feature-grid">
                <div class="feature-card">
                    <div class="feature-icon">🔧</div>
                    <div class="feature-title">Операторы</div>
                    <div class="feature-desc">Управляющие конструкции: Если, Для, Пока</div>
                </div>
                <div class="feature-card">
                    <div class="feature-icon">📝</div>
                    <div class="feature-title">Определения</div>
                    <div class="feature-desc">Типы и переменные: Строка, Число, Перем</div>
                </div>
                <div class="feature-card">
                    <div class="feature-icon">🔤</div>
                    <div class="feature-title">Выражения</div>
                    <div class="feature-desc">Операции и вычисления: Новый, Выполнить</div>
                </div>
                <div class="feature-card">
                    <div class="feature-icon">📦</div>
                    <div class="feature-title">Функции</div>
                    <div class="feature-desc">Глобальные функции платформы</div>
                </div>
            </div>
        </div>
    </div>
    
    <script>
        // Переключение узла дерева
        function toggleNode(header) {{
            const icon = header.querySelector('.tree-icon');
            const children = header.nextElementSibling;
            
            if (children && children.classList.contains('tree-children')) {{
                const isCollapsed = children.classList.toggle('collapsed');
                icon.textContent = isCollapsed ? '▶' : '▼';
            }}
        }}
        
        // Выбор элемента
        function selectItem(element, category, type, name) {{
            // Снимаем выделение с других элементов
            document.querySelectorAll('.tree-leaf.selected, .tree-node-header.selected').forEach(el => {{
                el.classList.remove('selected');
            }});
            
            // Выделяем текущий элемент
            element.classList.add('selected');
            
            // Показываем детали
            showDetails(category, type, name);
        }}
        
        // Показ деталей элемента
        function showDetails(category, type, name) {{
            const content = document.getElementById('content');
            
            let html = `
                <div class="content-header">
                    <div class="content-breadcrumb">
                        <span>Общее описание встроенного языка</span> / 
                        <span>${{category}}</span>
                    </div>
                    <h2 class="content-title">${{name}}</h2>
                </div>
            `;
            
            if (type === 'operator') {{
                html += `
                    <div class="detail-section">
                        <h3>Синтаксис</h3>
                        <div class="code-block">
                            <span class="code-keyword">${{name}}</span> <span class="code-comment">// условие или выражение</span><br>
                            <span class="code-comment">    // блок кода</span><br>
                            <span class="code-keyword">Конец${{name}}</span>
                        </div>
                    </div>
                    <div class="detail-section">
                        <h3>Описание</h3>
                        <p>Управляющая конструкция языка BSL для организации логики программы.</p>
                    </div>
                `;
            }} else if (type === 'definition') {{
                html += `
                    <div class="detail-section">
                        <h3>Тип данных</h3>
                        <p>Базовый тип данных языка BSL.</p>
                    </div>
                `;
            }} else if (type === 'function') {{
                html += `
                    <div class="detail-section">
                        <h3>Глобальная функция</h3>
                        <p>Встроенная функция платформы 1С:Предприятие.</p>
                    </div>
                `;
            }}
            
            content.innerHTML = html;
        }}
        
        // Фильтрация дерева
        function filterTree(query) {{
            query = query.toLowerCase();
            const allNodes = document.querySelectorAll('.tree-leaf, .tree-node-header');
            
            allNodes.forEach(node => {{
                const text = node.textContent.toLowerCase();
                const parent = node.closest('.tree-node');
                
                if (query === '' || text.includes(query)) {{
                    node.style.display = '';
                    // Раскрываем родительские узлы
                    let current = parent;
                    while (current && current.classList.contains('tree-node')) {{
                        const children = current.querySelector('.tree-children');
                        if (children) {{
                            children.classList.remove('collapsed');
                            const icon = current.querySelector('.tree-icon');
                            if (icon) icon.textContent = '▼';
                        }}
                        current = current.parentElement.closest('.tree-node');
                    }}
                }} else {{
                    node.style.display = 'none';
                }}
            }});
        }}
    </script>
</body>
</html>"#,
        operators_count = hierarchy.operators_and_constructs.len(),
        operators_html = generate_items_html(&hierarchy.operators_and_constructs, "operator", "Операторы и синтаксические конструкции"),
        definitions_count = hierarchy.definitions.len(),
        definitions_html = generate_items_html(&hierarchy.definitions, "definition", "Определения"),
        expressions_count = hierarchy.expressions.len(),
        expressions_html = generate_items_html(&hierarchy.expressions, "expression", "Выражения"),
        instructions_section = if hierarchy.instructions.is_empty() {
            String::new()
        } else {
            format!(r#"
                    <div class="tree-node tree-node-level-1">
                        <div class="tree-node-header" onclick="toggleNode(this)">
                            <span class="tree-icon">▶</span>
                            <span class="tree-label">⚙️ Инструкции препроцессора</span>
                            <span class="tree-count">{}</span>
                        </div>
                        <div class="tree-children collapsed">
                            {}
                        </div>
                    </div>"#,
                hierarchy.instructions.len(),
                generate_items_html(&hierarchy.instructions, "instruction", "Инструкции препроцессора")
            )
        },
        functions_count = database.global_functions.len(),
        functions_html = generate_functions_html(database),
        objects_count = database.global_objects.len()
    )
}

fn generate_items_html(items: &[KeywordItem], item_type: &str, category: &str) -> String {
    items.iter()
        .map(|item| format!(
            r#"<div class="tree-leaf" onclick="selectItem(this, '{}', '{}', '{}')">
                <span class="tree-leaf-name">{}</span>
                <span class="tree-leaf-english">{}</span>
            </div>"#,
            category, item_type, item.russian, item.russian, item.english
        ))
        .collect::<Vec<_>>()
        .join("\n")
}

fn generate_functions_html(database: &SyntaxHelperDatabase) -> String {
    // Группируем функции по первой букве для компактности
    let mut by_letter: BTreeMap<char, Vec<String>> = BTreeMap::new();
    
    for (name, func) in &database.global_functions {
        let first_char = name.chars().next().unwrap_or('?');
        let display = if let Some(eng) = &func.english_name {
            format!("{} / {}", name, eng)
        } else {
            name.clone()
        };
        by_letter.entry(first_char).or_default().push(display);
    }
    
    // Показываем только первые несколько для примера
    by_letter.values()
        .flat_map(|functions| functions.iter().take(3))
        .map(|name| format!(
            r#"<div class="tree-leaf" onclick="selectItem(this, 'Глобальный контекст', 'function', '{}')">
                <span class="tree-leaf-name">{}</span>
            </div>"#,
            name, name
        ))
        .collect::<Vec<_>>()
        .join("\n")
}