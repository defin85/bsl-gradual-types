//! Descriptor model for contextual implicit types.
//!
//! Keeps canonical semantic meaning separate from user-facing labels.

use serde::{Deserialize, Serialize};

use super::{FacetKind, MetadataKind};

/// Canonical semantic type for `FormModule.Объект`.
pub const FORM_DATA_CANONICAL_TYPE_NAME: &str = "ДанныеФормыСтруктура";
/// Metadata note marker for form-data semantics.
pub const FORM_DATA_SEMANTICS_NOTE: &str = "contextual:form_data_semantics";
/// Metadata note prefix with owner facet user-facing label.
pub const FORM_DATA_OWNER_FACET_NOTE_PREFIX: &str = "contextual:form_data_owner_facet=";
/// Metadata note prefix with resolved synthetic form type name.
pub const FORM_DATA_FORM_TYPE_NOTE_PREFIX: &str = "contextual:form_data_form_type=";
/// Metadata note prefix with resolved synthetic form-elements type name.
pub const FORM_DATA_ELEMENTS_TYPE_NOTE_PREFIX: &str = "contextual:form_data_elements_type=";

/// Descriptor-based contextual type for implicit symbols.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContextualTypeDescriptor {
    /// Plain platform/configuration-independent type (e.g. `Структура`).
    PlatformType { type_name: String },
    /// Configuration metadata type with required active facet.
    ConfigurationFacet {
        kind: MetadataKind,
        name: String,
        facet: FacetKind,
    },
    /// Synthetic form type `Формы.<Коллекция>.<ИмяОбъекта>.<ИмяФормы>`.
    FormType {
        kind: MetadataKind,
        owner_name: String,
        form_name: String,
    },
    /// Synthetic form elements type `ЭлементыФормы.<Коллекция>.<ИмяОбъекта>.<ИмяФормы>`.
    FormElementsType {
        kind: MetadataKind,
        owner_name: String,
        form_name: String,
    },
    /// Form-data object descriptor for `FormModule.Объект`.
    ///
    /// Canonical semantics stays form-data (`ДанныеФормыСтруктура`),
    /// while user-facing label is owner object facet.
    FormDataObject {
        kind: MetadataKind,
        owner_name: String,
        form_name: String,
    },
}

impl ContextualTypeDescriptor {
    /// Canonical semantic type name.
    pub fn canonical_type_name(&self) -> String {
        match self {
            Self::PlatformType { type_name } => type_name.clone(),
            Self::ConfigurationFacet { kind, name, facet } => {
                format!("{}.{}", kind.faceted_type_prefix(facet), name)
            }
            Self::FormType {
                kind,
                owner_name,
                form_name,
            } => format!("Формы.{}.{}.{}", kind.display_name(), owner_name, form_name),
            Self::FormElementsType {
                kind,
                owner_name,
                form_name,
            } => format!(
                "ЭлементыФормы.{}.{}.{}",
                kind.display_name(),
                owner_name,
                form_name
            ),
            Self::FormDataObject { .. } => FORM_DATA_CANONICAL_TYPE_NAME.to_string(),
        }
    }

    /// User-facing label for compact/standard representation.
    pub fn user_facing_type_name(&self) -> String {
        match self {
            Self::FormDataObject {
                kind, owner_name, ..
            } => {
                let object_prefix = kind.faceted_type_prefix(&FacetKind::Object);
                format!("{}.{}", object_prefix, owner_name)
            }
            _ => self.canonical_type_name(),
        }
    }

    /// Extra metadata notes for TypeResolution produced from this descriptor.
    pub fn resolution_metadata_notes(&self) -> Vec<String> {
        match self {
            Self::FormDataObject { .. } => {
                let mut notes = vec![FORM_DATA_SEMANTICS_NOTE.to_string()];
                notes.push(format!(
                    "{}{}",
                    FORM_DATA_OWNER_FACET_NOTE_PREFIX,
                    self.user_facing_type_name()
                ));
                if let Some(form_type_name) = self.form_type_name() {
                    notes.push(format!(
                        "{}{}",
                        FORM_DATA_FORM_TYPE_NOTE_PREFIX, form_type_name
                    ));
                }
                if let Some(elements_type_name) = self.form_elements_type_name() {
                    notes.push(format!(
                        "{}{}",
                        FORM_DATA_ELEMENTS_TYPE_NOTE_PREFIX, elements_type_name
                    ));
                }
                notes
            }
            _ => Vec::new(),
        }
    }

    /// Synthetic form type for descriptors bound to form context.
    pub fn form_type_name(&self) -> Option<String> {
        match self {
            Self::FormType {
                kind,
                owner_name,
                form_name,
            }
            | Self::FormDataObject {
                kind,
                owner_name,
                form_name,
            } => Some(format!(
                "Формы.{}.{}.{}",
                kind.display_name(),
                owner_name,
                form_name
            )),
            _ => None,
        }
    }

    /// Synthetic form elements type for descriptors bound to form context.
    pub fn form_elements_type_name(&self) -> Option<String> {
        match self {
            Self::FormElementsType {
                kind,
                owner_name,
                form_name,
            }
            | Self::FormDataObject {
                kind,
                owner_name,
                form_name,
            } => Some(format!(
                "ЭлементыФормы.{}.{}.{}",
                kind.display_name(),
                owner_name,
                form_name
            )),
            _ => None,
        }
    }
}
