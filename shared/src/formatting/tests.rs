use super::*;

#[test]
fn test_detail_level_from_str() {
    assert_eq!(DetailLevel::parse("compact"), DetailLevel::Compact);
    assert_eq!(DetailLevel::parse("full"), DetailLevel::Full);
    assert_eq!(DetailLevel::parse("detailed"), DetailLevel::Detailed);
    assert_eq!(DetailLevel::parse("FULL"), DetailLevel::Full);
    assert_eq!(DetailLevel::parse("unknown"), DetailLevel::Full); // default
}

#[test]
fn normalize_user_facing_type_name_rewrites_legacy_alias() {
    let value = "ДанныеФормыОбъект.Документы.РеализацияТоваровУслуг";
    assert_eq!(
        normalize_user_facing_type_name(value),
        "ДокументОбъект.РеализацияТоваровУслуг"
    );
}

#[test]
fn normalize_user_facing_type_name_keeps_unknown_collection() {
    let value = "ДанныеФормыОбъект.Неизвестные.Объект1";
    assert_eq!(normalize_user_facing_type_name(value), value);
}

#[test]
fn normalize_user_facing_type_name_rewrites_multiple_occurrences() {
    let value = "ДанныеФормыОбъект.Документы.Док1 | ДанныеФормыОбъект.Справочники.Спр1";
    assert_eq!(
        normalize_user_facing_type_name(value),
        "ДокументОбъект.Док1 | СправочникОбъект.Спр1"
    );
}

#[test]
fn user_facing_resolution_type_name_prefers_form_data_canonical_label() {
    let mut resolution = TypeResolution::metadata_type(
        crate::domain::types::MetadataKind::Document,
        "Док1",
        Some(FacetKind::Object),
    );
    resolution
        .metadata
        .notes
        .push(FORM_DATA_SEMANTICS_NOTE.to_string());

    assert_eq!(
        user_facing_resolution_type_name(&resolution),
        FORM_DATA_CANONICAL_TYPE_NAME
    );
}
