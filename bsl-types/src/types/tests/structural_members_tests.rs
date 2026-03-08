use crate::types::{
    Certainty, StructuralMember, StructuralMemberId, StructuralMemberSpan, TypeResolution,
};

#[test]
fn test_resolution_preserves_structural_member_contract() {
    let resolution =
        TypeResolution::explicit("Структура").with_structural_member(StructuralMember::new(
            "Идентификатор",
            TypeResolution::primitive("Строка"),
            Some(StructuralMemberSpan::new(12, 24)),
            Certainty::Inferred,
        ));

    let members = resolution.structural_members();
    assert_eq!(members.len(), 1);

    let member = resolution
        .find_structural_member("идентификатор")
        .expect("structural member should be found case-insensitively");

    assert_eq!(member.canonical_name, "Идентификатор");
    assert_eq!(
        member.member_id,
        StructuralMemberId::new("Идентификатор", Some(StructuralMemberSpan::new(12, 24)))
    );
    assert_eq!(member.member_type.type_name(), "Строка");
    assert_eq!(member.source_span, Some(StructuralMemberSpan::new(12, 24)));
    assert_eq!(member.certainty, Certainty::Inferred);
}

#[test]
fn test_find_structural_member_is_case_insensitive() {
    let resolution = TypeResolution::explicit("СтрокаТаблицыЗначений").with_structural_member(
        StructuralMember::new(
            "Количество",
            TypeResolution::primitive("Число"),
            None,
            Certainty::Known,
        ),
    );

    assert!(resolution.find_structural_member("количество").is_some());
    assert!(resolution.find_structural_member("КОЛИЧЕСТВО").is_some());
}

#[test]
fn test_replacing_same_structural_member_preserves_member_id() {
    let first = StructuralMember::new(
        "Идентификатор",
        TypeResolution::primitive("Строка"),
        Some(StructuralMemberSpan::new(12, 24)),
        Certainty::Known,
    );
    let second = StructuralMember::new(
        "Идентификатор",
        TypeResolution::primitive("Число"),
        Some(StructuralMemberSpan::new(48, 60)),
        Certainty::Inferred,
    );
    let expected_id = first.member_id.clone();

    let resolution = TypeResolution::explicit("Структура")
        .with_structural_member(first)
        .with_structural_member(second);
    let member = resolution
        .find_structural_member("идентификатор")
        .expect("replaced structural member");

    assert_eq!(member.member_id, expected_id);
    assert_eq!(member.member_type.type_name(), "Число");
}

#[test]
fn test_structural_member_contract_roundtrips_member_id_through_serde() {
    let resolution =
        TypeResolution::explicit("Структура").with_structural_member(StructuralMember::new(
            "Идентификатор",
            TypeResolution::primitive("Строка"),
            Some(StructuralMemberSpan::new(12, 24)),
            Certainty::Inferred,
        ));

    let json = serde_json::to_string(&resolution).expect("serialize structural resolution");
    let restored: TypeResolution =
        serde_json::from_str(&json).expect("deserialize structural resolution");
    let member = restored
        .find_structural_member("идентификатор")
        .expect("restored structural member");

    assert_eq!(
        member.member_id,
        StructuralMemberId::new("Идентификатор", Some(StructuralMemberSpan::new(12, 24)))
    );
}

#[test]
fn test_structural_member_contract_rehydrates_member_id_from_legacy_payload() {
    let resolution =
        TypeResolution::explicit("Структура").with_structural_member(StructuralMember::new(
            "Идентификатор",
            TypeResolution::primitive("Строка"),
            Some(StructuralMemberSpan::new(12, 24)),
            Certainty::Inferred,
        ));
    let mut legacy_payload =
        serde_json::to_value(&resolution).expect("serialize structural resolution to value");
    legacy_payload["metadata"]["structural_members"][0]
        .as_object_mut()
        .expect("structural member object")
        .remove("member_id");

    let restored: TypeResolution =
        serde_json::from_value(legacy_payload).expect("deserialize legacy structural resolution");
    let member = restored
        .find_structural_member("идентификатор")
        .expect("restored structural member");

    assert_eq!(
        member.member_id,
        StructuralMemberId::new("Идентификатор", Some(StructuralMemberSpan::new(12, 24)))
    );
}
