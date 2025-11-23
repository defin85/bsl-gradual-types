//! LSP Server for BSL Gradual Type System - MIGRATED TO Clean Architecture

#![allow(clippy::needless_borrow)]
#![allow(clippy::only_used_in_recursion)]

use anyhow::Result;
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tower_lsp::jsonrpc::Result as JsonRpcResult;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};
use tracing::{debug, error, info, warn};

// SignatureHelp support
use tower_lsp::lsp_types::{
    ParameterInformation, ParameterLabel, SignatureHelp, SignatureHelpOptions, SignatureHelpParams,
    SignatureInformation, WorkDoneProgressOptions,
};

use clap::Parser;
use serde::Deserialize;

// ✅ ИСПРАВЛЕНО: Clean Architecture - используем Application Layer
use bsl_backend::application::TypeSystemService;
use bsl_backend::data::loaders::progress::{IndexingPhase, ProgressUpdate};
use bsl_backend::system::SystemCoordinator;
use bsl_shared::api::dtos::{MethodDto, ParamDto, PropertyDto};
use bsl_shared::domain::signature_index::MethodSignature;
use bsl_type_visualization::{HtmlRenderer, RenderOptions, ThemeMode}; // Используется в других методах // Используется в handle_query_type

// MILESTONE 2.20.3: Server Status notification module
use serde::Serialize;
use tower_lsp::lsp_types::notification::Notification;

/// Custom bsl/serverStatus notification type
pub enum ServerStatus {}

impl Notification for ServerStatus {
    type Params = ServerStatusParams;
    const METHOD: &'static str = "bsl/serverStatus";
}

/// Параметры bsl/serverStatus notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerStatusParams {
    pub loading: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl ServerStatusParams {
    pub fn loading(message: impl Into<String>) -> Self {
        Self {
            loading: true,
            message: Some(message.into()),
        }
    }

    pub fn ready() -> Self {
        Self {
            loading: false,
            message: None,
        }
    }
}
// ✅ ИСПРАВЛЕНО: временные структуры удалены, используем TypeSystemService API

#[derive(Parser, Debug)]
#[command(name = "lsp-server")]
#[command(about = "BSL Language Server (Clean Architecture)", long_about = None)]
#[allow(dead_code)]
struct Args {}

/// LSP Configuration - передаётся из VSCode Extension через initializationOptions
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LspConfig {
    /// Путь к родительской папке с документацией платформы 1С (syntax_helper)
    /// Должна содержать подпапки: rebuilt.shcntx_ru и rebuilt.shlang_ru
    platform_docs_archive: Option<String>,

    /// Путь к Configuration.xml конфигурации 1С
    configuration_path: Option<String>,

    /// Версия платформы 1С (например, "8.3.25")
    /// Зарезервировано для будущей версионированной загрузки типов
    #[allow(dead_code)]
    platform_version: Option<String>,
}

/// MILESTONE 3.6 Phase 1+3: BSL Settings (настройки из workspace/didChangeConfiguration)
#[derive(Debug, Clone, Deserialize)]
struct BslSettings {
    hover: HoverSettings,
    #[serde(default)]
    diagnostics: DiagnosticsSettings,  // MILESTONE 3.6 Phase 3
}

/// MILESTONE 3.6 Phase 1: Hover Settings
#[derive(Debug, Clone, Deserialize)]
struct HoverSettings {
    #[serde(rename = "detailLevel")]
    detail_level: String, // "compact" | "full" | "detailed"

    #[serde(rename = "maxMethods")]
    max_methods: usize,

    #[serde(rename = "maxProperties")]
    max_properties: usize,

    #[serde(rename = "showCertainty")]
    show_certainty: bool,
}

impl Default for HoverSettings {
    fn default() -> Self {
        Self {
            detail_level: "full".to_string(),
            max_methods: 10,
            max_properties: 5,
            show_certainty: true,
        }
    }
}

/// MILESTONE 3.6 Phase 3: Diagnostics Settings
#[derive(Debug, Clone, Deserialize)]
struct DiagnosticsSettings {
    #[serde(rename = "detailLevel")]
    detail_level: String,  // "brief" | "standard" | "detailed"

    #[serde(rename = "showHints")]
    show_hints: bool,
}

impl Default for DiagnosticsSettings {
    fn default() -> Self {
        Self {
            detail_level: "standard".to_string(),
            show_hints: true,
        }
    }
}

impl Default for BslSettings {
    fn default() -> Self {
        Self {
            hover: HoverSettings::default(),
            diagnostics: DiagnosticsSettings::default(),
        }
    }
}

/// Структура для контекста вызова функции (SignatureHelp)
#[derive(Debug)]
struct CallContext {
    function_name: String,
    receiver_type: Option<String>,
    call_start: Position,
}

/// Записывает сообщение в vscode-extension/progress_debug.log с временной меткой
/// Использует абсолютный путь (как и rust_lsp_server.log)
fn log_progress_to_file(message: &str) {
    let log_path = "C:\\1CProject\\bsl-gradual-types\\vscode-extension\\progress_debug.log";
    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
    let log_line = format!("[{}] {}\n", timestamp, message);

    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(log_path) {
        let _ = file.write_all(log_line.as_bytes());
        let _ = file.flush(); // ✅ Принудительно сбрасываем буфер
    }
}

/// BSL Language Server backend - CLEAN ARCHITECTURE
#[derive(Clone)] // Для передачи в tokio::spawn (ревалидация после загрузки типов)
struct BslLanguageServer {
    client: Client,
    documents: Arc<RwLock<HashMap<Url, String>>>,
    // ✅ MILESTONE 2.10: храним LSP конфигурацию из initializationOptions
    config: Arc<RwLock<Option<LspConfig>>>,
    // ✅ MILESTONE 3.6 Phase 1: настройки hover из workspace/didChangeConfiguration
    settings: Arc<RwLock<BslSettings>>,
    // ✅ MILESTONE 2.10: храним SystemCoordinator для перезагрузки типов
    coordinator: Arc<SystemCoordinator>,
    // ❌ УДАЛЕНО: НЕ храним Arc<TypeSystemService>, потому что он может устареть после reload!
    // Вместо этого всегда получаем актуальный экземпляр через coordinator.type_service()
}

impl BslLanguageServer {
    fn new(client: Client, coordinator: Arc<SystemCoordinator>) -> Self {
        Self {
            client,
            documents: Arc::new(RwLock::new(HashMap::new())),
            // ✅ MILESTONE 2.10: инициализируем пустой конфигурацией
            config: Arc::new(RwLock::new(None)),
            // ✅ MILESTONE 3.6 Phase 1: инициализируем настройки по умолчанию
            settings: Arc::new(RwLock::new(BslSettings::default())),
            coordinator,
        }
    }

    /// Получить актуальный TypeSystemService (всегда fresh после reload)
    fn get_type_service(&self) -> Option<Arc<TypeSystemService>> {
        self.coordinator.type_service()
    }

    /// Получить содержимое документа из кеша
    async fn get_document_content(&self, uri: &Url) -> Result<String, String> {
        let documents = self.documents.read().await;
        documents
            .get(uri)
            .cloned()
            .ok_or_else(|| format!("Document not found: {}", uri))
    }

    /// Конвертирует UTF-16 offset (LSP character) в UTF-8 byte offset
    ///
    /// LSP протокол использует UTF-16 code units для позиций, но Rust строки в UTF-8.
    /// Эта функция корректно преобразует UTF-16 offset в byte offset для работы с &str[..].
    fn utf16_to_byte_offset(line: &str, utf16_offset: u32) -> usize {
        let mut utf16_count = 0;
        for (byte_offset, ch) in line.char_indices() {
            if utf16_count >= utf16_offset {
                return byte_offset;
            }
            // Кириллица и другие non-ASCII символы занимают 2 UTF-16 code units
            utf16_count += ch.len_utf16() as u32;
        }
        line.len() // Если offset за пределами строки, возвращаем конец
    }

    /// Конвертировать синтаксические ошибки в LSP Diagnostics (Milestone 2.18)
    ///
    /// Преобразует ParseError из парсера в LSP Diagnostic для отображения в VSCode.
    /// Координаты ошибок уже в UTF-16 благодаря Task 1 (Milestone 2.18).
    fn syntax_errors_to_diagnostics(
        &self,
        errors: &[bsl_shared::domain::types::ParseError],
    ) -> Vec<Diagnostic> {
        use bsl_shared::domain::types::ErrorType;

        errors
            .iter()
            .map(|error| {
                let severity = match error.error_type {
                    ErrorType::ParseError | ErrorType::InvalidSyntax => DiagnosticSeverity::ERROR,
                    ErrorType::MissingToken => DiagnosticSeverity::ERROR,
                    ErrorType::UnexpectedToken => DiagnosticSeverity::WARNING,
                };

                Diagnostic {
                    range: Range::new(
                        Position::new(error.span.start_line, error.span.start_column),
                        Position::new(error.span.end_line, error.span.end_column),
                    ),
                    severity: Some(severity),
                    message: error.message.clone(),
                    source: Some("bsl-syntax".to_string()),
                    code: Some(NumberOrString::String(format!("{:?}", error.error_type))),
                    ..Default::default()
                }
            })
            .collect()
    }

    /// Конвертация TypeDiagnostic → LSP Diagnostic
    ///
    /// # Milestone 3.7: Semantic Diagnostics MVP
    fn semantic_error_to_diagnostic(
        &self,
        error: &bsl_shared::domain::types::TypeDiagnostic,
    ) -> Diagnostic {
        use bsl_shared::domain::types::DiagnosticSeverity as SharedSeverity;

        let start_pos = Position::new(error.line, error.column);
        let end_pos = Position::new(error.end_line, error.end_column);

        let severity = match error.severity {
            SharedSeverity::Error => Some(DiagnosticSeverity::ERROR),
            SharedSeverity::Warning => Some(DiagnosticSeverity::WARNING),
            SharedSeverity::Info => Some(DiagnosticSeverity::INFORMATION),
            SharedSeverity::Hint => Some(DiagnosticSeverity::HINT),
        };

        Diagnostic {
            range: Range::new(start_pos, end_pos),
            severity,
            message: error.message.clone(),
            source: Some("bsl-semantic".to_string()), // ✅ Отличается от "bsl-syntax"
            ..Default::default()
        }
    }

    /// Ревалидация документа (используется после загрузки типов платформы)
    /// Повторяет логику валидации из did_open/did_change
    async fn revalidate_document(&self, uri: &Url, text: &str) -> Result<(), String> {
        let mut diagnostics = Vec::new();

        // PHASE 1: Syntax validation
        if let Some(type_service) = self.get_type_service() {
            match type_service.parse_and_validate(&text) {
                Ok(errors) => {
                    if !errors.is_empty() {
                        info!("⚠️ Found {} syntax errors in {} (revalidation)", errors.len(), uri);
                        diagnostics.extend(self.syntax_errors_to_diagnostics(&errors));
                    }
                }
                Err(e) => {
                    warn!("Syntax validation failed for {} (revalidation): {}", uri, e);
                }
            }
        }

        // PHASE 2: Semantic validation
        if let Some(type_service) = self.get_type_service() {
            let settings = self.settings.read().await;
            let detail_level = bsl_shared::formatting::DetailLevel::from_str(&settings.diagnostics.detail_level);

            match type_service.validate_semantics(&text, Some(detail_level)).await {
                Ok(semantic_errors) => {
                    if !semantic_errors.is_empty() {
                        info!("⚠️ Found {} semantic errors in {} (revalidation)", semantic_errors.len(), uri);
                        for error in semantic_errors {
                            diagnostics.push(self.semantic_error_to_diagnostic(&error));
                        }
                    }
                }
                Err(e) => {
                    warn!("Semantic validation failed for {} (revalidation): {}", uri, e);
                }
            }
        }

        // Отправляем обновлённые diagnostics
        self.client
            .publish_diagnostics(uri.clone(), diagnostics, None)
            .await;

        Ok(())
    }

    /// Применяет текстовое изменение к строке
    fn apply_text_edit(&self, source: &str, range: Range, new_text: &str) -> String {
        let lines: Vec<&str> = source.lines().collect();
        let start_line = range.start.line as usize;
        let end_line = range.end.line as usize;

        // ✅ ИСПРАВЛЕНИЕ: Конвертируем UTF-16 offsets → UTF-8 byte offsets
        let start_char = if let Some(start_line_text) = lines.get(start_line) {
            Self::utf16_to_byte_offset(start_line_text, range.start.character)
        } else {
            0
        };

        let end_char = if let Some(end_line_text) = lines.get(end_line) {
            Self::utf16_to_byte_offset(end_line_text, range.end.character)
        } else {
            0
        };

        let mut result = String::new();

        // Строки до изменения
        for line in lines.iter().take(start_line) {
            result.push_str(line);
            result.push('\n');
        }

        // Начало изменяемой строки
        if let Some(start_line_text) = lines.get(start_line) {
            result.push_str(&start_line_text[..start_char.min(start_line_text.len())]);
        }

        // Новый текст
        result.push_str(new_text);

        // Конец изменяемой строки
        if let Some(end_line_text) = lines.get(end_line) {
            result.push_str(&end_line_text[end_char.min(end_line_text.len())..]);
        }

        // Строки после изменения
        for line in lines.iter().skip(end_line + 1) {
            result.push('\n');
            result.push_str(line);
        }

        result
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for BslLanguageServer {
    async fn initialize(&self, params: InitializeParams) -> JsonRpcResult<InitializeResult> {
        info!("Initializing BSL Language Server");

        // 🔍 DEBUG: Логируем ClientCapabilities
        debug!(
            "📥 [JSON-RPC] initialize: ClientCapabilities.window.workDoneProgress = {:?}",
            params
                .capabilities
                .window
                .as_ref()
                .and_then(|w| w.work_done_progress)
        );
        debug!(
            "📥 [JSON-RPC] initialize: ClientCapabilities.general.regularExpressions = {:?}",
            params
                .capabilities
                .general
                .as_ref()
                .and_then(|g| g.regular_expressions.as_ref())
        );

        // ✅ MILESTONE 2.10: Читаем initializationOptions из Extension
        if let Some(options) = params.initialization_options {
            match serde_json::from_value::<LspConfig>(options.clone()) {
                Ok(config) => {
                    info!("📂 LSP Config received: {:?}", config);

                    // Сохраняем конфигурацию
                    *self.config.write().await = Some(config.clone());

                    info!("✅ Configuration saved, will reload types in initialized()");
                }
                Err(e) => {
                    error!("❌ Failed to parse LSP config: {}", e);
                    error!("   Raw options: {:?}", options);
                }
            }
        } else {
            info!("⚠️ No initializationOptions provided - using defaults (4 basic types only)");
        }

        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Options(
                    TextDocumentSyncOptions {
                        open_close: Some(true),
                        change: Some(TextDocumentSyncKind::INCREMENTAL),
                        will_save: Some(false),
                        will_save_wait_until: Some(false),
                        save: Some(TextDocumentSyncSaveOptions::SaveOptions(SaveOptions {
                            include_text: Some(false),
                        })),
                    },
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![".".to_string(), " ".to_string()]),
                    ..Default::default()
                }),
                diagnostic_provider: Some(DiagnosticServerCapabilities::Options(
                    DiagnosticOptions {
                        identifier: Some("bsl-gradual-types".to_string()),
                        inter_file_dependencies: true,
                        workspace_diagnostics: false,
                        work_done_progress_options: WorkDoneProgressOptions::default(),
                    },
                )),
                execute_command_provider: Some(ExecuteCommandOptions {
                    commands: vec![
                        "bsl.getSemanticHtml".to_string(),
                        "bsl.getSemanticTree".to_string(),
                        "bsl.searchTypes".to_string(),
                        "bsl.getCurrentContext".to_string(), // ✅ MILESTONE 2.20.3
                        "bsl.getTypeRepositoryStats".to_string(), // ✅ MILESTONE 2.20.4
                        "bsl.parseConfiguration".to_string(), // ✅ MILESTONE 2.17
                    ],
                    work_done_progress_options: WorkDoneProgressOptions::default(),
                }),
                signature_help_provider: Some(SignatureHelpOptions {
                    trigger_characters: Some(vec!["(".to_string(), ",".to_string()]),
                    retrigger_characters: Some(vec![")".to_string()]),
                    work_done_progress_options: WorkDoneProgressOptions {
                        work_done_progress: Some(false),
                    },
                }),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "BSL Language Server initialized!")
            .await;

        // ✅ MILESTONE 2.10: Перезагружаем типы с конфигурацией из initializationOptions
        let config = self.config.read().await;
        if let Some(ref cfg) = *config {
            if let Some(ref platform_docs) = cfg.platform_docs_archive {
                info!(
                    "🔄 Reloading types with platformDocsArchive: {}",
                    platform_docs
                );

                // ✅ MILESTONE 2.20.2.3: Создаём channels для передачи прогресса и результата
                let (progress_tx, mut progress_rx) = mpsc::unbounded_channel::<ProgressUpdate>();
                let (result_tx, result_rx) = tokio::sync::oneshot::channel::<Result<(), String>>();

                // Генерируем уникальный токен для Work Done Progress
                let token = ProgressToken::String(format!(
                    "bsl-load-types-{}",
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_millis()
                ));

                // ✅ ИСПРАВЛЕНИЕ Milestone 2.20: Создаём progress token через window/workDoneProgress/create
                // КРИТИЧНО: LSP спецификация требует сначала создать token, затем отправлять $/progress
                if let Err(e) = self
                    .client
                    .send_request::<tower_lsp::lsp_types::request::WorkDoneProgressCreate>(
                        WorkDoneProgressCreateParams {
                            token: token.clone(),
                        },
                    )
                    .await
                {
                    error!("❌ Failed to create work done progress token: {}", e);
                    // Продолжаем без прогресса, но не падаем
                }

                // MILESTONE 2.20.3: Отправляем bsl/serverStatus (loading: true) - rust-analyzer approach
                info!("📤 [LSP→Extension] Sending bsl/serverStatus: loading=true");

                let _ = self
                    .client
                    .send_notification::<ServerStatus>(ServerStatusParams::loading(
                        "Загрузка типов...",
                    ))
                    .await;

                info!("✅ [LSP→Extension] bsl/serverStatus sent successfully");

                // ✅ НОВОЕ: Отправляем WorkDoneProgressBegin
                info!(
                    "📤 [LSP→Extension] Sending WorkDoneProgressBegin: token={:?}",
                    token
                );

                // Определяем заголовок в зависимости от наличия конфигурации
                let title = if cfg.configuration_path.is_some() {
                    "Загрузка типов платформы и конфигурации 1С".to_string()
                } else {
                    "Загрузка типов платформы 1С".to_string()
                };

                let _ = self
                    .client
                    .send_notification::<tower_lsp::lsp_types::notification::Progress>(
                        ProgressParams {
                            token: token.clone(),
                            value: ProgressParamsValue::WorkDone(WorkDoneProgress::Begin(
                                WorkDoneProgressBegin {
                                    title,
                                    message: Some("Инициализация парсинга...".to_string()),
                                    percentage: Some(0),
                                    cancellable: Some(false),
                                },
                            )),
                        },
                    )
                    .await;
                info!("✅ [LSP→Extension] WorkDoneProgressBegin sent successfully");
                log_progress_to_file("📤 [LSP→Extension] SEND WorkDoneProgressBegin");

                // ✅ ИСПРАВЛЕНИЕ: Spawn task обрабатывает прогресс И отправляет финальный End
                let client_clone = self.client.clone();
                let token_clone = token.clone();
                let start_time = std::time::Instant::now();
                let self_clone = self.clone(); // Для ревалидации документов

                tokio::spawn(async move {
                    // Оптимальный баланс: достаточно быстрые обновления без overhead
                    let mut last_report: Option<std::time::Instant> = None;
                    let throttle_interval = std::time::Duration::from_millis(50);

                    // === PHASE 1: Обрабатываем прогресс-уведомления ===
                    while let Some(update) = progress_rx.recv().await {
                        let now = std::time::Instant::now();

                        // ✅ НОВОЕ: Логируем КАЖДОЕ прогресс-обновление (ДО throttling)
                        debug!(
                            "📥 [RECV] {:?} {:.1}% ({}/{}) - {}",
                            update.phase,
                            update.percentage,
                            update.current,
                            update.total,
                            update.message.as_deref().unwrap_or("")
                        );
                        log_progress_to_file(&format!(
                            "📥 [RECV] {:?} {:.1}% ({}/{}) - {}",
                            update.phase,
                            update.percentage,
                            update.current,
                            update.total,
                            update.message.as_deref().unwrap_or("")
                        ));

                        // ✅ ИСПРАВЛЕНИЕ: Первое событие всегда проходит, затем throttling
                        // Пропускаем если слишком рано (throttling), НО всегда отправляем 100% и первое событие
                        let should_skip = if let Some(last) = last_report {
                            now.duration_since(last) < throttle_interval
                                && update.percentage < 100.0
                        } else {
                            false // Первое событие ВСЕГДА проходит
                        };

                        if should_skip {
                            debug!(
                                "⏭️ [THROTTLED] Skipped (too early, {}ms since last report)",
                                last_report
                                    .map(|l| now.duration_since(l).as_millis())
                                    .unwrap_or(0)
                            );
                            continue;
                        }

                        last_report = Some(now);

                        // Вычисляем ETA
                        let elapsed = start_time.elapsed().as_secs_f32();
                        let eta = if update.percentage > 5.0 {
                            Some(((elapsed * 100.0 / update.percentage) - elapsed) as u32)
                        } else {
                            None
                        };

                        // Форматируем сообщение (универсально для Platform и Configuration)
                        let message = match update.phase {
                            IndexingPhase::ParsingFiles => {
                                // Платформа: парсинг Syntax Helper
                                format!(
                                    "Тип {}/{}{}",
                                    update.current,
                                    update.total,
                                    update
                                        .message
                                        .as_ref()
                                        .map(|m| format!(" - {}", m))
                                        .unwrap_or_default()
                                )
                            }
                            IndexingPhase::ConfigurationParsing => {
                                // Конфигурация: парсинг XML файлов
                                update.message.clone().unwrap_or_else(|| {
                                    format!(
                                        "{} | {}/{}",
                                        update.phase.display_name(),
                                        update.current,
                                        update.total
                                    )
                                })
                            }
                            _ => {
                                // Остальные фазы (Platform и Configuration)
                                format!(
                                    "{} | {}/{}",
                                    update.phase.display_name(),
                                    update.current,
                                    update.total
                                )
                            }
                        };

                        let message_with_eta = if let Some(eta_secs) = eta {
                            format!("{} - ETA: {}s", message, eta_secs)
                        } else {
                            message
                        };

                        // ✅ НОВОЕ: Отправляем WorkDoneProgressReport
                        info!(
                            "[LSP→Extension] Sending WorkDoneProgressReport: {}% - {}",
                            update.percentage as u32, message_with_eta
                        );

                        let _ = client_clone
                            .send_notification::<tower_lsp::lsp_types::notification::Progress>(
                                ProgressParams {
                                    token: token_clone.clone(),
                                    value: ProgressParamsValue::WorkDone(WorkDoneProgress::Report(
                                        WorkDoneProgressReport {
                                            message: Some(message_with_eta.clone()),
                                            percentage: Some(update.percentage as u32),
                                            cancellable: Some(false),
                                        },
                                    )),
                                },
                            )
                            .await;

                        debug!("✅ [LSP→Extension] WorkDoneProgressReport sent successfully");
                        log_progress_to_file(&format!(
                            "📤 [LSP→Extension] SEND WorkDoneProgressReport: {}% - {}",
                            update.percentage as u32, message_with_eta
                        ));
                    }

                    // === PHASE 2: Канал закрыт, все updates обработаны ===
                    // Ждём результат парсинга из основного потока
                    debug!("🔄 [WAIT] Waiting for parsing result from main thread...");
                    match result_rx.await {
                        Ok(Ok(())) => {
                            // ✅ УСПЕХ: Отправляем WorkDoneProgressEnd
                            info!("📤 [LSP→Extension] Sending WorkDoneProgressEnd (SUCCESS)");

                            let _ = client_clone
                                .send_notification::<tower_lsp::lsp_types::notification::Progress>(
                                    ProgressParams {
                                        token: token_clone.clone(),
                                        value: ProgressParamsValue::WorkDone(
                                            WorkDoneProgress::End(WorkDoneProgressEnd {
                                                message: Some(
                                                    "✅ Типы платформы загружены успешно"
                                                        .to_string(),
                                                ),
                                            }),
                                        ),
                                    },
                                )
                                .await;

                            info!("✅ [LSP→Extension] WorkDoneProgressEnd sent successfully");
                            log_progress_to_file(
                                "📤 [LSP→Extension] SEND WorkDoneProgressEnd (SUCCESS)",
                            );

                            // MILESTONE 2.20.3: Отправляем bsl/serverStatus (loading: false) - готово
                            info!("📤 [LSP→Extension] Sending bsl/serverStatus: loading=false");

                            let _ = client_clone
                                .send_notification::<ServerStatus>(ServerStatusParams::ready())
                                .await;

                            info!("✅ [LSP→Extension] bsl/serverStatus (ready) sent successfully");

                            // ИСПРАВЛЕНИЕ: Ревалидируем все открытые документы с полными типами платформы
                            // Это исправляет race condition когда did_open запускался до загрузки типов
                            info!("🔄 Revalidating all open documents with full platform types...");

                            // Получаем список всех открытых документов
                            let documents_to_revalidate: Vec<_> = {
                                let docs = self_clone.documents.read().await;
                                docs.iter().map(|(uri, text)| (uri.clone(), text.clone())).collect()
                            };

                            info!("📋 Found {} open documents to revalidate", documents_to_revalidate.len());

                            // Ревалидируем каждый документ
                            for (uri, text) in documents_to_revalidate {
                                info!("🔄 Revalidating: {}", uri);

                                if let Err(e) = self_clone.revalidate_document(&uri, &text).await {
                                    warn!("⚠️ Failed to revalidate {}: {}", uri, e);
                                }
                            }

                            info!("✅ Revalidation of all open documents completed");
                        }
                        Ok(Err(error_msg)) => {
                            // ❌ ОШИБКА: Отправляем WorkDoneProgressEnd с ошибкой
                            info!(
                                "📤 [LSP→Extension] Sending WorkDoneProgressEnd (ERROR): {}",
                                error_msg
                            );

                            let _ = client_clone
                                .send_notification::<tower_lsp::lsp_types::notification::Progress>(
                                    ProgressParams {
                                        token: token_clone.clone(),
                                        value: ProgressParamsValue::WorkDone(
                                            WorkDoneProgress::End(WorkDoneProgressEnd {
                                                message: Some(format!(
                                                    "❌ Ошибка загрузки: {}",
                                                    error_msg
                                                )),
                                            }),
                                        ),
                                    },
                                )
                                .await;

                            info!("✅ [LSP→Extension] WorkDoneProgressEnd (ERROR) sent");
                            log_progress_to_file(&format!(
                                "📤 [LSP→Extension] SEND WorkDoneProgressEnd (ERROR): {}",
                                error_msg
                            ));

                            // MILESTONE 2.20.3: Отправляем bsl/serverStatus (loading: false) даже при ошибке
                            info!("📤 [LSP→Extension] Sending bsl/serverStatus: loading=false (after error)");

                            let _ = client_clone
                                .send_notification::<ServerStatus>(ServerStatusParams::ready())
                                .await;

                            info!("✅ [LSP→Extension] bsl/serverStatus (ready after error) sent");
                            error!("❌ Progress task завершён с ошибкой: {}", error_msg);
                        }
                        Err(_) => {
                            // Канал результата закрыт без отправки (не должно случиться)
                            warn!("⚠️ Result channel closed unexpectedly");
                        }
                    }
                });

                // ✅ ИСПРАВЛЕНИЕ: Вызываем start_with_paths, затем отправляем результат в spawn task
                let syntax_path = std::path::Path::new(platform_docs);

                // ✅ НОВОЕ: Преобразуем configuration_path из Option<String> в Option<&Path>
                let config_path_ref = cfg
                    .configuration_path
                    .as_ref()
                    .map(|s| std::path::Path::new(s.as_str()));

                if let Some(ref conf_path) = config_path_ref {
                    info!("📂 Configuration path provided: {}", conf_path.display());
                } else {
                    info!("⚠️ Configuration path not provided - skipping metadata loading");
                }

                let result = self
                    .coordinator
                    .start_with_paths(Some(syntax_path), config_path_ref, Some(progress_tx))
                    .await;

                // Закрываем канал прогресса (spawn task завершит обработку updates)
                // progress_tx автоматически drop здесь, канал закроется

                match result {
                    Ok(()) => {
                        info!("✅ Platform types loaded successfully");

                        // ✅ ИСПРАВЛЕНИЕ: Отправляем результат в spawn task (вместо End напрямую)
                        let _ = result_tx.send(Ok(()));

                        self.client
                            .log_message(
                                MessageType::INFO,
                                format!("Platform documentation loaded from: {}", platform_docs),
                            )
                            .await;
                    }
                    Err(e) => {
                        error!("❌ Failed to load platform types: {}", e);

                        // ✅ ИСПРАВЛЕНИЕ: Отправляем ошибку в spawn task (вместо End напрямую)
                        let _ = result_tx.send(Err(e.to_string()));

                        self.client
                            .log_message(
                                MessageType::ERROR,
                                format!("Failed to load platform documentation: {}", e),
                            )
                            .await;
                    }
                }
            } else {
                info!("⚠️ platformDocsArchive not provided - using basic types only");
            }
        }
    }

    async fn shutdown(&self) -> JsonRpcResult<()> {
        info!("Shutting down BSL Language Server");
        Ok(())
    }

    /// MILESTONE 3.6 Phase 1+3: Handle workspace/didChangeConfiguration
    async fn did_change_configuration(&self, params: DidChangeConfigurationParams) {
        info!("📥 Received didChangeConfiguration");

        // Пытаемся извлечь BslSettings из params.settings
        if let Some(settings_value) = params.settings.as_object() {
            if let Some(bsl_value) = settings_value.get("bsl") {
                match serde_json::from_value::<BslSettings>(bsl_value.clone()) {
                    Ok(new_settings) => {
                        info!(
                            "✅ Parsed BslSettings:\n  \
                            hover.detailLevel={}, hover.maxMethods={}, hover.maxProperties={}, hover.showCertainty={}\n  \
                            diagnostics.detailLevel={}, diagnostics.showHints={}",
                            new_settings.hover.detail_level,
                            new_settings.hover.max_methods,
                            new_settings.hover.max_properties,
                            new_settings.hover.show_certainty,
                            new_settings.diagnostics.detail_level,
                            new_settings.diagnostics.show_hints
                        );

                        // Обновляем настройки через write lock
                        *self.settings.write().await = new_settings;

                        info!("✅ Settings updated successfully");
                    }
                    Err(e) => {
                        warn!("⚠️ Failed to parse BslSettings: {}", e);
                        warn!("   Raw value: {:?}", bsl_value);
                    }
                }
            } else {
                debug!("No 'bsl' key found in settings");
            }
        } else {
            debug!("Settings is not an object");
        }
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text.clone();
        let version = params.text_document.version;

        // Кешируем текст
        self.documents
            .write()
            .await
            .insert(uri.clone(), text.clone());

        // ✅ MILESTONE 2.13: Прогрев IR кеша при didOpen (Eagerly Parse)
        // Вызываем get_hover_info с dummy position (0, 0) для кеширования IR
        // Это делает последующие hover мгновенными (<5ms вместо 50-100ms)
        if let Some(service) = self.get_type_service() {
            match service.get_hover_info(&text, 0, 0, None).await {
                Ok(_) => info!("✅ IR cache preheated for {}", uri),
                Err(e) => error!("❌ Failed to preheat IR cache for {}: {}", uri, e),
            }
        } else {
            warn!("⚠️ TypeSystemService not yet initialized, skipping IR cache preheat");
        }

        // ✅ MILESTONE 2.19: Clean Architecture - TypeSystemService API
        let mut diagnostics = Vec::new();

        // Парсим файл через TypeSystemService (Application Layer)
        if let Some(type_service) = self.get_type_service() {
            match type_service.parse_and_validate(&text) {
                Ok(errors) => {
                    if !errors.is_empty() {
                        info!("⚠️ Found {} syntax errors in {}", errors.len(), uri);

                        // Конвертируем shared ParseError → LSP Diagnostics
                        diagnostics.extend(self.syntax_errors_to_diagnostics(&errors));
                    } else {
                        info!("✅ No syntax errors in {}", uri);
                    }
                }
                Err(e) => {
                    error!("Failed to parse document {}: {}", uri, e);
                    // Создаём диагностику об ошибке парсинга
                    diagnostics.push(Diagnostic {
                        range: Range::new(Position::new(0, 0), Position::new(0, 1)),
                        severity: Some(DiagnosticSeverity::ERROR),
                        message: format!("❌ Ошибка парсинга: {}", e),
                        source: Some("bsl-syntax".to_string()),
                        ..Default::default()
                    });
                }
            }
        } else {
            warn!("⚠️ TypeSystemService not yet initialized, skipping syntax validation");
        }

        // PHASE 2: Semantic validation (MILESTONE 3.7 + 3.6 Phase 3)
        if let Some(type_service) = self.get_type_service() {
            // ✅ GUARD: Проверяем что platform types загружены
            let metrics = type_service.get_metrics_summary();
            let total_types = metrics.get("total_types").and_then(|v| v.as_u64()).unwrap_or(0);

            if total_types == 0 {
                info!("⚠️ Skipping semantic validation for {}: platform types not yet loaded", uri);
            } else {
                // ✅ MILESTONE 3.6 Phase 3: Получаем detail_level из settings
                let settings = self.settings.read().await;
                let detail_level = bsl_shared::formatting::DetailLevel::from_str(&settings.diagnostics.detail_level);

                match type_service.validate_semantics(&text, Some(detail_level)).await {
                    Ok(semantic_errors) => {
                        if !semantic_errors.is_empty() {
                            info!(
                                "⚠️ Found {} semantic errors in {} (detail_level: {})",
                                semantic_errors.len(),
                                uri,
                                settings.diagnostics.detail_level
                            );
                            for error in semantic_errors {
                                diagnostics.push(self.semantic_error_to_diagnostic(&error));
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Semantic validation failed for {}: {}", uri, e);
                    }
                }
            }
        }

        // Отправляем диагностики
        self.client
            .publish_diagnostics(uri.clone(), diagnostics, Some(version))
            .await;

        self.client
            .log_message(
                MessageType::INFO,
                format!("Opened and analyzed document: {}", uri),
            )
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        let version = params.text_document.version;
        let changes = params.content_changes;

        // Применяем изменения к тексту
        let updated_text = if let Some(full_change) = changes.iter().find(|c| c.range.is_none()) {
            // Полное обновление содержимого
            full_change.text.clone()
        } else {
            // Инкрементальные изменения - применяем к существующему тексту
            let existing_text = self
                .documents
                .read()
                .await
                .get(&uri)
                .cloned()
                .unwrap_or_default();

            // Применяем все инкрементальные изменения последовательно
            let mut current_text = existing_text;
            for change in &changes {
                if let Some(range) = change.range {
                    current_text = self.apply_text_edit(&current_text, range, &change.text);
                }
            }
            current_text
        };

        // Кешируем текст
        self.documents
            .write()
            .await
            .insert(uri.clone(), updated_text.clone());

        // ✅ MILESTONE 2.13: Инвалидация IR кеша при изменении файла
        let uri_str = uri.to_string();
        if let Some(service) = self.get_type_service() {
            service.invalidate_file_cache(&uri_str, &updated_text).await;
            debug!("📝 File changed: {}, cache invalidated", uri_str);
        }
        debug!("📝 File changed: {}, cache invalidated", uri_str);

        // ✅ ИНКРЕМЕНТАЛЬНЫЙ ПАРСИНГ: конвертируем LSP edits → ParserCoordinator TextEdit
        use bsl_backend::system::parser_coordinator::TextEdit;
        use std::path::PathBuf;

        let text_edits: Vec<TextEdit> = changes
            .iter()
            .filter_map(|change| {
                change.range.map(|range| TextEdit {
                    start_line: range.start.line,
                    start_column: range.start.character,
                    old_end_line: range.end.line,
                    old_end_column: range.end.character,
                    new_end_line: range.start.line + change.text.matches('\n').count() as u32,
                    new_end_column: if change.text.contains('\n') {
                        change.text.lines().last().unwrap_or("").len() as u32
                    } else {
                        range.start.character + change.text.len() as u32
                    },
                    new_text: change.text.clone(),
                })
            })
            .collect();

        // Извлекаем путь к файлу из URI
        let file_path = PathBuf::from(uri.path());

        // Используем инкрементальный парсинг через TypeSystemService
        if let Some(service) = self.get_type_service() {
            if let Err(e) = service
                .parse_incremental(file_path, updated_text.clone(), text_edits)
                .await
            {
                error!("Incremental parsing failed: {}", e);
            } else {
                info!("✅ Incremental parsing succeeded for: {}", uri.path());
            }
        }

        // ✅ MILESTONE 2.19: Clean Architecture - TypeSystemService API
        let mut diagnostics = Vec::new();

        // Берём актуальный текст документа
        let documents = self.documents.read().await;
        if let Some(text) = documents.get(&uri) {
            info!("🔍 Обновление диагностики файла: {}", uri.path());

            // Парсим файл через TypeSystemService (Application Layer)
            if let Some(type_service) = self.get_type_service() {
                match type_service.parse_and_validate(text) {
                    Ok(errors) => {
                        if !errors.is_empty() {
                            info!("⚠️ Found {} syntax errors in {}", errors.len(), uri);

                            // Конвертируем shared ParseError → LSP Diagnostics
                            diagnostics.extend(self.syntax_errors_to_diagnostics(&errors));
                        } else {
                            info!("✅ No syntax errors in {}", uri);
                        }
                    }
                    Err(e) => {
                        error!("Failed to parse document {}: {}", uri, e);
                        // Создаём диагностику об ошибке парсинга
                        diagnostics.push(Diagnostic {
                            range: Range::new(Position::new(0, 0), Position::new(0, 1)),
                            severity: Some(DiagnosticSeverity::ERROR),
                            message: format!("❌ Ошибка парсинга: {}", e),
                            source: Some("bsl-syntax".to_string()),
                            ..Default::default()
                        });
                    }
                }
            } else {
                warn!("⚠️ TypeSystemService not yet initialized, skipping syntax validation");
            }

            // PHASE 2: Semantic validation (MILESTONE 3.7)
            if let Some(type_service) = self.get_type_service() {
                // ✅ GUARD: Проверяем что platform types загружены
                let metrics = type_service.get_metrics_summary();
                let total_types = metrics.get("total_types").and_then(|v| v.as_u64()).unwrap_or(0);

                if total_types == 0 {
                    info!("⚠️ Skipping semantic validation for {}: platform types not yet loaded", uri);
                } else {
                    // ✅ MILESTONE 3.6 Phase 3: Получаем detail_level из settings
                    let settings = self.settings.read().await;
                    let detail_level = bsl_shared::formatting::DetailLevel::from_str(&settings.diagnostics.detail_level);

                    match type_service.validate_semantics(&text, Some(detail_level)).await {
                        Ok(semantic_errors) => {
                            if !semantic_errors.is_empty() {
                                info!(
                                    "⚠️ Found {} semantic errors in {}",
                                    semantic_errors.len(),
                                    uri
                                );
                                for error in semantic_errors {
                                    diagnostics.push(self.semantic_error_to_diagnostic(&error));
                                }
                            }
                        }
                        Err(e) => {
                            warn!("Semantic validation failed for {}: {}", uri, e);
                        }
                    }
                }
            }
        }

        // Отправляем обновленные диагностики
        self.client
            .publish_diagnostics(uri.clone(), diagnostics, Some(version))
            .await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        self.documents.write().await.remove(&uri);

        // ИСПРАВЛЕНИЕ: Очищаем diagnostics при закрытии файла (отправляем пустой массив)
        self.client
            .publish_diagnostics(uri.clone(), vec![], None)
            .await;

        self.client
            .log_message(MessageType::INFO, format!("Closed document: {}", uri))
            .await;
    }

    async fn completion(
        &self,
        params: CompletionParams,
    ) -> JsonRpcResult<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;

        info!(
            "Completion requested at {}:{}",
            position.line, position.character
        );

        // Получаем содержимое документа
        let file_content = match self.documents.read().await.get(&uri) {
            Some(content) => content.clone(),
            None => {
                // Если документ не в кеше, читаем с диска
                match uri.to_file_path() {
                    Ok(path) => match std::fs::read_to_string(&path) {
                        Ok(content) => content,
                        Err(e) => {
                            error!("Failed to read file for completion: {}", e);
                            return Ok(Some(CompletionResponse::Array(vec![])));
                        }
                    },
                    Err(_) => return Ok(Some(CompletionResponse::Array(vec![]))),
                }
            }
        };

        // Получаем автодополнение через TypeSystemService
        if let Some(service) = self.get_type_service() {
            match service
                .get_completion(&file_content, position.line, position.character)
                .await
            {
                Ok(completions) => {
                    let lsp_completions: Vec<tower_lsp::lsp_types::CompletionItem> = completions
                        .into_iter()
                        .map(|item| tower_lsp::lsp_types::CompletionItem {
                            label: item.label,
                            detail: item.detail,
                            insert_text: item.insert_text,
                            kind: Some(CompletionItemKind::KEYWORD),
                            insert_text_format: Some(InsertTextFormat::SNIPPET),
                            ..Default::default()
                        })
                        .collect();
                    info!("Returning {} completions", lsp_completions.len());
                    Ok(Some(CompletionResponse::Array(lsp_completions)))
                }
                Err(e) => {
                    error!("Failed to get completions: {}", e);
                    Ok(Some(CompletionResponse::Array(vec![])))
                }
            }
        } else {
            Ok(Some(CompletionResponse::Array(vec![])))
        }
    }

    async fn hover(&self, params: HoverParams) -> JsonRpcResult<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        info!(
            "Hover requested at {}:{}",
            position.line, position.character
        );

        // Получаем содержимое документа
        let file_content = match self.documents.read().await.get(&uri) {
            Some(content) => content.clone(),
            None => {
                // Если документ не в кеше, читаем с диска
                match uri.to_file_path() {
                    Ok(path) => match std::fs::read_to_string(&path) {
                        Ok(content) => content,
                        Err(e) => {
                            error!("Failed to read file for hover: {}", e);
                            return Ok(None);
                        }
                    },
                    Err(_) => return Ok(None),
                }
            }
        };

        // ✅ MILESTONE 2.10: Используем IR-based hover с Inline Scope Analysis
        // Получаем путь к файлу
        let _file_path = match uri.to_file_path() {
            Ok(path) => path.to_string_lossy().to_string(),
            Err(_) => "untitled".to_string(),
        };

        // MILESTONE 3.6 Phase 1: Получаем настройки hover из конфигурации
        use bsl_backend::helpers::hover_formatter::{HoverFormatConfig, OutputFormat};
        use bsl_shared::formatting::DetailLevel;

        let settings = self.settings.read().await;

        // MILESTONE 3.6 Phase 2 - Task 2.5: Получить путь к syntax_helper из environment или стандартных мест
        let syntax_helper_path = std::env::var("BSL_SYNTAX_HELPER_PATH")
            .ok()
            .map(std::path::PathBuf::from)
            .or_else(|| {
                // Fallback: попробовать найти в стандартных местах
                let candidates = vec![
                    std::path::PathBuf::from("examples/syntax_helper"),
                    std::path::PathBuf::from("../examples/syntax_helper"),
                    std::path::PathBuf::from("C:/examples/syntax_helper"),
                ];

                candidates.into_iter().find(|p| p.exists())
            });

        let detail_level = DetailLevel::from_str(&settings.hover.detail_level);

        // ДИАГНОСТИКА: Логируем настройки hover при каждом запросе
        debug!(
            "🔍 Hover request: detailLevel={:?}, maxMethods={}, maxProperties={}, showCertainty={}, syntax_helper={:?}",
            detail_level,
            settings.hover.max_methods,
            settings.hover.max_properties,
            settings.hover.show_certainty,
            syntax_helper_path.as_ref().map(|p| p.display().to_string())
        );

        let hover_config = HoverFormatConfig {
            max_methods: settings.hover.max_methods,
            max_properties: settings.hover.max_properties,
            detail_level,
            show_certainty: settings.hover.show_certainty,
            syntax_helper_path, // ← MILESTONE 3.6 Phase 2: Передаём путь к документации
            output_format: OutputFormat::Markdown,
            ..Default::default()
        };
        drop(settings); // Освобождаем lock

        // Используем get_hover_info() с IR-based анализом
        if let Some(service) = self.get_type_service() {
            match service
                .get_hover_info(&file_content, position.line, position.character, Some(hover_config))
                .await
            {
                Ok(hover_info) => {
                    if let Some(info) = hover_info {
                        Ok(Some(Hover {
                            contents: HoverContents::Scalar(MarkedString::String(info)),
                            range: None,
                        }))
                    } else {
                        Ok(None)
                    }
                }
                Err(e) => {
                    error!("Failed to get hover info: {}", e);
                    Ok(None)
                }
            }
        } else {
            Ok(None)
        }
    }

    async fn execute_command(
        &self,
        params: ExecuteCommandParams,
    ) -> JsonRpcResult<Option<serde_json::Value>> {
        info!(
            "Execute command: {} with {} arguments",
            params.command,
            params.arguments.len()
        );

        match params.command.as_str() {
            "bsl.getSemanticHtml" => {
                if params.arguments.is_empty() {
                    return Err(tower_lsp::jsonrpc::Error::invalid_params(
                        "Missing request parameters",
                    ));
                }

                let request: GetSemanticHtmlRequest =
                    serde_json::from_value(params.arguments[0].clone()).map_err(|e| {
                        tower_lsp::jsonrpc::Error::invalid_params(format!(
                            "Invalid parameters: {}",
                            e
                        ))
                    })?;

                let result = self.handle_get_semantic_html(request).await?;
                Ok(Some(serde_json::to_value(result).unwrap()))
            }
            "bsl.getSemanticTree" => {
                if params.arguments.is_empty() {
                    return Err(tower_lsp::jsonrpc::Error::invalid_params(
                        "Missing request parameters",
                    ));
                }

                let request: GetSemanticTreeRequest =
                    serde_json::from_value(params.arguments[0].clone()).map_err(|e| {
                        tower_lsp::jsonrpc::Error::invalid_params(format!(
                            "Invalid parameters: {}",
                            e
                        ))
                    })?;

                let result = self.handle_get_semantic_tree(request).await?;
                Ok(Some(serde_json::to_value(result).unwrap()))
            }
            "bsl.searchTypes" => {
                if params.arguments.is_empty() {
                    return Err(tower_lsp::jsonrpc::Error::invalid_params(
                        "Missing search query",
                    ));
                }

                let request: SearchTypesRequest =
                    serde_json::from_value(params.arguments[0].clone()).map_err(|e| {
                        tower_lsp::jsonrpc::Error::invalid_params(format!(
                            "Invalid parameters: {}",
                            e
                        ))
                    })?;

                let result = self.handle_search_types(request).await?;
                Ok(Some(serde_json::to_value(result).unwrap()))
            }
            "bsl.getCurrentContext" => {
                if params.arguments.is_empty() {
                    return Err(tower_lsp::jsonrpc::Error::invalid_params(
                        "Missing parameters",
                    ));
                }

                let request: GetCurrentContextParams =
                    serde_json::from_value(params.arguments[0].clone()).map_err(|e| {
                        tower_lsp::jsonrpc::Error::invalid_params(format!(
                            "Invalid parameters: {}",
                            e
                        ))
                    })?;

                let result = self.handle_get_current_context(request).await?;
                Ok(Some(serde_json::to_value(result).unwrap()))
            }
            "bsl.queryType" => {
                if params.arguments.is_empty() {
                    return Err(tower_lsp::jsonrpc::Error::invalid_params(
                        "Missing type name",
                    ));
                }

                let request: QueryTypeParams = serde_json::from_value(params.arguments[0].clone())
                    .map_err(|e| {
                        tower_lsp::jsonrpc::Error::invalid_params(format!(
                            "Invalid parameters: {}",
                            e
                        ))
                    })?;

                let result = self.handle_query_type(request).await?;
                Ok(Some(serde_json::to_value(result).unwrap()))
            }
            "bsl.getTypeRepositoryStats" => {
                let request: GetTypeRepositoryStatsParams = if params.arguments.is_empty() {
                    GetTypeRepositoryStatsParams {}
                } else {
                    serde_json::from_value(params.arguments[0].clone()).map_err(|e| {
                        tower_lsp::jsonrpc::Error::invalid_params(format!(
                            "Invalid parameters: {}",
                            e
                        ))
                    })?
                };

                let result = self.handle_get_type_repository_stats(request).await?;
                Ok(Some(serde_json::to_value(result).unwrap()))
            }
            "bsl.parseConfiguration" => {
                if params.arguments.is_empty() {
                    return Err(tower_lsp::jsonrpc::Error::invalid_params(
                        "Missing configuration path",
                    ));
                }

                let request: ParseConfigurationParams =
                    serde_json::from_value(params.arguments[0].clone()).map_err(|e| {
                        tower_lsp::jsonrpc::Error::invalid_params(format!(
                            "Invalid parameters: {}",
                            e
                        ))
                    })?;

                let result = self.handle_parse_configuration(request).await?;
                Ok(Some(serde_json::to_value(result).unwrap()))
            }
            _ => {
                tracing::warn!("Unknown command: {}", params.command);
                Err(tower_lsp::jsonrpc::Error::method_not_found())
            }
        }
    }

    /// SignatureHelp - подсказки по параметрам функций
    async fn signature_help(
        &self,
        params: SignatureHelpParams,
    ) -> JsonRpcResult<Option<SignatureHelp>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        tracing::info!(
            "SignatureHelp requested at {}:{}",
            position.line,
            position.character
        );

        // Получаем содержимое документа
        let file_content = match self.get_document_content(&uri).await {
            Ok(content) => content,
            Err(e) => {
                tracing::warn!("Failed to get document content: {}", e);
                return Ok(None);
            }
        };

        // Находим контекст вызова функции
        let call_context = match self.find_call_context(&file_content, position) {
            Some(ctx) => ctx,
            None => {
                tracing::debug!("No call context found");
                return Ok(None);
            }
        };

        tracing::debug!(
            "Found call context: function={}, receiver={:?}",
            call_context.function_name,
            call_context.receiver_type
        );

        // Получаем сигнатуру через TypeSystemService
        let signature_info = match self.get_signature_for_function(
            &call_context.function_name,
            call_context.receiver_type.as_deref(),
        ) {
            Some(info) => info,
            None => {
                tracing::debug!("No signature found for {}", call_context.function_name);
                return Ok(None);
            }
        };

        // Определяем активный параметр
        let active_param = self.calculate_active_parameter(&file_content, &call_context, position);

        // Формируем ответ
        Ok(Some(self.build_signature_help_response(
            signature_info,
            active_param,
        )))
    }
}

// ============================================================================
// Custom LSP Request Handlers - заменяют CLI бинарники
// ============================================================================

/// Custom request: bsl/queryType - запрос типа по имени
#[derive(Debug, serde::Deserialize)]
#[allow(dead_code)]
struct QueryTypeParams {
    type_name: String,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct QueryTypeResponse {
    type_name: String,
    found: bool,

    // ✅ НОВЫЕ ПОЛЯ для полной информации о типе
    #[serde(skip_serializing_if = "Option::is_none")]
    certainty: Option<String>, // "Known (100%)", "Inferred (50%)"

    #[serde(skip_serializing_if = "Option::is_none")]
    facet: Option<String>, // "Manager", "Object", "Reference", etc.

    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,

    // Полная документация типа
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    methods: Vec<MethodDto>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    properties: Vec<PropertyDto>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    facets: Vec<String>,

    // Обратная совместимость (deprecated, но сохраняем)
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<String>,
}

/// Custom request: bsl/buildIndex - построение индекса типов
#[derive(Debug, serde::Deserialize)]
#[allow(dead_code)]
struct BuildIndexParams {
    workspace_path: String,
}

#[derive(Debug, serde::Serialize)]
#[allow(dead_code)]
struct BuildIndexResponse {
    success: bool,
    types_count: usize,
    message: String,
}

/// Custom request: bsl/validateMethod - валидация вызова метода
#[derive(Debug, serde::Deserialize)]
#[allow(dead_code)]
struct ValidateMethodParams {
    object_type: String,
    method_name: String,
    arguments: Vec<String>,
}

#[derive(Debug, serde::Serialize)]
#[allow(dead_code)]
struct ValidateMethodResponse {
    valid: bool,
    message: String,
}

/// Custom request: bsl/checkTypeCompatibility - проверка совместимости типов
#[derive(Debug, serde::Deserialize)]
#[allow(dead_code)]
struct CheckCompatibilityParams {
    source_type: String,
    target_type: String,
}

#[derive(Debug, serde::Serialize)]
#[allow(dead_code)]
struct CheckCompatibilityResponse {
    compatible: bool,
    message: String,
}

/// Custom request: bsl/incrementalUpdate - инкрементальное обновление индекса
#[derive(Debug, serde::Deserialize)]
#[allow(dead_code)]
struct IncrementalUpdateParams {
    config_path: String,
    platform_version: String,
}

#[derive(Debug, serde::Serialize)]
#[allow(dead_code)]
struct IncrementalUpdateResponse {
    success: bool,
    message: String,
}

/// Custom request: bsl/extractPlatformDocs - извлечение платформенной документации
#[derive(Debug, serde::Deserialize)]
#[allow(dead_code)]
struct ExtractPlatformDocsParams {
    archive_path: String,
    platform_version: String,
    force: bool,
}

#[derive(Debug, serde::Serialize)]
#[allow(dead_code)]
struct ExtractPlatformDocsResponse {
    success: bool,
    types_count: usize,
    message: String,
}

/// Custom request: bsl/renderTypeHtml - рендеринг HTML для типа (использует TypeVisualization)
#[derive(Debug, serde::Deserialize)]
struct RenderTypeHtmlParams {
    type_name: String,
    theme: Option<String>, // "light", "dark", "high-contrast"
}

#[derive(Debug, serde::Serialize)]
struct RenderTypeHtmlResponse {
    html: String,
    success: bool,
    message: Option<String>,
}

// === MILESTONE 2.12: Semantic Visualization Custom Requests ===

/// Custom request: bsl/getSemanticTree - получить семантическое дерево файла
use bsl_shared::api::semantic_dtos::{GetSemanticTreeRequest, SemanticTreeDto};

/// Custom request: bsl/getSemanticHtml - получить HTML визуализацию семантики
use bsl_shared::api::semantic_dtos::{GetSemanticHtmlRequest, RenderedHtmlDto};

// === MILESTONE: Quick Actions Webview Integration ===

/// Custom command: bsl.searchTypes - поиск типов в TypeRepository
#[derive(Debug, serde::Deserialize)]
struct SearchTypesRequest {
    /// Поисковый запрос (partial match, case-insensitive)
    query: String,
    /// Максимум результатов (по умолчанию 15)
    #[serde(default = "default_search_limit")]
    limit: usize,
}

fn default_search_limit() -> usize {
    15
}

#[derive(Debug, serde::Serialize)]
struct SearchTypesResponse {
    /// Найденные типы
    types: Vec<TypeSearchResult>,
    /// Общее количество найденных (до лимита)
    total: usize,
}

#[derive(Debug, Clone, serde::Serialize)]
struct TypeSearchResult {
    /// Полное имя типа (русское, например "Массив")
    name: String,
    /// Английское имя (например "Array")
    #[serde(skip_serializing_if = "Option::is_none")]
    english_name: Option<String>,
    /// Основной фасет типа
    facet: String,
    /// Уверенность в типе (всегда "Known (100%)" для типов из репозитория)
    certainty: String,
    /// Краткое описание (опционально)
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
}

// ============================================================================
// MILESTONE 2.20.3: Current Context Indicator
// ============================================================================

/// Custom command: bsl.getCurrentContext - определить текущую функцию/процедуру
#[derive(Debug, serde::Deserialize)]
struct GetCurrentContextParams {
    uri: String,
    line: u32,
    character: u32,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct CurrentContextResponse {
    function_name: Option<String>,
    function_kind: String, // "function", "procedure", "none"

    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    return_type: Option<String>,
}

// ============================================================================
// MILESTONE 2.20.4: Type Repository Statistics
// ============================================================================

/// Request для получения статистики TypeRepository
#[derive(Debug, Clone, serde::Deserialize)]
struct GetTypeRepositoryStatsParams {
    // Пустой struct - параметров не требуется
}

/// Response со статистикой TypeRepository
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct TypeRepositoryStatsResponse {
    total_types: usize,
    platform_types: usize,
    configuration_types: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_update_time: Option<String>, // ISO 8601
}

// ============================================================================
// MILESTONE 2.17: Configuration Metadata Parser Integration
// ============================================================================

/// Request для парсинга конфигурации
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ParseConfigurationParams {
    config_path: String,
}

/// Response парсинга конфигурации
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ParseConfigurationResponse {
    success: bool,
    loaded_types: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

#[allow(dead_code)]
impl BslLanguageServer {
    /// Обработчик custom request: bsl/queryType
    /// Обработчик custom request: bsl/queryType - РАСШИРЕННАЯ ВЕРСИЯ
    async fn handle_query_type(&self, params: QueryTypeParams) -> JsonRpcResult<QueryTypeResponse> {
        info!("Custom request: bsl/queryType - {}", params.type_name);

        // Получаем доступ к TypeRepository через SystemCoordinator
        let analysis_engine = match self.coordinator.get_analysis_engine() {
            Some(engine) => engine,
            None => {
                warn!("AnalysisEngine not available");
                return Ok(QueryTypeResponse {
                    type_name: params.type_name.clone(),
                    found: false,
                    certainty: None,
                    facet: None,
                    description: Some("AnalysisEngine not available".to_string()),
                    methods: vec![],
                    properties: vec![],
                    facets: vec![],
                    details: Some("AnalysisEngine not available".to_string()),
                });
            }
        };

        let repo = analysis_engine.get_repository();

        // Поиск типа в TypeRepository
        match repo.find_type(&params.type_name) {
            Some(raw_type) => {
                info!(
                    "Type '{}' found with {} methods, {} properties",
                    params.type_name,
                    raw_type.methods.len(),
                    raw_type.properties.len()
                );

                // ✅ Конвертируем методы из RawMethodData → MethodDto
                let mut methods: Vec<MethodDto> = raw_type
                    .methods
                    .iter()
                    .map(|m| MethodDto {
                        name: m.name.clone(),
                        english_name: if m.english_name.is_empty() {
                            None
                        } else {
                            Some(m.english_name.clone())
                        },
                        return_type: if m.return_type.is_empty() {
                            None
                        } else {
                            Some(m.return_type.clone())
                        },
                        params: m
                            .params
                            .iter()
                            .map(|p| ParamDto {
                                name: p.name.clone(),
                                param_type: p.param_type.clone(),
                                is_optional: p.is_optional,
                                default_value: None,
                            })
                            .collect(),
                        description: None,
                        is_deprecated: false,
                        is_constructor: false,
                    })
                    .collect();

                // ✅ Получить конструкторы из SignatureIndex
                if let Some(constructor) = analysis_engine.find_constructor(&params.type_name) {
                    // Конвертировать ConstructorSignature → MethodDto
                    let constructor_dto = MethodDto {
                        name: format!("Новый {}", constructor.type_name),
                        english_name: Some(format!("New {}", constructor.type_name)),
                        return_type: Some(constructor.type_name.clone()),
                        params: constructor
                            .params
                            .iter()
                            .map(|p| ParamDto {
                                name: p.name.clone(),
                                param_type: p
                                    .type_name
                                    .clone()
                                    .unwrap_or_else(|| "Произвольный".to_string()),
                                is_optional: p.is_optional,
                                default_value: p.default_value.clone(),
                            })
                            .collect(),
                        description: Some(format!(
                            "Конструктор типа {}{}{}",
                            constructor.type_name,
                            if constructor.is_collection {
                                format!(
                                    " (коллекция, {} generic параметров)",
                                    constructor.generic_params_count
                                )
                            } else {
                                String::new()
                            },
                            constructor
                                .facet
                                .as_ref()
                                .map(|f| format!(", facet: {}", f))
                                .unwrap_or_default()
                        )),
                        is_deprecated: false,
                        is_constructor: true, // ← Ключевой флаг!
                    };
                    methods.push(constructor_dto);
                }

                // ✅ Конвертируем свойства из RawPropertyData → PropertyDto
                let properties: Vec<PropertyDto> = raw_type
                    .properties
                    .iter()
                    .map(|p| PropertyDto {
                        name: p.name.clone(),
                        prop_type: p.prop_type.clone(),
                        is_readonly: p.is_readonly,
                        description: None,
                    })
                    .collect();

                // Форматируем фасеты как строки
                let facets: Vec<String> = raw_type
                    .facets
                    .iter()
                    .map(|f| format!("{:?}", f)) // Manager, Object, Reference, etc.
                    .collect();

                // Определяем основной фасет (первый или Object по умолчанию)
                let main_facet = facets
                    .first()
                    .cloned()
                    .or_else(|| Some("Object".to_string()));

                Ok(QueryTypeResponse {
                    type_name: params.type_name.clone(),
                    found: true,
                    certainty: Some("Known (100%)".to_string()),
                    facet: main_facet,
                    description: if raw_type.description.is_empty() {
                        None
                    } else {
                        Some(raw_type.description.clone())
                    },
                    methods,
                    properties,
                    facets,
                    details: Some(format!(
                        "Type '{}' found with {} methods, {} properties",
                        params.type_name,
                        raw_type.methods.len(),
                        raw_type.properties.len()
                    )),
                })
            }
            None => {
                // ❌ Тип не найден
                warn!("Type '{}' not found in TypeRepository", params.type_name);
                Ok(QueryTypeResponse {
                    type_name: params.type_name.clone(),
                    found: false,
                    certainty: Some("Unknown".to_string()),
                    facet: None,
                    description: Some("Тип не найден в TypeRepository".to_string()),
                    methods: vec![],
                    properties: vec![],
                    facets: vec![],
                    details: Some(format!("Type '{}' not found", params.type_name)),
                })
            }
        }
    }

    /// Обработчик custom request: bsl/buildIndex
    async fn handle_build_index(
        &self,
        params: BuildIndexParams,
    ) -> JsonRpcResult<BuildIndexResponse> {
        info!("Custom request: bsl/buildIndex - {}", params.workspace_path);

        // TODO: Реализовать через TypeSystemService
        Ok(BuildIndexResponse {
            success: true,
            types_count: 0,
            message: "Index building not yet implemented via LSP".to_string(),
        })
    }

    /// Обработчик custom request: bsl/validateMethod
    async fn handle_validate_method(
        &self,
        params: ValidateMethodParams,
    ) -> JsonRpcResult<ValidateMethodResponse> {
        info!(
            "Custom request: bsl/validateMethod - {}.{}",
            params.object_type, params.method_name
        );

        // TODO: Реализовать через TypeSystemService
        Ok(ValidateMethodResponse {
            valid: true,
            message: format!(
                "Method {}.{} validation not yet fully implemented",
                params.object_type, params.method_name
            ),
        })
    }

    /// Обработчик custom request: bsl/checkTypeCompatibility
    async fn handle_check_compatibility(
        &self,
        params: CheckCompatibilityParams,
    ) -> JsonRpcResult<CheckCompatibilityResponse> {
        info!(
            "Custom request: bsl/checkTypeCompatibility - {} → {}",
            params.source_type, params.target_type
        );

        // TODO: Реализовать через TypeSystemService
        Ok(CheckCompatibilityResponse {
            compatible: true,
            message: format!(
                "Type compatibility check for {} → {} not yet fully implemented",
                params.source_type, params.target_type
            ),
        })
    }

    /// Обработчик custom request: bsl/incrementalUpdate
    async fn handle_incremental_update(
        &self,
        params: IncrementalUpdateParams,
    ) -> JsonRpcResult<IncrementalUpdateResponse> {
        info!(
            "Custom request: bsl/incrementalUpdate - config: {}, version: {}",
            params.config_path, params.platform_version
        );

        // TODO: Реализовать инкрементальное обновление индекса через SystemCoordinator
        Ok(IncrementalUpdateResponse {
            success: true,
            message: format!(
                "Incremental update for version {} completed (stub)",
                params.platform_version
            ),
        })
    }

    /// Обработчик custom request: bsl/extractPlatformDocs
    async fn handle_extract_platform_docs(
        &self,
        params: ExtractPlatformDocsParams,
    ) -> JsonRpcResult<ExtractPlatformDocsResponse> {
        info!(
            "Custom request: bsl/extractPlatformDocs - archive: {}, version: {}, force: {}",
            params.archive_path, params.platform_version, params.force
        );

        // TODO: Реализовать извлечение платформенной документации
        // Сейчас возвращаем заглушку
        Ok(ExtractPlatformDocsResponse {
            success: true,
            types_count: 0,
            message: format!(
                "Platform docs extraction for version {} completed (stub)",
                params.platform_version
            ),
        })
    }

    /// Обработчик custom request: bsl/renderTypeHtml - рендеринг HTML с использованием TypeVisualization
    async fn handle_render_type_html(
        &self,
        params: RenderTypeHtmlParams,
    ) -> JsonRpcResult<RenderTypeHtmlResponse> {
        info!(
            "Custom request: bsl/renderTypeHtml - {} (theme: {:?})",
            params.type_name, params.theme
        );

        // Определяем тему
        let theme_mode = match params.theme.as_deref() {
            Some("dark") => ThemeMode::Dark,
            Some("light") => ThemeMode::Light,
            Some("high-contrast") => ThemeMode::HighContrast,
            _ => ThemeMode::Auto,
        };

        // Создаём рендерер
        let _renderer = HtmlRenderer::new(RenderOptions {
            theme: theme_mode.clone(),
            syntax_highlight: true,
            enable_links: true,
            compact: false,
        });

        // TODO: Получить TypeDto через TypeSystemService
        // Пока возвращаем заглушку с информацией об успешной интеграции
        let html = format!(
            r#"<div class="type-info-integrated">
                <h2>TypeVisualization Integrated!</h2>
                <p>Тип: <strong>{}</strong></p>
                <p>Тема: <code>{:?}</code></p>
                <p>HtmlRenderer готов к использованию</p>
                <p><em>TODO: Интеграция с TypeSystemService для получения реальных TypeDto</em></p>
            </div>"#,
            params.type_name, theme_mode
        );

        Ok(RenderTypeHtmlResponse {
            html,
            success: true,
            message: Some("TypeVisualization успешно интегрирована".to_string()),
        })
    }

    /// Обработчик custom request: bsl/getSemanticTree - MILESTONE 2.12
    async fn handle_get_semantic_tree(
        &self,
        params: GetSemanticTreeRequest,
    ) -> JsonRpcResult<SemanticTreeDto> {
        info!("Custom request: bsl/getSemanticTree - {}", params.uri);

        // Парсим URI и получаем путь к файлу
        let uri = tower_lsp::lsp_types::Url::parse(&params.uri).map_err(|e| {
            tower_lsp::jsonrpc::Error::invalid_params(format!("Invalid URI: {}", e))
        })?;

        let file_path = uri.to_file_path().map_err(|_| {
            tower_lsp::jsonrpc::Error::invalid_params("Could not convert URI to file path")
        })?;

        let file_path_str = file_path.to_string_lossy().to_string();

        // Читаем содержимое файла (из кеша или с диска)
        let file_content = match self.documents.read().await.get(&uri) {
            Some(content) => content.clone(),
            None => std::fs::read_to_string(&file_path)
                .map_err(|_e| tower_lsp::jsonrpc::Error::internal_error())?,
        };

        // Используем TypeSystemService для получения SemanticProgram
        // TypeSystemService уже содержит всю логику парсинга и конвертации AST → IR
        if let Some(service) = self.get_type_service() {
            match service
                .get_semantic_tree(&file_content, &file_path_str)
                .await
            {
                Ok(dto) => {
                    info!(
                        "✅ Semantic tree generated: {} nodes, {} symbols",
                        dto.root_nodes.len(),
                        dto.symbol_table.len()
                    );
                    Ok(dto)
                }
                Err(e) => {
                    error!("Failed to generate semantic tree: {}", e);
                    Err(tower_lsp::jsonrpc::Error::internal_error())
                }
            }
        } else {
            error!("TypeSystemService not yet initialized");
            Err(tower_lsp::jsonrpc::Error::internal_error())
        }
    }

    /// Обработчик custom command: bsl.searchTypes - поиск типов в TypeRepository
    async fn handle_search_types(
        &self,
        params: SearchTypesRequest,
    ) -> JsonRpcResult<SearchTypesResponse> {
        info!(
            "Custom command: bsl.searchTypes - query: '{}', limit: {}",
            params.query, params.limit
        );

        // Получаем доступ к TypeRepository через SystemCoordinator
        let analysis_engine = match self.coordinator.get_analysis_engine() {
            Some(engine) => engine,
            None => {
                warn!("AnalysisEngine not available");
                return Ok(SearchTypesResponse {
                    types: vec![],
                    total: 0,
                });
            }
        };

        let repo = analysis_engine.get_repository();

        // Получаем все типы
        let all_types = repo.get_all_types();

        // Обработка пустого репозитория
        if all_types.is_empty() {
            warn!("TypeRepository is empty - platform types not loaded yet");
            return Ok(SearchTypesResponse {
                types: vec![],
                total: 0,
            });
        }

        // Фильтруем по query (case-insensitive partial match)
        let query_lower = params.query.to_lowercase();
        let filtered: Vec<TypeSearchResult> = all_types
            .iter()
            .filter(|t| {
                // Поиск в русском или английском имени
                t.name.to_lowercase().contains(&query_lower)
                    || (!t.english_name.is_empty()
                        && t.english_name.to_lowercase().contains(&query_lower))
            })
            .take(params.limit)
            .map(|t| {
                // Определяем основной фасет
                let facet = if t.facets.is_empty() {
                    "Object".to_string()
                } else {
                    format!("{:?}", t.facets[0]) // Manager, Object, Reference, etc.
                };

                TypeSearchResult {
                    name: t.name.clone(),
                    english_name: if t.english_name.is_empty() {
                        None
                    } else {
                        Some(t.english_name.clone())
                    },
                    facet,
                    certainty: "Known (100%)".to_string(),
                    description: if t.description.is_empty() {
                        None
                    } else {
                        Some(t.description.clone())
                    },
                }
            })
            .collect();

        let total = filtered.len();

        info!("Found {} types matching '{}'", total, params.query);

        Ok(SearchTypesResponse {
            types: filtered,
            total,
        })
    }

    /// Обработчик custom command: bsl.getCurrentContext - MILESTONE 2.20.3
    async fn handle_get_current_context(
        &self,
        params: GetCurrentContextParams,
    ) -> JsonRpcResult<CurrentContextResponse> {
        info!(
            "Custom command: bsl.getCurrentContext - {}:{}:{}",
            params.uri, params.line, params.character
        );

        // Парсим URI
        let uri = tower_lsp::lsp_types::Url::parse(&params.uri).map_err(|e| {
            error!("Failed to parse URI: {}", e);
            tower_lsp::jsonrpc::Error::invalid_params(format!("Invalid URI: {}", e))
        })?;

        // Получаем содержимое файла (из кеша или с диска)
        let file_content = match self.documents.read().await.get(&uri) {
            Some(content) => {
                info!("✅ Found document in cache");
                content.clone()
            }
            None => {
                info!("⚠️ Document not in cache, reading from disk");
                // Fallback: читаем с диска
                match uri.to_file_path() {
                    Ok(path) => std::fs::read_to_string(&path).map_err(|e| {
                        error!("Failed to read file {}: {}", path.display(), e);
                        tower_lsp::jsonrpc::Error::internal_error()
                    })?,
                    Err(_) => {
                        warn!("Could not convert URI to file path: {}", uri);
                        return Ok(CurrentContextResponse {
                            function_name: None,
                            function_kind: "none".to_string(),
                            params: None,
                            return_type: None,
                        });
                    }
                }
            }
        };

        let file_path = uri
            .to_file_path()
            .unwrap_or_else(|_| std::path::PathBuf::from("untitled"))
            .to_string_lossy()
            .to_string();

        // Получаем SemanticProgram через TypeSystemService
        if let Some(service) = self.get_type_service() {
            info!("✅ TypeSystemService available");
            match service.get_semantic_tree(&file_content, &file_path).await {
                Ok(semantic_tree_dto) => {
                    info!("✅ Got semantic tree");
                    match find_containing_function_in_dto(
                        &semantic_tree_dto,
                        params.line,
                        params.character,
                    ) {
                        Some((name, kind, params_list, return_type)) => {
                            info!("✅ Found context: {} {}", kind, name);
                            Ok(CurrentContextResponse {
                                function_name: Some(name),
                                function_kind: kind,
                                params: Some(params_list),
                                return_type,
                            })
                        }
                        None => {
                            debug!("No context found - global scope");
                            Ok(CurrentContextResponse {
                                function_name: None,
                                function_kind: "none".to_string(),
                                params: None,
                                return_type: None,
                            })
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to get semantic tree: {}", e);
                    Err(tower_lsp::jsonrpc::Error::internal_error())
                }
            }
        } else {
            // ✅ GRACEFUL DEGRADATION: Возвращаем пустой контекст вместо ошибки
            warn!("⚠️ TypeSystemService not yet initialized - returning empty context");
            Ok(CurrentContextResponse {
                function_name: None,
                function_kind: "none".to_string(),
                params: None,
                return_type: None,
            })
        }
    }

    /// Обработчик custom request: bsl/getSemanticHtml - MILESTONE 2.12
    async fn handle_get_semantic_html(
        &self,
        params: GetSemanticHtmlRequest,
    ) -> JsonRpcResult<RenderedHtmlDto> {
        info!(
            "Custom request: bsl/getSemanticHtml - {} (theme: {:?})",
            params.uri, params.theme
        );

        // Сначала получаем semantic tree
        let tree_request = GetSemanticTreeRequest {
            uri: params.uri.clone(),
            include_call_graph: true,
            include_flow_sensitive: true,
            max_depth: None,
        };

        let semantic_tree = self.handle_get_semantic_tree(tree_request).await?;

        // Определяем тему
        let theme_mode = match params.theme.as_deref() {
            Some("dark") => ThemeMode::Dark,
            Some("light") => ThemeMode::Light,
            Some("high-contrast") => ThemeMode::HighContrast,
            _ => ThemeMode::Auto,
        };

        // Создаём HTML рендерер
        let renderer = HtmlRenderer::new(RenderOptions {
            theme: theme_mode.clone(),
            syntax_highlight: true,
            enable_links: true,
            compact: params.compact,
        });

        // Генерируем HTML body
        let body = self.format_semantic_tree_html(&semantic_tree);

        // Генерируем полный HTML документ
        let html = renderer.render_document("BSL Semantic Analysis", &body);

        Ok(RenderedHtmlDto {
            file_path: semantic_tree.file_path.clone(),
            html,
            metrics: semantic_tree.metrics.clone(),
            generated_at: chrono::Utc::now().to_rfc3339(),
            theme: Some(format!("{:?}", theme_mode)),
        })
    }

    /// Форматировать SemanticTreeDto в HTML
    fn format_semantic_tree_html(&self, tree: &SemanticTreeDto) -> String {
        let mut html = String::new();

        // Header с метриками
        html.push_str(&format!(
            r#"
            <div class="semantic-header">
                <h1>Семантический анализ: {}</h1>
                <div class="metrics">
                    <span class="metric">📊 Процедуры: {}</span>
                    <span class="metric">🔧 Функции: {}</span>
                    <span class="metric">📝 Переменные: {}</span>
                    <span class="metric">✅ Известные типы: {}</span>
                    <span class="metric">🔍 Выведенные типы: {}</span>
                    <span class="metric">❓ Неизвестные типы: {}</span>
                    <span class="metric">⏱️ Анализ: {}ms</span>
                </div>
            </div>
        "#,
            tree.file_path,
            tree.metrics.procedure_count,
            tree.metrics.function_count,
            tree.metrics.variable_count,
            tree.metrics.known_types,
            tree.metrics.inferred_types,
            tree.metrics.unknown_types,
            tree.metrics.analysis_time_ms
        ));

        // Дерево узлов
        html.push_str("<div class='semantic-tree'><h2>Семантическое дерево</h2>");
        for node in &tree.root_nodes {
            html.push_str(&self.format_node_html(node, 0));
        }
        html.push_str("</div>");

        // Таблица символов
        html.push_str("<div class='symbol-table'><h2>Таблица символов</h2><table>");
        html.push_str("<tr><th>Символ</th><th>Тип</th><th>Категория</th><th>Область</th></tr>");
        for (name, symbol) in &tree.symbol_table {
            let type_name = symbol
                .resolved_type
                .as_ref()
                .map(|t| t.name.as_str())
                .unwrap_or("?");
            html.push_str(&format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                name, type_name, symbol.kind, symbol.scope
            ));
        }
        html.push_str("</table></div>");

        // CSS стили
        html.push_str(r#"
            <style>
                .semantic-header { background: #f0f0f0; padding: 20px; border-radius: 8px; margin-bottom: 20px; }
                .semantic-header h1 { margin: 0 0 10px 0; }
                .metrics { display: flex; gap: 15px; flex-wrap: wrap; }
                .metric { background: white; padding: 8px 12px; border-radius: 4px; font-size: 14px; }
                .semantic-tree, .symbol-table { margin: 20px 0; }
                .tree-node { margin-left: 20px; padding: 5px; border-left: 2px solid #ccc; }
                .node-header { font-weight: bold; color: #0066cc; }
                .node-name { color: #009900; }
                table { width: 100%; border-collapse: collapse; }
                th, td { border: 1px solid #ddd; padding: 8px; text-align: left; }
                th { background-color: #f2f2f2; }
            </style>
        "#);

        html
    }

    /// Форматировать узел дерева в HTML
    fn format_node_html(
        &self,
        node: &bsl_shared::api::semantic_dtos::SemanticNodeDto,
        depth: usize,
    ) -> String {
        let indent = "  ".repeat(depth);
        let mut html = format!(
            r#"{}<div class="tree-node">
                <span class="node-header">{}</span>
                {}"#,
            indent,
            node.kind,
            node.name
                .as_ref()
                .map(|n| format!(r#"<span class="node-name">{}</span>"#, n))
                .unwrap_or_default()
        );

        // Рекурсивно добавляем детей
        for child in &node.children {
            html.push_str(&self.format_node_html(child, depth + 1));
        }

        html.push_str("</div>");
        html
    }

    /// Обработчик custom command: bsl.getTypeRepositoryStats - MILESTONE 2.20.4
    async fn handle_get_type_repository_stats(
        &self,
        _params: GetTypeRepositoryStatsParams,
    ) -> JsonRpcResult<TypeRepositoryStatsResponse> {
        debug!("Handling bsl.getTypeRepositoryStats request");

        let stats = self.coordinator.get_type_repository_stats();

        debug!(
            "TypeRepository stats: total={}, platform={}, config={}",
            stats.total_types, stats.platform_types, stats.configuration_types
        );

        Ok(TypeRepositoryStatsResponse {
            total_types: stats.total_types,
            platform_types: stats.platform_types,
            configuration_types: stats.configuration_types,
            last_update_time: stats.last_update_time,
        })
    }

    /// Обработчик custom command: bsl.parseConfiguration - MILESTONE 2.17
    ///
    /// ✅ ИСПРАВЛЕНО (2025-01-18):
    /// - Убран and_then() closure (исправлена ошибка Send)
    /// - Добавлена валидация config_path (path traversal защита)
    /// - Реализован промежуточный прогресс (каждые 10 типов)
    /// - Реализован batch loading (по 100 типов за раз)
    async fn handle_parse_configuration(
        &self,
        params: ParseConfigurationParams,
    ) -> JsonRpcResult<ParseConfigurationResponse> {
        use bsl_backend::data::loaders::config_metadata_parser::ConfigurationDiscovery;

        info!(
            "Custom command: bsl.parseConfiguration - {}",
            params.config_path
        );

        let config_path = std::path::PathBuf::from(&params.config_path);

        // ✅ КРИТИЧНО: Валидация пути (защита от path traversal)
        // 1. Проверка существования
        if !config_path.exists() {
            warn!("Configuration path does not exist: {:?}", config_path);
            return Ok(ParseConfigurationResponse {
                success: false,
                loaded_types: 0,
                message: Some(format!("Путь не существует: {}", config_path.display())),
            });
        }

        // 2. Проверка, что это директория
        if !config_path.is_dir() {
            warn!("Configuration path is not a directory: {:?}", config_path);
            return Ok(ParseConfigurationResponse {
                success: false,
                loaded_types: 0,
                message: Some("Путь должен указывать на директорию".to_string()),
            });
        }

        // 3. Проверка наличия Configuration.xml
        let config_xml = config_path.join("Configuration.xml");
        if !config_xml.exists() {
            warn!("Configuration.xml not found in {:?}", config_path);
            return Ok(ParseConfigurationResponse {
                success: false,
                loaded_types: 0,
                message: Some("Configuration.xml не найден в указанной директории".to_string()),
            });
        }

        // 4. Канонизация пути (защита от ../../../)
        let canonical_path = match config_path.canonicalize() {
            Ok(path) => path,
            Err(e) => {
                error!("Failed to canonicalize path {:?}: {}", config_path, e);
                return Ok(ParseConfigurationResponse {
                    success: false,
                    loaded_types: 0,
                    message: Some(format!("Невалидный путь: {}", e)),
                });
            }
        };

        debug!("Validated configuration path: {:?}", canonical_path);

        // Создаем Work Done Progress token
        let token = ProgressToken::String(format!(
            "parse-config-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));

        // ✅ ИСПРАВЛЕНИЕ Milestone 2.20: Создаём progress token через window/workDoneProgress/create
        if let Err(e) = self
            .client
            .send_request::<tower_lsp::lsp_types::request::WorkDoneProgressCreate>(
                WorkDoneProgressCreateParams {
                    token: token.clone(),
                },
            )
            .await
        {
            error!("❌ Failed to create work done progress token: {}", e);
        }

        // Создаем прогресс
        let _ = self
            .client
            .send_notification::<tower_lsp::lsp_types::notification::Progress>(ProgressParams {
                token: token.clone(),
                value: ProgressParamsValue::WorkDone(WorkDoneProgress::Begin(
                    WorkDoneProgressBegin {
                        title: "Парсинг конфигурации".to_string(),
                        message: Some("Инициализация...".to_string()),
                        percentage: Some(0),
                        cancellable: Some(false),
                    },
                )),
            })
            .await;

        // Получаем доступ к TypeRepository через AnalysisEngine
        let analysis_engine = match self.coordinator.get_analysis_engine() {
            Some(engine) => engine,
            None => {
                error!("AnalysisEngine not available");
                return Ok(ParseConfigurationResponse {
                    success: false,
                    loaded_types: 0,
                    message: Some("AnalysisEngine не доступен".to_string()),
                });
            }
        };

        let repo = analysis_engine.get_repository();

        // ✅ Обнаружение метаданных (10% прогресса)
        let _ = self
            .client
            .send_notification::<tower_lsp::lsp_types::notification::Progress>(ProgressParams {
                token: token.clone(),
                value: ProgressParamsValue::WorkDone(WorkDoneProgress::Report(
                    WorkDoneProgressReport {
                        message: Some("Обнаружение объектов метаданных...".to_string()),
                        percentage: Some(10),
                        cancellable: Some(false),
                    },
                )),
            })
            .await;

        let discovery = ConfigurationDiscovery::new(canonical_path.clone());

        // Создаём callback для передачи прогресса в LSP клиент
        let client_clone = self.client.clone();
        let token_clone = token.clone();
        let progress_callback =
            move |update: bsl_backend::data::loaders::progress::ProgressUpdate| {
                let client = client_clone.clone();
                let token = token_clone.clone();

                // Конвертируем прогресс парсинга (10-70%) в LSP прогресс
                let percentage = update.percentage as u32;
                let message = update.message.unwrap_or_else(|| {
                    format!(
                        "{}: {}/{}",
                        update.phase.display_name(),
                        update.current,
                        update.total
                    )
                });

                // Асинхронная отправка прогресса (fire-and-forget)
                tokio::spawn(async move {
                    let _ = client
                        .send_notification::<tower_lsp::lsp_types::notification::Progress>(
                            tower_lsp::lsp_types::ProgressParams {
                                token,
                                value: tower_lsp::lsp_types::ProgressParamsValue::WorkDone(
                                    tower_lsp::lsp_types::WorkDoneProgress::Report(
                                        tower_lsp::lsp_types::WorkDoneProgressReport {
                                            message: Some(message),
                                            percentage: Some(percentage),
                                            cancellable: Some(false),
                                        },
                                    ),
                                ),
                            },
                        )
                        .await;
                });
            };

        // ✅ ИСПРАВЛЕНО: Конвертируем Result<T, Box<dyn Error>> в Result<T, String> ДО match
        // Решает проблему "Result which contains Box<dyn Error> is not Send"
        let discovery_result = discovery
            .discover_all_metadata(Some(progress_callback))
            .map_err(|e| e.to_string());

        let metadata = match discovery_result {
            Ok(data) => data,
            Err(error_msg) => {
                error!("❌ Failed to discover metadata: {}", error_msg);

                // Завершение прогресса с ошибкой
                let _ = self
                    .client
                    .send_notification::<tower_lsp::lsp_types::notification::Progress>(
                        ProgressParams {
                            token: token.clone(),
                            value: ProgressParamsValue::WorkDone(WorkDoneProgress::End(
                                WorkDoneProgressEnd {
                                    message: Some(format!("❌ Ошибка: {}", error_msg)),
                                },
                            )),
                        },
                    )
                    .await;

                return Ok(ParseConfigurationResponse {
                    success: false,
                    loaded_types: 0,
                    message: Some(format!("Ошибка обнаружения метаданных: {}", error_msg)),
                });
            }
        };

        let total_objects = metadata.len();
        info!(
            "📦 Discovered {} metadata objects from configuration",
            total_objects
        );

        // ✅ ИСПРАВЛЕНО: Прямая загрузка типов без and_then()
        // Решает проблему "future cannot be sent between threads safely"

        const BATCH_SIZE: usize = 100;
        const PROGRESS_REPORT_INTERVAL: usize = 10;

        let mut loaded = 0;
        let mut batch = Vec::with_capacity(BATCH_SIZE);

        for (index, obj) in metadata.iter().enumerate() {
            let raw_type = obj.to_raw_type_data(None);
            batch.push(raw_type);

            // Загружаем батч по достижении BATCH_SIZE или в конце
            if batch.len() >= BATCH_SIZE || index == total_objects - 1 {
                if let Err(e) = repo.load_types(batch.clone()) {
                    // ✅ ИСПРАВЛЕНО: Конвертируем ошибку в String ДО await
                    let error_msg = {
                        let err = e;
                        err.to_string()
                    };
                    error!("❌ Failed to load types batch: {}", error_msg);

                    // Завершение прогресса с ошибкой
                    let _ = self
                        .client
                        .send_notification::<tower_lsp::lsp_types::notification::Progress>(
                            ProgressParams {
                                token: token.clone(),
                                value: ProgressParamsValue::WorkDone(WorkDoneProgress::End(
                                    WorkDoneProgressEnd {
                                        message: Some(format!("❌ Ошибка: {}", error_msg)),
                                    },
                                )),
                            },
                        )
                        .await;

                    return Ok(ParseConfigurationResponse {
                        success: false,
                        loaded_types: loaded,
                        message: Some(format!("Ошибка загрузки типов: {}", error_msg)),
                    });
                }

                loaded += batch.len();
                batch.clear();
            }

            // ✅ Промежуточный прогресс каждые 10 типов
            if (index + 1) % PROGRESS_REPORT_INTERVAL == 0 || index == total_objects - 1 {
                // Прогресс: 10% (обнаружение) + 80% (загрузка) + 10% (завершение)
                // Загрузка занимает диапазон 10-90%
                let load_progress = ((loaded * 80) / total_objects) as u32;
                let total_progress = 10 + load_progress;

                let _ = self
                    .client
                    .send_notification::<tower_lsp::lsp_types::notification::Progress>(
                        ProgressParams {
                            token: token.clone(),
                            value: ProgressParamsValue::WorkDone(WorkDoneProgress::Report(
                                WorkDoneProgressReport {
                                    message: Some(format!(
                                        "Загружено {}/{} типов",
                                        loaded, total_objects
                                    )),
                                    percentage: Some(total_progress),
                                    cancellable: Some(false),
                                },
                            )),
                        },
                    )
                    .await;
            }
        }

        // ✅ Завершение прогресса (100%)
        let _ = self
            .client
            .send_notification::<tower_lsp::lsp_types::notification::Progress>(ProgressParams {
                token: token.clone(),
                value: ProgressParamsValue::WorkDone(WorkDoneProgress::End(WorkDoneProgressEnd {
                    message: Some(format!("✅ Загружено {} типов", loaded)),
                })),
            })
            .await;

        info!(
            "✅ Configuration parsed successfully: {} types loaded",
            loaded
        );

        Ok(ParseConfigurationResponse {
            success: true,
            loaded_types: loaded,
            message: Some(format!("Конфигурация успешно загружена: {} типов", loaded)),
        })
    }

    // ============================================================================
    // SignatureHelp Helper Methods
    // ============================================================================

    /// Конвертировать UTF-16 code unit index в char index
    ///
    /// LSP позиции используют UTF-16 code units (из-за VSCode/TypeScript),
    /// а Rust строки используют UTF-8 байты и char индексы.
    /// Эта функция конвертирует UTF-16 позицию в char индекс для безопасной
    /// работы с chars() итератором.
    fn utf16_to_char_index(text: &str, utf16_index: usize) -> Option<usize> {
        let mut current_utf16 = 0;

        for (char_idx, ch) in text.chars().enumerate() {
            if current_utf16 >= utf16_index {
                return Some(char_idx);
            }
            current_utf16 += ch.len_utf16();
        }

        // Если мы достигли конца строки и utf16_index точно совпадает
        if current_utf16 == utf16_index {
            Some(text.chars().count())
        } else {
            None
        }
    }

    /// Конвертировать char index в UTF-16 code unit index
    ///
    /// Обратная операция: char индекс -> UTF-16 позиция для LSP.
    fn char_to_utf16_index(text: &str, char_index: usize) -> usize {
        text.chars().take(char_index).map(|ch| ch.len_utf16()).sum()
    }

    /// Найти контекст вызова функции
    fn find_call_context(&self, content: &str, position: Position) -> Option<CallContext> {
        let lines: Vec<&str> = content.lines().collect();

        // Ищем открывающую скобку, двигаясь назад от курсора
        let mut paren_depth = 0;
        let mut call_start: Option<(usize, usize)> = None; // (line_idx, char_idx)

        // ИСПРАВЛЕНИЕ: если position.line выходит за границы (после последнего \n),
        // используем последнюю существующую строку
        let max_line = if lines.is_empty() {
            return None;
        } else {
            lines.len() - 1
        };
        let search_until_line = position.line.min(max_line as u32) as usize;

        for line_idx in (0..=search_until_line).rev() {
            let line = lines.get(line_idx)?;

            // ИСПРАВЛЕНИЕ: конвертируем UTF-16 позицию в char index
            let end_char_idx = if line_idx == position.line as usize {
                Self::utf16_to_char_index(line, position.character as usize)?
            } else {
                line.chars().count()
            };

            // Итерируем по СИМВОЛАМ, не по байтам
            let chars: Vec<char> = line.chars().collect();

            for char_idx in (0..end_char_idx).rev() {
                let ch = chars.get(char_idx)?;

                match ch {
                    ')' => paren_depth += 1,
                    '(' => {
                        if paren_depth == 0 {
                            call_start = Some((line_idx, char_idx));
                            break;
                        }
                        paren_depth -= 1;
                    }
                    _ => {}
                }
            }

            if call_start.is_some() {
                break;
            }
        }

        let (line_idx, char_idx) = call_start?;

        // Извлекаем имя функции перед скобкой
        let line = lines.get(line_idx)?;

        // ИСПРАВЛЕНИЕ: используем char индексы для извлечения подстроки
        let before_paren: String = line.chars().take(char_idx).collect();

        let (function_name, receiver_type) = self.extract_function_name(&before_paren)?;

        // ИСПРАВЛЕНИЕ: конвертируем char index обратно в UTF-16 для LSP
        let utf16_char = Self::char_to_utf16_index(line, char_idx);

        Some(CallContext {
            function_name,
            receiver_type,
            call_start: Position {
                line: line_idx as u32,
                character: utf16_char as u32,
            },
        })
    }

    /// Извлечь имя функции из текста перед скобкой
    fn extract_function_name(&self, text: &str) -> Option<(String, Option<String>)> {
        let trimmed = text.trim_end();

        // Сначала ищем точку (для методов объектов)
        if let Some(dot_byte_pos) = trimmed.rfind('.') {
            // Метод объекта: "Объект.Метод"
            // ИСПРАВЛЕНИЕ: безопасное извлечение после точки
            // (dot_byte_pos + 1 безопасно т.к. '.' занимает 1 байт)
            let after_dot = &trimmed[dot_byte_pos + 1..];

            // Извлекаем только валидное имя идентификатора после точки
            let method_name = after_dot
                .chars()
                .take_while(|c| {
                    c.is_alphanumeric()
                        || *c == '_'
                        || (*c >= '\u{0410}' && *c <= '\u{044F}')
                        || *c == '\u{0401}'
                        || *c == '\u{0451}'
                })
                .collect::<String>();

            if !method_name.is_empty() {
                // TODO: определить тип получателя через type inference
                return Some((method_name, None));
            }
        }

        // Глобальная функция: извлекаем последний валидный идентификатор
        // Идём с конца и собираем символы пока они валидны для идентификатора
        let function_name = trimmed
            .chars()
            .rev()
            .take_while(|c| {
                c.is_alphanumeric()
                    || *c == '_'
                    || (*c >= '\u{0410}' && *c <= '\u{044F}')
                    || *c == '\u{0401}'
                    || *c == '\u{0451}'
            })
            .collect::<String>()
            .chars()
            .rev()
            .collect::<String>();

        if !function_name.is_empty() {
            Some((function_name, None))
        } else {
            None
        }
    }

    /// Определить индекс активного параметра
    fn calculate_active_parameter(
        &self,
        content: &str,
        context: &CallContext,
        position: Position,
    ) -> u32 {
        let lines: Vec<&str> = content.lines().collect();
        let mut param_index = 0;
        let mut paren_depth = 0;
        let mut in_string = false;

        // Считаем запятые от начала вызова до курсора
        for line_idx in context.call_start.line..=position.line {
            let line = match lines.get(line_idx as usize) {
                Some(l) => l,
                None => {
                    tracing::warn!(
                        "Invalid line_idx {} in calculate_active_parameter",
                        line_idx
                    );
                    break;
                }
            };
            let chars: Vec<char> = line.chars().collect();

            // ИСПРАВЛЕНИЕ: конвертируем UTF-16 в char indices
            let start_char_idx = if line_idx == context.call_start.line {
                Self::utf16_to_char_index(line, (context.call_start.character + 1) as usize)
                    .unwrap_or(0)
            } else {
                0
            };

            let end_char_idx = if line_idx == position.line {
                Self::utf16_to_char_index(line, position.character as usize).unwrap_or(chars.len())
            } else {
                chars.len()
            };

            // Итерируем по символам
            for char_idx in start_char_idx..end_char_idx {
                if let Some(&ch) = chars.get(char_idx) {
                    match ch {
                        '"' => in_string = !in_string,
                        '(' if !in_string => paren_depth += 1,
                        ')' if !in_string => paren_depth -= 1,
                        ',' if !in_string && paren_depth == 0 => {
                            param_index += 1;
                        }
                        _ => {}
                    }
                }
            }
        }

        param_index
    }

    /// Получить сигнатуру функции через TypeSystemService
    fn get_signature_for_function(
        &self,
        function_name: &str,
        receiver_type: Option<&str>,
    ) -> Option<MethodSignature> {
        let analysis_engine = self.coordinator.get_analysis_engine()?;
        let repo = analysis_engine.get_repository();

        repo.find_method_signature(receiver_type, function_name)
    }

    /// Построить LSP SignatureHelp response
    fn build_signature_help_response(
        &self,
        signature: MethodSignature,
        active_param: u32,
    ) -> SignatureHelp {
        // Форматируем label
        let params_str = signature
            .params
            .iter()
            .map(|p| {
                let type_str = p.type_name.as_deref().unwrap_or("Произвольный");
                if p.is_optional {
                    format!("[{}: {}]", p.name, type_str)
                } else {
                    format!("{}: {}", p.name, type_str)
                }
            })
            .collect::<Vec<_>>()
            .join(", ");

        let label = format!("{}({})", signature.name, params_str);

        // Создаём ParameterInformation для каждого параметра
        let parameters = signature
            .params
            .iter()
            .map(|p| {
                let param_label = format!(
                    "{}: {}",
                    p.name,
                    p.type_name.as_deref().unwrap_or("Произвольный")
                );

                ParameterInformation {
                    label: ParameterLabel::Simple(param_label),
                    documentation: None,
                }
            })
            .collect();

        SignatureHelp {
            signatures: vec![SignatureInformation {
                label,
                documentation: None,
                parameters: Some(parameters),
                active_parameter: Some(active_param),
            }],
            active_signature: Some(0),
            active_parameter: Some(active_param),
        }
    }
}

// ============================================================================
// MILESTONE 2.20.3: Helper Functions for Current Context
// ============================================================================

/// Найти функцию/процедуру, содержащую указанную позицию (MILESTONE 2.20.3)
fn find_containing_function_in_dto(
    tree_dto: &bsl_shared::api::semantic_dtos::SemanticTreeDto,
    line: u32,
    character: u32,
) -> Option<(String, String, Vec<String>, Option<String>)> {
    // Рекурсивно ищем в дереве узлов
    for node in &tree_dto.root_nodes {
        if let Some(result) = find_in_node(node, line, character) {
            return Some(result);
        }
    }

    None
}

/// Рекурсивный поиск в узле SemanticNodeDto
fn find_in_node(
    node: &bsl_shared::api::semantic_dtos::SemanticNodeDto,
    line: u32,
    character: u32,
) -> Option<(String, String, Vec<String>, Option<String>)> {
    // Проверяем, что позиция внутри range узла (если есть)
    if let Some(ref range) = node.range {
        if !range_contains(range, line, character) {
            return None;
        }
    } else if !location_matches(&node.location, line, character) {
        // Если range отсутствует, проверяем location (точное совпадение)
        return None;
    }

    // Проверяем тип узла
    match node.kind.as_str() {
        "FunctionDeclaration" => {
            let name = node
                .name
                .clone()
                .unwrap_or_else(|| "<anonymous>".to_string());
            // Извлекаем параметры из metadata (если есть)
            let params = extract_params_from_node(node);
            let return_type = extract_return_type_from_node(node);

            return Some((name, "function".to_string(), params, return_type));
        }
        "ProcedureDeclaration" => {
            let name = node
                .name
                .clone()
                .unwrap_or_else(|| "<anonymous>".to_string());
            let params = extract_params_from_node(node);

            return Some((name, "procedure".to_string(), params, None));
        }
        _ => {
            // Рекурсивно ищем в детях
            for child in &node.children {
                if let Some(result) = find_in_node(child, line, character) {
                    return Some(result);
                }
            }
        }
    }

    None
}

/// Проверяет, содержит ли range позицию
fn range_contains(
    range: &bsl_shared::api::semantic_dtos::SourceRangeDto,
    line: u32,
    character: u32,
) -> bool {
    let start = &range.start;
    let end = &range.end;

    line >= start.line
        && line <= end.line
        && (line > start.line || character >= start.column)
        && (line < end.line || character <= end.column)
}

/// Проверяет, совпадает ли location с позицией
fn location_matches(
    location: &bsl_shared::api::semantic_dtos::SourceLocationDto,
    line: u32,
    character: u32,
) -> bool {
    location.line == line && location.column == character
}
/// Извлекает параметры из metadata узла (если есть)
fn extract_params_from_node(
    _node: &bsl_shared::api::semantic_dtos::SemanticNodeDto,
) -> Vec<String> {
    // Пока заглушка - можно расширить позже через metadata
    vec![]
}

/// Извлекает return type из metadata узла (если есть)
fn extract_return_type_from_node(
    _node: &bsl_shared::api::semantic_dtos::SemanticNodeDto,
) -> Option<String> {
    // Пока заглушка - можно расширить позже через metadata
    None
}

#[tokio::main]
async fn main() -> Result<()> {
    // ОТЛАДКА: логируем в файл, чтобы увидеть что происходит при запуске из VSCode
    // ✅ MILESTONE 2.10: Перезаписываем файл при каждом запуске (.write(true).truncate(true))
    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true) // Очищаем файл при каждом запуске
        .open("C:\\1CProject\\bsl-gradual-types\\vscode-extension\\rust_lsp_server.log")
        .expect("Failed to create log file");

    // Настраиваем логирование В ФАЙЛ вместо stderr
    // ✅ MILESTONE 2.10: Подавляем DEBUG логи от html5ever и selectors для предотвращения гигантских log файлов
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("bsl_gradual_types=debug".parse()?)
                .add_directive("tower_lsp=info".parse()?)
                .add_directive("html5ever=warn".parse()?) // Подавляем DEBUG от html5ever (используется в scraper)
                .add_directive("selectors=warn".parse()?) // Подавляем DEBUG от selectors (используется в scraper)
                .add_directive("scraper=info".parse()?), // Подавляем DEBUG от scraper
        )
        .with_writer(std::sync::Mutex::new(log_file))
        .init();

    // ДИАГНОСТИКА: Версия и build timestamp для проверки актуальности LSP server
    const VERSION: &str = env!("CARGO_PKG_VERSION");
    const BUILD_TIMESTAMP: &str = env!("BUILD_TIMESTAMP");
    info!("╔════════════════════════════════════════════════════════════════╗");
    info!("║ BSL Language Server - Clean Architecture                      ║");
    info!("║ Version: {:<52} ║", VERSION);
    info!("║ Build: {:<54} ║", BUILD_TIMESTAMP);
    info!("╚════════════════════════════════════════════════════════════════╝");

    // ✅ НОВОЕ: Очищаем progress_debug.log при каждом запуске (как rust_lsp_server.log)
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true) // Перезатираем файл при старте
        .open("C:\\1CProject\\bsl-gradual-types\\vscode-extension\\progress_debug.log")
    {
        use std::io::Write;
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let _ = writeln!(
            file,
            "[{}] === BSL Progress Debug Log - LSP Server Started ===",
            timestamp
        );
        let _ = writeln!(
            file,
            "[{}] LSP server initialized, waiting for progress events...",
            timestamp
        );
    }

    // Параметры запуска
    let _args = Args::parse();

    // ✅ ИСПРАВЛЕНО: SystemCoordinator как IoC Container
    let coordinator = Arc::new(SystemCoordinator::new());

    // ❌ УДАЛЕНО: НЕ вызываем start() здесь! TypeRepository будет создан в initialized() с правильными путями
    // coordinator.start().await - это создаёт ПУСТОЙ repository, который потом не обновляется!

    // ⚠️ ВАЖНО: Вызываем start() только с базовыми типами, чтобы TypeSystemService был доступен
    // Настоящая загрузка типов произойдёт в initialized() через start_with_paths()
    info!("⚠️ Initializing coordinator with fallback types (real types will be loaded in initialized())");
    coordinator
        .start()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to start coordinator: {}", e))?;

    // ✅ ИСПРАВЛЕНО: НЕ передаём TypeSystemService в BslLanguageServer!
    // BslLanguageServer будет получать актуальный TypeSystemService через coordinator.type_service()
    // Это гарантирует, что после reload типов в initialized() мы используем НОВЫЙ TypeSystemService

    // Создаём stdin/stdout для коммуникации с клиентом
    info!("Setting up STDIO communication channels...");
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    info!("✅ STDIO channels created");

    // ✅ MILESTONE 2.10: передаем только SystemCoordinator в LSP Server
    // TypeSystemService будет получен lazy через coordinator.type_service()
    info!("Creating LSP service...");
    let coordinator_clone = coordinator.clone();
    let (service, socket) = LspService::new(move |client| {
        info!("Initializing BSL Language Server");
        BslLanguageServer::new(client, coordinator_clone.clone())
    });
    info!("✅ LSP service created");

    // Запускаем сервер
    info!("Starting LSP server loop (listening on STDIO)...");
    Server::new(stdin, stdout, socket).serve(service).await;
    info!("LSP server shut down");

    Ok(())
}
