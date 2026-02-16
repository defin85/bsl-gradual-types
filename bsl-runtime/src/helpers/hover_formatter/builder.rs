//! HoverBuilder - builder pattern для построения hover content
//!
//! Предоставляет fluent API для пошагового построения hover responses.

use bsl_shared::domain::metadata_lookup::TypeMetadataLookup;
use bsl_shared::domain::types::{Certainty, TypeResolution, FORM_DATA_SEMANTICS_NOTE};
use bsl_shared::formatting::{normalize_user_facing_type_name, user_facing_resolution_type_name};

use super::config::{HoverFormatConfig, HoverOutputFormat};
use super::sections;
use super::type_display;

/// Builder для пошагового построения hover content
pub struct HoverBuilder<'a> {
    config: &'a HoverFormatConfig,
    sections: Vec<String>,
}

impl<'a> HoverBuilder<'a> {
    /// Создаёт новый HoverBuilder с указанной конфигурацией
    pub fn new(config: &'a HoverFormatConfig) -> Self {
        Self {
            config,
            sections: Vec::new(),
        }
    }

    /// Добавляет заголовок (например: "Переменная: МассивДанных")
    pub fn add_header(mut self, label: &str, value: &str) -> Self {
        let section = match self.config.output_format {
            HoverOutputFormat::Markdown => format!("**{}:** {}", label, value),
            HoverOutputFormat::PlainText => format!("{}: {}", label, value),
        };
        self.sections.push(section);
        self
    }

    /// Добавляет произвольную секцию с меткой и значением
    pub fn add_section(mut self, label: &str, value: &str) -> Self {
        let section = match self.config.output_format {
            HoverOutputFormat::Markdown => format!("**{}:** {}", label, value),
            HoverOutputFormat::PlainText => format!("{}: {}", label, value),
        };
        self.sections.push(section);
        self
    }

    /// Добавляет информацию о типе
    pub fn add_type_info(self, resolution: &TypeResolution) -> Self {
        let is_form_data = resolution
            .metadata
            .notes
            .iter()
            .any(|note| note == FORM_DATA_SEMANTICS_NOTE);
        let type_str = if is_form_data {
            user_facing_resolution_type_name(resolution)
        } else {
            type_display::format_type_string(resolution)
        };
        self.add_section("Тип", &type_str)
    }

    /// Добавляет информацию об уверенности (certainty)
    pub fn add_certainty(self, certainty: &Certainty) -> Self {
        // Показывать certainty только если включено в конфигурации
        if !self.config.show_certainty {
            return self;
        }

        let certainty_str = type_display::format_certainty(certainty);
        self.add_section("Уверенность", &certainty_str)
    }

    /// Добавляет секцию методов
    pub fn add_methods(
        mut self,
        resolution: &TypeResolution,
        metadata_lookup: &TypeMetadataLookup,
    ) -> Self {
        if let Some(section) =
            sections::format_methods_section(self.config, resolution, metadata_lookup)
        {
            self.sections.push(section);
        }
        self
    }

    /// Добавляет секцию свойств
    pub fn add_properties(
        mut self,
        resolution: &TypeResolution,
        metadata_lookup: &TypeMetadataLookup,
    ) -> Self {
        if let Some(section) =
            sections::format_properties_section(self.config, resolution, metadata_lookup)
        {
            self.sections.push(section);
        }
        self
    }

    /// Добавляет секцию табличных частей
    pub fn add_tabular_sections(
        mut self,
        resolution: &TypeResolution,
        metadata_lookup: &TypeMetadataLookup,
    ) -> Self {
        if let Some(section) =
            sections::format_tabular_sections_section(self.config, resolution, metadata_lookup)
        {
            self.sections.push(section);
        }
        self
    }

    /// Добавляет информацию о фасете (MILESTONE 3.11 Phase 4)
    pub fn add_facet_info(mut self, resolution: &TypeResolution) -> Self {
        if let Some(section) = sections::format_facet_info(self.config, resolution) {
            self.sections.push(section);
        }
        self
    }

    /// Добавляет информацию о Generic типе (MILESTONE 3.6 Phase 2)
    pub fn add_generic_info(mut self, resolution: &TypeResolution) -> Self {
        if let Some(section) = sections::format_generic_info(self.config, resolution) {
            self.sections.push(section);
        }
        self
    }

    /// Добавляет ссылки на документацию (MILESTONE 3.6 Phase 2)
    pub fn add_documentation_links(mut self, resolution: &TypeResolution) -> Self {
        if let Some(section) = sections::format_documentation_links(self.config, resolution) {
            self.sections.push(section);
        }
        self
    }

    /// Собирает финальную строку hover content
    pub fn build(self) -> String {
        // Используем двойной перенос для разделения секций (параграфы в Markdown)
        let output = self.sections.join("\n\n");
        normalize_user_facing_type_name(&output)
    }
}
