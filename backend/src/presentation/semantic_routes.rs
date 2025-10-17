use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Json},
    routing::get,
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::application::type_system_service::TypeSystemService;

/// Query параметры для semantic API
#[derive(Debug, Deserialize)]
pub struct SemanticQuery {
    /// Формат ответа: "json" (по умолчанию) или "html"
    #[serde(default = "default_format")]
    pub format: String,

    /// Тема для HTML: "light" или "dark"
    #[serde(default = "default_theme")]
    pub theme: String,

    /// Компактный режим для HTML
    #[serde(default)]
    pub compact: bool,
}

fn default_format() -> String {
    "json".to_string()
}

fn default_theme() -> String {
    "light".to_string()
}

/// Ответ в случае ошибки
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
}

/// Создаёт router для semantic visualization API
///
/// # Endpoints
/// - `GET /api/semantic/:file_path` - получить семантическое дерево
///   - Поддерживает query параметры: `?format=html&theme=dark&compact=false`
///
/// # Examples
/// ```text
/// GET /api/semantic/test.bsl?format=json
/// GET /api/semantic/test.bsl?format=html&theme=dark
/// ```
pub fn semantic_routes(type_service: Arc<TypeSystemService>) -> Router {
    Router::new()
        .route("/api/semantic/:file_path", get(get_semantic_tree))
        .with_state(type_service)
}

/// Handler для получения семантического дерева
///
/// # Arguments
/// - `file_path` - путь к файлу (из URL)
/// - `params` - query параметры (format, theme, compact)
/// - `type_service` - сервис типизации
///
/// # Response
/// - `format=json` → возвращает SemanticTreeDto в JSON
/// - `format=html` → возвращает HTML визуализацию
///
/// # Examples
/// ```text
/// GET /api/semantic/cli/test_semantic_viz.bsl?format=json
/// GET /api/semantic/cli/test_semantic_viz.bsl?format=html&theme=dark
/// ```
async fn get_semantic_tree(
    Path(file_path): Path<String>,
    Query(params): Query<SemanticQuery>,
    State(_type_service): State<Arc<TypeSystemService>>,
) -> impl IntoResponse {
    // TODO (MVP): Интеграция с TypeSystemService для реального парсинга
    // Для MVP возвращаем заглушки, чтобы проверить работу API

    if params.format == "html" {
        // HTML ответ
        let html = generate_html_stub(&file_path, &params.theme, params.compact);
        (StatusCode::OK, Html(html)).into_response()
    } else {
        // JSON ответ
        let json_response = generate_json_stub(&file_path);
        (StatusCode::OK, Json(json_response)).into_response()
    }
}

/// Генерирует HTML заглушку для MVP
///
/// В полной реализации здесь будет вызов:
/// ```rust,ignore
/// type_service.get_semantic_tree_html(uri, theme, compact)
/// ```
fn generate_html_stub(file_path: &str, theme: &str, compact: bool) -> String {
    let bg_color = if theme == "dark" { "#1e1e1e" } else { "#ffffff" };
    let text_color = if theme == "dark" { "#d4d4d4" } else { "#000000" };
    let compact_mode = if compact { "Да" } else { "Нет" };

    format!(
        r#"<!DOCTYPE html>
<html lang="ru">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Semantic Tree: {file_path}</title>
    <style>
        body {{
            font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
            background-color: {bg_color};
            color: {text_color};
            padding: 20px;
            margin: 0;
        }}
        h1 {{
            border-bottom: 2px solid {text_color};
            padding-bottom: 10px;
        }}
        .info {{
            background: rgba(128, 128, 128, 0.1);
            padding: 15px;
            border-radius: 5px;
            margin: 20px 0;
        }}
        .info p {{
            margin: 5px 0;
        }}
        .note {{
            background: rgba(255, 165, 0, 0.1);
            border-left: 4px solid orange;
            padding: 10px;
            margin: 20px 0;
        }}
    </style>
</head>
<body>
    <h1>📊 Семантическое дерево: {file_path}</h1>

    <div class="info">
        <p><strong>Тема:</strong> {theme}</p>
        <p><strong>Компактный режим:</strong> {compact_mode}</p>
        <p><strong>Запрос:</strong> /api/semantic/{file_path}?format=html</p>
    </div>

    <div class="note">
        <strong>⚠️ MVP заглушка</strong><br>
        Полная интеграция с TypeSystemService будет реализована после базового тестирования API.
        <br><br>
        В финальной версии здесь будет:
        <ul>
            <li>Дерево узлов (procedures, functions, variables)</li>
            <li>Таблица символов с типами</li>
            <li>Граф вызовов</li>
            <li>Метрики анализа</li>
        </ul>
    </div>

    <h2>🔜 Пример структуры</h2>
    <pre style="background: rgba(128, 128, 128, 0.1); padding: 10px; border-radius: 5px; overflow-x: auto;">
📄 {file_path}
  📦 Процедура: ОбработатьДанные(Данные, Настройки)
    └─ 📌 МассивДанных: Массив
    └─ 🔁 Цикл по МассивДанных
  ⚙️ Функция: ВычислитьСумму() → Число
    └─ 📌 Сумма: Число
    └─ 🔁 Цикл по МассивДанных
  ⚙️ Функция: ПолучитьСправочник() → СправочникСсылка.Контрагенты
    └─ 📌 Справочник: СправочникСсылка.Контрагенты
    </pre>
</body>
</html>"#,
        file_path = file_path,
        bg_color = bg_color,
        text_color = text_color,
        theme = theme,
        compact_mode = compact_mode
    )
}

/// Генерирует JSON заглушку для MVP
///
/// В полной реализации здесь будет вызов:
/// ```rust,ignore
/// type_service.get_semantic_tree(uri, include_call_graph, include_flow_sensitive)
/// ```
fn generate_json_stub(file_path: &str) -> serde_json::Value {
    serde_json::json!({
        "file_path": file_path,
        "root_nodes": [
            {
                "kind": "Procedure",
                "name": "ОбработатьДанные",
                "location": { "line": 5, "column": 1 },
                "children": [],
                "attributes": {
                    "parameter_count": "2",
                    "is_export": "false"
                }
            },
            {
                "kind": "Function",
                "name": "ВычислитьСумму",
                "location": { "line": 15, "column": 1 },
                "children": [],
                "attributes": {
                    "return_type": "Число"
                }
            }
        ],
        "symbol_table": {
            "МассивДанных": {
                "name": "МассивДанных",
                "kind": "Variable",
                "resolved_type": {
                    "name": "Массив",
                    "certainty": "Inferred",
                    "certainty_percent": 75
                },
                "scope": "Local"
            },
            "Сумма": {
                "name": "Сумма",
                "kind": "Variable",
                "resolved_type": {
                    "name": "Число",
                    "certainty": "Known",
                    "certainty_percent": 100
                },
                "scope": "Local"
            }
        },
        "call_graph": [],
        "metrics": {
            "procedure_count": 1,
            "function_count": 2,
            "variable_count": 2,
            "known_types": 1,
            "inferred_types": 1,
            "unknown_types": 0,
            "analysis_time_ms": 0
        },
        "note": "⚠️ MVP заглушка - полная интеграция с TypeSystemService в следующей итерации"
    })
}
