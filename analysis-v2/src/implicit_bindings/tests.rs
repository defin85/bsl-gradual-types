use std::collections::HashMap;

use super::{ImplicitBindingResolver, ModuleType};

fn to_map(bindings: Vec<super::ImplicitBinding>) -> HashMap<String, Option<String>> {
    bindings
        .into_iter()
        .map(|binding| {
            (
                binding.name.to_string(),
                binding
                    .descriptor
                    .map(|descriptor| descriptor.user_facing_type_name()),
            )
        })
        .collect()
}

#[test]
fn form_module_binding_matrix_is_contextual() {
    let resolver = ImplicitBindingResolver::new();
    let bindings = resolver.bindings_for_module(&ModuleType::FormModule {
        form_name: "Форма1".to_string(),
        owner_type: "Document.Док1".to_string(),
    });
    let map = to_map(bindings);

    assert_eq!(
        map.get("ЭтотОбъект").and_then(Clone::clone).as_deref(),
        Some("Формы.Документы.Док1.Форма1")
    );
    assert_eq!(
        map.get("ЭтаФорма").and_then(Clone::clone).as_deref(),
        Some("Формы.Документы.Док1.Форма1")
    );
    assert_eq!(
        map.get("Форма").and_then(Clone::clone).as_deref(),
        Some("Формы.Документы.Док1.Форма1")
    );
    assert_eq!(
        map.get("Объект").and_then(Clone::clone).as_deref(),
        Some("ДанныеФормыСтруктура")
    );
    assert_eq!(
        map.get("Элементы").and_then(Clone::clone).as_deref(),
        Some("ЭлементыФормы.Документы.Док1.Форма1")
    );
    assert_eq!(
        map.get("Параметры").and_then(Clone::clone).as_deref(),
        Some("Структура")
    );
}

#[test]
fn manager_module_binding_matrix_uses_manager_facet() {
    let resolver = ImplicitBindingResolver::new();
    let bindings = resolver.bindings_for_module(&ModuleType::ManagerModule {
        owner_type: "Document.Док1".to_string(),
    });
    let map = to_map(bindings);

    assert_eq!(
        map.get("ЭтотОбъект").and_then(Clone::clone).as_deref(),
        Some("ДокументМенеджер.Док1")
    );
    assert_eq!(
        map.get("Объект").and_then(Clone::clone).as_deref(),
        Some("ДокументМенеджер.Док1")
    );
}

#[test]
fn object_and_recordset_binding_matrix_uses_object_facet() {
    let resolver = ImplicitBindingResolver::new();

    let object_bindings = resolver.bindings_for_module(&ModuleType::ObjectModule {
        owner_type: "Document.Док1".to_string(),
    });
    let object_map = to_map(object_bindings);
    assert_eq!(
        object_map
            .get("ЭтотОбъект")
            .and_then(Clone::clone)
            .as_deref(),
        Some("ДокументОбъект.Док1")
    );
    assert_eq!(
        object_map.get("Объект").and_then(Clone::clone).as_deref(),
        Some("ДокументОбъект.Док1")
    );

    let recordset_bindings = resolver.bindings_for_module(&ModuleType::RecordSetModule {
        owner_type: "InformationRegister.Регистр1".to_string(),
    });
    let recordset_map = to_map(recordset_bindings);
    assert_eq!(
        recordset_map
            .get("ЭтотОбъект")
            .and_then(Clone::clone)
            .as_deref(),
        Some("РегистрСведенийНаборЗаписей.Регистр1")
    );
    assert_eq!(
        recordset_map
            .get("Объект")
            .and_then(Clone::clone)
            .as_deref(),
        Some("РегистрСведенийНаборЗаписей.Регистр1")
    );
}

#[test]
fn form_object_binding_does_not_use_legacy_alias() {
    let resolver = ImplicitBindingResolver::new();
    let bindings = resolver.bindings_for_module(&ModuleType::FormModule {
        form_name: "Форма1".to_string(),
        owner_type: "Document.Док1".to_string(),
    });

    let object_binding = bindings
        .iter()
        .find(|binding| binding.name == "Объект")
        .expect("Объект binding");
    let type_name = object_binding
        .descriptor
        .as_ref()
        .map(|descriptor| descriptor.user_facing_type_name())
        .expect("Объект type name");
    assert!(
        !type_name.contains("ДанныеФормыОбъект"),
        "legacy alias leaked into form object binding: {}",
        type_name
    );
    assert_eq!(type_name, "ДанныеФормыСтруктура");
}
