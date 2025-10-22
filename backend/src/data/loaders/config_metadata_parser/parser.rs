//! Универсальный парсер XML объектов метаданных

use super::types::{AttributeInfo, TabularSectionInfo, UniversalMetadataObject};
use quick_xml::events::Event;
use quick_xml::Reader;
use std::fs;
use std::path::Path;

/// Результат операций парсинга
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Универсальный парсер объектов метаданных
///
/// Парсит ЛЮБОЙ тип объекта метаданных из XML файла.
/// Использует структурированный подход:
/// 1. Извлечение базовой информации (UUID, Name, Synonym)
/// 2. Парсинг InternalInfo (для определения фасетов)
/// 3. Парсинг Properties (дополнительные свойства)
/// 4. Парсинг Attributes (атрибуты объекта)
/// 5. Парсинг TabularSections (табличные части)
pub struct UniversalMetadataParser;

impl UniversalMetadataParser {
    /// Парсит ЛЮБОЙ объект метаданных из XML файла
    ///
    /// Автоматически определяет тип объекта из корневого тега.
    /// Graceful fallback для неизвестных типов (object_type: None).
    pub fn parse_any_object(xml_path: &Path) -> Result<UniversalMetadataObject> {
        tracing::debug!("📄 Парсинг объекта метаданных: {:?}", xml_path);

        let content = fs::read_to_string(xml_path)?;
        let mut reader = Reader::from_str(&content);
        reader.trim_text(true);

        let mut buf = Vec::new();
        let mut object_type_raw = String::new();
        let mut uuid = String::new();
        let mut name = String::new();
        let mut synonym: Option<String> = None;
        let mut attributes = Vec::new();
        let mut tabular_sections = Vec::new();

        let mut current_element = String::new();
        let mut current_attribute: Option<AttributeInfo> = None;
        let mut current_tabular: Option<TabularSectionInfo> = None;
        let mut in_tabular_attributes = false;
        let mut in_type_tag = false; // Флаг для отслеживания вхождения в <Type>

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();

                    // Первый тег после MetaDataObject - это тип объекта (Catalog, Document, etc.)
                    // И он содержит UUID в атрибутах
                    if object_type_raw.is_empty() && tag_name != "MetaDataObject" && uuid.is_empty()
                    {
                        object_type_raw = tag_name.clone();
                        tracing::trace!("🔖 Тип объекта: {}", object_type_raw);

                        // Извлекаем UUID из атрибута
                        for a in e.attributes().flatten() {
                            let key = String::from_utf8_lossy(a.key.as_ref());
                            if key == "uuid" {
                                uuid = String::from_utf8_lossy(&a.value).to_string();
                                tracing::trace!("🆔 UUID: {}", uuid);
                            }
                        }
                    }

                    if tag_name == "Attribute" {
                        current_attribute = Some(AttributeInfo {
                            name: String::new(),
                            type_name: String::new(),
                            synonym: None,
                        });
                    } else if tag_name == "TabularSection" {
                        current_tabular = Some(TabularSectionInfo {
                            name: String::new(),
                            synonym: None,
                            attributes: Vec::new(),
                        });
                    } else if tag_name == "ChildObjects" && current_tabular.is_some() {
                        in_tabular_attributes = true;
                        tracing::trace!("🔄 Вход в ChildObjects табличной части");
                    } else if tag_name == "Type" {
                        in_type_tag = true;
                        tracing::trace!("📋 Вход в <Type> для атрибута");
                    } else {
                        current_element = tag_name;
                    }
                }
                Ok(Event::Text(e)) => {
                    let text = e.unescape()?.trim().to_string();

                    if text.is_empty() {
                        continue;
                    }

                    // Если находимся внутри <Type>, читаем ВСЕ текстовые значения
                    // (включая вложенные теги <v8:Type>, <cfg:CatalogRef> и т.д.)
                    if in_type_tag {
                        if let Some(ref mut attr) = current_attribute {
                            // Пропускаем числовые значения (v8:Length, v8:Digits и т.д.)
                            // Берём только названия типов
                            if !text.chars().all(|c| c.is_numeric()) {
                                // Если тип уже задан (композитный тип), добавляем через запятую
                                if !attr.type_name.is_empty() && !attr.type_name.ends_with(", ") {
                                    attr.type_name.push_str(", ");
                                }
                                attr.type_name.push_str(&text);
                                tracing::trace!("  📝 Тип атрибута: {}", text);
                            }
                        }
                        continue; // Не обрабатываем дальше
                    }

                    match current_element.as_str() {
                        "Uuid" => uuid = text,
                        "Name" => {
                            if let Some(ref mut attr) = current_attribute {
                                attr.name = text.clone();
                            } else if let Some(ref mut tab) = current_tabular {
                                tab.name = text.clone();
                            } else if name.is_empty() {
                                name = text;
                            }
                        }
                        "content" => {
                            // v8:content для синонима
                            if let Some(ref mut attr) = current_attribute {
                                if attr.synonym.is_none() {
                                    attr.synonym = Some(text.clone());
                                }
                            } else if let Some(ref mut tab) = current_tabular {
                                if tab.synonym.is_none() {
                                    tab.synonym = Some(text.clone());
                                }
                            } else if synonym.is_none() {
                                synonym = Some(text);
                            }
                        }
                        _ => {}
                    }
                }
                Ok(Event::End(e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();

                    if tag_name == "Attribute" {
                        if let Some(attr) = current_attribute.take() {
                            if in_tabular_attributes {
                                if let Some(ref mut tab) = current_tabular {
                                    tracing::trace!(
                                        "  ➕ Добавлен атрибут '{}' в ТЧ '{}'",
                                        attr.name,
                                        tab.name
                                    );
                                    tab.attributes.push(attr);
                                }
                            } else {
                                attributes.push(attr);
                            }
                        }
                    } else if tag_name == "TabularSection" {
                        if let Some(tab) = current_tabular.take() {
                            tabular_sections.push(tab);
                        }
                        in_tabular_attributes = false;
                    } else if tag_name == "ChildObjects" && in_tabular_attributes {
                        in_tabular_attributes = false;
                        tracing::trace!("🔄 Выход из ChildObjects табличной части");
                    } else if tag_name == "Type" && in_type_tag {
                        in_type_tag = false;
                        tracing::trace!("📋 Выход из <Type>");
                    }

                    current_element.clear();
                }
                Ok(Event::Eof) => break,
                Err(e) => {
                    tracing::error!("❌ Ошибка парсинга XML: {:?}", e);
                    return Err(Box::new(e));
                }
                _ => {}
            }
            buf.clear();
        }

        // Создаём объект метаданных
        let mut metadata = UniversalMetadataObject::new(object_type_raw, name, uuid);
        metadata.synonym = synonym;
        metadata.attributes = attributes;
        metadata.tabular_sections = tabular_sections;

        tracing::debug!(
            "✅ Объект распарсен: {} (UUID: {}, атрибутов: {}, табличных частей: {})",
            metadata.name,
            metadata.uuid,
            metadata.attributes.len(),
            metadata.tabular_sections.len()
        );

        Ok(metadata)
    }
}
