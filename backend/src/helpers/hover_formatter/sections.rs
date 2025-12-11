//! Форматирование секций hover (методы, свойства, табличные части)
//!
//! Содержит функции для форматирования различных секций hover content.

use bsl_shared::domain::metadata_lookup::TypeMetadataLookup;
use bsl_shared::domain::types::{ResolutionResult, TypeResolution};
use bsl_shared::formatting::DetailLevel;

use super::config::{HoverFormatConfig, HoverOutputFormat};

/// Форматирует секцию методов
pub fn format_methods_section(
    config: &HoverFormatConfig,
    resolution: &TypeResolution,
    metadata_lookup: &TypeMetadataLookup,
) -> Option<String> {
    let methods = metadata_lookup.get_methods(resolution);

    if methods.is_empty() {
        // Проверяем - если это тип-коллекция без методов, показываем warning
        let type_name = resolution.type_name();
        let collection_types = [
            "Массив",
            "Соответствие",
            "СписокЗначений",
            "Структура",
            "ТаблицаЗначений",
            "Array",
            "Map",
            "ValueList",
            "Structure",
            "ValueTable",
        ];

        if collection_types.iter().any(|t| type_name.contains(t)) {
            return Some("*Методы недоступны. Укажите путь к syntax_helper.*".to_string());
        }
        return None;
    }

    let total_count = methods.len();
    // Для DetailLevel::Detailed показываем ВСЕ методы без ограничений
    let display_count = if matches!(config.detail_level, DetailLevel::Detailed) {
        total_count
    } else {
        config.max_methods.min(total_count)
    };

    let mut method_lines = vec![format!(
        "Методы (показано {} из {}):",
        display_count, total_count
    )];

    for method in methods.iter().take(display_count) {
        let param_count = method.params.len();
        let return_str = if method.return_type.is_empty() {
            "void".to_string()
        } else {
            method.return_type.clone()
        };

        // MILESTONE 3.11 Phase 4: Context badge для методов
        let context_badge = if matches!(config.detail_level, DetailLevel::Detailed) {
            method
                .context_requirements
                .as_ref()
                .map(|req| {
                    use bsl_shared::domain::runtime_context::ContextRequirements;
                    match req {
                        ContextRequirements::ServerOnly => " (Server)",
                        ContextRequirements::ClientOnly => " (Client)",
                        ContextRequirements::Universal => " (Universal)",
                        ContextRequirements::ServerPreferred => " (Server Preferred)",
                    }
                })
                .unwrap_or("")
        } else {
            ""
        };

        let line = if param_count >= 4 {
            // Multiline формат для методов с 4+ параметров
            format_multiline_method(config, method, &return_str, context_badge)
        } else {
            // Inline формат для методов с < 4 параметров
            format_inline_method(config, method, &return_str, context_badge)
        };

        method_lines.push(line);
    }

    if total_count > display_count {
        method_lines.push(format!(
            "\n... и ещё {} методов",
            total_count - display_count
        ));
    }

    // Используем "  \n" (два пробела + \n) для Markdown hard break
    Some(method_lines.join("  \n"))
}

/// Форматирует метод в multiline формате (для 4+ параметров)
fn format_multiline_method(
    config: &HoverFormatConfig,
    method: &bsl_shared::domain::types::RawMethodData,
    return_str: &str,
    context_badge: &str,
) -> String {
    let param_count = method.params.len();
    let mut result = match config.output_format {
        HoverOutputFormat::Markdown => format!("* **{}**(\n", method.name),
        HoverOutputFormat::PlainText => format!("  - {}(\n", method.name),
    };

    for (i, param) in method.params.iter().enumerate() {
        let optional_marker = if param.is_optional { "?" } else { "" };
        let default_suffix = param
            .default_value
            .as_ref()
            .map(|v| format!(" = {}", v))
            .unwrap_or_default();
        let comma = if i < param_count - 1 { "," } else { "" };

        result.push_str(&format!(
            "    {}{}: {}{}{}\n",
            param.name, optional_marker, param.param_type, default_suffix, comma
        ));
    }

    result.push_str(&format!("  ) -> {}{}", return_str, context_badge));
    result
}

/// Форматирует метод в inline формате (для < 4 параметров)
fn format_inline_method(
    config: &HoverFormatConfig,
    method: &bsl_shared::domain::types::RawMethodData,
    return_str: &str,
    context_badge: &str,
) -> String {
    let params_str = method
        .params
        .iter()
        .map(|p| {
            let optional_marker = if p.is_optional { "?" } else { "" };
            let default_suffix = p
                .default_value
                .as_ref()
                .map(|v| format!(" = {}", v))
                .unwrap_or_default();
            format!(
                "{}{}: {}{}",
                p.name, optional_marker, p.param_type, default_suffix
            )
        })
        .collect::<Vec<_>>()
        .join(", ");

    match config.output_format {
        HoverOutputFormat::Markdown => {
            format!(
                "* **{}({})** -> {}{}",
                method.name, params_str, return_str, context_badge
            )
        }
        HoverOutputFormat::PlainText => {
            format!(
                "  - {}({}) -> {}{}",
                method.name, params_str, return_str, context_badge
            )
        }
    }
}

/// Форматирует секцию свойств
pub fn format_properties_section(
    config: &HoverFormatConfig,
    resolution: &TypeResolution,
    metadata_lookup: &TypeMetadataLookup,
) -> Option<String> {
    let properties = metadata_lookup.get_properties(resolution);

    if properties.is_empty() {
        return None;
    }

    let total_count = properties.len();
    // Для DetailLevel::Detailed показываем ВСЕ свойства без ограничений
    let display_count = if matches!(config.detail_level, DetailLevel::Detailed) {
        total_count
    } else {
        config.max_properties.min(total_count)
    };

    let mut property_lines = vec![format!(
        "Свойства (показано {} из {}):",
        display_count, total_count
    )];

    for property in properties.iter().take(display_count) {
        let line = match config.output_format {
            HoverOutputFormat::Markdown => {
                format!("* **{}**: {}", property.name, property.prop_type)
            }
            HoverOutputFormat::PlainText => {
                format!("  - {}: {}", property.name, property.prop_type)
            }
        };
        property_lines.push(line);
    }

    if total_count > display_count {
        property_lines.push(format!(
            "\n... и ещё {} свойств",
            total_count - display_count
        ));
    }

    Some(property_lines.join("  \n"))
}

/// Форматирует секцию табличных частей
pub fn format_tabular_sections_section(
    config: &HoverFormatConfig,
    resolution: &TypeResolution,
    metadata_lookup: &TypeMetadataLookup,
) -> Option<String> {
    // Только для Detailed уровня
    if !matches!(config.detail_level, DetailLevel::Detailed) {
        return None;
    }

    let sections = metadata_lookup.get_tabular_sections(resolution);

    if sections.is_empty() {
        return None;
    }

    let total = sections.len();
    let display_count = total; // Показываем все табличные части

    let mut lines = vec![format!(
        "Табличные части (показано {} из {}):",
        display_count, total
    )];

    for section in sections.iter().take(display_count) {
        let attr_count = section.attributes.len();
        let line = match config.output_format {
            HoverOutputFormat::Markdown => {
                format!("* **{}** ({} колонок)", section.name, attr_count)
            }
            HoverOutputFormat::PlainText => {
                format!("  - {} ({} колонок)", section.name, attr_count)
            }
        };
        lines.push(line);
    }

    if total > display_count {
        lines.push(format!(
            "... и ещё {} табличных частей",
            total - display_count
        ));
    }

    Some(lines.join("  \n"))
}

/// Форматирует информацию о фасете
pub fn format_facet_info(config: &HoverFormatConfig, resolution: &TypeResolution) -> Option<String> {
    // Только для Detailed уровня
    if !matches!(config.detail_level, DetailLevel::Detailed) {
        return None;
    }

    let active_facet = resolution.active_facet.as_ref()?;

    let (facet_russian, facet_description) = match active_facet {
        bsl_shared::domain::types::FacetKind::Manager => ("Менеджер", "создание, поиск элементов"),
        bsl_shared::domain::types::FacetKind::Object => ("Объект", "изменяемый объект"),
        bsl_shared::domain::types::FacetKind::Reference => ("Ссылка", "ссылка на элемент"),
        bsl_shared::domain::types::FacetKind::Selection => ("Выборка", "обход элементов"),
        bsl_shared::domain::types::FacetKind::List => ("Список", "UI представление"),
        bsl_shared::domain::types::FacetKind::Metadata => ("Метаданные", "метаданные объекта"),
        bsl_shared::domain::types::FacetKind::Constructor => ("Конструктор", "создание объектов"),
        bsl_shared::domain::types::FacetKind::Collection => ("Коллекция", "набор элементов"),
        bsl_shared::domain::types::FacetKind::Singleton => ("Одиночный", "одиночный объект"),
    };

    let mut result = format!("**Фасет:** {} ({})", facet_russian, facet_description);

    // Показать доступные фасеты для данного типа
    if !resolution.available_facets.is_empty() {
        let facets_list = resolution
            .available_facets
            .iter()
            .map(|f| match f {
                bsl_shared::domain::types::FacetKind::Manager => "Менеджер",
                bsl_shared::domain::types::FacetKind::Object => "Объект",
                bsl_shared::domain::types::FacetKind::Reference => "Ссылка",
                bsl_shared::domain::types::FacetKind::Selection => "Выборка",
                bsl_shared::domain::types::FacetKind::List => "Список",
                bsl_shared::domain::types::FacetKind::Metadata => "Метаданные",
                bsl_shared::domain::types::FacetKind::Constructor => "Конструктор",
                bsl_shared::domain::types::FacetKind::Collection => "Коллекция",
                bsl_shared::domain::types::FacetKind::Singleton => "Одиночный",
            })
            .collect::<Vec<_>>()
            .join(", ");

        result.push_str(&format!("\n\n**Доступные фасеты:** {}", facets_list));
    }

    Some(result)
}

/// Форматирует информацию о Generic типе
pub fn format_generic_info(config: &HoverFormatConfig, resolution: &TypeResolution) -> Option<String> {
    // Только для Detailed уровня
    if !matches!(config.detail_level, DetailLevel::Detailed) {
        return None;
    }

    // Проверить что это Generic тип
    if let ResolutionResult::Generic(generic) = &resolution.result {
        let params_str = generic
            .type_params
            .iter()
            .map(|p| format!("{}", p))
            .collect::<Vec<_>>()
            .join(", ");

        let mut result = format!(
            "**Generic тип:**\n* Базовый тип: {}\n* Параметры типа: {}",
            generic.base_type, params_str
        );

        // Добавить пояснение что означает Generic тип
        let explanation = match generic.base_type.as_str() {
            "Массив" | "Array" => {
                "Generic тип означает, что массив содержит элементы определённого типа"
            }
            "Соответствие" | "Map" => {
                "Generic тип означает, что соответствие хранит пары ключ-значение определённых типов"
            }
            "ТаблицаЗначений" | "ValueTable" => {
                "Generic тип означает, что строки таблицы содержат данные определённого типа"
            }
            "Список" | "List" => {
                "Generic тип означает, что список содержит элементы определённого типа"
            }
            "Структура" | "Structure" => {
                "Generic тип означает, что структура содержит поля определённых типов"
            }
            _ => "Generic тип параметризован одним или несколькими типами",
        };

        result.push_str(&format!("\n\n{}", explanation));

        return Some(result);
    }

    None
}

/// Форматирует ссылки на документацию
pub fn format_documentation_links(
    config: &HoverFormatConfig,
    resolution: &TypeResolution,
) -> Option<String> {
    // Только для Detailed уровня
    if !matches!(config.detail_level, DetailLevel::Detailed) {
        return None;
    }

    // Получить имя типа для документации
    let type_name = super::type_display::get_platform_type_name(resolution)?;

    let mut links = Vec::new();

    // 1. Ссылка на локальный Syntax Helper (если доступен)
    if let Some(path) = &config.syntax_helper_path {
        let html_path = path.join(format!("{}.html", type_name));

        if html_path.exists() {
            let file_url = format!("file:///{}", html_path.display());
            links.push(format!(
                "[Синтакс Помощник: {}]({})",
                type_name,
                file_url.replace("\\", "/") // Windows path fix
            ));
        }
    }

    // 2. Ссылка на онлайн документацию 1С
    let online_url = format!("https://docs.1c.ru/search?q={}", type_name);
    links.push(format!("[1С Platform Docs]({})", online_url));

    if links.is_empty() {
        return None;
    }

    let links_section = format!(
        "**Документация:**\n{}",
        links
            .iter()
            .map(|l| format!("* {}", l))
            .collect::<Vec<_>>()
            .join("  \n")
    );

    Some(links_section)
}
