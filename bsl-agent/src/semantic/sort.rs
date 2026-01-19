use super::dto::{DiagnosticDto, RangeDto};

pub(crate) fn range_sort_key(range: &RangeDto) -> (u32, u32, u32, u32) {
    (
        range.start.line,
        range.start.character,
        range.end.line,
        range.end.character,
    )
}

pub fn sort_diagnostics(diagnostics: &mut [DiagnosticDto]) {
    diagnostics.sort_by(|a, b| {
        (
            &a.file.root_id,
            &a.file.path,
            range_sort_key(&a.range),
            a.severity.sort_key(),
            &a.code,
            &a.message,
            &a.diagnostic_id,
        )
            .cmp(&(
                &b.file.root_id,
                &b.file.path,
                range_sort_key(&b.range),
                b.severity.sort_key(),
                &b.code,
                &b.message,
                &b.diagnostic_id,
            ))
    });
}
