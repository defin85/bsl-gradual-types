use crate::types::{Certainty, StructuralMember, StructuralMemberSpan, TypeResolution};

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
