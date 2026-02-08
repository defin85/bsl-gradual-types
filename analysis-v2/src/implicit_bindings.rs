use bsl_shared::domain::types::{FacetKind, MetadataKind};
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
    pub(crate) type_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FormModuleTypeNames {
    pub(crate) form_type_name: String,
    pub(crate) form_object_type_name: String,
    pub(crate) form_elements_type_name: String,
}

fn parse_owner_kind(owner_type: &str) -> Option<(MetadataKind, &str)> {
    let (xml_kind, object_name) = owner_type.split_once('.')?;
    let kind = MetadataKind::from_xml_tag(xml_kind)?;
    Some((kind, object_name))
}

fn faceted_owner_type_name(owner_type: &str, facet: FacetKind) -> Option<String> {
    let (kind, object_name) = parse_owner_kind(owner_type)?;
    let prefix = kind.faceted_type_prefix(&facet);
    Some(format!("{}.{}", prefix, object_name))
}

pub(crate) fn form_module_type_names(
    owner_type: &str,
    form_name: &str,
) -> Option<FormModuleTypeNames> {
    let (kind, object_name) = parse_owner_kind(owner_type)?;
    let collection = kind.display_name();
    let object_type_prefix = kind.faceted_type_prefix(&FacetKind::Object);
    Some(FormModuleTypeNames {
        form_type_name: format!("Формы.{}.{}.{}", collection, object_name, form_name),
        form_object_type_name: format!("{}.{}", object_type_prefix, object_name),
        form_elements_type_name: format!(
            "ЭлементыФормы.{}.{}.{}",
            collection, object_name, form_name
        ),
    })
}

pub(crate) fn module_implicit_bindings(module_type: &ModuleType) -> Vec<ImplicitBinding> {
    match module_type {
        ModuleType::FormModule {
            form_name,
            owner_type,
        } => {
            let names = form_module_type_names(owner_type, form_name);
            vec![
                ImplicitBinding {
                    name: "ЭтотОбъект",
                    type_name: names.as_ref().map(|n| n.form_type_name.clone()),
                },
                ImplicitBinding {
                    name: "ЭтаФорма",
                    type_name: names.as_ref().map(|n| n.form_type_name.clone()),
                },
                ImplicitBinding {
                    name: "Форма",
                    type_name: names.as_ref().map(|n| n.form_type_name.clone()),
                },
                ImplicitBinding {
                    name: "Объект",
                    type_name: names.as_ref().map(|n| n.form_object_type_name.clone()),
                },
                ImplicitBinding {
                    name: "Элементы",
                    type_name: names.as_ref().map(|n| n.form_elements_type_name.clone()),
                },
                ImplicitBinding {
                    name: "Параметры",
                    type_name: Some("Структура".to_string()),
                },
            ]
        }
        ModuleType::ManagerModule { owner_type } => {
            let manager_type_name = faceted_owner_type_name(owner_type, FacetKind::Manager);
            vec![
                ImplicitBinding {
                    name: "ЭтотОбъект",
                    type_name: manager_type_name.clone(),
                },
                ImplicitBinding {
                    name: "Объект",
                    type_name: manager_type_name,
                },
            ]
        }
        ModuleType::ObjectModule { owner_type } | ModuleType::RecordSetModule { owner_type } => {
            let object_type_name = faceted_owner_type_name(owner_type, FacetKind::Object);
            vec![
                ImplicitBinding {
                    name: "ЭтотОбъект",
                    type_name: object_type_name.clone(),
                },
                ImplicitBinding {
                    name: "Объект",
                    type_name: object_type_name,
                },
            ]
        }
        _ => Vec::new(),
    }
}

pub(crate) fn directive_disables_form_context(directive: Option<CompilerDirective>) -> bool {
    matches!(
        directive,
        Some(CompilerDirective::OnServerNoContext | CompilerDirective::OnClientOnServerNoContext)
    )
}
