//! Универсальный парсер XML объектов метаданных

use super::types::{
    AttributeInfo, CommonModuleProperties, ReturnValuesReuse, TabularSectionInfo,
    UniversalMetadataObject,
};
use quick_xml::events::Event;
use quick_xml::Reader;
use std::collections::HashSet;
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
    fn local_name(name: &str) -> &str {
        name.rsplit(':').next().unwrap_or(name)
    }

    fn push_unique_case_insensitive(out: &mut Vec<String>, seen: &mut HashSet<String>, name: &str) {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return;
        }
        let key = trimmed.to_lowercase();
        if seen.insert(key) {
            out.push(trimmed.to_string());
        }
    }

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
        let mut standard_attributes = Vec::new();
        let mut standard_attribute_seen = HashSet::new();
        let mut tabular_sections = Vec::new();
        let mut enum_values = Vec::new();
        let mut number_type: Option<String> = None;
        let mut posting_mode: Option<String> = None;

        let mut current_element = String::new();
        let mut current_attribute: Option<AttributeInfo> = None;
        let mut current_tabular: Option<TabularSectionInfo> = None;
        let mut current_enum_value: Option<String> = None;
        let mut in_tabular_attributes = false;
        let mut in_object_properties = false;
        let mut in_standard_attributes = false;
        let mut seen_root_child_objects = false;
        let mut in_type_tag = false; // Флаг для отслеживания вхождения в <Type>

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) => {
                    let raw_tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    let tag_name = Self::local_name(&raw_tag_name).to_string();

                    // Первый тег после MetaDataObject - это тип объекта (Catalog, Document, etc.)
                    // И он содержит UUID в атрибутах
                    if object_type_raw.is_empty() && tag_name != "MetaDataObject" && uuid.is_empty()
                    {
                        object_type_raw = tag_name.clone();
                        tracing::trace!("🔖 Тип объекта: {}", object_type_raw);

                        // Извлекаем UUID из атрибута
                        for a in e.attributes().flatten() {
                            let raw_key = String::from_utf8_lossy(a.key.as_ref()).to_string();
                            let key = Self::local_name(&raw_key);
                            if key == "uuid" {
                                uuid = String::from_utf8_lossy(&a.value).to_string();
                                tracing::trace!("🆔 UUID: {}", uuid);
                            }
                        }
                    }

                    if tag_name == "Properties"
                        && current_attribute.is_none()
                        && current_tabular.is_none()
                        && current_enum_value.is_none()
                        && !in_tabular_attributes
                        && !seen_root_child_objects
                    {
                        in_object_properties = true;
                    } else if tag_name == "StandardAttributes" && in_object_properties {
                        in_standard_attributes = true;
                    } else if tag_name == "StandardAttribute"
                        && in_object_properties
                        && in_standard_attributes
                    {
                        for attr in e.attributes().flatten() {
                            let raw_key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                            if Self::local_name(&raw_key) != "name" {
                                continue;
                            }
                            let raw_value = String::from_utf8_lossy(&attr.value).to_string();
                            Self::push_unique_case_insensitive(
                                &mut standard_attributes,
                                &mut standard_attribute_seen,
                                &raw_value,
                            );
                        }
                    } else if tag_name == "Attribute" {
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
                        tracing::trace!("📋 Создана новая табличная часть");
                    } else if tag_name == "EnumValue" {
                        current_enum_value = Some(String::new());
                    } else if tag_name == "ChildObjects"
                        && current_attribute.is_none()
                        && current_tabular.is_none()
                        && current_enum_value.is_none()
                        && !in_tabular_attributes
                    {
                        seen_root_child_objects = true;
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

                    if in_object_properties
                        && current_attribute.is_none()
                        && current_tabular.is_none()
                        && current_enum_value.is_none()
                    {
                        match current_element.as_str() {
                            "NumberType" => {
                                number_type = Some(text.clone());
                            }
                            "Posting" => {
                                posting_mode = Some(text.clone());
                            }
                            _ => {}
                        }
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
                            if let Some(ref mut enum_value) = current_enum_value {
                                *enum_value = text.clone();
                                tracing::trace!("  ✅ Значение перечисления: {}", text);
                            } else if let Some(ref mut attr) = current_attribute {
                                attr.name = text.clone();
                                tracing::trace!("  ✅ Имя атрибута: {}", text);
                            } else if let Some(ref mut tab) = current_tabular {
                                tab.name = text.clone();
                                tracing::trace!("  ✅ Имя табличной части: {}", text);
                            } else if name.is_empty() {
                                name = text.clone();
                                tracing::trace!("  ✅ Имя объекта: {}", text);
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
                    let raw_tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    let tag_name = Self::local_name(&raw_tag_name).to_string();

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
                            tracing::trace!(
                                "  ✅ Табличная часть '{}' добавлена (атрибутов: {})",
                                tab.name,
                                tab.attributes.len()
                            );
                            tabular_sections.push(tab);
                        }
                        in_tabular_attributes = false;
                    } else if tag_name == "StandardAttributes" {
                        in_standard_attributes = false;
                    } else if tag_name == "Properties"
                        && current_attribute.is_none()
                        && current_tabular.is_none()
                        && current_enum_value.is_none()
                    {
                        in_object_properties = false;
                    } else if tag_name == "EnumValue" {
                        if let Some(value) = current_enum_value.take() {
                            if !value.is_empty() {
                                enum_values.push(value);
                            }
                        }
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

        if object_type_raw == "Document" {
            let posting_capable = matches!(posting_mode.as_deref(), Some("Allow"));

            // Для документов standard attrs должны присутствовать даже когда блок
            // <StandardAttributes> в XML отсутствует (старые выгрузки/совместимость).
            Self::push_unique_case_insensitive(
                &mut standard_attributes,
                &mut standard_attribute_seen,
                "Ref",
            );
            Self::push_unique_case_insensitive(
                &mut standard_attributes,
                &mut standard_attribute_seen,
                "DeletionMark",
            );
            Self::push_unique_case_insensitive(
                &mut standard_attributes,
                &mut standard_attribute_seen,
                "Date",
            );
            Self::push_unique_case_insensitive(
                &mut standard_attributes,
                &mut standard_attribute_seen,
                "Number",
            );
            if posting_capable {
                Self::push_unique_case_insensitive(
                    &mut standard_attributes,
                    &mut standard_attribute_seen,
                    "Posted",
                );
            } else {
                standard_attributes.retain(|name| !name.eq_ignore_ascii_case("Posted"));
            }
        }

        // Создаём объект метаданных
        let mut metadata = UniversalMetadataObject::new(object_type_raw.clone(), name, uuid);
        metadata.synonym = synonym;
        metadata.attributes = attributes;
        metadata.standard_attributes = standard_attributes;
        metadata.tabular_sections = tabular_sections;
        metadata.enum_values = enum_values;
        if let Some(value) = number_type {
            metadata.properties.insert("NumberType".to_string(), value);
        }
        if let Some(value) = posting_mode {
            metadata.properties.insert("Posting".to_string(), value);
        }

        // Парсим дополнительные свойства для CommonModule
        if object_type_raw == "CommonModule" {
            tracing::debug!("🔧 Обнаружен CommonModule, парсинг контекстных свойств...");
            match Self::parse_common_module_properties(&content) {
                Ok(props) => {
                    metadata.execution_contexts = props.get_execution_contexts();
                    metadata.common_module_properties = Some(props);
                    tracing::debug!(
                        "✅ CommonModule '{}': контекстов={}, global={}",
                        metadata.name,
                        metadata.execution_contexts.len(),
                        metadata
                            .common_module_properties
                            .as_ref()
                            .map(|p| p.global)
                            .unwrap_or(false)
                    );
                }
                Err(e) => {
                    tracing::debug!(
                        "⚠️ Не удалось распарсить свойства CommonModule '{}': {}",
                        metadata.name,
                        e
                    );
                }
            }
        }

        tracing::debug!(
            "✅ Объект распарсен: {} (UUID: {}, атрибутов: {}, табличных частей: {})",
            metadata.name,
            metadata.uuid,
            metadata.attributes.len(),
            metadata.tabular_sections.len()
        );

        Ok(metadata)
    }

    /// Парсит свойства общего модуля из XML контента
    ///
    /// Извлекает контекстные свойства CommonModule:
    /// - Server, Client, ExternalConnection - контексты выполнения
    /// - Global, Privileged, ServerCall - специальные свойства
    /// - ReturnValuesReuse - режим повторного использования
    ///
    /// # Errors
    ///
    /// Возвращает ошибку, если XML некорректен или обязательные теги отсутствуют
    pub fn parse_common_module_properties(content: &str) -> Result<CommonModuleProperties> {
        tracing::debug!("🔍 Парсинг свойств CommonModule");

        let mut reader = Reader::from_str(content);
        reader.trim_text(true);

        let mut buf = Vec::new();
        let mut props = CommonModuleProperties::default();

        let mut current_element = String::new();
        let mut in_properties = false;

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    if tag_name == "Properties" {
                        in_properties = true;
                        tracing::trace!("📋 Вход в секцию Properties");
                    } else {
                        current_element = tag_name;
                    }
                }
                Ok(Event::Text(e)) => {
                    if !in_properties {
                        continue;
                    }

                    let text = e.unescape()?.trim().to_string();
                    if text.is_empty() {
                        continue;
                    }

                    // Парсим boolean свойства
                    match current_element.as_str() {
                        "Server" => {
                            props.server = text == "true";
                            tracing::trace!("  Server: {}", props.server);
                        }
                        "ClientManagedApplication" => {
                            props.client_managed_application = text == "true";
                            tracing::trace!(
                                "  ClientManagedApplication: {}",
                                props.client_managed_application
                            );
                        }
                        "ClientOrdinaryApplication" => {
                            props.client_ordinary_application = text == "true";
                            tracing::trace!(
                                "  ClientOrdinaryApplication: {}",
                                props.client_ordinary_application
                            );
                        }
                        "ExternalConnection" => {
                            props.external_connection = text == "true";
                            tracing::trace!("  ExternalConnection: {}", props.external_connection);
                        }
                        "ServerCall" => {
                            props.server_call = text == "true";
                            tracing::trace!("  ServerCall: {}", props.server_call);
                        }
                        "Global" => {
                            props.global = text == "true";
                            tracing::trace!("  Global: {}", props.global);
                        }
                        "Privileged" => {
                            props.privileged = text == "true";
                            tracing::trace!("  Privileged: {}", props.privileged);
                        }
                        "ReturnValuesReuse" => {
                            props.return_values_reuse = match text.as_str() {
                                "DontUse" => ReturnValuesReuse::DontUse,
                                "DuringRequest" => ReturnValuesReuse::DuringRequest,
                                "DuringSession" => ReturnValuesReuse::DuringSession,
                                _ => {
                                    tracing::debug!(
                                        "⚠️ Неизвестное значение ReturnValuesReuse: {}",
                                        text
                                    );
                                    ReturnValuesReuse::DontUse
                                }
                            };
                            tracing::trace!("  ReturnValuesReuse: {:?}", props.return_values_reuse);
                        }
                        _ => {}
                    }
                }
                Ok(Event::End(e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    if tag_name == "Properties" {
                        in_properties = false;
                        tracing::trace!("📋 Выход из секции Properties");
                    }
                    current_element.clear();
                }
                Ok(Event::Eof) => break,
                Err(e) => {
                    tracing::error!("❌ Ошибка парсинга XML свойств CommonModule: {:?}", e);
                    return Err(Box::new(e));
                }
                _ => {}
            }
            buf.clear();
        }

        tracing::debug!(
            "✅ Свойства CommonModule распарсены: Server={}, Client={}, Global={}",
            props.server,
            props.client_managed_application || props.client_ordinary_application,
            props.global
        );

        Ok(props)
    }

    /// Парсит список предопределённых элементов из `Ext/Predefined.xml`
    ///
    /// Извлекает имена `Item` по тегам `Name` (и `PredefinedDataName` как fallback),
    /// поддерживая вложенные `ChildItems`.
    pub fn parse_predefined_items(predefined_xml_path: &Path) -> Result<Vec<String>> {
        let content = fs::read_to_string(predefined_xml_path)?;
        let mut reader = Reader::from_str(&content);
        reader.trim_text(true);

        let mut buf = Vec::new();
        let mut item_depth = 0usize;
        let mut current_element = String::new();
        let mut items = Vec::new();
        let mut seen = HashSet::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    if tag_name == "Item" {
                        item_depth += 1;
                    }
                    current_element = tag_name;
                }
                Ok(Event::Text(e)) => {
                    if item_depth == 0 {
                        continue;
                    }
                    if !matches!(current_element.as_str(), "Name" | "PredefinedDataName") {
                        continue;
                    }

                    let text = e.unescape()?.trim().to_string();
                    if text.is_empty() {
                        continue;
                    }
                    let key = text.to_lowercase();
                    if seen.insert(key) {
                        items.push(text);
                    }
                }
                Ok(Event::End(e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    if tag_name == "Item" {
                        item_depth = item_depth.saturating_sub(1);
                    }
                    current_element.clear();
                }
                Ok(Event::Eof) => break,
                Err(e) => return Err(Box::new(e)),
                _ => {}
            }
            buf.clear();
        }

        Ok(items)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn parses_enum_values_from_xml() {
        let xml = r#"
<?xml version="1.0" encoding="UTF-8"?>
<MetaDataObject xmlns="http://v8.1c.ru/8.3/MDClasses">
    <Enum uuid="00000000-0000-0000-0000-000000000000">
        <Properties>
            <Name>ТестПеречисление</Name>
        </Properties>
        <ChildObjects>
            <EnumValue uuid="11111111-1111-1111-1111-111111111111">
                <Properties>
                    <Name>Первое</Name>
                </Properties>
            </EnumValue>
            <EnumValue uuid="22222222-2222-2222-2222-222222222222">
                <Properties>
                    <Name>Второе</Name>
                </Properties>
            </EnumValue>
        </ChildObjects>
    </Enum>
</MetaDataObject>
"#;

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(xml.as_bytes()).unwrap();

        let parsed = UniversalMetadataParser::parse_any_object(file.path()).unwrap();
        assert_eq!(parsed.enum_values.len(), 2);
        assert!(parsed.enum_values.contains(&"Первое".to_string()));
        assert!(parsed.enum_values.contains(&"Второе".to_string()));
    }

    #[test]
    fn parses_predefined_items_from_xml_with_nested_items() {
        let xml = r#"
<?xml version="1.0" encoding="UTF-8"?>
<PredefinedData xmlns="http://v8.1c.ru/8.3/xcf/predef">
    <Item id="root">
        <Name>Корень</Name>
        <ChildItems>
            <Item id="child-1">
                <Name>Потомок1</Name>
            </Item>
            <Item id="child-2">
                <PredefinedDataName>Потомок2</PredefinedDataName>
            </Item>
        </ChildItems>
    </Item>
</PredefinedData>
"#;

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(xml.as_bytes()).unwrap();

        let items = UniversalMetadataParser::parse_predefined_items(file.path()).unwrap();
        assert_eq!(items, vec!["Корень", "Потомок1", "Потомок2"]);
    }

    #[test]
    fn parses_document_standard_attributes_with_fallback_from_properties() {
        let xml = r#"
<?xml version="1.0" encoding="UTF-8"?>
<MetaDataObject xmlns="http://v8.1c.ru/8.3/MDClasses">
    <Document uuid="00000000-0000-0000-0000-000000000001">
        <Properties>
            <Name>Док1</Name>
            <NumberType>String</NumberType>
            <Posting>Allow</Posting>
        </Properties>
    </Document>
</MetaDataObject>
"#;

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(xml.as_bytes()).unwrap();

        let parsed = UniversalMetadataParser::parse_any_object(file.path()).unwrap();
        let attrs: BTreeSet<_> = parsed.standard_attributes.iter().cloned().collect();
        assert!(attrs.contains("Ref"));
        assert!(attrs.contains("DeletionMark"));
        assert!(attrs.contains("Date"));
        assert!(attrs.contains("Number"));
        assert!(attrs.contains("Posted"));
        assert_eq!(
            parsed.properties.get("NumberType").map(String::as_str),
            Some("String")
        );
        assert_eq!(
            parsed.properties.get("Posting").map(String::as_str),
            Some("Allow")
        );
    }

    #[test]
    fn excludes_posted_standard_attribute_for_non_posting_document() {
        let xml = r#"
<?xml version="1.0" encoding="UTF-8"?>
<MetaDataObject xmlns="http://v8.1c.ru/8.3/MDClasses" xmlns:xr="http://v8.1c.ru/8.3/xcf/readable">
    <Document uuid="00000000-0000-0000-0000-000000000002">
        <Properties>
            <Name>Док2</Name>
            <Posting>Deny</Posting>
            <StandardAttributes>
                <xr:StandardAttribute name="Posted"/>
                <xr:StandardAttribute name="Ref"/>
            </StandardAttributes>
        </Properties>
    </Document>
</MetaDataObject>
"#;

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(xml.as_bytes()).unwrap();

        let parsed = UniversalMetadataParser::parse_any_object(file.path()).unwrap();
        assert!(parsed.standard_attributes.iter().any(|name| name == "Ref"));
        assert!(
            parsed
                .standard_attributes
                .iter()
                .all(|name| !name.eq_ignore_ascii_case("Posted")),
            "Posted should not be present for non-posting document"
        );
    }
}
