//! Генератор HTML отчёта с иерархией типов
//! 
//! Создаёт интерактивный HTML файл для просмотра в браузере

use bsl_gradual_types::adapters::syntax_helper_parser::SyntaxHelperParser;
use std::fs;

fn main() -> anyhow::Result<()> {
    println!("Генерация HTML отчёта...");
    
    // Загружаем данные
    let json_path = "examples/syntax_helper/syntax_database.json";
    if !std::path::Path::new(json_path).exists() {
        println!("❌ База данных не найдена. Запустите: cargo run --example syntax_helper_parser_demo");
        return Ok(());
    }
    
    let database = SyntaxHelperParser::load_from_file(json_path)?;
    
    // Создаём HTML
    let mut html = String::new();
    
    // Начало документа
    html.push_str("<!DOCTYPE html>\n");
    html.push_str("<html lang=\"ru\">\n");
    html.push_str("<head>\n");
    html.push_str("    <meta charset=\"UTF-8\">\n");
    html.push_str("    <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n");
    html.push_str("    <title>BSL Type Hierarchy - Иерархия типов 1С</title>\n");
    html.push_str("    <style>\n");
    html.push_str("        body {\n");
    html.push_str("            font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;\n");
    html.push_str("            margin: 0;\n");
    html.push_str("            padding: 20px;\n");
    html.push_str("            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);\n");
    html.push_str("            min-height: 100vh;\n");
    html.push_str("        }\n");
    html.push_str("        .container {\n");
    html.push_str("            max-width: 1400px;\n");
    html.push_str("            margin: 0 auto;\n");
    html.push_str("            background: white;\n");
    html.push_str("            border-radius: 10px;\n");
    html.push_str("            box-shadow: 0 20px 60px rgba(0,0,0,0.3);\n");
    html.push_str("            overflow: hidden;\n");
    html.push_str("        }\n");
    html.push_str("        .header {\n");
    html.push_str("            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);\n");
    html.push_str("            color: white;\n");
    html.push_str("            padding: 30px;\n");
    html.push_str("            text-align: center;\n");
    html.push_str("        }\n");
    html.push_str("        h1 {\n");
    html.push_str("            margin: 0;\n");
    html.push_str("            font-size: 2.5em;\n");
    html.push_str("            text-shadow: 2px 2px 4px rgba(0,0,0,0.3);\n");
    html.push_str("        }\n");
    html.push_str("        .subtitle {\n");
    html.push_str("            margin-top: 10px;\n");
    html.push_str("            opacity: 0.9;\n");
    html.push_str("        }\n");
    html.push_str("        .stats {\n");
    html.push_str("            display: flex;\n");
    html.push_str("            justify-content: space-around;\n");
    html.push_str("            padding: 20px;\n");
    html.push_str("            background: #f8f9fa;\n");
    html.push_str("            border-bottom: 1px solid #dee2e6;\n");
    html.push_str("        }\n");
    html.push_str("        .stat-card {\n");
    html.push_str("            text-align: center;\n");
    html.push_str("            padding: 15px;\n");
    html.push_str("            background: white;\n");
    html.push_str("            border-radius: 8px;\n");
    html.push_str("            box-shadow: 0 2px 4px rgba(0,0,0,0.1);\n");
    html.push_str("            min-width: 120px;\n");
    html.push_str("        }\n");
    html.push_str("        .stat-number {\n");
    html.push_str("            font-size: 2em;\n");
    html.push_str("            font-weight: bold;\n");
    html.push_str("            color: #667eea;\n");
    html.push_str("        }\n");
    html.push_str("        .stat-label {\n");
    html.push_str("            color: #6c757d;\n");
    html.push_str("            font-size: 0.9em;\n");
    html.push_str("            margin-top: 5px;\n");
    html.push_str("        }\n");
    html.push_str("        .content {\n");
    html.push_str("            padding: 30px;\n");
    html.push_str("        }\n");
    html.push_str("        .section {\n");
    html.push_str("            margin-bottom: 40px;\n");
    html.push_str("            padding: 20px;\n");
    html.push_str("            background: #f8f9fa;\n");
    html.push_str("            border-radius: 8px;\n");
    html.push_str("        }\n");
    html.push_str("        .section-title {\n");
    html.push_str("            font-size: 1.5em;\n");
    html.push_str("            color: #495057;\n");
    html.push_str("            margin-bottom: 20px;\n");
    html.push_str("            padding-bottom: 10px;\n");
    html.push_str("            border-bottom: 2px solid #667eea;\n");
    html.push_str("        }\n");
    html.push_str("        .function-grid {\n");
    html.push_str("            display: grid;\n");
    html.push_str("            grid-template-columns: repeat(auto-fill, minmax(250px, 1fr));\n");
    html.push_str("            gap: 10px;\n");
    html.push_str("            max-height: 400px;\n");
    html.push_str("            overflow-y: auto;\n");
    html.push_str("        }\n");
    html.push_str("        .function-card {\n");
    html.push_str("            padding: 10px;\n");
    html.push_str("            background: white;\n");
    html.push_str("            border-radius: 5px;\n");
    html.push_str("            border-left: 3px solid #667eea;\n");
    html.push_str("            transition: all 0.3s;\n");
    html.push_str("        }\n");
    html.push_str("        .function-card:hover {\n");
    html.push_str("            box-shadow: 0 2px 8px rgba(0,0,0,0.1);\n");
    html.push_str("            transform: translateY(-2px);\n");
    html.push_str("        }\n");
    html.push_str("        .function-name {\n");
    html.push_str("            font-weight: bold;\n");
    html.push_str("            color: #2c3e50;\n");
    html.push_str("        }\n");
    html.push_str("        .function-english {\n");
    html.push_str("            color: #7f8c8d;\n");
    html.push_str("            font-size: 0.9em;\n");
    html.push_str("        }\n");
    html.push_str("        .keyword-list {\n");
    html.push_str("            display: flex;\n");
    html.push_str("            flex-wrap: wrap;\n");
    html.push_str("            gap: 10px;\n");
    html.push_str("        }\n");
    html.push_str("        .keyword {\n");
    html.push_str("            padding: 5px 15px;\n");
    html.push_str("            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);\n");
    html.push_str("            color: white;\n");
    html.push_str("            border-radius: 20px;\n");
    html.push_str("            font-size: 0.9em;\n");
    html.push_str("        }\n");
    html.push_str("        .search-box {\n");
    html.push_str("            padding: 10px;\n");
    html.push_str("            width: 100%;\n");
    html.push_str("            border: 2px solid #dee2e6;\n");
    html.push_str("            border-radius: 5px;\n");
    html.push_str("            font-size: 1em;\n");
    html.push_str("            margin-bottom: 20px;\n");
    html.push_str("        }\n");
    html.push_str("        .search-box:focus {\n");
    html.push_str("            outline: none;\n");
    html.push_str("            border-color: #667eea;\n");
    html.push_str("        }\n");
    html.push_str("    </style>\n");
    html.push_str("</head>\n");
    html.push_str("<body>\n");
    html.push_str("    <div class=\"container\">\n");
    html.push_str("        <div class=\"header\">\n");
    html.push_str("            <h1>🏗️ BSL Type Hierarchy</h1>\n");
    html.push_str("            <div class=\"subtitle\">Иерархия типов и функций языка 1С:Предприятие</div>\n");
    html.push_str("        </div>\n");
    
    // Статистика
    html.push_str("        <div class=\"stats\">\n");
    
    let stats = vec![
        ("Функций", database.global_functions.len(), "📦"),
        ("Объектов", database.global_objects.len(), "🏢"),
        ("Методов", database.object_methods.len(), "🎯"),
        ("Свойств", database.object_properties.len(), "⚙️"),
        ("Перечислений", database.system_enums.len(), "📝"),
        ("Ключевых слов", database.keywords.len(), "🔤"),
    ];
    
    for (label, count, icon) in stats {
        html.push_str(&format!(
            "            <div class=\"stat-card\">\n\
             <div style=\"font-size: 2em;\">{}</div>\n\
             <div class=\"stat-number\">{}</div>\n\
             <div class=\"stat-label\">{}</div>\n\
             </div>\n", 
            icon, count, label
        ));
    }
    
    html.push_str("        </div>\n");
    
    // Контент
    html.push_str("        <div class=\"content\">\n");
    
    // Поиск
    html.push_str("            <input type=\"text\" class=\"search-box\" id=\"searchBox\" placeholder=\"🔍 Поиск функций, ключевых слов...\" onkeyup=\"filterContent()\">\n");
    
    // Глобальные функции
    html.push_str("            <div class=\"section\" id=\"functions\">\n");
    html.push_str("                <h2 class=\"section-title\">📦 Глобальные функции</h2>\n");
    html.push_str("                <div class=\"function-grid\">\n");
    
    for (name, func) in &database.global_functions {
        let english = func.english_name.as_deref().unwrap_or("");
        html.push_str(&format!(
            "                    <div class=\"function-card searchable\" data-search=\"{}\">\n\
             <div class=\"function-name\">{}</div>\n\
             <div class=\"function-english\">{}</div>\n\
             </div>\n",
            format!("{} {}", name, english).to_lowercase(), name, english
        ));
    }
    
    html.push_str("                </div>\n");
    html.push_str("            </div>\n");
    
    // Ключевые слова
    html.push_str("            <div class=\"section\" id=\"keywords\">\n");
    html.push_str("                <h2 class=\"section-title\">🔤 Ключевые слова языка</h2>\n");
    html.push_str("                <div class=\"keyword-list\">\n");
    
    for keyword in &database.keywords {
        html.push_str(&format!(
            "                    <span class=\"keyword searchable\" data-search=\"{} {}\">{} / {}</span>\n",
            keyword.russian.to_lowercase(), keyword.english.to_lowercase(),
            keyword.russian, keyword.english
        ));
    }
    
    html.push_str("                </div>\n");
    html.push_str("            </div>\n");
    
    html.push_str("        </div>\n");
    html.push_str("    </div>\n");
    
    // JavaScript для поиска
    html.push_str("    <script>\n");
    html.push_str("        function filterContent() {\n");
    html.push_str("            const searchTerm = document.getElementById('searchBox').value.toLowerCase();\n");
    html.push_str("            const elements = document.querySelectorAll('.searchable');\n");
    html.push_str("            \n");
    html.push_str("            elements.forEach(element => {\n");
    html.push_str("                const text = element.getAttribute('data-search');\n");
    html.push_str("                if (text.includes(searchTerm)) {\n");
    html.push_str("                    element.style.display = '';\n");
    html.push_str("                } else {\n");
    html.push_str("                    element.style.display = 'none';\n");
    html.push_str("                }\n");
    html.push_str("            });\n");
    html.push_str("        }\n");
    html.push_str("    </script>\n");
    
    html.push_str("</body>\n");
    html.push_str("</html>\n");
    
    // Сохраняем HTML файл
    let output_path = "type_hierarchy_report.html";
    fs::write(output_path, html)?;
    
    println!("✅ HTML отчёт создан: {}", output_path);
    println!("📂 Откройте файл в браузере для просмотра интерактивной иерархии");
    
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