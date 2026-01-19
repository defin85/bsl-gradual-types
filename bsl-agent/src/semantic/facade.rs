use super::dto::{DiagnosticDto, DiagnosticSeverityDto, DocumentRefDto, RangeDto};
use super::{ids, sort};

#[derive(Debug, Default, Clone, Copy)]
pub struct SemanticFacade;

impl SemanticFacade {
    pub fn document_id(file: &DocumentRefDto) -> String {
        ids::document_id(&file.root_id, &file.path)
    }

    pub fn diagnostic(
        &self,
        analysis_revision: u64,
        file: DocumentRefDto,
        range: RangeDto,
        severity: DiagnosticSeverityDto,
        code: Option<String>,
        message: String,
    ) -> DiagnosticDto {
        let document_id = ids::document_id(&file.root_id, &file.path);
        let diagnostic_id = ids::diagnostic_id(
            analysis_revision,
            &document_id,
            &range,
            code.as_deref(),
            &message,
        );

        DiagnosticDto {
            diagnostic_id,
            file,
            range,
            severity,
            code,
            message,
        }
    }

    pub fn sort_diagnostics(&self, diagnostics: &mut Vec<DiagnosticDto>) {
        sort::sort_diagnostics(diagnostics);
    }
}
