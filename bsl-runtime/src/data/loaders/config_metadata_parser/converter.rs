//! Конвертер UniversalMetadataObject -> RawTypeData для TypeRepository

#![allow(clippy::doc_overindented_list_items)]

use super::types::UniversalMetadataObject;
use bsl_shared::domain::facet_utils;
use bsl_shared::domain::types::MetadataKind;
use bsl_shared::domain::types::{
    RawAttributeData, RawDataSource, RawPropertyData, RawTabularSectionData, RawTypeData,
    FORM_DATA_CANONICAL_TYPE_NAME,
};

const PREDEFINED_MANAGER_PROP_TYPE_PREFIX: &str = "__predefined_manager__:";

impl UniversalMetadataObject {
    /// Конвертирует UniversalMetadataObject в набор RawTypeData, включая синтетические типы форм
    ///
    /// Возвращает:
    /// - основной тип объекта метаданных (как `to_raw_type_data`)
    /// - (если есть формы) дополнительные типы:
    ///   - `Объект` внутри `Формы.*` указывает на strict form-data тип (`ДанныеФормыСтруктура`)
    ///   - `Формы.<Коллекция>.<Объект>.<Форма>` для каждой формы
    ///   - `ЭлементыФормы.<Коллекция>.<Объект>.<Форма>` для каждой формы
    ///   - типы строк табличных частей (`Строка<ИмяТЧ>`) для табличных частей объекта
    pub fn to_raw_type_data_with_forms(&self, prefix: Option<&str>) -> Vec<RawTypeData> {
        let mut out = Vec::new();
        out.push(self.to_raw_type_data(prefix));

        let Some(kind) = self.object_type else {
            return out;
        };

        if self.forms.is_empty() {
            return out;
        }

        let collection = kind.display_name();
        let object_name = Self::apply_prefix(prefix, &self.name);

        // Тип строк табличных частей: "Строка<ИмяТЧ>"
        for ts in &self.tabular_sections {
            out.push(Self::build_tabular_row_type(ts, "Число"));
        }

        // Типы формы и контейнера элементов — по каждой форме
        for form in &self.forms {
            let form_type_name = format!("Формы.{}.{}.{}", collection, object_name, form.name);
            out.push(self.build_form_type(&form_type_name, &object_name, collection, form));

            let elements_type_name =
                format!("ЭлементыФормы.{}.{}.{}", collection, object_name, form.name);
            out.push(Self::build_form_elements_container_type(
                &elements_type_name,
                form,
            ));
        }

        out
    }

    /// Конвертирует UniversalMetadataObject в RawTypeData для загрузки в TypeRepository
    ///
    /// Создаёт полное имя типа (например, "Справочники.Контрагенты")
    /// и заполняет все поля RawTypeData.
    ///
    /// # Параметры
    /// - `prefix` - Опциональный префикс расширения (например, "Тест_")
    ///   Применяется ТОЛЬКО к имени объекта метаданных.
    ///   Для основной конфигурации передавайте `None`.
    ///
    /// # Примеры
    /// ```text
    /// // Без префикса (основная конфигурация)
    /// obj.to_raw_type_data(None) // → "Справочники.Контрагенты"
    ///
    /// // С префиксом (расширение)
    /// obj.to_raw_type_data(Some("Тест_")) // → "Справочники.Тест_Контрагенты"
    /// ```
    pub fn to_raw_type_data(&self, prefix: Option<&str>) -> RawTypeData {
        let type_name = self.get_full_type_name(prefix);
        let module_paths = self.build_module_paths();
        let mut properties = self.convert_attributes_to_properties();
        properties.extend(self.convert_standard_attributes_to_properties(prefix));
        properties.extend(self.convert_predefined_items_to_marker_properties(prefix));
        let properties = Self::dedup_properties_case_insensitive(properties);

        RawTypeData {
            name: type_name.clone(),
            english_name: Self::apply_prefix(prefix, &self.name), // Префикс и к english name
            description: self.synonym.clone().unwrap_or_default(),
            category: self.get_category(),
            source: RawDataSource::Configuration,
            methods: Vec::new(), // Методы извлекаются отдельно из модулей
            properties,
            facets: self.facets.clone(),
            kind: self.object_type,
            attributes: self.convert_attributes(),
            tabular_sections: self.convert_tabular_sections(),
            enum_values: self.enum_values.clone(),
            generic_info: None, // Конфигурационные типы не имеют Generic метаданных (пока)
            collection_item_type: None,
            module_paths, // Milestone 3.14: пути к модулям для Go To Definition
            metadata_path: self.metadata_xml_path.clone(),
        }
    }

    /// Построить ModulePaths из путей к модулям объекта метаданных (Milestone 3.14)
    ///
    /// Конвертирует пути из UniversalMetadataObject в структуру ModulePaths
    /// для использования в Go To Definition навигации.
    fn build_module_paths(
        &self,
    ) -> Option<bsl_shared::domain::type_definition_location::ModulePaths> {
        let mut object_module = self.object_module_path.clone();
        let manager_module = self.manager_module_path.clone();
        let recordset_module = self.record_set_module_path.clone();

        // Common modules have a dedicated module file; wire it into ModulePaths so
        // "Go to Definition" can navigate to the module file itself.
        if self.object_type == Some(MetadataKind::CommonModule) && object_module.is_none() {
            object_module = self.common_module_path.clone();
        }

        // Если нет ни одного пути к модулю - возвращаем None
        if object_module.is_none() && manager_module.is_none() && recordset_module.is_none() {
            return None;
        }

        Some(bsl_shared::domain::type_definition_location::ModulePaths {
            object_module,
            manager_module,
            recordset_module,
        })
    }

    /// Применяет префикс к имени объекта
    ///
    /// # Примеры
    /// ```text
    /// apply_prefix(None, "Контрагенты") // → "Контрагенты"
    /// apply_prefix(Some(""), "Контрагенты") // → "Контрагенты"
    /// apply_prefix(Some("Тест_"), "Контрагенты") // → "Тест_Контрагенты"
    /// ```
    fn apply_prefix(prefix: Option<&str>, name: &str) -> String {
        match prefix {
            Some(p) if !p.is_empty() => format!("{}{}", p, name),
            _ => name.to_string(),
        }
    }

    /// Получить полное имя типа (Справочники.Контрагенты или Справочники.Тест_Контрагенты)
    fn get_full_type_name(&self, prefix: Option<&str>) -> String {
        let object_name = Self::apply_prefix(prefix, &self.name);

        if let Some(kind) = self.object_type {
            format!("{}.{}", kind.display_name(), object_name)
        } else {
            // Для неизвестных типов - просто имя с префиксом
            object_name
        }
    }

    /// Получить категорию объекта для TypeRepository
    fn get_category(&self) -> String {
        if let Some(kind) = self.object_type {
            kind.display_name().to_string()
        } else {
            "Неизвестные".to_string()
        }
    }

    /// Конвертировать атрибуты в свойства для RawTypeData
    fn convert_attributes_to_properties(&self) -> Vec<bsl_shared::domain::types::RawPropertyData> {
        self.attributes
            .iter()
            .map(|attr| bsl_shared::domain::types::RawPropertyData {
                name: attr.name.clone(),
                prop_type: attr.type_name.clone(),
                is_readonly: false, // TODO: определить из метаданных
            })
            .collect()
    }

    fn dedup_properties_case_insensitive(properties: Vec<RawPropertyData>) -> Vec<RawPropertyData> {
        let mut out = Vec::new();
        let mut seen = std::collections::HashSet::<String>::new();
        for property in properties {
            let key = property.name.to_lowercase();
            if seen.insert(key) {
                out.push(property);
            }
        }
        out
    }

    fn is_posting_capable_document(&self) -> bool {
        self.object_type == Some(MetadataKind::Document)
            && matches!(
                self.properties.get("Posting").map(String::as_str),
                Some("Allow")
            )
    }

    fn number_standard_attribute_type(&self) -> String {
        match self.properties.get("NumberType").map(String::as_str) {
            Some("Number") => "Число".to_string(),
            Some("String") => "Строка".to_string(),
            _ => "Строка".to_string(),
        }
    }

    fn reference_type_for_standard_attribute(&self, prefix: Option<&str>) -> Option<String> {
        let kind = self.object_type?;
        let template = match kind {
            MetadataKind::Document => "ДокументСсылка.<Имя документа>",
            MetadataKind::Catalog => "СправочникСсылка.<Имя справочника>",
            MetadataKind::Enum => "ПеречислениеСсылка.<Имя перечисления>",
            MetadataKind::ChartOfAccounts => "ПланСчетовСсылка.<Имя плана счетов>",
            MetadataKind::ChartOfCharacteristicTypes => {
                "ПланВидовХарактеристикСсылка.<Имя плана видов характеристик>"
            }
            MetadataKind::ChartOfCalculationTypes => {
                "ПланВидовРасчетаСсылка.<Имя плана видов расчета>"
            }
            _ => return None,
        };
        let object_name = Self::apply_prefix(prefix, &self.name);
        Some(facet_utils::substitute_type_name(template, &object_name))
    }

    fn convert_standard_attributes_to_properties(
        &self,
        prefix: Option<&str>,
    ) -> Vec<RawPropertyData> {
        let mut out = Vec::new();
        for standard_attr in &self.standard_attributes {
            let key = standard_attr.trim().to_lowercase();
            match key.as_str() {
                "ref" => {
                    let Some(reference_type) = self.reference_type_for_standard_attribute(prefix)
                    else {
                        continue;
                    };
                    out.push(RawPropertyData {
                        name: "Ссылка".to_string(),
                        prop_type: reference_type,
                        is_readonly: true,
                    });
                }
                "deletionmark" => out.push(RawPropertyData {
                    name: "ПометкаУдаления".to_string(),
                    prop_type: "Булево".to_string(),
                    is_readonly: true,
                }),
                "date" => out.push(RawPropertyData {
                    name: "Дата".to_string(),
                    prop_type: "Дата".to_string(),
                    is_readonly: false,
                }),
                "number" => out.push(RawPropertyData {
                    name: "Номер".to_string(),
                    prop_type: self.number_standard_attribute_type(),
                    is_readonly: false,
                }),
                "posted" => {
                    if !self.is_posting_capable_document() {
                        continue;
                    }
                    out.push(RawPropertyData {
                        name: "Проведен".to_string(),
                        prop_type: "Булево".to_string(),
                        is_readonly: false,
                    });
                }
                _ => {}
            }
        }
        out
    }

    fn predefined_manager_reference_type(kind: MetadataKind, object_name: &str) -> Option<String> {
        let template = match kind {
            MetadataKind::Catalog => "СправочникСсылка.<Имя справочника>",
            MetadataKind::ChartOfAccounts => "ПланСчетовСсылка.<Имя плана счетов>",
            MetadataKind::ChartOfCharacteristicTypes => {
                "ПланВидовХарактеристикСсылка.<Имя плана видов характеристик>"
            }
            MetadataKind::ChartOfCalculationTypes => {
                "ПланВидовРасчетаСсылка.<Имя плана видов расчета>"
            }
            _ => return None,
        };

        Some(facet_utils::substitute_type_name(template, object_name))
    }

    fn convert_predefined_items_to_marker_properties(
        &self,
        prefix: Option<&str>,
    ) -> Vec<RawPropertyData> {
        let Some(kind) = self.object_type else {
            return Vec::new();
        };
        if self.predefined_items.is_empty() {
            return Vec::new();
        }

        let object_name = Self::apply_prefix(prefix, &self.name);
        let Some(reference_type) = Self::predefined_manager_reference_type(kind, &object_name)
        else {
            return Vec::new();
        };

        let mut out = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for name in &self.predefined_items {
            if name.trim().is_empty() {
                continue;
            }
            let key = name.to_lowercase();
            if !seen.insert(key) {
                continue;
            }
            out.push(RawPropertyData {
                name: name.clone(),
                prop_type: format!("{PREDEFINED_MANAGER_PROP_TYPE_PREFIX}{reference_type}"),
                is_readonly: true,
            });
        }
        out
    }

    /// Конвертировать атрибуты в RawAttributeData
    fn convert_attributes(&self) -> Vec<RawAttributeData> {
        self.attributes
            .iter()
            .map(|attr| RawAttributeData {
                name: attr.name.clone(),
                attr_type: attr.type_name.clone(),
            })
            .collect()
    }

    /// Конвертировать табличные части в RawTabularSectionData
    fn convert_tabular_sections(&self) -> Vec<RawTabularSectionData> {
        self.tabular_sections
            .iter()
            .map(|ts| RawTabularSectionData {
                name: ts.name.clone(),
                attributes: ts
                    .attributes
                    .iter()
                    .map(|attr| RawAttributeData {
                        name: attr.name.clone(),
                        attr_type: attr.type_name.clone(),
                    })
                    .collect(),
            })
            .collect()
    }

    fn build_form_type(
        &self,
        type_name: &str,
        object_name: &str,
        collection: &str,
        form: &super::form_types::FormMetadata,
    ) -> RawTypeData {
        let mut properties: Vec<bsl_shared::domain::types::RawPropertyData> = Vec::new();

        // "Объект" в модуле формы — strict form-data контекст.
        properties.push(bsl_shared::domain::types::RawPropertyData {
            name: "Объект".to_string(),
            prop_type: FORM_DATA_CANONICAL_TYPE_NAME.to_string(),
            is_readonly: false,
        });

        fn normalize_form_attr_type(raw: &str) -> String {
            let trimmed = raw.trim();
            match trimmed {
                "xs:string" | "String" => "Строка".to_string(),
                "xs:boolean" | "Boolean" => "Булево".to_string(),
                "xs:dateTime" | "Date" => "Дата".to_string(),
                "xs:decimal" | "xs:int" | "xs:integer" | "xs:long" | "Number" => {
                    "Число".to_string()
                }
                other => other.to_string(),
            }
        }

        fn normalize_form_attr_types(attr: &super::form_types::FormAttribute) -> Option<String> {
            if attr.type_description.types.is_empty() {
                return None;
            }
            let parts: Vec<String> = attr
                .type_description
                .types
                .iter()
                .map(|t| normalize_form_attr_type(t))
                .filter(|t| !t.is_empty())
                .collect();
            if parts.is_empty() {
                return None;
            }
            if parts.len() == 1 {
                return Some(parts[0].clone());
            }
            Some(parts.join(" | "))
        }

        // Form attributes from Form.xml: available as identifiers in the form module.
        for attr in &form.attributes {
            if attr.name == "Объект" {
                continue;
            }
            if properties.iter().any(|p| p.name == attr.name) {
                continue;
            }
            let Some(prop_type) = normalize_form_attr_types(attr) else {
                continue;
            };
            properties.push(bsl_shared::domain::types::RawPropertyData {
                name: attr.name.clone(),
                prop_type,
                is_readonly: false,
            });
        }

        // Привязанные реквизиты формы по DataPath: Объект.<ТЧ> где имя элемента совпадает с именем ТЧ
        for ts in &self.tabular_sections {
            let has_binding = Self::form_has_direct_object_binding(form, &ts.name);
            if !has_binding {
                continue;
            }

            properties.push(bsl_shared::domain::types::RawPropertyData {
                name: ts.name.clone(),
                prop_type: format!("ДанныеФормыКоллекция<Строка{}>", ts.name),
                is_readonly: false,
            });
        }

        RawTypeData {
            name: type_name.to_string(),
            english_name: type_name.to_string(),
            description: String::new(),
            category: format!("Формы.{}.{}", collection, object_name),
            source: RawDataSource::Configuration,
            methods: Vec::new(),
            properties,
            facets: Vec::new(),
            kind: None,
            attributes: Vec::new(),
            tabular_sections: Vec::new(),
            enum_values: Vec::new(),
            generic_info: None,
            collection_item_type: None,
            module_paths: None,
            metadata_path: None,
        }
    }

    fn form_has_direct_object_binding(form: &super::form_types::FormMetadata, name: &str) -> bool {
        fn walk(nodes: &[super::form_types::FormElementBinding], name: &str) -> bool {
            let expected = format!("Объект.{}", name);
            for n in nodes {
                let is_match = n.name.as_deref() == Some(name)
                    && n.data_path.as_deref() == Some(expected.as_str());
                if is_match {
                    return true;
                }
                if walk(&n.children, name) {
                    return true;
                }
            }
            false
        }

        walk(&form.elements, name)
    }

    fn build_tabular_row_type(
        ts: &super::types::TabularSectionInfo,
        line_number_type: &str,
    ) -> RawTypeData {
        let mut attributes: Vec<RawAttributeData> = ts
            .attributes
            .iter()
            .map(|a| RawAttributeData {
                name: a.name.clone(),
                attr_type: a.type_name.clone(),
            })
            .collect();

        // В форме 1С есть системное поле LineNumber (минимально необходимое)
        attributes.push(RawAttributeData {
            name: "LineNumber".to_string(),
            attr_type: line_number_type.to_string(),
        });

        let type_name = format!("Строка{}", ts.name);

        RawTypeData {
            name: type_name.clone(),
            english_name: type_name.clone(),
            description: String::new(),
            category: "СтрокиТабличныхЧастей".to_string(),
            source: RawDataSource::Configuration,
            methods: Vec::new(),
            properties: attributes
                .iter()
                .map(|a| bsl_shared::domain::types::RawPropertyData {
                    name: a.name.clone(),
                    prop_type: a.attr_type.clone(),
                    is_readonly: false,
                })
                .collect(),
            facets: Vec::new(),
            kind: None,
            attributes,
            tabular_sections: Vec::new(),
            enum_values: Vec::new(),
            generic_info: None,
            collection_item_type: None,
            module_paths: None,
            metadata_path: None,
        }
    }

    fn build_form_elements_container_type(
        type_name: &str,
        form: &super::form_types::FormMetadata,
    ) -> RawTypeData {
        // Важно: коллекция `Элементы` в модуле формы соответствует платформенному типу
        // `ЭлементыФормы (FormItems)`, где элементы коллекции гетерогенны
        // (ГруппаФормы/ДекорацияФормы/КнопкаФормы/ПолеФормы/ТаблицаФормы).
        //
        // Поэтому:
        // - type container `ЭлементыФормы.*` — это record с properties по именам элементов формы;
        // - для каждого элемента задаём UI-тип из ограниченного набора FormItems.
        // - вспомогательные сущности, которые в XML тоже имеют `name` (ContextMenu, ExtendedTooltip,
        //   AutoCommandBar и т.п.), НЕ добавляем в `Элементы.*`, чтобы не подменять реальные form items.
        fn ui_type_from_kind(kind: &str) -> Option<&'static str> {
            match kind {
                // Таблица формы
                "Table" => Some("ТаблицаФормы"),

                // Поля формы (в т.ч. колонки таблиц)
                "InputField" | "LabelField" => Some("ПолеФормы"),

                // Страницы в XML — это разновидность группировки элементов формы
                "Pages" | "Page" => Some("ГруппаФормы"),

                // UsualGroup in Form.xml should be treated as a form group.
                "UsualGroup" => Some("ГруппаФормы"),

                // TODO: расширять по мере необходимости (Button/Decoration/etc.)
                _ => None,
            }
        }

        fn collect_props(
            props: &mut Vec<bsl_shared::domain::types::RawPropertyData>,
            nodes: &[super::form_types::FormElementBinding],
            seen: &mut std::collections::HashSet<String>,
        ) {
            for n in nodes {
                if let (Some(ref name), Some(ui_type)) = (&n.name, ui_type_from_kind(&n.kind)) {
                    // NOTE: Names in 1C are case-insensitive; prefer first occurrence for
                    // deterministic, stable output.
                    let key = name.to_lowercase();
                    if seen.insert(key) {
                        props.push(bsl_shared::domain::types::RawPropertyData {
                            name: name.clone(),
                            prop_type: ui_type.to_string(),
                            is_readonly: false,
                        });
                    }
                }
                collect_props(props, &n.children, seen);
            }
        }

        let mut properties: Vec<bsl_shared::domain::types::RawPropertyData> = Vec::new();
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        collect_props(&mut properties, &form.elements, &mut seen);

        RawTypeData {
            name: type_name.to_string(),
            english_name: type_name.to_string(),
            description: String::new(),
            category: "ЭлементыФормы".to_string(),
            source: RawDataSource::Configuration,
            methods: Vec::new(),
            properties,
            facets: Vec::new(),
            kind: None,
            attributes: Vec::new(),
            tabular_sections: Vec::new(),
            enum_values: Vec::new(),
            generic_info: None,
            collection_item_type: None,
            module_paths: None,
            metadata_path: None,
        }
    }
}

#[cfg(test)]
#[path = "converter/tests.rs"]
mod tests;
