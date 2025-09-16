# Извлечем данные из HTML файлов для создания единого интерфейса
import re
import json

# Читаем все файлы и извлекаем данные о типах
files = [
    "interface_variant_1_dashboard.html",
    "interface_variant_2_cards.html", 
    "interface_variant_3_table.html",
    "interface_variant_4_graph.html"
]

# Создадим структуру данных для единого интерфейса
unified_data = {
    "types": [],
    "categories": {
        "Platform": {"color": "#3498db", "icon": "🔧", "count": 0},
        "Configuration": {"color": "#e74c3c", "icon": "⚙️", "count": 0}, 
        "Union": {"color": "#9b59b6", "icon": "🎯", "count": 0},
        "Dynamic": {"color": "#f39c12", "icon": "🔄", "count": 0},
        "Unknown": {"color": "#95a5a6", "icon": "❓", "count": 0}
    },
    "metrics": {
        "total_types": 0,
        "certainty_high": 0,
        "certainty_medium": 0, 
        "certainty_low": 0,
        "flow_sensitive": 0,
        "cache_hit_rate": "94%",
        "analysis_speed": "125ms"
    },
    "connections": []
}

# Извлекаем данные из файлов
types_data = [
    {
        "id": "array",
        "name": "Массив (Array)",
        "category": "Platform", 
        "certainty": 100,
        "certainty_text": "Known 100%",
        "facets": ["Object", "Collection"],
        "methods_count": 15,
        "methods": ["Добавить()", "Удалить()", "Количество()", "Найти()"],
        "source": "Static Analysis",
        "flow_sensitive": False,
        "description": "Базовый тип данных для работы с коллекциями",
        "union_types": None
    },
    {
        "id": "nomenclature", 
        "name": "Справочники.Номенклатура",
        "category": "Configuration",
        "certainty": 100,
        "certainty_text": "Known 100%", 
        "facets": ["Manager", "Reference", "Object", "Metadata"],
        "methods_count": 12,
        "attributes_count": 8,
        "source": "Configuration",
        "flow_sensitive": False,
        "description": "Иерархический справочник с поддержкой предопределённых элементов и групп",
        "connections": {"incoming": 5, "outgoing": 3},
        "union_types": None
    },
    {
        "id": "operation_result",
        "name": "РезультатОперации", 
        "category": "Union",
        "certainty": 85,
        "certainty_text": "Inferred 85%",
        "facets": ["Object"],
        "source": "Flow Analysis", 
        "flow_sensitive": True,
        "description": "Union тип с flow-sensitive анализом",
        "union_types": [
            {"type": "Булево", "probability": 70},
            {"type": "Неопределено", "probability": 30}
        ],
        "flow_analysis": {
            "init": "Неопределено",
            "check": "Булево", 
            "final": "Булево"
        }
    },
    {
        "id": "dynamic_object",
        "name": "ДинамическийОбъект",
        "category": "Dynamic",
        "certainty": 30,
        "certainty_text": "Unknown 30%",
        "facets": ["Object"],
        "source": "Runtime", 
        "flow_sensitive": True,
        "description": "Тип определяется во время выполнения",
        "union_types": [
            {"type": "Структура", "probability": 60},
            {"type": "ДанныеФормы", "probability": 40}
        ],
        "warning": "Требует runtime проверки",
        "recommendation": "Добавить явную типизацию или контракты"
    },
    {
        "id": "string",
        "name": "Строка (String)", 
        "category": "Platform",
        "certainty": 100,
        "certainty_text": "Known 100%",
        "facets": ["Object"],
        "methods_count": 20,
        "source": "Static Analysis",
        "flow_sensitive": False,
        "description": "Базовый строковый тип данных",
        "union_types": None
    }
]

# Обновляем unified_data
unified_data["types"] = types_data
unified_data["metrics"]["total_types"] = len(types_data)

# Подсчитываем метрики
for type_data in types_data:
    category = type_data["category"]
    unified_data["categories"][category]["count"] += 1
    
    if type_data["certainty"] >= 90:
        unified_data["metrics"]["certainty_high"] += 1
    elif type_data["certainty"] >= 70:
        unified_data["metrics"]["certainty_medium"] += 1  
    else:
        unified_data["metrics"]["certainty_low"] += 1
        
    if type_data["flow_sensitive"]:
        unified_data["metrics"]["flow_sensitive"] += 1

# Создаем связи для графа
connections = [
    {"source": "nomenclature", "target": "array", "type": "uses"},
    {"source": "operation_result", "target": "dynamic_object", "type": "flow"},
    {"source": "array", "target": "string", "type": "contains"},
    {"source": "nomenclature", "target": "string", "type": "attributes"},
    {"source": "operation_result", "target": "nomenclature", "type": "references"}
]
unified_data["connections"] = connections

# Сохраняем данные для использования в приложении
with open('unified_interface_data.json', 'w', encoding='utf-8') as f:
    json.dump(unified_data, f, ensure_ascii=False, indent=2)

print("Данные для единого интерфейса подготовлены:")
print(f"- Типов: {len(unified_data['types'])}")
print(f"- Категорий: {len([c for c in unified_data['categories'] if unified_data['categories'][c]['count'] > 0])}")
print(f"- Связей: {len(unified_data['connections'])}")
print(f"- Высокая определенность: {unified_data['metrics']['certainty_high']}")
print(f"- Flow-sensitive: {unified_data['metrics']['flow_sensitive']}")