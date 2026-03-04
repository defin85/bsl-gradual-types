use super::*;

#[test]
fn test_html_escaping() {
    assert_eq!(escape_html("<script>"), "&lt;script&gt;");
    assert_eq!(escape_html("\"quoted\""), "&quot;quoted&quot;");
}
