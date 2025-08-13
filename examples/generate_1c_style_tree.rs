//! Генератор HTML отчёта с деревом типов в стиле синтакс-помощника 1С
//! 
//! Создаёт интерактивное дерево с иконками папок и файлов как в 1С

use bsl_gradual_types::adapters::syntax_helper_parser::SyntaxHelperParser;
use std::fs;
use std::collections::BTreeMap;

fn main() -> anyhow::Result<()> {
    println!("Генерация дерева типов в стиле 1С...");
    
    // Загружаем данные
    let json_path = "examples/syntax_helper/syntax_database.json";
    if !std::path::Path::new(json_path).exists() {
        println!("❌ База данных не найдена. Запустите: cargo run --example syntax_helper_parser_demo");
        return Ok(());
    }
    
    let database = SyntaxHelperParser::load_from_file(json_path)?;
    
    // Группируем функции по категориям
    let mut functions_by_category: BTreeMap<String, Vec<(String, Option<String>)>> = BTreeMap::new();
    
    for (name, func) in &database.global_functions {
        let category = categorize_function(name);
        functions_by_category.entry(category)
            .or_default()
            .push((name.clone(), func.english_name.clone()));
    }
    
    // Создаём HTML в стиле 1С
    let mut html = String::new();
    
    html.push_str("<!DOCTYPE html>\n");
    html.push_str("<html lang=\"ru\">\n");
    html.push_str("<head>\n");
    html.push_str("    <meta charset=\"UTF-8\">\n");
    html.push_str("    <title>Синтакс-помощник BSL</title>\n");
    html.push_str("    <style>\n");
    html.push_str("        * { margin: 0; padding: 0; box-sizing: border-box; }\n");
    html.push_str("        body {\n");
    html.push_str("            font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;\n");
    html.push_str("            font-size: 13px;\n");
    html.push_str("            background: #f0f0f0;\n");
    html.push_str("        }\n");
    html.push_str("        .container {\n");
    html.push_str("            display: flex;\n");
    html.push_str("            height: 100vh;\n");
    html.push_str("        }\n");
    html.push_str("        .sidebar {\n");
    html.push_str("            width: 350px;\n");
    html.push_str("            background: white;\n");
    html.push_str("            border-right: 1px solid #d0d0d0;\n");
    html.push_str("            overflow-y: auto;\n");
    html.push_str("        }\n");
    html.push_str("        .header {\n");
    html.push_str("            background: linear-gradient(to bottom, #fafafa, #e8e8e8);\n");
    html.push_str("            padding: 8px 12px;\n");
    html.push_str("            border-bottom: 1px solid #c0c0c0;\n");
    html.push_str("            font-weight: bold;\n");
    html.push_str("        }\n");
    html.push_str("        .tabs {\n");
    html.push_str("            display: flex;\n");
    html.push_str("            background: #f8f8f8;\n");
    html.push_str("            border-bottom: 1px solid #d0d0d0;\n");
    html.push_str("        }\n");
    html.push_str("        .tab {\n");
    html.push_str("            padding: 6px 12px;\n");
    html.push_str("            border: 1px solid transparent;\n");
    html.push_str("            cursor: pointer;\n");
    html.push_str("            background: #e8e8e8;\n");
    html.push_str("            margin-right: 2px;\n");
    html.push_str("        }\n");
    html.push_str("        .tab.active {\n");
    html.push_str("            background: white;\n");
    html.push_str("            border: 1px solid #d0d0d0;\n");
    html.push_str("            border-bottom: 1px solid white;\n");
    html.push_str("        }\n");
    html.push_str("        .tree {\n");
    html.push_str("            padding: 4px;\n");
    html.push_str("            font-family: 'Segoe UI', Tahoma, sans-serif;\n");
    html.push_str("            line-height: 20px;\n");
    html.push_str("        }\n");
    html.push_str("        .tree-item {\n");
    html.push_str("            cursor: pointer;\n");
    html.push_str("            padding: 2px 0;\n");
    html.push_str("            white-space: nowrap;\n");
    html.push_str("            user-select: none;\n");
    html.push_str("        }\n");
    html.push_str("        .tree-item:hover {\n");
    html.push_str("            background: #e8f4ff;\n");
    html.push_str("        }\n");
    html.push_str("        .tree-item.selected {\n");
    html.push_str("            background: #cce8ff;\n");
    html.push_str("        }\n");
    html.push_str("        .tree-children {\n");
    html.push_str("            margin-left: 18px;\n");
    html.push_str("            display: none;\n");
    html.push_str("        }\n");
    html.push_str("        .tree-children.expanded {\n");
    html.push_str("            display: block;\n");
    html.push_str("        }\n");
    html.push_str("        .tree-icon {\n");
    html.push_str("            display: inline-block;\n");
    html.push_str("            width: 16px;\n");
    html.push_str("            height: 16px;\n");
    html.push_str("            margin-right: 4px;\n");
    html.push_str("            vertical-align: middle;\n");
    html.push_str("        }\n");
    html.push_str("        .icon-folder-closed {\n");
    html.push_str("            background: url('data:image/svg+xml,<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\"><path fill=\"%23dcb67a\" d=\"M1 3h6l2 2h6v8H1z\"/></svg>') center/contain no-repeat;\n");
    html.push_str("        }\n");
    html.push_str("        .icon-folder-open {\n");
    html.push_str("            background: url('data:image/svg+xml,<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\"><path fill=\"%23dcb67a\" d=\"M1 3h6l2 2h6v1l-1 7H0l1-7V3z\"/></svg>') center/contain no-repeat;\n");
    html.push_str("        }\n");
    html.push_str("        .icon-function {\n");
    html.push_str("            background: url('data:image/svg+xml,<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\"><rect fill=\"%234a90e2\" x=\"2\" y=\"2\" width=\"12\" height=\"12\" rx=\"1\"/><text x=\"8\" y=\"11\" text-anchor=\"middle\" fill=\"white\" font-size=\"10\">f</text></svg>') center/contain no-repeat;\n");
    html.push_str("        }\n");
    html.push_str("        .icon-property {\n");
    html.push_str("            background: url('data:image/svg+xml,<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\"><circle fill=\"%2342b883\" cx=\"8\" cy=\"8\" r=\"6\"/><text x=\"8\" y=\"11\" text-anchor=\"middle\" fill=\"white\" font-size=\"10\">P</text></svg>') center/contain no-repeat;\n");
    html.push_str("        }\n");
    html.push_str("        .icon-type {\n");
    html.push_str("            background: url('data:image/svg+xml,<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\"><rect fill=\"%23ff7b00\" x=\"2\" y=\"2\" width=\"12\" height=\"12\" rx=\"1\"/><text x=\"8\" y=\"11\" text-anchor=\"middle\" fill=\"white\" font-size=\"10\">T</text></svg>') center/contain no-repeat;\n");
    html.push_str("        }\n");
    html.push_str("        .icon-keyword {\n");
    html.push_str("            background: url('data:image/svg+xml,<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\"><rect fill=\"%239b59b6\" x=\"2\" y=\"2\" width=\"12\" height=\"12\" rx=\"1\"/><text x=\"8\" y=\"11\" text-anchor=\"middle\" fill=\"white\" font-size=\"10\">K</text></svg>') center/contain no-repeat;\n");
    html.push_str("        }\n");
    html.push_str("        .toggle {\n");
    html.push_str("            display: inline-block;\n");
    html.push_str("            width: 12px;\n");
    html.push_str("            height: 12px;\n");
    html.push_str("            margin-right: 2px;\n");
    html.push_str("            vertical-align: middle;\n");
    html.push_str("            cursor: pointer;\n");
    html.push_str("        }\n");
    html.push_str("        .toggle.collapsed::before {\n");
    html.push_str("            content: '▶';\n");
    html.push_str("            font-size: 10px;\n");
    html.push_str("            color: #666;\n");
    html.push_str("        }\n");
    html.push_str("        .toggle.expanded::before {\n");
    html.push_str("            content: '▼';\n");
    html.push_str("            font-size: 10px;\n");
    html.push_str("            color: #666;\n");
    html.push_str("        }\n");
    html.push_str("        .content {\n");
    html.push_str("            flex: 1;\n");
    html.push_str("            padding: 20px;\n");
    html.push_str("            background: white;\n");
    html.push_str("            overflow-y: auto;\n");
    html.push_str("        }\n");
    html.push_str("        .search-box {\n");
    html.push_str("            width: 100%;\n");
    html.push_str("            padding: 6px;\n");
    html.push_str("            border: 1px solid #d0d0d0;\n");
    html.push_str("            margin: 4px 0;\n");
    html.push_str("        }\n");
    html.push_str("    </style>\n");
    html.push_str("</head>\n");
    html.push_str("<body>\n");
    html.push_str("    <div class=\"container\">\n");
    html.push_str("        <div class=\"sidebar\">\n");
    html.push_str("            <div class=\"header\">Синтакс-помощник</div>\n");
    html.push_str("            <div class=\"tabs\">\n");
    html.push_str("                <div class=\"tab active\">Содержание</div>\n");
    html.push_str("                <div class=\"tab\">Индекс</div>\n");
    html.push_str("                <div class=\"tab\">Поиск</div>\n");
    html.push_str("            </div>\n");
    html.push_str("            <input type=\"text\" class=\"search-box\" placeholder=\"Быстрый поиск...\" onkeyup=\"filterTree(this.value)\">\n");
    html.push_str("            <div class=\"tree\">\n");
    
    // Главное дерево
    html.push_str("                <div class=\"tree-item\" onclick=\"toggleNode(this)\">\n");
    html.push_str("                    <span class=\"toggle expanded\"></span>\n");
    html.push_str("                    <span class=\"tree-icon icon-folder-open\"></span>\n");
    html.push_str("                    <span>Общее описание встроенного языка</span>\n");
    html.push_str("                </div>\n");
    html.push_str("                <div class=\"tree-children expanded\">\n");
    
    // Группируем ключевые слова по категориям
    use bsl_gradual_types::adapters::syntax_helper_parser::KeywordCategory;
    use std::collections::BTreeMap;
    
    let mut keywords_by_category: BTreeMap<&str, Vec<&bsl_gradual_types::adapters::syntax_helper_parser::KeywordInfo>> = BTreeMap::new();
    
    for keyword in &database.keywords {
        let category_name = match keyword.category {
            KeywordCategory::Structure => "Управляющие конструкции",
            KeywordCategory::Definition => "Определения",
            KeywordCategory::Root => "Операторы",
            KeywordCategory::Operator => "Специальные операторы",
            KeywordCategory::Instruction => "Инструкции",
            KeywordCategory::Other => "Прочее",
        };
        keywords_by_category.entry(category_name)
            .or_default()
            .push(keyword);
    }
    
    // Выводим категории с ключевыми словами
    for (category, keywords) in &keywords_by_category {
        if keywords.is_empty() { continue; }
        
        html.push_str(&format!(
            "                    <div class=\"tree-item\" onclick=\"toggleNode(this)\">\n\
                                 <span class=\"toggle collapsed\"></span>\n\
                                 <span class=\"tree-icon icon-folder-closed\"></span>\n\
                                 <span>{} ({})</span>\n\
                             </div>\n\
                             <div class=\"tree-children\">\n",
            category, keywords.len()
        ));
        
        for keyword in keywords {
            let display_text = if keyword.russian != keyword.english {
                format!("{} ({})", keyword.russian, keyword.english)
            } else {
                keyword.russian.clone()
            };
            
            html.push_str(&format!(
                "                        <div class=\"tree-item\">\n\
                                     <span style=\"width:14px;display:inline-block;\"></span>\n\
                                     <span class=\"tree-icon icon-keyword\"></span>\n\
                                     <span>{}</span>\n\
                                 </div>\n",
                display_text
            ));
        }
        
        html.push_str("                    </div>\n");
    }
    
    // Примитивные типы
    html.push_str("                    <div class=\"tree-item\" onclick=\"toggleNode(this)\">\n");
    html.push_str("                        <span class=\"toggle collapsed\"></span>\n");
    html.push_str("                        <span class=\"tree-icon icon-folder-closed\"></span>\n");
    html.push_str("                        <span>Примитивные типы</span>\n");
    html.push_str("                    </div>\n");
    html.push_str("                    <div class=\"tree-children\">\n");
    
    let primitive_types = vec!["Null", "Неопределено", "Число", "Строка", "Дата", "Булево", "Тип"];
    for ptype in primitive_types {
        html.push_str(&format!(
            "                        <div class=\"tree-item\">\n\
                                 <span style=\"width:14px;display:inline-block;\"></span>\n\
                                 <span class=\"tree-icon icon-type\"></span>\n\
                                 <span>{}</span>\n\
                             </div>\n",
            ptype
        ));
    }
    
    html.push_str("                    </div>\n");
    
    html.push_str("                </div>\n");
    
    // Глобальный контекст
    html.push_str("                <div class=\"tree-item\" onclick=\"toggleNode(this)\">\n");
    html.push_str("                    <span class=\"toggle collapsed\"></span>\n");
    html.push_str("                    <span class=\"tree-icon icon-folder-closed\"></span>\n");
    html.push_str("                    <span>Глобальный контекст</span>\n");
    html.push_str("                </div>\n");
    html.push_str("                <div class=\"tree-children\">\n");
    
    // Группы функций
    for (category, functions) in &functions_by_category {
        if functions.is_empty() { continue; }
        
        html.push_str(&format!(
            "                    <div class=\"tree-item\" onclick=\"toggleNode(this)\">\n\
                                 <span class=\"toggle collapsed\"></span>\n\
                                 <span class=\"tree-icon icon-folder-closed\"></span>\n\
                                 <span>{} ({})</span>\n\
                             </div>\n\
                             <div class=\"tree-children\">\n",
            category, functions.len()
        ));
        
        for (name, english) in functions.iter().take(50) { // Ограничиваем для производительности
            let display_name = if let Some(eng) = english {
                format!("{} ({})", name, eng)
            } else {
                name.clone()
            };
            
            html.push_str(&format!(
                "                        <div class=\"tree-item\">\n\
                                     <span style=\"width:14px;display:inline-block;\"></span>\n\
                                     <span class=\"tree-icon icon-function\"></span>\n\
                                     <span>{}</span>\n\
                                 </div>\n",
                display_name
            ));
        }
        
        if functions.len() > 50 {
            html.push_str(&format!(
                "                        <div class=\"tree-item\" style=\"color:#888;\">\n\
                                     <span style=\"width:14px;display:inline-block;\"></span>\n\
                                     <span style=\"margin-left:20px;\">...ещё {} функций</span>\n\
                                 </div>\n",
                functions.len() - 50
            ));
        }
        
        html.push_str("                    </div>\n");
    }
    
    html.push_str("                </div>\n");
    
    html.push_str("            </div>\n");
    html.push_str("        </div>\n");
    html.push_str("        <div class=\"content\">\n");
    html.push_str("            <h2>BSL Type System</h2>\n");
    html.push_str("            <p>Выберите элемент в дереве слева для просмотра подробной информации.</p>\n");
    html.push_str("            <br>\n");
    html.push_str("            <h3>Статистика:</h3>\n");
    html.push_str("            <ul>\n");
    html.push_str(&format!("                <li>Глобальных функций: {}</li>\n", database.global_functions.len()));
    html.push_str(&format!("                <li>Ключевых слов: {}</li>\n", database.keywords.len()));
    html.push_str(&format!("                <li>Категорий функций: {}</li>\n", functions_by_category.len()));
    html.push_str("            </ul>\n");
    html.push_str("        </div>\n");
    html.push_str("    </div>\n");
    
    // JavaScript
    html.push_str("    <script>\n");
    html.push_str("        function toggleNode(element) {\n");
    html.push_str("            const toggle = element.querySelector('.toggle');\n");
    html.push_str("            const icon = element.querySelector('.tree-icon');\n");
    html.push_str("            const children = element.nextElementSibling;\n");
    html.push_str("            \n");
    html.push_str("            if (toggle && children && children.classList.contains('tree-children')) {\n");
    html.push_str("                toggle.classList.toggle('collapsed');\n");
    html.push_str("                toggle.classList.toggle('expanded');\n");
    html.push_str("                children.classList.toggle('expanded');\n");
    html.push_str("                \n");
    html.push_str("                if (icon) {\n");
    html.push_str("                    if (icon.classList.contains('icon-folder-closed')) {\n");
    html.push_str("                        icon.classList.remove('icon-folder-closed');\n");
    html.push_str("                        icon.classList.add('icon-folder-open');\n");
    html.push_str("                    } else if (icon.classList.contains('icon-folder-open')) {\n");
    html.push_str("                        icon.classList.remove('icon-folder-open');\n");
    html.push_str("                        icon.classList.add('icon-folder-closed');\n");
    html.push_str("                    }\n");
    html.push_str("                }\n");
    html.push_str("            }\n");
    html.push_str("            \n");
    html.push_str("            // Выделение выбранного элемента\n");
    html.push_str("            document.querySelectorAll('.tree-item').forEach(item => {\n");
    html.push_str("                item.classList.remove('selected');\n");
    html.push_str("            });\n");
    html.push_str("            element.classList.add('selected');\n");
    html.push_str("        }\n");
    html.push_str("        \n");
    html.push_str("        function filterTree(searchTerm) {\n");
    html.push_str("            const term = searchTerm.toLowerCase();\n");
    html.push_str("            const items = document.querySelectorAll('.tree-item');\n");
    html.push_str("            \n");
    html.push_str("            items.forEach(item => {\n");
    html.push_str("                const text = item.textContent.toLowerCase();\n");
    html.push_str("                if (term === '' || text.includes(term)) {\n");
    html.push_str("                    item.style.display = '';\n");
    html.push_str("                } else {\n");
    html.push_str("                    item.style.display = 'none';\n");
    html.push_str("                }\n");
    html.push_str("            });\n");
    html.push_str("        }\n");
    html.push_str("    </script>\n");
    
    html.push_str("</body>\n");
    html.push_str("</html>\n");
    
    // Сохраняем HTML файл
    let output_path = "1c_style_tree.html";
    fs::write(output_path, html)?;
    
    println!("✅ HTML дерево в стиле 1С создано: {}", output_path);
    println!("📂 Откройте файл в браузере для просмотра");
    
    // Пытаемся открыть в браузере
    #[cfg(windows)]
    {
        std::process::Command::new("cmd")
            .args(&["/C", "start", output_path])
            .spawn()
            .ok();
    }
    
    Ok(())
}

// Функция для категоризации функций
fn categorize_function(name: &str) -> String {
    match name {
        n if n.starts_with("Строк") || n.starts_with("Стр") || n.starts_with("Str") => "Процедуры и функции работы со строками".to_string(),
        n if n.starts_with("Дата") || n.starts_with("Date") || n.contains("День") || n.contains("Месяц") || n.contains("Год") => "Процедуры и функции работы с датами".to_string(),
        n if n.starts_with("Тип") || n.starts_with("Type") || n == "ЗначениеЗаполнено" => "Процедуры и функции работы с типами".to_string(),
        n if n.starts_with("Формат") || n.starts_with("Format") => "Процедуры и функции форматирования".to_string(),
        n if n.contains("XML") => "Процедуры и функции работы с XML".to_string(),
        n if n.contains("JSON") => "Процедуры и функции работы с JSON".to_string(),
        n if n.contains("Файл") || n.contains("File") || n.contains("Каталог") => "Процедуры и функции работы с файлами".to_string(),
        n if n.contains("COM") => "Процедуры и функции работы с COM".to_string(),
        n if n.contains("Крипто") || n.contains("Crypto") => "Процедуры и функции работы с криптографией".to_string(),
        n if n.contains("База64") || n.contains("Base64") || n.contains("Двоич") => "Процедуры и функции работы с двоичными данными".to_string(),
        n if n.contains("Число") || n == "Цел" || n == "Окр" || n.starts_with("Sin") || n.starts_with("Cos") => "Процедуры и функции работы с числами".to_string(),
        n if n.contains("Сообщ") || n.contains("Message") || n.contains("Вопрос") => "Процедуры и функции интерактивной работы".to_string(),
        n if n.contains("Транзакц") || n.contains("Transaction") => "Процедуры и функции управления транзакциями".to_string(),
        n if n.contains("Пользовател") || n.contains("User") => "Процедуры и функции сеанса работы".to_string(),
        _ => "Прочие процедуры и функции".to_string(),
    }
}