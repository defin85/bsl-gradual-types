//! Facet system for multiple type representations

use super::types::{FacetKind, Method, Property};
use std::collections::HashMap;

/// Registry of facet templates
pub struct FacetRegistry {
    templates: HashMap<String, FacetTemplates>,
}

/// Templates for different facets of a type
pub struct FacetTemplates {
    pub manager: Option<FacetTemplate>,
    pub object: Option<FacetTemplate>,
    pub reference: Option<FacetTemplate>,
    pub metadata: Option<FacetTemplate>,
}

/// Template for a specific facet
pub struct FacetTemplate {
    pub kind: FacetKind,
    pub methods: Vec<Method>,
    pub properties: Vec<Property>,
}

impl Default for FacetRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl FacetRegistry {
    pub fn new() -> Self {
        Self {
            templates: HashMap::new(),
        }
    }

    /// Load facet templates from platform documentation
    pub fn load_templates(&mut self, _platform_version: &str) {
        // TODO: Load from platform docs
        self.init_catalog_facets();
        self.init_document_facets();
    }

    /// Get facet for a specific type and kind
    pub fn get_facet(&self, type_category: &str, facet_kind: FacetKind) -> Option<&FacetTemplate> {
        self.templates
            .get(type_category)
            .and_then(|templates| match facet_kind {
                FacetKind::Manager => templates.manager.as_ref(),
                FacetKind::Object => templates.object.as_ref(),
                FacetKind::Reference => templates.reference.as_ref(),
                FacetKind::Metadata => templates.metadata.as_ref(),
                _ => None,
            })
    }

    /// Register a facet for a specific type
    pub fn register_facet(
        &mut self,
        type_name: &str,
        facet_kind: FacetKind,
        methods: Vec<Method>,
        properties: Vec<Property>,
    ) {
        let template = FacetTemplate {
            kind: facet_kind,
            methods,
            properties,
        };

        let templates = self
            .templates
            .entry(type_name.to_string())
            .or_insert_with(|| FacetTemplates {
                manager: None,
                object: None,
                reference: None,
                metadata: None,
            });

        match facet_kind {
            FacetKind::Manager => templates.manager = Some(template),
            FacetKind::Object => templates.object = Some(template),
            FacetKind::Reference => templates.reference = Some(template),
            FacetKind::Metadata => templates.metadata = Some(template),
            _ => {} // Ignore other facet kinds for now
        }
    }

    fn init_catalog_facets(&mut self) {
        let templates = FacetTemplates {
            manager: Some(FacetTemplate {
                kind: FacetKind::Manager,
                methods: vec![
                    Method {
                        name: "СоздатьЭлемент".to_string(),
                        parameters: vec![],
                        return_type: Some("СправочникОбъект".to_string()),
                        is_function: true,
                    },
                    Method {
                        name: "НайтиПоКоду".to_string(),
                        parameters: vec![],
                        return_type: Some("СправочникСсылка".to_string()),
                        is_function: true,
                    },
                ],
                properties: vec![],
            }),
            object: Some(FacetTemplate {
                kind: FacetKind::Object,
                methods: vec![Method {
                    name: "Записать".to_string(),
                    parameters: vec![],
                    return_type: None,
                    is_function: false,
                }],
                properties: vec![
                    Property {
                        name: "Код".to_string(),
                        type_: "Строка".to_string(),
                        readonly: false,
                    },
                    Property {
                        name: "Наименование".to_string(),
                        type_: "Строка".to_string(),
                        readonly: false,
                    },
                ],
            }),
            reference: Some(FacetTemplate {
                kind: FacetKind::Reference,
                methods: vec![Method {
                    name: "ПолучитьОбъект".to_string(),
                    parameters: vec![],
                    return_type: Some("СправочникОбъект".to_string()),
                    is_function: true,
                }],
                properties: vec![Property {
                    name: "Код".to_string(),
                    type_: "Строка".to_string(),
                    readonly: true,
                }],
            }),
            metadata: Some(FacetTemplate {
                kind: FacetKind::Metadata,
                methods: vec![],
                properties: vec![Property {
                    name: "Имя".to_string(),
                    type_: "Строка".to_string(),
                    readonly: true,
                }],
            }),
        };

        self.templates.insert("Catalog".to_string(), templates);
    }

    fn init_document_facets(&mut self) {
        // Similar to catalog facets but for documents
        // TODO: Implement document facets
    }
}
