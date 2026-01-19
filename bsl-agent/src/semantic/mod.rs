pub mod dto;
pub mod facade;
pub mod ids;
pub mod sort;

#[cfg(test)]
mod tests {
    use super::dto::{DiagnosticDto, DiagnosticSeverityDto, DocumentRefDto, PositionDto, RangeDto};
    use super::ids::{diagnostic_id, document_id, stable_id_hex, IdPart};
    use super::sort::sort_diagnostics;

    #[test]
    fn stable_id_hex_is_deterministic() {
        let id_a = stable_id_hex(&[IdPart::Str("a"), IdPart::U64(1), IdPart::U32(2)]);
        let id_b = stable_id_hex(&[IdPart::Str("a"), IdPart::U64(1), IdPart::U32(2)]);
        assert_eq!(id_a, id_b);
    }

    #[test]
    fn diagnostic_id_depends_on_revision() {
        let range = RangeDto {
            start: PositionDto {
                line: 10,
                character: 5,
            },
            end: PositionDto {
                line: 10,
                character: 12,
            },
        };
        let doc_id = document_id("root", "src/Foo.bsl");
        let id_v1 = diagnostic_id(1, &doc_id, &range, Some("E001"), "msg");
        let id_v2 = diagnostic_id(2, &doc_id, &range, Some("E001"), "msg");
        assert_ne!(id_v1, id_v2);
        assert_eq!(
            id_v1,
            diagnostic_id(1, &doc_id, &range, Some("E001"), "msg")
        );
    }

    #[test]
    fn diagnostics_sort_is_deterministic() {
        let file_a = DocumentRefDto {
            root_id: "a".to_string(),
            path: "b.bsl".to_string(),
        };
        let file_b = DocumentRefDto {
            root_id: "b".to_string(),
            path: "a.bsl".to_string(),
        };

        let range_1 = RangeDto {
            start: PositionDto {
                line: 1,
                character: 1,
            },
            end: PositionDto {
                line: 1,
                character: 2,
            },
        };
        let range_2 = RangeDto {
            start: PositionDto {
                line: 0,
                character: 9,
            },
            end: PositionDto {
                line: 0,
                character: 10,
            },
        };

        let mut diagnostics = vec![
            DiagnosticDto {
                diagnostic_id: "2".to_string(),
                file: file_b.clone(),
                range: range_1,
                severity: DiagnosticSeverityDto::Warning,
                code: Some("W".to_string()),
                message: "b".to_string(),
            },
            DiagnosticDto {
                diagnostic_id: "1".to_string(),
                file: file_a.clone(),
                range: range_2,
                severity: DiagnosticSeverityDto::Error,
                code: Some("E".to_string()),
                message: "a".to_string(),
            },
        ];

        sort_diagnostics(&mut diagnostics);

        assert_eq!(diagnostics[0].file, file_a);
        assert_eq!(diagnostics[0].range, range_2);
        assert_eq!(diagnostics[1].file, file_b);
    }
}
