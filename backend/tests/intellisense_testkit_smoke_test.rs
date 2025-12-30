mod intellisense_testkit;

#[test]
fn test_intellisense_testkit_paths() {
    let fixtures = intellisense_testkit::fixtures_dir();
    assert!(fixtures.exists(), "fixtures dir missing: {}", fixtures.display());

    let golden = intellisense_testkit::golden_dir();
    assert!(golden.exists(), "golden dir missing: {}", golden.display());

    let content = intellisense_testkit::read_fixture("m8_minimal_completion.bsl");
    assert!(!content.is_empty(), "fixture content is empty");
}
