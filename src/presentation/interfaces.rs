//! Presentation layer interfaces - заглушки для завершения миграции

use std::sync::Arc;
use crate::application::{LspTypeService, WebTypeService, AnalysisTypeService};

/// LSP Interface для взаимодействия с LSP сервером
pub struct LspInterface {
    lsp_service: Arc<LspTypeService>,
}

impl LspInterface {
    pub fn new(lsp_service: Arc<LspTypeService>) -> Self {
        Self { lsp_service }
    }
}

/// Web Interface для веб-интерфейса
pub struct WebInterface {
    web_service: Arc<WebTypeService>,
}

impl WebInterface {
    pub fn new(web_service: Arc<WebTypeService>) -> Self {
        Self { web_service }
    }
}

/// CLI Interface для командной строки
pub struct CliInterface {
    analysis_service: Arc<AnalysisTypeService>,
}

impl CliInterface {
    pub fn new(analysis_service: Arc<AnalysisTypeService>) -> Self {
        Self { analysis_service }
    }
}
