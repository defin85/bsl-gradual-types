//! Парсер типов из HTML документов синтакс-помощника 1С
//!
//! Модуль отвечает за извлечение и нормализацию типов из HTML:
//! - Простых типов: `Строка`, `Число`
//! - Union типов: `Число | Строка`
//! - Faceted типов: `СправочникСсылка.<T>`
//! - Комбинаций: `СправочникСсылка.<T> | Неопределено`

use scraper::{ElementRef, Html};
use tracing::debug;

/// Разделитель union типов
pub const UNION_SEPARATOR: &str = " | ";

/// Фрагмент типа из HTML для DOM-based парсера
///
/// Используется для парсинга строки типа в формате:
/// `<a>СправочникСсылка.</a><span>&lt;</span><a>Имя справочника</a><span>&gt;</span>, <a>Неопределено</a>`
#[derive(Debug, Clone, PartialEq)]
pub enum TypeFragment {
    /// Имя типа из <a> тега
    TypeName(String),
    /// Открывающая угловая скобка <span>&lt;</span> или <
    GenericOpen,
    /// Закрывающая угловая скобка <span>&gt;</span> или >
    GenericClose,
    /// Разделитель union типов (запятая)
    UnionSeparator,
    /// Прочий текст
    Text(String),
}

/// Парсер типов из HTML синтакс-помощника
///
/// Отвечает за:
/// - Поиск секции "Возвращаемое значение:" в HTML
/// - Извлечение строки типа от "Тип:" до "<br>"
/// - Парсинг HTML фрагментов в структурированные `TypeFragment`
/// - Сборку faceted и union типов с нормализацией placeholder'ов
pub struct TypeParser;

impl TypeParser {
    /// Парсит возвращаемый тип из HTML документа
    ///
    /// Основной entry point для парсинга типов. Использует DOM-based парсер
    /// для корректной обработки всех форматов типов.
    ///
    /// # Arguments
    ///
    /// * `html_text` - HTML текст документа
    ///
    /// # Returns
    ///
    /// * `(Option<String>, Option<String>)` - кортеж (тип, описание)
    ///   - `Some(type_string)` - нормализованный тип, например "СправочникСсылка.<T> | Неопределено"
    ///   - `None` - если секция не найдена или тип пуст
    pub fn parse_return_type(html_text: &str) -> (Option<String>, Option<String>) {
        // Проверяем наличие раздела "Возвращаемое значение:"
        if !html_text.contains("Возвращаемое значение:") && !html_text.contains("Return value:")
        {
            return (None, None);
        }

        // ШАГ 1: Найти секцию "Возвращаемое значение:"
        let return_section = match Self::find_return_value_section(html_text) {
            Some(section) => section,
            None => {
                debug!("⚠️  Секция 'Возвращаемое значение' не найдена");
                return (None, None);
            }
        };

        // ШАГ 2: Извлечь строку типа от "Тип:" до "<br>"
        let (type_line, description) = Self::extract_type_line(&return_section);
        if type_line.is_empty() {
            debug!("⚠️  Строка 'Тип:' не найдена в секции возвращаемого значения");
            return (None, None);
        }

        // ШАГ 3: Парсить HTML фрагменты типа
        let fragments = Self::parse_type_fragments(&type_line);
        if fragments.is_empty() {
            debug!("⚠️  Не удалось извлечь фрагменты типа");
            return (None, None);
        }

        // ШАГ 4: Собрать faceted типы и union
        let return_type = Self::assemble_types(&fragments);

        debug!(
            "✓ Извлечён возвращаемый тип: {} {:?}",
            return_type, description
        );

        (Some(return_type), description)
    }

    /// Найти секцию "Возвращаемое значение:" в HTML
    ///
    /// Ищет начало секции по маркеру и возвращает текст до следующего заголовка
    /// или конца документа.
    ///
    /// # Arguments
    ///
    /// * `html_text` - HTML текст документа
    ///
    /// # Returns
    ///
    /// * `Some(String)` - содержимое секции "Возвращаемое значение:"
    /// * `None` - если секция не найдена
    pub fn find_return_value_section(html_text: &str) -> Option<String> {
        // Ищем начало секции
        const RU_RETURN: &str = "Возвращаемое значение:";
        const EN_RETURN: &str = "Return value:";

        let section_start = html_text
            .find(RU_RETURN)
            .map(|p| p + RU_RETURN.len())
            .or_else(|| html_text.find(EN_RETURN).map(|p| p + EN_RETURN.len()))?;

        let remaining = &html_text[section_start..];

        // Находим конец секции (следующий заголовок или конец)
        let section_end = remaining
            .find("<p class=\"V8SH_chapter\">")
            .or_else(|| remaining.find("</body>"))
            .unwrap_or(remaining.len());

        Some(remaining[..section_end].to_string())
    }

    /// Извлечь строку типа от "Тип:" до "<br>" и описание после <br>
    ///
    /// Парсит секцию возвращаемого значения, извлекая:
    /// 1. HTML-строку типа (содержит <a>, <span> теги)
    /// 2. Текстовое описание после <br>
    ///
    /// # Arguments
    ///
    /// * `section` - содержимое секции "Возвращаемое значение:"
    ///
    /// # Returns
    ///
    /// * `(String, Option<String>)` - кортеж (HTML строка типа, описание)
    ///   - Первый элемент - HTML от "Тип:" до "<br>"
    ///   - Второй элемент - текст описания после <br> (если есть)
    pub fn extract_type_line(section: &str) -> (String, Option<String>) {
        const RU_TYPE: &str = "Тип:";
        const EN_TYPE: &str = "Type:";

        // Находим "Тип:"
        let (type_start, marker_len) = section
            .find(RU_TYPE)
            .map(|p| (p + RU_TYPE.len(), RU_TYPE.len()))
            .or_else(|| {
                section
                    .find(EN_TYPE)
                    .map(|p| (p + EN_TYPE.len(), EN_TYPE.len()))
            })
            .unwrap_or((0, 0));

        if marker_len == 0 {
            return (String::new(), None);
        }

        let after_type = &section[type_start..];

        // Находим конец строки типа (до <br> или <br/> или <br >)
        let type_end = after_type.find("<br").unwrap_or(after_type.len());

        let type_line = after_type[..type_end].to_string();

        // Извлекаем описание после <br>
        let description = if let Some(br_pos) = after_type.find("<br") {
            // Пропускаем <br>, <br/>, <br >
            let after_br = &after_type[br_pos..];
            let desc_start = after_br.find('>').map(|p| p + 1).unwrap_or(0);
            let desc_text = &after_br[desc_start..];

            // Берём текст до следующего тега или конца
            let desc_end = desc_text.find('<').unwrap_or(desc_text.len());

            let clean_desc = desc_text[..desc_end]
                .replace("&nbsp;", " ")
                .trim()
                .to_string();

            if clean_desc.is_empty() {
                None
            } else {
                Some(clean_desc)
            }
        } else {
            None
        };

        (type_line, description)
    }

    /// Парсить HTML в вектор фрагментов типа
    ///
    /// Использует DOM-парсер (scraper) для обхода HTML и извлечения
    /// структурированных фрагментов типа.
    ///
    /// # Arguments
    ///
    /// * `type_line` - HTML строка типа (от "Тип:" до "<br>")
    ///
    /// # Returns
    ///
    /// * `Vec<TypeFragment>` - вектор фрагментов:
    ///   - `TypeName` - имена типов из <a> тегов
    ///   - `GenericOpen`/`GenericClose` - угловые скобки из <span>
    ///   - `UnionSeparator` - запятые между типами
    ///   - `Text` - прочий текст
    pub fn parse_type_fragments(type_line: &str) -> Vec<TypeFragment> {
        let mut fragments = Vec::new();

        // Парсим HTML фрагмент
        let fragment = Html::parse_fragment(type_line);

        // Обходим все узлы
        for node in fragment.root_element().descendants() {
            match node.value() {
                scraper::node::Node::Element(elem) => {
                    if elem.name() == "a" {
                        // Извлекаем текст из <a> тега
                        if let Some(elem_ref) = ElementRef::wrap(node) {
                            let text: String = elem_ref.text().collect();
                            let text = text.trim();
                            if !text.is_empty() {
                                fragments.push(TypeFragment::TypeName(text.to_string()));
                            }
                        }
                    } else if elem.name() == "span" {
                        // Проверяем содержимое <span> на &lt; или &gt;
                        if let Some(elem_ref) = ElementRef::wrap(node) {
                            let text: String = elem_ref.text().collect();
                            let text = text.trim();
                            if text == "<" || text == "&lt;" {
                                fragments.push(TypeFragment::GenericOpen);
                            } else if text == ">" || text == "&gt;" {
                                fragments.push(TypeFragment::GenericClose);
                            }
                        }
                    }
                }
                scraper::node::Node::Text(text) => {
                    let t = text.text.trim();
                    // Проверяем на разделители и специальные символы
                    if t.contains(',') {
                        // Может быть запятая между типами
                        fragments.push(TypeFragment::UnionSeparator);
                    } else if t == "<" || t == "&lt;" {
                        fragments.push(TypeFragment::GenericOpen);
                    } else if t == ">" || t == "&gt;" {
                        fragments.push(TypeFragment::GenericClose);
                    } else if !t.is_empty() && t != "." && t != " " {
                        fragments.push(TypeFragment::Text(t.to_string()));
                    }
                }
                _ => {}
            }
        }

        fragments
    }

    /// Собрать faceted типы и union из фрагментов
    ///
    /// Обрабатывает вектор `TypeFragment` и собирает финальную строку типа:
    /// - Placeholder'ы внутри `<...>` нормализуются в `<T>`
    /// - Union типы объединяются через ` | `
    /// - Trailing точки удаляются
    ///
    /// # Arguments
    ///
    /// * `fragments` - вектор фрагментов типа из `parse_type_fragments`
    ///
    /// # Returns
    ///
    /// * `String` - нормализованная строка типа, например:
    ///   - `"Строка"` - простой тип
    ///   - `"Число | Строка"` - union тип
    ///   - `"СправочникСсылка.<T>"` - faceted тип
    ///   - `"СправочникСсылка.<T> | Неопределено"` - faceted + union
    ///
    /// # Robustness
    ///
    /// Метод корректно обрабатывает malformed HTML:
    /// - Unclosed generic (GenericOpen без GenericClose) - тип добавляется без паники
    /// - Nested generics - обрабатываются последовательно
    pub fn assemble_types(fragments: &[TypeFragment]) -> String {
        let mut types: Vec<String> = Vec::new();
        let mut current_type = String::new();
        let mut in_generic = false;

        for fragment in fragments {
            match fragment {
                TypeFragment::TypeName(name) => {
                    if in_generic {
                        // Это placeholder внутри <...> - нормализуем в <T>
                        // Debug лог для отладки нормализации
                        debug!(
                            "Placeholder внутри generic: '{}' -> будет нормализован в <T>",
                            name
                        );
                        // Просто продолжаем, т.к. добавим <T> при закрытии
                    } else {
                        // Добавляем имя типа
                        if !current_type.is_empty() && !current_type.ends_with('.') {
                            // Если предыдущий тип завершён, сохраняем его
                            types.push(current_type.clone());
                            current_type.clear();
                        }
                        current_type.push_str(name);
                    }
                }
                TypeFragment::GenericOpen => {
                    in_generic = true;
                }
                TypeFragment::GenericClose => {
                    if in_generic {
                        // Добавляем нормализованный generic параметр
                        current_type.push_str("<T>");
                        in_generic = false;
                    }
                }
                TypeFragment::UnionSeparator => {
                    // Сохраняем текущий тип и начинаем новый
                    if !current_type.is_empty() {
                        // Убираем trailing точку если есть
                        if current_type.ends_with('.') {
                            current_type.pop();
                        }
                        types.push(current_type.clone());
                        current_type.clear();
                    }
                    // Reset in_generic flag при переходе к следующему типу в union
                    in_generic = false;
                }
                TypeFragment::Text(_) => {
                    // Игнорируем прочий текст
                }
            }
        }

        // Добавляем последний тип
        // ВАЖНО: Если generic не закрыт (malformed HTML), всё равно добавляем тип
        if !current_type.is_empty() {
            if current_type.ends_with('.') {
                current_type.pop();
            }
            // Если generic был открыт но не закрыт, добавляем <T> для robustness
            if in_generic {
                debug!(
                    "⚠️  Unclosed generic detected, adding <T> for robustness: {}",
                    current_type
                );
                current_type.push_str("<T>");
            }
            types.push(current_type);
        }

        // Объединяем через UNION_SEPARATOR
        types.join(UNION_SEPARATOR)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Тест парсинга простого типа
    #[test]
    fn test_assemble_types_simple() {
        let fragments = vec![TypeFragment::TypeName("Строка".to_string())];
        assert_eq!(TypeParser::assemble_types(&fragments), "Строка");
    }

    /// Тест парсинга union типа
    #[test]
    fn test_assemble_types_union() {
        let fragments = vec![
            TypeFragment::TypeName("Число".to_string()),
            TypeFragment::UnionSeparator,
            TypeFragment::TypeName("Строка".to_string()),
        ];
        assert_eq!(TypeParser::assemble_types(&fragments), "Число | Строка");
    }

    /// Тест парсинга faceted типа
    #[test]
    fn test_assemble_types_faceted() {
        let fragments = vec![
            TypeFragment::TypeName("СправочникСсылка.".to_string()),
            TypeFragment::GenericOpen,
            TypeFragment::TypeName("Имя справочника".to_string()),
            TypeFragment::GenericClose,
        ];
        assert_eq!(
            TypeParser::assemble_types(&fragments),
            "СправочникСсылка.<T>"
        );
    }

    /// Тест парсинга faceted + union
    #[test]
    fn test_assemble_types_faceted_union() {
        let fragments = vec![
            TypeFragment::TypeName("СправочникСсылка.".to_string()),
            TypeFragment::GenericOpen,
            TypeFragment::TypeName("Имя справочника".to_string()),
            TypeFragment::GenericClose,
            TypeFragment::UnionSeparator,
            TypeFragment::TypeName("Неопределено".to_string()),
        ];
        assert_eq!(
            TypeParser::assemble_types(&fragments),
            "СправочникСсылка.<T> | Неопределено"
        );
    }

    /// Тест для malformed HTML: unclosed generic
    ///
    /// Проверяет, что парсер корректно обрабатывает ситуацию,
    /// когда GenericOpen не имеет парного GenericClose
    #[test]
    fn test_assemble_types_unclosed_generic() {
        // Malformed: открыли generic, но не закрыли
        let fragments = vec![
            TypeFragment::TypeName("СправочникСсылка.".to_string()),
            TypeFragment::GenericOpen,
            TypeFragment::TypeName("Имя".to_string()),
            // GenericClose отсутствует
        ];

        let result = TypeParser::assemble_types(&fragments);

        // Тип должен быть возвращен БЕЗ паники
        assert!(!result.is_empty(), "Результат не должен быть пустым");

        // Проверяем, что добавлен <T> для robustness
        // Trailing точка удаляется при финализации типа, затем добавляется <T>
        assert_eq!(
            result, "СправочникСсылка<T>",
            "Unclosed generic должен быть закрыт автоматически"
        );
    }

    /// Тест для unclosed generic с union separator после
    #[test]
    fn test_assemble_types_unclosed_generic_with_union() {
        let fragments = vec![
            TypeFragment::TypeName("СправочникСсылка.".to_string()),
            TypeFragment::GenericOpen,
            TypeFragment::TypeName("Имя".to_string()),
            // GenericClose отсутствует, но есть union separator
            TypeFragment::UnionSeparator,
            TypeFragment::TypeName("Строка".to_string()),
        ];

        let result = TypeParser::assemble_types(&fragments);

        // in_generic сбрасывается при UnionSeparator
        // Поэтому СправочникСсылка. без <T>, затем Строка
        assert!(!result.is_empty(), "Результат не должен быть пустым");
        assert!(
            result.contains("Строка"),
            "Union тип должен содержать 'Строка'"
        );
    }

    /// Тест парсинга фрагментов из HTML
    #[test]
    fn test_parse_type_fragments() {
        let type_line = r#" <a href="...">СправочникСсылка.</a><span>&lt;</span><a href="...">Имя справочника</a><span>&gt;</span>"#;
        let fragments = TypeParser::parse_type_fragments(type_line);

        // Должны быть: TypeName("СправочникСсылка."), GenericOpen, TypeName("Имя справочника"), GenericClose
        assert!(fragments
            .iter()
            .any(|f| matches!(f, TypeFragment::TypeName(s) if s == "СправочникСсылка.")));
        assert!(fragments
            .iter()
            .any(|f| matches!(f, TypeFragment::GenericOpen)));
        assert!(fragments
            .iter()
            .any(|f| matches!(f, TypeFragment::TypeName(s) if s == "Имя справочника")));
        assert!(fragments
            .iter()
            .any(|f| matches!(f, TypeFragment::GenericClose)));
    }

    /// Тест извлечения строки типа
    #[test]
    fn test_extract_type_line() {
        let section = r#"
            Тип: <a href="type">Строка</a>. <br>
            Описание результата.
        "#;

        let (type_line, description) = TypeParser::extract_type_line(section);

        assert!(type_line.contains("<a"));
        assert!(type_line.contains("Строка"));
        assert_eq!(description, Some("Описание результата.".to_string()));
    }

    /// Тест поиска секции возвращаемого значения
    #[test]
    fn test_find_return_value_section() {
        let html = r#"
            <p class="V8SH_chapter">Параметры:</p>
            ...
            <p class="V8SH_chapter">Возвращаемое значение:</p>
            Тип: <a href="type">Строка</a>. <br>
            Описание.
            <p class="V8SH_chapter">Пример:</p>
        "#;

        let section = TypeParser::find_return_value_section(html);

        assert!(section.is_some());
        let section = section.unwrap();
        assert!(section.contains("Тип:"));
        assert!(section.contains("Строка"));
        assert!(!section.contains("Пример:"));
    }

    /// Тест полного парсинга возвращаемого типа
    #[test]
    fn test_parse_return_type_full() {
        let html = r#"
            <html>
            <p class="V8SH_chapter">Возвращаемое значение:</p>
            Тип: <a href="type">СправочникСсылка.</a><span>&lt;</span><a href="type">Имя</a><span>&gt;</span>, <a href="type">Неопределено</a>. <br>
            Описание результата.
            </html>
        "#;

        let (return_type, description) = TypeParser::parse_return_type(html);

        assert_eq!(
            return_type,
            Some("СправочникСсылка.<T> | Неопределено".to_string())
        );
        assert_eq!(description, Some("Описание результата.".to_string()));
    }

    /// Тест: нет секции возвращаемого значения
    #[test]
    fn test_parse_return_type_no_section() {
        let html = r#"
            <html>
            <p class="V8SH_chapter">Параметры:</p>
            Только параметры, нет возвращаемого значения.
            </html>
        "#;

        let (return_type, description) = TypeParser::parse_return_type(html);

        assert_eq!(return_type, None);
        assert_eq!(description, None);
    }
}
