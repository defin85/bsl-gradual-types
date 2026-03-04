use super::*;

#[test]
fn compute_line_edits_replaces_only_changed_lines() {
    let old = "a  \n  b\n";
    let new = "a\n    b\n";
    let edits = compute_line_edits(old, new);
    assert_eq!(edits.len(), 2);
    assert_eq!(edits[0].new_text, "a");
    assert_eq!(edits[1].new_text, "    b");
}
