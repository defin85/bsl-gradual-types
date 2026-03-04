use super::*;

#[test]
fn test_syntax_helper_source() {
    let types = vec![RawTypeData {
        name: "TestType".to_string(),
        ..Default::default()
    }];

    let source = SyntaxHelperSource::new(types);

    assert_eq!(source.name(), "SyntaxHelper");
    assert_eq!(source.priority(), 10);
    assert_eq!(source.load().len(), 1);
}

#[test]
fn test_syntax_helper_source_empty() {
    let source = SyntaxHelperSource::new(vec![]);

    assert_eq!(source.load().len(), 0);
}
