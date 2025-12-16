//! Парсер форм конфигурации 1С (Form.xml)

use anyhow::{anyhow, Result};
use quick_xml::events::Event;
use quick_xml::Reader;
use std::fs;
use std::path::Path;
use tracing::trace;

use super::form_types::*;
use super::types::ExecutionContext;

/// Парсер форм конфигурации 1С
pub struct FormParser;

impl FormParser {
    /// Парсит Form.xml файл формы объекта метаданных
    ///
    /// # Параметры
    ///
    /// - `form_xml_path` - путь к файлу Form.xml
    /// - `owner_type` - тип владельца формы (например, "Document.ЗаказНаряды")
    /// - `form_name` - имя формы (например, "ФормаДокумента")
    ///
    /// # Возвращает
    ///
    /// `FormMetadata` с информацией о реквизитах, событиях и модуле формы
    ///
    /// # Errors
    ///
    /// Возвращает ошибку при:
    /// - Невозможности прочитать файл
    /// - Некорректном XML
    pub fn parse_form_xml(
        form_xml_path: &Path,
        owner_type: &str,
        form_name: &str,
    ) -> Result<FormMetadata> {
        trace!("📄 Parsing form: {} ({})", form_name, owner_type);

        let content = fs::read_to_string(form_xml_path)?;

        // Парсим атрибуты формы
        let attributes = Self::parse_attributes(&content)?;
        trace!("  ✅ Parsed {} attributes", attributes.len());

        // Парсим события формы
        let events = Self::parse_events(&content)?;
        trace!("  ✅ Parsed {} events", events.len());

        // Парсим UI элементы формы (ChildItems + DataPath)
        let elements = Self::parse_child_items(&content)?;
        trace!("  ✅ Parsed {} form elements", elements.len());

        // Определяем путь к Module.bsl
        // Form.xml находится в Ext/Form.xml
        // Module.bsl находится в Ext/Form/Module.bsl
        let module_path = form_xml_path
            .parent()  // Получаем Ext/
            .map(|ext_dir| ext_dir.join("Form").join("Module.bsl"));

        // Парсим Module.bsl для определения ExecutionContext (если есть)
        let execution_contexts = if let Some(ref path) = module_path {
            if path.exists() {
                Self::parse_module_contexts(path)?
            } else {
                trace!("  ℹ️ Module.bsl not found at {:?}", path);
                Vec::new()
            }
        } else {
            Vec::new()
        };

        Ok(FormMetadata {
            name: form_name.to_string(),
            owner_type: owner_type.to_string(),
            attributes,
            events,
            module_path,
            execution_contexts,
            elements,
        })
    }

    /// Парсит секции `<ChildItems>` из Form.xml и извлекает дерево UI-элементов
    ///
    /// Извлекает:
    /// - kind (имя тега)
    /// - name/id (атрибуты)
    /// - DataPath (вложенный тег)
    /// - вложенные элементы из ChildItems
    fn parse_child_items(content: &str) -> Result<Vec<FormElementBinding>> {
        let mut reader = Reader::from_str(content);
        reader.trim_text(true);

        let mut buf = Vec::new();
        let mut child_items_depth: usize = 0;

        let mut roots: Vec<FormElementBinding> = Vec::new();
        let mut stack: Vec<FormElementBinding> = Vec::new();

        let mut in_data_path = false;

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();

                    if tag_name == "ChildItems" {
                        child_items_depth += 1;
                    } else if child_items_depth > 0 {
                        if tag_name == "DataPath" {
                            in_data_path = true;
                        } else {
                            let mut name: Option<String> = None;
                            let mut id: Option<i32> = None;

                            for attr in e.attributes().flatten() {
                                let key = String::from_utf8_lossy(attr.key.as_ref());
                                let value = String::from_utf8_lossy(&attr.value);

                                if key == "name" {
                                    name = Some(value.to_string());
                                } else if key == "id" {
                                    id = value.parse::<i32>().ok();
                                }
                            }

                            // В Roadmap: парсим любые элементы с name/id/DataPath.
                            // DataPath узнаем позже, поэтому берём всё с name или id.
                            if name.is_some() || id.is_some() {
                                stack.push(FormElementBinding {
                                    kind: tag_name,
                                    name,
                                    id,
                                    data_path: None,
                                    children: Vec::new(),
                                });
                            }
                        }
                    }
                }
                Ok(Event::Empty(e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    if child_items_depth == 0 || tag_name == "ChildItems" {
                        // вне ChildItems — игнорируем
                    } else {
                        let mut name: Option<String> = None;
                        let mut id: Option<i32> = None;

                        for attr in e.attributes().flatten() {
                            let key = String::from_utf8_lossy(attr.key.as_ref());
                            let value = String::from_utf8_lossy(&attr.value);

                            if key == "name" {
                                name = Some(value.to_string());
                            } else if key == "id" {
                                id = value.parse::<i32>().ok();
                            }
                        }

                        if name.is_some() || id.is_some() {
                            let node = FormElementBinding {
                                kind: tag_name,
                                name,
                                id,
                                data_path: None,
                                children: Vec::new(),
                            };

                            if let Some(parent) = stack.last_mut() {
                                parent.children.push(node);
                            } else {
                                roots.push(node);
                            }
                        }
                    }
                }
                Ok(Event::Text(e)) => {
                    if child_items_depth > 0 && in_data_path {
                        let text = e.unescape()?.trim().to_string();
                        if !text.is_empty() {
                            if let Some(current) = stack.last_mut() {
                                current.data_path = Some(text);
                            }
                        }
                    }
                }
                Ok(Event::End(e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();

                    if tag_name == "ChildItems" {
                        child_items_depth = child_items_depth.saturating_sub(1);
                    } else if tag_name == "DataPath" {
                        in_data_path = false;
                    } else if child_items_depth > 0 {
                        // Если закрывается элемент, который мы добавляли в stack
                        let should_pop = stack
                            .last()
                            .map(|n| n.kind == tag_name)
                            .unwrap_or(false);
                        if should_pop {
                            let node = stack.pop().expect("checked above");
                            if let Some(parent) = stack.last_mut() {
                                parent.children.push(node);
                            } else {
                                roots.push(node);
                            }
                        }
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => {
                    return Err(anyhow!("XML parsing error in ChildItems: {}", e));
                }
                _ => {}
            }

            buf.clear();
        }

        Ok(roots)
    }

    /// Парсит секцию `<Attributes>` из Form.xml
    ///
    /// Извлекает информацию о реквизитах формы:
    /// - Имя и ID реквизита
    /// - Типы данных (TypeDescription)
    /// - MainAttribute флаг
    /// - SavedData флаг
    fn parse_attributes(content: &str) -> Result<Vec<FormAttribute>> {
        let mut reader = Reader::from_str(content);
        reader.trim_text(true);

        let mut attributes = Vec::new();
        let mut buf = Vec::new();
        let mut in_attributes_section = false;
        let mut in_attribute = false;
        let mut current_attr_name = String::new();
        let mut current_attr_id = 0;
        let mut current_types = Vec::new();
        let mut is_main = false;
        let mut is_saved = false;
        let mut current_element = String::new();
        let mut in_type_tag = false;

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();

                    if tag_name == "Attributes" {
                        in_attributes_section = true;
                        trace!("  → Entered <Attributes> section");
                    } else if tag_name == "Attribute" && in_attributes_section {
                        in_attribute = true;

                        // Извлечь id и name из атрибутов тега
                        for attr in e.attributes().flatten() {
                            let key = String::from_utf8_lossy(attr.key.as_ref());
                            let value = String::from_utf8_lossy(&attr.value);

                            if key == "name" {
                                current_attr_name = value.to_string();
                            } else if key == "id" {
                                current_attr_id = value.parse().unwrap_or(0);
                            }
                        }
                        trace!("    → Found attribute: {} (id={})", current_attr_name, current_attr_id);
                    } else if in_attribute && tag_name == "Type" {
                        in_type_tag = true;
                        trace!("      → Entered <Type>");
                    } else if in_attribute {
                        current_element = tag_name;
                    }
                }
                Ok(Event::Text(e)) if in_attribute => {
                    let text = e.unescape()?.trim().to_string();

                    if text.is_empty() {
                        continue;
                    }

                    // Внутри <Type> читаем все типы
                    if in_type_tag {
                        // Пропускаем числовые значения (v8:Length, v8:Digits)
                        if !text.chars().all(|c| c.is_numeric() || c == '.') {
                            current_types.push(text.clone());
                            trace!("        Type: {}", text);
                        }
                    } else {
                        match current_element.as_str() {
                            "MainAttribute" => {
                                is_main = text == "true";
                                trace!("        MainAttribute: {}", is_main);
                            }
                            "SavedData" => {
                                is_saved = text == "true";
                                trace!("        SavedData: {}", is_saved);
                            }
                            _ => {}
                        }
                    }
                }
                Ok(Event::End(e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();

                    if tag_name == "Attributes" {
                        in_attributes_section = false;
                        trace!("  ← Left <Attributes> section");
                    } else if tag_name == "Attribute" && in_attribute {
                        // Создаём FormAttribute
                        attributes.push(FormAttribute {
                            name: current_attr_name.clone(),
                            id: current_attr_id,
                            type_description: TypeDescription {
                                types: current_types.clone(),
                            },
                            is_main_attribute: is_main,
                            saved_data: is_saved,
                        });

                        // Сброс состояния
                        in_attribute = false;
                        current_attr_name.clear();
                        current_attr_id = 0;
                        current_types.clear();
                        is_main = false;
                        is_saved = false;
                        current_element.clear();
                    } else if tag_name == "Type" && in_type_tag {
                        in_type_tag = false;
                        trace!("      ← Left <Type>");
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => {
                    return Err(anyhow!("XML parsing error in attributes: {}", e));
                }
                _ => {}
            }
            buf.clear();
        }

        Ok(attributes)
    }

    /// Парсит секцию `<Events>` из Form.xml
    ///
    /// Извлекает события формы и их обработчики:
    /// - Имя события (name атрибут тега Event)
    /// - Имя обработчика (текстовое содержимое тега Event)
    fn parse_events(content: &str) -> Result<Vec<FormEvent>> {
        let mut reader = Reader::from_str(content);
        reader.trim_text(true);

        let mut events = Vec::new();
        let mut buf = Vec::new();
        let mut in_events_section = false;
        let mut current_event_name = String::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();

                    if tag_name == "Events" {
                        in_events_section = true;
                        trace!("  → Entered <Events> section");
                    } else if tag_name == "Event" && in_events_section {
                        // Извлекаем атрибут name
                        for attr in e.attributes().flatten() {
                            let key = String::from_utf8_lossy(attr.key.as_ref());
                            if key == "name" {
                                current_event_name = String::from_utf8_lossy(&attr.value).to_string();
                                trace!("    → Event: {}", current_event_name);
                            }
                        }
                    }
                }
                Ok(Event::Text(e)) if !current_event_name.is_empty() => {
                    let handler_name = e.unescape()?.trim().to_string();
                    if !handler_name.is_empty() {
                        trace!("      Handler: {}", handler_name);
                        events.push(FormEvent {
                            name: current_event_name.clone(),
                            handler_name,
                        });
                        current_event_name.clear();
                    }
                }
                Ok(Event::End(e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    if tag_name == "Events" {
                        in_events_section = false;
                        trace!("  ← Left <Events> section");
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => {
                    return Err(anyhow!("XML parsing error in events: {}", e));
                }
                _ => {}
            }
            buf.clear();
        }

        Ok(events)
    }

    /// Парсит Module.bsl формы для определения контекстов выполнения
    ///
    /// Ищет директивы компиляции:
    /// - `&НаСервере` → ExecutionContext::Server
    /// - `&НаКлиенте` → ExecutionContext::ClientManaged
    ///
    /// Если директив нет - возвращает оба контекста (форма может иметь и клиент и сервер)
    fn parse_module_contexts(module_path: &Path) -> Result<Vec<ExecutionContext>> {
        trace!("  🔍 Parsing module contexts from {:?}", module_path);

        let content = fs::read_to_string(module_path)?;
        let mut contexts = Vec::new();

        // Поиск директив компиляции
        if content.contains("&НаСервере") || content.contains("&AtServer") {
            contexts.push(ExecutionContext::Server);
            trace!("    → Found Server context");
        }
        if content.contains("&НаКлиенте") || content.contains("&AtClient") {
            contexts.push(ExecutionContext::ClientManaged);
            trace!("    → Found Client context");
        }

        // Если нет директив - возвращаем оба контекста (модуль формы может иметь и сервер и клиент)
        if contexts.is_empty() {
            trace!("    → No directives found, assuming both contexts");
            contexts.push(ExecutionContext::Server);
            contexts.push(ExecutionContext::ClientManaged);
        }

        Ok(contexts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_attributes_simple() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Form>
    <Attributes>
        <Attribute name="Объект" id="1">
            <Type>
                <v8:Type>cfg:DocumentObject.ЗаказНаряды</v8:Type>
            </Type>
            <MainAttribute>true</MainAttribute>
            <SavedData>true</SavedData>
        </Attribute>
    </Attributes>
</Form>"#;

        let attributes = FormParser::parse_attributes(xml).unwrap();

        assert_eq!(attributes.len(), 1);
        assert_eq!(attributes[0].name, "Объект");
        assert_eq!(attributes[0].id, 1);
        assert!(attributes[0].is_main_attribute);
        assert!(attributes[0].saved_data);
        assert_eq!(attributes[0].type_description.types.len(), 1);
        assert_eq!(
            attributes[0].type_description.types[0],
            "cfg:DocumentObject.ЗаказНаряды"
        );
    }

    #[test]
    fn test_parse_events_simple() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Form>
    <Events>
        <Event name="OnCreateAtServer">ПриСозданииНаСервере</Event>
        <Event name="OnOpen">ПриОткрытии</Event>
    </Events>
</Form>"#;

        let events = FormParser::parse_events(xml).unwrap();

        assert_eq!(events.len(), 2);
        assert_eq!(events[0].name, "OnCreateAtServer");
        assert_eq!(events[0].handler_name, "ПриСозданииНаСервере");
        assert_eq!(events[1].name, "OnOpen");
        assert_eq!(events[1].handler_name, "ПриОткрытии");
    }

    #[test]
    fn test_parse_module_contexts_server_only() {
        use tempfile::NamedTempFile;
        use std::io::Write;

        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "&НаСервере").unwrap();
        writeln!(temp_file, "Процедура ПриСозданииНаСервере()").unwrap();
        writeln!(temp_file, "КонецПроцедуры").unwrap();

        let contexts = FormParser::parse_module_contexts(temp_file.path()).unwrap();

        assert_eq!(contexts.len(), 1);
        assert_eq!(contexts[0], ExecutionContext::Server);
    }

    #[test]
    fn test_parse_module_contexts_no_directives() {
        use tempfile::NamedTempFile;
        use std::io::Write;

        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "Процедура Тест()").unwrap();
        writeln!(temp_file, "КонецПроцедуры").unwrap();

        let contexts = FormParser::parse_module_contexts(temp_file.path()).unwrap();

        // Без директив - оба контекста
        assert_eq!(contexts.len(), 2);
        assert!(contexts.contains(&ExecutionContext::Server));
        assert!(contexts.contains(&ExecutionContext::ClientManaged));
    }
}
