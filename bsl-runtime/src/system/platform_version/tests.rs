use super::*;

#[test]
fn parse_platform_version_accepts_normalized_and_prefixed_forms() {
    let direct = parse_platform_version("8.3.25").expect("direct");
    assert_eq!(direct.to_string(), "8.3.25");

    let prefixed = parse_platform_version("Version8_3_25").expect("prefixed");
    assert_eq!(prefixed.to_string(), "8.3.25");
}

#[test]
fn parse_platform_version_rejects_invalid_forms() {
    assert!(parse_platform_version("Version8_3").is_none());
    assert!(parse_platform_version("invalid").is_none());
}
