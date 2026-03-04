use bsl_shared::domain::types::{ContextualTypeDescriptor, FacetKind, MetadataKind};
use bsl_shared::domain::{CompilerDirective, ModuleType};

pub(crate) const FORM_CONTEXT_BOUND_SYMBOL_KEYS: [&str; 6] = [
    "этотобъект",
    "этаформа",
    "форма",
    "объект",
    "элементы",
    "параметры",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ImplicitBinding {
    pub(crate) name: &'static str,
    pub(crate) descriptor: Option<ContextualTypeDescriptor>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FormModuleTypeNames {
    pub(crate) form_type_name: String,
    pub(crate) form_object_type_name: String,
    pub(crate) form_elements_type_name: String,
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct ImplicitBindingResolver;

impl ImplicitBindingResolver {
    pub(crate) fn new() -> Self {
        Self
    }

    pub(crate) fn form_module_type_names(
        &self,
        owner_type: &str,
        form_name: &str,
    ) -> Option<FormModuleTypeNames> {
        let (kind, object_name) = Self::parse_owner_kind(owner_type)?;
        let form_descriptor = ContextualTypeDescriptor::FormType {
            kind,
            owner_name: object_name.to_string(),
            form_name: form_name.to_string(),
        };
        let form_object_descriptor = ContextualTypeDescriptor::FormDataObject {
            kind,
            owner_name: object_name.to_string(),
            form_name: form_name.to_string(),
        };
        let form_elements_descriptor = ContextualTypeDescriptor::FormElementsType {
            kind,
            owner_name: object_name.to_string(),
            form_name: form_name.to_string(),
        };
        Some(FormModuleTypeNames {
            form_type_name: form_descriptor.canonical_type_name(),
            form_object_type_name: form_object_descriptor.user_facing_type_name(),
            form_elements_type_name: form_elements_descriptor.canonical_type_name(),
        })
    }

    pub(crate) fn bindings_for_module(&self, module_type: &ModuleType) -> Vec<ImplicitBinding> {
        match module_type {
            ModuleType::FormModule {
                form_name,
                owner_type,
            } => {
                let owner = Self::parse_owner_kind(owner_type)
                    .map(|(kind, object_name)| (kind, object_name.to_string()));
                let form_descriptor =
                    owner
                        .as_ref()
                        .map(|(kind, object_name)| ContextualTypeDescriptor::FormType {
                            kind: *kind,
                            owner_name: object_name.clone(),
                            form_name: form_name.clone(),
                        });
                let form_object_descriptor = owner.as_ref().map(|(kind, object_name)| {
                    ContextualTypeDescriptor::FormDataObject {
                        kind: *kind,
                        owner_name: object_name.clone(),
                        form_name: form_name.clone(),
                    }
                });
                let form_elements_descriptor = owner.as_ref().map(|(kind, object_name)| {
                    ContextualTypeDescriptor::FormElementsType {
                        kind: *kind,
                        owner_name: object_name.clone(),
                        form_name: form_name.clone(),
                    }
                });
                vec![
                    ImplicitBinding {
                        name: "ЭтотОбъект",
                        descriptor: form_descriptor.clone(),
                    },
                    ImplicitBinding {
                        name: "ЭтаФорма",
                        descriptor: form_descriptor.clone(),
                    },
                    ImplicitBinding {
                        name: "Форма",
                        descriptor: form_descriptor,
                    },
                    ImplicitBinding {
                        name: "Объект",
                        descriptor: form_object_descriptor,
                    },
                    ImplicitBinding {
                        name: "Элементы",
                        descriptor: form_elements_descriptor,
                    },
                    ImplicitBinding {
                        name: "Параметры",
                        descriptor: Some(ContextualTypeDescriptor::PlatformType {
                            type_name: "Структура".to_string(),
                        }),
                    },
                ]
            }
            ModuleType::ManagerModule { owner_type } => {
                let manager_descriptor =
                    Self::faceted_owner_descriptor(owner_type, FacetKind::Manager);
                vec![
                    ImplicitBinding {
                        name: "ЭтотОбъект",
                        descriptor: manager_descriptor.clone(),
                    },
                    ImplicitBinding {
                        name: "Объект",
                        descriptor: manager_descriptor,
                    },
                ]
            }
            ModuleType::ObjectModule { owner_type }
            | ModuleType::RecordSetModule { owner_type } => {
                let object_descriptor =
                    Self::faceted_owner_descriptor(owner_type, FacetKind::Object);
                vec![
                    ImplicitBinding {
                        name: "ЭтотОбъект",
                        descriptor: object_descriptor.clone(),
                    },
                    ImplicitBinding {
                        name: "Объект",
                        descriptor: object_descriptor,
                    },
                ]
            }
            _ => Vec::new(),
        }
    }

    fn parse_owner_kind(owner_type: &str) -> Option<(MetadataKind, &str)> {
        let (xml_kind, object_name) = owner_type.split_once('.')?;
        let kind = MetadataKind::from_xml_tag(xml_kind)?;
        Some((kind, object_name))
    }

    fn faceted_owner_descriptor(
        owner_type: &str,
        facet: FacetKind,
    ) -> Option<ContextualTypeDescriptor> {
        let (kind, object_name) = Self::parse_owner_kind(owner_type)?;
        Some(ContextualTypeDescriptor::ConfigurationFacet {
            kind,
            name: object_name.to_string(),
            facet,
        })
    }
}

pub(crate) fn directive_disables_form_context(directive: Option<CompilerDirective>) -> bool {
    matches!(
        directive,
        Some(CompilerDirective::OnServerNoContext | CompilerDirective::OnClientOnServerNoContext)
    )
}

#[cfg(test)]
#[path = "implicit_bindings/tests.rs"]
mod tests;
