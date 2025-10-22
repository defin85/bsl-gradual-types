//! JSON Renderer для API и обмена данными

use anyhow::Result;
use bsl_shared::api::dtos::TypeDto;
use serde_json::json;

use crate::TypeRenderer;

/// JSON рендерер
pub struct JsonRenderer {
    pretty: bool,
}

impl JsonRenderer {
    pub fn new(pretty: bool) -> Self {
        Self { pretty }
    }

    pub fn with_pretty_print() -> Self {
        Self { pretty: true }
    }
}

impl TypeRenderer for JsonRenderer {
    fn render_type_info(&self, type_dto: &TypeDto) -> Result<String> {
        let json = if self.pretty {
            serde_json::to_string_pretty(type_dto)?
        } else {
            serde_json::to_string(type_dto)?
        };
        Ok(json)
    }

    fn render_methods(&self, type_name: &str, methods: &[String]) -> Result<String> {
        let json = json!({
            "typeName": type_name,
            "methodsCount": methods.len(),
            "methods": methods
        });

        let result = if self.pretty {
            serde_json::to_string_pretty(&json)?
        } else {
            serde_json::to_string(&json)?
        };

        Ok(result)
    }

    fn render_properties(&self, type_name: &str, properties: &[String]) -> Result<String> {
        let json = json!({
            "typeName": type_name,
            "propertiesCount": properties.len(),
            "properties": properties
        });

        let result = if self.pretty {
            serde_json::to_string_pretty(&json)?
        } else {
            serde_json::to_string(&json)?
        };

        Ok(result)
    }

    fn render_metrics(&self, total_types: usize, total_methods: usize) -> Result<String> {
        let json = json!({
            "totalTypes": total_types,
            "totalMethods": total_methods
        });

        let result = if self.pretty {
            serde_json::to_string_pretty(&json)?
        } else {
            serde_json::to_string(&json)?
        };

        Ok(result)
    }
}
