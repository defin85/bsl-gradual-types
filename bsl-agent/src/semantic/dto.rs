use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PositionDto {
    pub line: u32,
    pub character: u32,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RangeDto {
    pub start: PositionDto,
    pub end: PositionDto,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentRefDto {
    pub root_id: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileRefDto {
    pub doc: DocumentRefDto,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub version: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiagnosticSeverityDto {
    Error,
    Warning,
    Info,
}

impl DiagnosticSeverityDto {
    pub(crate) fn sort_key(self) -> u8 {
        match self {
            DiagnosticSeverityDto::Error => 0,
            DiagnosticSeverityDto::Warning => 1,
            DiagnosticSeverityDto::Info => 2,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticDto {
    pub diagnostic_id: String,
    pub file: DocumentRefDto,
    pub range: RangeDto,
    pub severity: DiagnosticSeverityDto,
    #[serde(default)]
    pub code: Option<String>,
    pub message: String,
}
