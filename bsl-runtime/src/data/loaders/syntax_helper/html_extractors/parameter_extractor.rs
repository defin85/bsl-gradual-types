//! Извлечение параметров методов из HTML документов синтакс-помощника
//!
//! КРИТИЧНЫЙ модуль: Парсинг параметров с regex и корректной обработкой UTF-8

use regex::Regex;
use scraper::Html;
use tracing::debug;

use super::super::type_parser::UNION_SEPARATOR;
use super::super::types::{MethodOverloadInfo, ParameterInfo};

/// Экстрактор параметров методов
pub struct ParameterExtractor;

impl ParameterExtractor {
    /// Извлекает параметры метода из HTML документа
    ///
    /// Извлекает параметры из HTML в формате:
    /// ```html
    /// <p class="V8SH_chapter">Параметры:</p>
    /// <div class="V8SH_rubric">
    ///     <p>&lt;Индекс&gt; (обязательный)</div>
    /// Тип: Число. <br>
    /// Описание параметра.
    /// ```
    ///
    /// ВАЖНО: Парсер извлекает параметры ТОЛЬКО из секции "Параметры:" до "Возвращаемое значение:"
    /// чтобы избежать захвата placeholder'ов из заголовков типа "СправочникМенеджер.<Имя справочника>"
    pub fn extract_parameters(document: &Html) -> Vec<ParameterInfo> {
        let mut parameters = Vec::new();
        let html_text = document.html();

        // Проверяем наличие раздела "Параметры:"
        if !html_text.contains("Параметры:") && !html_text.contains("Parameters:") {
            debug!("Section 'Параметры' not found in HTML");
            return parameters;
        }

        // ШАГ 1: Извлекаем ТОЛЬКО секцию "Параметры:" (до следующей секции)
        let params_section = Self::extract_parameters_section(&html_text);
        if params_section.is_empty() {
            debug!("Section 'Параметры' is empty");
            return parameters;
        }

        debug!(
            "Starting parameter extraction from section ({} chars)",
            params_section.len()
        );

        // ШАГ 2: Парсер параметров внутри секции
        let param_regex = match Regex::new(
            r#"<div class="V8SH_rubric">[^<]*<p[^>]*>&lt;([^>]+)&gt;\s*\(([^)]+)\)(?:</p>)?</div>"#,
        ) {
            Ok(r) => r,
            Err(e) => {
                debug!("Failed to compile param regex: {}", e);
                return parameters;
            }
        };

        // Regex для извлечения union типов из <a> тегов
        let type_link_regex = match Regex::new(r#"<a[^>]*>([^<]+)</a>"#) {
            Ok(r) => r,
            Err(e) => {
                debug!("Failed to compile type link regex: {}", e);
                return parameters;
            }
        };

        for cap in param_regex.captures_iter(&params_section) {
            let name = cap
                .get(1)
                .map(|m| m.as_str().trim().to_string())
                .unwrap_or_default();
            let optional_text = cap.get(2).map(|m| m.as_str().trim()).unwrap_or("");

            if name.is_empty() {
                continue;
            }

            // ШАГ 3: Найти "Тип:" после этого параметра и извлечь union типы
            let match_end = cap.get(0).map(|m| m.end()).unwrap_or(0);
            let remaining = if match_end < params_section.len() {
                &params_section[match_end..]
            } else {
                ""
            };

            // Находим "Тип:" и извлекаем типы до "<br>" или следующего "<div"
            const RU_TYPE: &str = "Тип:";
            const EN_TYPE: &str = "Type:";
            let mut param_type = String::new();
            let (type_pos, type_marker_len) = remaining
                .find(RU_TYPE)
                .map(|p| (p, RU_TYPE.len()))
                .or_else(|| remaining.find(EN_TYPE).map(|p| (p, EN_TYPE.len())))
                .unwrap_or((0, 0));

            if type_marker_len > 0 {
                let type_section_start = type_pos + type_marker_len;
                let type_section_end = remaining[type_section_start..]
                    .find("<br")
                    .or_else(|| remaining[type_section_start..].find("<div"))
                    .map(|p| type_section_start + p)
                    .unwrap_or_else(|| std::cmp::min(type_section_start + 500, remaining.len()));

                let type_section = &remaining[type_section_start..type_section_end];

                // Извлекаем все типы из <a> тегов
                let types: Vec<String> = type_link_regex
                    .captures_iter(type_section)
                    .filter_map(|c| c.get(1).map(|m| m.as_str().trim().to_string()))
                    .filter(|t| !t.is_empty())
                    .collect();

                if !types.is_empty() {
                    param_type = types.join(UNION_SEPARATOR);
                } else {
                    // Fallback: если нет <a> тегов, берём текст напрямую
                    let clean_text = type_section
                        .replace("&nbsp;", " ")
                        .trim()
                        .trim_end_matches('.')
                        .to_string();
                    if !clean_text.is_empty() {
                        param_type = clean_text;
                    }
                }
            }

            // Удаляем точку в конце типа если она есть
            if param_type.ends_with('.') {
                param_type.pop();
            }
            param_type = param_type.trim().to_string();

            // Определяем опциональность
            let is_optional = optional_text.contains("необязательный")
                || optional_text.contains("optional")
                || optional_text.to_lowercase().contains("optional");

            // Извлекаем описание параметра
            let description = Self::extract_parameter_description(remaining);

            parameters.push(ParameterInfo {
                name: name.clone(),
                type_name: Some(param_type.clone()),
                is_optional,
                default_value: None,
                description,
            });

            debug!(
                "Extracted parameter: {} : {} ({})",
                name,
                param_type,
                if is_optional { "optional" } else { "required" }
            );
        }

        if parameters.is_empty() {
            debug!("No parameters extracted from HTML");
        } else {
            debug!("Extracted {} parameters", parameters.len());
        }

        parameters
    }

    /// Извлекает варианты синтаксиса (overloads) метода.
    ///
    /// В документации 1С часто встречается структура:
    /// - "Вариант синтаксиса: ..."
    /// - "Синтаксис:"
    /// - "Параметры:"
    ///   ... и так несколько раз в одном HTML.
    ///
    /// Этот метод возвращает параметры отдельно для каждого варианта, чтобы не "склеивать"
    /// несколько секций `Параметры:` в один список.
    pub fn extract_method_overloads(document: &Html) -> Vec<MethodOverloadInfo> {
        let html_text = document.html();

        // Быстрый путь: нет вариантов — возвращаем пусто или единичный вариант (если параметры есть).
        if !html_text.contains("Вариант синтаксиса:") && !html_text.contains("Syntax variant:")
        {
            let params = Self::extract_parameters(document);
            if params.is_empty() {
                return vec![];
            }
            return vec![MethodOverloadInfo {
                variant_name: None,
                parameters: params,
                description: None,
            }];
        }

        Self::extract_overloads_from_html(&html_text)
    }

    fn extract_overloads_from_html(html_text: &str) -> Vec<MethodOverloadInfo> {
        const RU_VARIANT: &str = "Вариант синтаксиса:";
        const EN_VARIANT: &str = "Syntax variant:";

        let mut overloads = Vec::new();
        let mut cursor = 0usize;

        while cursor < html_text.len() {
            let (variant_pos, marker_len) = html_text[cursor..]
                .find(RU_VARIANT)
                .map(|p| (cursor + p, RU_VARIANT.len()))
                .or_else(|| {
                    html_text[cursor..]
                        .find(EN_VARIANT)
                        .map(|p| (cursor + p, EN_VARIANT.len()))
                })
                .unwrap_or((usize::MAX, 0));

            if marker_len == 0 {
                break;
            }

            // Имя варианта: текст после маркера до следующего тега/закрытия </p>
            let after_marker = variant_pos + marker_len;
            let tail = &html_text[after_marker..];
            let end = tail
                .find("</p>")
                .or_else(|| tail.find("<p"))
                .or_else(|| tail.find("<"))
                .unwrap_or(tail.len());
            let variant_name = tail[..end]
                .replace("&nbsp;", " ")
                .trim()
                .trim_end_matches(':')
                .trim()
                .to_string();
            let variant_name = if variant_name.is_empty() {
                None
            } else {
                Some(variant_name)
            };

            // Ищем секцию параметров после этого варианта
            let slice_after_variant = &html_text[after_marker..];
            let params_marker = slice_after_variant
                .find("Параметры:</p>")
                .or_else(|| slice_after_variant.find("Parameters:</p>"));

            if let Some(params_rel) = params_marker {
                let params_start = after_marker + params_rel;

                // Границы варианта: до следующего варианта или до "Возвращаемое значение"
                let rest = &html_text[params_start..];
                let next_variant_rel = rest.find(RU_VARIANT).or_else(|| rest.find(EN_VARIANT));
                let return_rel = rest
                    .find("Возвращаемое значение:")
                    .or_else(|| rest.find("Return value:"));

                let end_rel = match (next_variant_rel, return_rel) {
                    (Some(a), Some(b)) => a.min(b),
                    (Some(a), None) => a,
                    (None, Some(b)) => b,
                    (None, None) => rest.len(),
                };

                // Вытаскиваем параметры ТОЛЬКО из этого варианта
                let section = &rest[..end_rel];
                let params_section = Self::extract_parameters_section(section);
                let parameters = Self::extract_parameters_from_section(&params_section);

                if !parameters.is_empty() {
                    overloads.push(MethodOverloadInfo {
                        variant_name,
                        parameters,
                        description: None,
                    });
                }
            }

            cursor = after_marker;
        }

        overloads
    }

    /// Извлекает секцию "Параметры:" из HTML до следующей секции
    fn extract_parameters_section(html_text: &str) -> String {
        const RU_PARAMS: &str = "Параметры:</p>";
        const EN_PARAMS: &str = "Parameters:</p>";

        let params_start = html_text
            .find(RU_PARAMS)
            .map(|p| p + RU_PARAMS.len())
            .or_else(|| html_text.find(EN_PARAMS).map(|p| p + EN_PARAMS.len()))
            .unwrap_or(0);

        if params_start == 0 {
            return String::new();
        }

        let remaining = &html_text[params_start..];
        let section_end = remaining
            .find("Возвращаемое значение:")
            .or_else(|| remaining.find("Return value:"))
            .or_else(|| remaining.find("Описание:</p>"))
            .or_else(|| remaining.find("Description:</p>"))
            .or_else(|| remaining.find("Доступность:</p>"))
            .or_else(|| remaining.find("Availability:</p>"))
            .unwrap_or(remaining.len());

        remaining[..section_end].to_string()
    }

    fn extract_parameters_from_section(params_section: &str) -> Vec<ParameterInfo> {
        let mut parameters = Vec::new();

        let param_regex = match Regex::new(
            r#"<div class="V8SH_rubric">[^<]*<p[^>]*>&lt;([^>]+)&gt;\s*\(([^)]+)\)(?:</p>)?</div>"#,
        ) {
            Ok(r) => r,
            Err(e) => {
                debug!("Failed to compile param regex: {}", e);
                return parameters;
            }
        };

        let type_link_regex = match Regex::new(r#"<a[^>]*>([^<]+)</a>"#) {
            Ok(r) => r,
            Err(e) => {
                debug!("Failed to compile type link regex: {}", e);
                return parameters;
            }
        };

        for cap in param_regex.captures_iter(params_section) {
            let name = cap
                .get(1)
                .map(|m| m.as_str().trim().to_string())
                .unwrap_or_default();
            let optional_text = cap.get(2).map(|m| m.as_str().trim()).unwrap_or("");

            if name.is_empty() {
                continue;
            }

            let match_end = cap.get(0).map(|m| m.end()).unwrap_or(0);
            let remaining = if match_end < params_section.len() {
                &params_section[match_end..]
            } else {
                ""
            };

            const RU_TYPE: &str = "Тип:";
            const EN_TYPE: &str = "Type:";
            let mut param_type = String::new();
            let (type_pos, type_marker_len) = remaining
                .find(RU_TYPE)
                .map(|p| (p, RU_TYPE.len()))
                .or_else(|| remaining.find(EN_TYPE).map(|p| (p, EN_TYPE.len())))
                .unwrap_or((0, 0));

            if type_marker_len > 0 {
                let type_section_start = type_pos + type_marker_len;
                let type_section_end = remaining[type_section_start..]
                    .find("<br")
                    .or_else(|| remaining[type_section_start..].find("<div"))
                    .map(|p| type_section_start + p)
                    .unwrap_or_else(|| std::cmp::min(type_section_start + 500, remaining.len()));

                let type_section = &remaining[type_section_start..type_section_end];

                let types: Vec<String> = type_link_regex
                    .captures_iter(type_section)
                    .filter_map(|c| c.get(1).map(|m| m.as_str().trim().to_string()))
                    .filter(|t| !t.is_empty())
                    .collect();

                if !types.is_empty() {
                    param_type = types.join(UNION_SEPARATOR);
                } else {
                    let clean_text = type_section
                        .replace("&nbsp;", " ")
                        .trim()
                        .trim_end_matches('.')
                        .to_string();
                    if !clean_text.is_empty() {
                        param_type = clean_text;
                    }
                }
            }

            if param_type.ends_with('.') {
                param_type.pop();
            }
            param_type = param_type.trim().to_string();

            let is_optional = optional_text.contains("необязательный")
                || optional_text.contains("optional")
                || optional_text.to_lowercase().contains("optional");

            let description = Self::extract_parameter_description(remaining);

            parameters.push(ParameterInfo {
                name: name.clone(),
                type_name: Some(param_type.clone()),
                is_optional,
                default_value: None,
                description,
            });
        }

        parameters
    }

    /// Извлекает описание параметра из оставшегося текста
    fn extract_parameter_description(remaining: &str) -> Option<String> {
        if let Some(br_pos) = remaining.find("<br>") {
            let text_after_br = &remaining[br_pos + 4..];
            let desc_text = if let Some(next_div) = text_after_br.find("<div") {
                text_after_br[..next_div].trim()
            } else {
                text_after_br.trim()
            };
            let clean_desc = desc_text
                .replace("<br>", " ")
                .replace("&nbsp;", " ")
                .trim()
                .to_string();
            if clean_desc.is_empty() {
                None
            } else {
                Some(clean_desc)
            }
        } else if let Some(br_pos) = remaining.find("<br") {
            let text_after_br = &remaining[br_pos..];
            if let Some(gt_pos) = text_after_br.find('>') {
                let desc = &text_after_br[gt_pos + 1..];
                let desc_text = if let Some(next_div) = desc.find("<div") {
                    desc[..next_div].trim()
                } else {
                    desc.trim()
                };
                let clean_desc = desc_text.replace("&nbsp;", " ").trim().to_string();
                if clean_desc.is_empty() {
                    None
                } else {
                    Some(clean_desc)
                }
            } else {
                None
            }
        } else {
            None
        }
    }
}
