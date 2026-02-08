use crate::types::{
    ContextualTypeDescriptor, FacetKind, MetadataKind, FORM_DATA_CANONICAL_TYPE_NAME,
    FORM_DATA_ELEMENTS_TYPE_NOTE_PREFIX, FORM_DATA_FORM_TYPE_NOTE_PREFIX,
    FORM_DATA_OWNER_FACET_NOTE_PREFIX, FORM_DATA_SEMANTICS_NOTE,
};

#[test]
fn configuration_facet_descriptor_uses_faceted_name_for_canonical_and_user_facing() {
    let descriptor = ContextualTypeDescriptor::ConfigurationFacet {
        kind: MetadataKind::Catalog,
        name: "Контрагенты".to_string(),
        facet: FacetKind::Manager,
    };

    assert_eq!(
        descriptor.canonical_type_name(),
        "СправочникМенеджер.Контрагенты"
    );
    assert_eq!(
        descriptor.user_facing_type_name(),
        "СправочникМенеджер.Контрагенты"
    );
    assert!(descriptor.resolution_metadata_notes().is_empty());
}

#[test]
fn form_descriptor_builds_synthetic_form_and_elements_type_names() {
    let descriptor = ContextualTypeDescriptor::FormType {
        kind: MetadataKind::Document,
        owner_name: "Док1".to_string(),
        form_name: "Форма1".to_string(),
    };
    assert_eq!(
        descriptor.canonical_type_name(),
        "Формы.Документы.Док1.Форма1"
    );
    assert_eq!(
        descriptor.form_type_name().as_deref(),
        Some("Формы.Документы.Док1.Форма1")
    );
    assert_eq!(descriptor.form_elements_type_name(), None);
}

#[test]
fn form_data_descriptor_separates_canonical_and_user_facing_layers() {
    let descriptor = ContextualTypeDescriptor::FormDataObject {
        kind: MetadataKind::Document,
        owner_name: "Док1".to_string(),
        form_name: "Форма1".to_string(),
    };

    assert_eq!(
        descriptor.canonical_type_name(),
        FORM_DATA_CANONICAL_TYPE_NAME
    );
    assert_eq!(descriptor.user_facing_type_name(), "ДокументОбъект.Док1");
}

#[test]
fn form_data_descriptor_emits_resolution_notes_for_dual_layer_contract() {
    let descriptor = ContextualTypeDescriptor::FormDataObject {
        kind: MetadataKind::Document,
        owner_name: "Док1".to_string(),
        form_name: "Форма1".to_string(),
    };

    let notes = descriptor.resolution_metadata_notes();
    assert!(
        notes.iter().any(|n| n == FORM_DATA_SEMANTICS_NOTE),
        "missing form-data semantics marker: {:?}",
        notes
    );
    assert!(
        notes
            .iter()
            .any(|n| n == "contextual:form_data_owner_facet=ДокументОбъект.Док1"),
        "missing owner facet label note: {:?}",
        notes
    );
    assert!(
        notes
            .iter()
            .any(|n| n == "contextual:form_data_form_type=Формы.Документы.Док1.Форма1"),
        "missing form type note: {:?}",
        notes
    );
    assert!(
        notes
            .iter()
            .any(|n| n == "contextual:form_data_elements_type=ЭлементыФормы.Документы.Док1.Форма1"),
        "missing form elements type note: {:?}",
        notes
    );

    assert!(notes
        .iter()
        .any(|n| n.starts_with(FORM_DATA_OWNER_FACET_NOTE_PREFIX)));
    assert!(notes
        .iter()
        .any(|n| n.starts_with(FORM_DATA_FORM_TYPE_NOTE_PREFIX)));
    assert!(notes
        .iter()
        .any(|n| n.starts_with(FORM_DATA_ELEMENTS_TYPE_NOTE_PREFIX)));
}
