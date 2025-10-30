//! LSP Server for BSL Gradual Type System - MIGRATED TO Clean Architecture

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result as JsonRpcResult;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};
use tracing::{error, info, warn};

use clap::Parser;
use serde::Deserialize;

// ✅ ИСПРАВЛЕНО: Clean Architecture - используем Application Layer
use bsl_backend::application::TypeSystemService;
use bsl_backend::system::SystemCoordinator;
use bsl_shared::api::dtos::{MethodDto, ParamDto, PropertyDto};
use bsl_type_visualization::{HtmlRenderer, RenderOptions, ThemeMode}; // Используется в других методах // Используется в handle_query_type

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
    #[allow(dead_code)]
    configuration_path: Option<String>,

    /// Версия платформы 1С (например, "8.3.25")
    #[allow(dead_code)]
    platform_version: Option<String>,
}

/// BSL Language Server backend - CLEAN ARCHITECTURE
struct BslLanguageServer {
    client: Client,
    documents: Arc<RwLock<HashMap<Url, String>>>,
    // ✅ MILESTONE 2.10: храним LSP конфигурацию из initializationOptions
    config: Arc<RwLock<Option<LspConfig>>>,
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
            coordinator,
        }
    }

    /// Получить актуальный TypeSystemService (всегда fresh после reload)
    fn get_type_service(&self) -> Arc<TypeSystemService> {
        self.coordinator
            .type_service()
            .expect("TypeSystemService not available - coordinator not initialized")
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

    /// Конвертировать backend ParseError → shared ParseError (Milestone 2.18)
    ///
    /// Преобразует локальный ParseError из bsl::ast в shared::domain::types::ParseError
    fn convert_parse_errors(
        &self,
        backend_errors: &[bsl_backend::parsing::bsl::ast::ParseError],
    ) -> Vec<bsl_shared::domain::types::ParseError> {
        use bsl_backend::parsing::bsl::ast::ErrorType as BackendErrorType;
        use bsl_shared::domain::types::{
            ErrorType as SharedErrorType, ParseError as SharedParseError,
        };

        backend_errors
            .iter()
            .map(|error| {
                // Конвертируем ErrorType
                let shared_error_type = match error.error_type {
                    BackendErrorType::ParseError => SharedErrorType::ParseError,
                    BackendErrorType::InvalidSyntax => SharedErrorType::InvalidSyntax,
                    BackendErrorType::MissingToken => SharedErrorType::MissingToken,
                    BackendErrorType::UnexpectedToken => SharedErrorType::UnexpectedToken,
                };

                // Конвертируем Span (backend::bsl::ast::Span → shared::ir::Span)
                let shared_span = bsl_shared::ir::Span::new(
                    error.span.start_line,
                    error.span.start_column,
                    error.span.end_line,
                    error.span.end_column,
                );

                // Создаём shared ParseError
                SharedParseError {
                    error_type: shared_error_type,
                    message: error.message.clone(),
                    span: shared_span,
                }
            })
            .collect()
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
                    ],
                    work_done_progress_options: WorkDoneProgressOptions::default(),
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

                // ✅ НОВОЕ: Отправляем LSP Progress notification (стандартный протокол LSP)
                use tower_lsp::lsp_types::{
                    NumberOrString, ProgressParams, ProgressParamsValue, WorkDoneProgress,
                    WorkDoneProgressBegin, WorkDoneProgressEnd, WorkDoneProgressReport,
                };

                // Генерируем уникальный токен для прогресса (timestamp-based)
                let token = NumberOrString::String(format!(
                    "bsl-load-types-{}",
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_millis()
                ));

                // 1. Начинаем прогресс (VSCode автоматически покажет progress bar в UI)
                self.client
                    .send_notification::<tower_lsp::lsp_types::notification::Progress>(
                        ProgressParams {
                            token: token.clone(),
                            value: ProgressParamsValue::WorkDone(WorkDoneProgress::Begin(
                                WorkDoneProgressBegin {
                                    title: "Загрузка типов платформы 1С".to_string(),
                                    message: Some("Парсинг Syntax Helper...".to_string()),
                                    percentage: Some(0),
                                    cancellable: Some(false),
                                },
                            )),
                        },
                    )
                    .await;

                // ✅ FIX: Даём VSCode время обработать Begin notification
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

                // 2. Обновляем прогресс на 50%
                self.client
                    .send_notification::<tower_lsp::lsp_types::notification::Progress>(
                        ProgressParams {
                            token: token.clone(),
                            value: ProgressParamsValue::WorkDone(WorkDoneProgress::Report(
                                WorkDoneProgressReport {
                                    message: Some(format!(
                                        "Обработка документации из {}...",
                                        platform_docs
                                    )),
                                    percentage: Some(50),
                                    cancellable: Some(false),
                                },
                            )),
                        },
                    )
                    .await;

                // ✅ FIX: Даём VSCode время обработать Report notification
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

                let syntax_path = std::path::Path::new(platform_docs);

                // 3. Перезапускаем SystemCoordinator с новым путём
                match self
                    .coordinator
                    .start_with_paths(Some(syntax_path), None)
                    .await
                {
                    Ok(()) => {
                        info!("✅ Types reloaded successfully with platform documentation");

                        // 4. Завершаем прогресс с успехом
                        self.client
                            .send_notification::<tower_lsp::lsp_types::notification::Progress>(
                                ProgressParams {
                                    token: token.clone(),
                                    value: ProgressParamsValue::WorkDone(WorkDoneProgress::End(
                                        WorkDoneProgressEnd {
                                            message: Some(
                                                "✅ Типы платформы загружены успешно".to_string(),
                                            ),
                                        },
                                    )),
                                },
                            )
                            .await;

                        self.client
                            .log_message(
                                MessageType::INFO,
                                format!("Platform documentation loaded from: {}", platform_docs),
                            )
                            .await;
                    }
                    Err(e) => {
                        error!("❌ Failed to reload types: {}", e);

                        // 5. Завершаем прогресс с ошибкой
                        self.client
                            .send_notification::<tower_lsp::lsp_types::notification::Progress>(
                                ProgressParams {
                                    token: token.clone(),
                                    value: ProgressParamsValue::WorkDone(WorkDoneProgress::End(
                                        WorkDoneProgressEnd {
                                            message: Some(format!("❌ Ошибка загрузки: {}", e)),
                                        },
                                    )),
                                },
                            )
                            .await;

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
        match self.get_type_service().get_hover_info(&text, 0, 0).await {
            Ok(_) => info!("✅ IR cache preheated for {}", uri),
            Err(e) => error!("❌ Failed to preheat IR cache for {}: {}", uri, e),
        }

        // ✅ MILESTONE 2.18: Получаем синтаксические ошибки из парсера
        let mut diagnostics = Vec::new();

        // Парсим файл через ParserCoordinator (доступен через SystemCoordinator)
        match self.coordinator.parser_coordinator() {
            Some(parser) => {
                match parser.parse(&text) {
                    Ok(parse_result) => {
                        if parse_result.has_errors() {
                            info!(
                                "⚠️ Found {} syntax errors in {}",
                                parse_result.syntax_errors.len(),
                                uri
                            );

                            // Конвертируем backend ParseError → shared ParseError
                            let shared_errors =
                                self.convert_parse_errors(&parse_result.syntax_errors);

                            // Конвертируем shared ParseError → LSP Diagnostics
                            diagnostics.extend(self.syntax_errors_to_diagnostics(&shared_errors));
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
            }
            None => {
                error!("ParserCoordinator not available");
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
        if let Err(e) = self
            .get_type_service()
            .parse_incremental(file_path, updated_text.clone(), text_edits)
            .await
        {
            error!("Incremental parsing failed: {}", e);
        } else {
            info!("✅ Incremental parsing succeeded for: {}", uri.path());
        }

        // ✅ MILESTONE 2.18: Получаем синтаксические ошибки из парсера
        let mut diagnostics = Vec::new();

        // Берём актуальный текст документа
        let documents = self.documents.read().await;
        if let Some(text) = documents.get(&uri) {
            info!("🔍 Обновление диагностики файла: {}", uri.path());

            // Парсим файл через ParserCoordinator
            match self.coordinator.parser_coordinator() {
                Some(parser) => {
                    match parser.parse(text) {
                        Ok(parse_result) => {
                            if parse_result.has_errors() {
                                info!(
                                    "⚠️ Found {} syntax errors in {}",
                                    parse_result.syntax_errors.len(),
                                    uri
                                );

                                // Конвертируем backend ParseError → shared ParseError
                                let shared_errors =
                                    self.convert_parse_errors(&parse_result.syntax_errors);

                                // Конвертируем shared ParseError → LSP Diagnostics
                                diagnostics
                                    .extend(self.syntax_errors_to_diagnostics(&shared_errors));
                            } else {
                                info!("✅ No syntax errors in {}", uri);
                            }
                        }
                        Err(e) => {
                            error!("Failed to parse document {}: {}", uri, e);
                        }
                    }
                }
                None => {
                    error!("ParserCoordinator not available");
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
        match self
            .get_type_service()
            .get_completion(&file_content, position.line, position.character)
            .await
        {
            Ok(completions) => {
                // Преобразуем наши CompletionItem в LSP CompletionItem
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

        // Используем get_hover_info() с IR-based анализом
        match self
            .get_type_service()
            .get_hover_info(&file_content, position.line, position.character)
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
            _ => {
                tracing::warn!("Unknown command: {}", params.command);
                Err(tower_lsp::jsonrpc::Error::method_not_found())
            }
        }
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
                let methods: Vec<MethodDto> = raw_type
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
        match self
            .get_type_service()
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

    info!("Starting BSL Language Server - Clean Architecture");

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
