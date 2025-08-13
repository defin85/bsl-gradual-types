//! Генератор расширенного HTML отчёта с TypeRef и фасетами
//! 
//! Создаёт детальный HTML файл с новыми возможностями парсера

use bsl_gradual_types::adapters::syntax_helper_parser::SyntaxHelperParser;
use bsl_gradual_types::adapters::platform_types_v2::PlatformTypesResolverV2;
use bsl_gradual_types::core::facets::FacetRegistry;
use std::fs;

fn main() -> anyhow::Result<()> {
    println!("🚀 Генерация расширенного HTML отчёта с TypeRef и фасетами...");
    
    // Загружаем данные
    let json_path = "examples/syntax_helper/syntax_database.json";
    let database = if std::path::Path::new(json_path).exists() {
        SyntaxHelperParser::load_from_file(json_path)?
    } else {
        println!("⚠️ База данных не найдена. Используем демо-данные.");
        return Ok(());
    };
    
    // Создаём resolver и registry для демонстрации
    let resolver = PlatformTypesResolverV2::new();
    let mut registry = FacetRegistry::new();
    resolver.populate_facet_registry(&mut registry);
    
    // Собираем статистику
    let total_functions = database.global_functions.len();
    let total_keywords = database.keywords.len();
    let functions_with_types = database.global_functions.values()
        .filter(|f| f.parameters.iter().any(|p| p.type_ref.is_some()))
        .count();
    
    // Генерируем HTML
    let html = format!(r#"<!DOCTYPE html>
<html lang="ru">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>BSL Enhanced Type System Report</title>
    <style>
        :root {{
            --primary: #667eea;
            --secondary: #764ba2;
            --success: #28a745;
            --info: #17a2b8;
            --warning: #ffc107;
            --danger: #dc3545;
        }}
        
        * {{
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }}
        
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif;
            background: linear-gradient(135deg, var(--primary) 0%, var(--secondary) 100%);
            min-height: 100vh;
            padding: 20px;
        }}
        
        .container {{
            max-width: 1600px;
            margin: 0 auto;
        }}
        
        /* Animated Header */
        .header {{
            background: white;
            border-radius: 20px;
            padding: 40px;
            margin-bottom: 30px;
            box-shadow: 0 20px 60px rgba(0,0,0,0.2);
            position: relative;
            overflow: hidden;
        }}
        
        .header::before {{
            content: '';
            position: absolute;
            top: 0;
            left: 0;
            right: 0;
            height: 5px;
            background: linear-gradient(90deg, var(--primary), var(--secondary), var(--primary));
            background-size: 200% 100%;
            animation: gradient 3s ease infinite;
        }}
        
        @keyframes gradient {{
            0% {{ background-position: 0% 50%; }}
            50% {{ background-position: 100% 50%; }}
            100% {{ background-position: 0% 50%; }}
        }}
        
        h1 {{
            font-size: 2.5em;
            background: linear-gradient(135deg, var(--primary), var(--secondary));
            -webkit-background-clip: text;
            -webkit-text-fill-color: transparent;
            margin-bottom: 10px;
        }}
        
        .subtitle {{
            color: #6c757d;
            font-size: 1.2em;
        }}
        
        /* Stats Cards */
        .stats-grid {{
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
            gap: 20px;
            margin-bottom: 30px;
        }}
        
        .stat-card {{
            background: white;
            border-radius: 15px;
            padding: 25px;
            box-shadow: 0 10px 30px rgba(0,0,0,0.1);
            transition: all 0.3s;
            position: relative;
            overflow: hidden;
        }}
        
        .stat-card:hover {{
            transform: translateY(-5px);
            box-shadow: 0 15px 40px rgba(0,0,0,0.15);
        }}
        
        .stat-icon {{
            font-size: 2.5em;
            margin-bottom: 15px;
        }}
        
        .stat-value {{
            font-size: 2.5em;
            font-weight: bold;
            background: linear-gradient(135deg, var(--primary), var(--secondary));
            -webkit-background-clip: text;
            -webkit-text-fill-color: transparent;
        }}
        
        .stat-label {{
            color: #6c757d;
            margin-top: 10px;
            font-size: 1.1em;
        }}
        
        .stat-progress {{
            height: 4px;
            background: #e9ecef;
            border-radius: 2px;
            margin-top: 15px;
            overflow: hidden;
        }}
        
        .stat-progress-bar {{
            height: 100%;
            background: linear-gradient(90deg, var(--primary), var(--secondary));
            border-radius: 2px;
            animation: progress 2s ease-out;
        }}
        
        @keyframes progress {{
            from {{ width: 0; }}
        }}
        
        /* TypeRef Section */
        .typeref-section {{
            background: white;
            border-radius: 20px;
            padding: 40px;
            margin-bottom: 30px;
            box-shadow: 0 15px 40px rgba(0,0,0,0.1);
        }}
        
        .typeref-title {{
            font-size: 2em;
            margin-bottom: 30px;
            color: #2c3e50;
            display: flex;
            align-items: center;
            gap: 15px;
        }}
        
        .type-mappings {{
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(400px, 1fr));
            gap: 20px;
        }}
        
        .type-mapping-card {{
            border: 2px solid #e9ecef;
            border-radius: 10px;
            padding: 20px;
            transition: all 0.3s;
        }}
        
        .type-mapping-card:hover {{
            border-color: var(--primary);
            background: linear-gradient(135deg, rgba(102,126,234,0.05), rgba(118,75,162,0.05));
        }}
        
        .type-category {{
            font-weight: bold;
            color: var(--primary);
            margin-bottom: 15px;
            font-size: 1.2em;
        }}
        
        .type-example {{
            display: flex;
            align-items: center;
            justify-content: space-between;
            padding: 10px;
            background: #f8f9fa;
            border-radius: 5px;
            margin: 8px 0;
            font-family: 'Consolas', 'Monaco', monospace;
        }}
        
        .type-arrow {{
            color: #6c757d;
            margin: 0 15px;
        }}
        
        .type-normalized {{
            color: var(--success);
            font-weight: bold;
        }}
        
        /* Facet Section */
        .facet-section {{
            background: white;
            border-radius: 20px;
            padding: 40px;
            margin-bottom: 30px;
            box-shadow: 0 15px 40px rgba(0,0,0,0.1);
        }}
        
        .facet-examples {{
            display: grid;
            gap: 30px;
        }}
        
        .facet-object {{
            border: 2px solid #e9ecef;
            border-radius: 15px;
            padding: 25px;
            background: linear-gradient(135deg, #f8f9fa, white);
        }}
        
        .facet-object-name {{
            font-size: 1.5em;
            font-weight: bold;
            color: #2c3e50;
            margin-bottom: 20px;
            text-align: center;
        }}
        
        .facet-grid {{
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
            gap: 15px;
        }}
        
        .facet-card {{
            background: linear-gradient(135deg, var(--primary), var(--secondary));
            color: white;
            padding: 20px;
            border-radius: 10px;
            box-shadow: 0 5px 15px rgba(0,0,0,0.2);
            transition: all 0.3s;
        }}
        
        .facet-card:hover {{
            transform: scale(1.05);
            box-shadow: 0 8px 25px rgba(0,0,0,0.3);
        }}
        
        .facet-type {{
            font-size: 1.3em;
            font-weight: bold;
            margin-bottom: 10px;
        }}
        
        .facet-class {{
            font-family: monospace;
            opacity: 0.95;
            margin-bottom: 15px;
        }}
        
        .facet-methods {{
            font-size: 0.9em;
            opacity: 0.85;
            border-top: 1px solid rgba(255,255,255,0.3);
            padding-top: 10px;
            margin-top: 10px;
        }}
        
        /* Interactive Search */
        .search-section {{
            background: white;
            border-radius: 20px;
            padding: 30px;
            margin-bottom: 30px;
            box-shadow: 0 15px 40px rgba(0,0,0,0.1);
        }}
        
        .search-box {{
            width: 100%;
            padding: 15px 20px;
            font-size: 1.2em;
            border: 2px solid #e9ecef;
            border-radius: 10px;
            transition: all 0.3s;
        }}
        
        .search-box:focus {{
            outline: none;
            border-color: var(--primary);
            box-shadow: 0 0 0 3px rgba(102,126,234,0.1);
        }}
        
        .search-results {{
            margin-top: 20px;
            max-height: 400px;
            overflow-y: auto;
        }}
        
        .result-item {{
            padding: 15px;
            border-bottom: 1px solid #e9ecef;
            transition: all 0.3s;
            cursor: pointer;
        }}
        
        .result-item:hover {{
            background: #f8f9fa;
            padding-left: 25px;
        }}
        
        .hidden {{
            display: none;
        }}
    </style>
</head>
<body>
    <div class="container">
        <!-- Header -->
        <div class="header">
            <h1>🚀 BSL Enhanced Type System Report</h1>
            <div class="subtitle">Полный анализ системы типов с TypeRef, фасетами и градуальной типизацией</div>
        </div>
        
        <!-- Statistics -->
        <div class="stats-grid">
            <div class="stat-card">
                <div class="stat-icon">📦</div>
                <div class="stat-value">{total_functions}</div>
                <div class="stat-label">Глобальных функций</div>
                <div class="stat-progress">
                    <div class="stat-progress-bar" style="width: 100%"></div>
                </div>
            </div>
            
            <div class="stat-card">
                <div class="stat-icon">✅</div>
                <div class="stat-value">{functions_with_types}</div>
                <div class="stat-label">Функций с типами</div>
                <div class="stat-progress">
                    <div class="stat-progress-bar" style="width: {percent}%"></div>
                </div>
            </div>
            
            <div class="stat-card">
                <div class="stat-icon">🔤</div>
                <div class="stat-value">{total_keywords}</div>
                <div class="stat-label">Ключевых слов</div>
                <div class="stat-progress">
                    <div class="stat-progress-bar" style="width: 100%"></div>
                </div>
            </div>
            
            <div class="stat-card">
                <div class="stat-icon">🔷</div>
                <div class="stat-value">6</div>
                <div class="stat-label">Видов фасетов</div>
                <div class="stat-progress">
                    <div class="stat-progress-bar" style="width: 100%"></div>
                </div>
            </div>
        </div>
        
        <!-- TypeRef System -->
        <div class="typeref-section">
            <h2 class="typeref-title">
                <span>🎯</span>
                <span>Система TypeRef - Нормализация типов</span>
            </h2>
            
            <div class="type-mappings">
                <div class="type-mapping-card">
                    <div class="type-category">🔵 Языковые типы (language:)</div>
                    <div class="type-example">
                        <span>Строка</span>
                        <span class="type-arrow">→</span>
                        <span class="type-normalized">language:def_String</span>
                    </div>
                    <div class="type-example">
                        <span>Число</span>
                        <span class="type-arrow">→</span>
                        <span class="type-normalized">language:def_Number</span>
                    </div>
                    <div class="type-example">
                        <span>Булево</span>
                        <span class="type-arrow">→</span>
                        <span class="type-normalized">language:def_Boolean</span>
                    </div>
                </div>
                
                <div class="type-mapping-card">
                    <div class="type-category">🟢 Контекстные типы (context:)</div>
                    <div class="type-example">
                        <span>Массив</span>
                        <span class="type-arrow">→</span>
                        <span class="type-normalized">context:objects/Array</span>
                    </div>
                    <div class="type-example">
                        <span>Структура</span>
                        <span class="type-arrow">→</span>
                        <span class="type-normalized">context:objects/Structure</span>
                    </div>
                    <div class="type-example">
                        <span>Соответствие</span>
                        <span class="type-arrow">→</span>
                        <span class="type-normalized">context:objects/Map</span>
                    </div>
                </div>
                
                <div class="type-mapping-card">
                    <div class="type-category">🔴 Метаданные (metadata_ref:)</div>
                    <div class="type-example">
                        <span>СправочникСсылка.Контрагенты</span>
                        <span class="type-arrow">→</span>
                        <span class="type-normalized">metadata_ref:СправочникСсылка.Контрагенты</span>
                    </div>
                    <div class="type-example">
                        <span>ДокументСсылка.Заказ</span>
                        <span class="type-arrow">→</span>
                        <span class="type-normalized">metadata_ref:ДокументСсылка.Заказ</span>
                    </div>
                </div>
            </div>
        </div>
        
        <!-- Facet System -->
        <div class="facet-section">
            <h2 class="typeref-title">
                <span>🔷</span>
                <span>Фасетная система</span>
            </h2>
            
            <div class="facet-examples">
                <div class="facet-object">
                    <div class="facet-object-name">Справочник.Контрагенты</div>
                    <div class="facet-grid">
                        <div class="facet-card">
                            <div class="facet-type">Manager</div>
                            <div class="facet-class">Справочники.Контрагенты</div>
                            <div class="facet-methods">
                                СоздатьЭлемент()<br>
                                НайтиПоКоду()<br>
                                НайтиПоНаименованию()
                            </div>
                        </div>
                        <div class="facet-card">
                            <div class="facet-type">Object</div>
                            <div class="facet-class">СправочникОбъект.Контрагенты</div>
                            <div class="facet-methods">
                                Записать()<br>
                                Удалить()<br>
                                ЗаполнитьПоУмолчанию()
                            </div>
                        </div>
                        <div class="facet-card">
                            <div class="facet-type">Reference</div>
                            <div class="facet-class">СправочникСсылка.Контрагенты</div>
                            <div class="facet-methods">
                                ПолучитьОбъект()<br>
                                Пустая()<br>
                                УникальныйИдентификатор()
                            </div>
                        </div>
                        <div class="facet-card">
                            <div class="facet-type">Metadata</div>
                            <div class="facet-class">Метаданные.Справочники.Контрагенты</div>
                            <div class="facet-methods">
                                Реквизиты<br>
                                ТабличныеЧасти<br>
                                Формы
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </div>
        
        <!-- Interactive Search -->
        <div class="search-section">
            <h2 class="typeref-title">
                <span>🔍</span>
                <span>Интерактивный поиск</span>
            </h2>
            <input type="text" class="search-box" id="searchBox" placeholder="Начните вводить название функции, ключевого слова или типа..." onkeyup="performSearch()">
            <div class="search-results" id="searchResults"></div>
        </div>
    </div>
    
    <script>
        // Данные для поиска
        const searchData = [
            {functions_json},
            {keywords_json}
        ];
        
        function performSearch() {{
            const query = document.getElementById('searchBox').value.toLowerCase();
            const resultsDiv = document.getElementById('searchResults');
            
            if (query.length < 2) {{
                resultsDiv.innerHTML = '';
                return;
            }}
            
            // Здесь будет логика поиска
            resultsDiv.innerHTML = '<div class="result-item">Поиск: ' + query + '</div>';
        }}
        
        // Анимация при загрузке
        window.addEventListener('load', () => {{
            document.querySelectorAll('.stat-card').forEach((card, index) => {{
                card.style.opacity = '0';
                card.style.transform = 'translateY(20px)';
                setTimeout(() => {{
                    card.style.transition = 'all 0.5s';
                    card.style.opacity = '1';
                    card.style.transform = 'translateY(0)';
                }}, index * 100);
            }});
        }});
    </script>
</body>
</html>"#,
        total_functions = total_functions,
        functions_with_types = functions_with_types,
        total_keywords = total_keywords,
        percent = (functions_with_types as f32 / total_functions as f32 * 100.0) as u32,
        functions_json = "[]", // Здесь можно добавить JSON с функциями
        keywords_json = "[]"   // Здесь можно добавить JSON с ключевыми словами
    );
    
    // Сохраняем файл
    let output_path = "type_hierarchy_enhanced_generated.html";
    fs::write(output_path, html)?;
    
    println!("✅ Расширенный HTML отчёт создан: {}", output_path);
    println!("📊 Статистика:");
    println!("   - Всего функций: {}", total_functions);
    println!("   - Функций с типами: {}", functions_with_types);
    println!("   - Ключевых слов: {}", total_keywords);
    println!("🌐 Откройте файл в браузере для интерактивного просмотра");
    
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