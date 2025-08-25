//! LSP Server for BSL Gradual Type System (target-only)

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result as JsonRpcResult;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};
use tracing::{error, info};

use clap::Parser;

// Target architecture
use bsl_gradual_types::system::{CentralSystemConfig, CentralTypeSystem};

#[derive(Parser, Debug)]
#[command(name = "lsp-server")]
#[command(about = "BSL Language Server (target engine)", long_about = None)]
struct Args {}

/// BSL Language Server backend (target-only)
struct BslLanguageServer {
    client: Client,
    documents: Arc<RwLock<HashMap<Url, String>>>,
    central: Arc<CentralTypeSystem>,
}

impl BslLanguageServer {
    fn new(client: Client, central: Arc<CentralTypeSystem>) -> Self {
        Self {
            client,
            documents: Arc::new(RwLock::new(HashMap::new())),
            central,
        }
    }

    /// Извлекает префикс для автодополнения из текущей позиции
    fn get_completion_prefix(&self, text: &str, position: Position) -> String {
        let lines: Vec<&str> = text.lines().collect();

        if let Some(line) = lines.get(position.line as usize) {
            // Конвертируем позицию символа LSP (UTF-16) в байтовый индекс
            let mut byte_pos = 0;
            let mut char_count = 0u32;

            for ch in line.chars() {
                if char_count >= position.character {
                    break;
                }
                byte_pos += ch.len_utf8();
                char_count += ch.len_utf16() as u32;
            }

            if byte_pos <= line.len() {
                let line_before_cursor = &line[..byte_pos];

                // Ищем начало идентификатора, работая с char итератором для корректной обработки Unicode
                let mut last_non_ident_byte = 0;

                for (idx, ch) in line_before_cursor.char_indices() {
                    if !ch.is_alphanumeric()
                        && ch != '.'
                        && ch != '_'
                        && !('а'..='я').contains(&ch)
                        && !('А'..='Я').contains(&ch)
                    {
                        last_non_ident_byte = idx + ch.len_utf8();
                    }
                }

                return line_before_cursor[last_non_ident_byte..].to_string();
            }
        }

        String::new()
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for BslLanguageServer {
    async fn initialize(&self, params: InitializeParams) -> JsonRpcResult<InitializeResult> {
        info!("Initializing BSL Language Server");
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
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "BSL Language Server initialized!")
            .await;
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

        // Диагностики через CentralTypeSystem (target)
        let base_diagnostics: Result<Vec<Diagnostic>, anyhow::Error> = Ok(Vec::new());

        // Если target-движок, добавим базовые диагностики совместимости типов через CentralTypeSystem
        let mut all_diagnostics: Vec<Diagnostic> = match base_diagnostics {
            Ok(d) => d,
            Err(e) => {
                error!("Failed to analyze document {}: {}", uri, e);
                Vec::new()
            }
        };
        // Берём актуальный текст документа
        let documents = self.documents.read().await;
        if let Some(text) = documents.get(&uri) {
            match self
                .central
                .lsp_interface()
                .analyze_text_for_diagnostics(&uri.to_string(), text)
                .await
            {
                Ok(diags) => {
                    for d in diags {
                        all_diagnostics.push(Diagnostic {
                            range: Range {
                                start: Position {
                                    line: d.range.start.line,
                                    character: d.range.start.character,
                                },
                                end: Position {
                                    line: d.range.end.line,
                                    character: d.range.end.character,
                                },
                            },
                            severity: Some(match d.severity {
                                1 => DiagnosticSeverity::ERROR,
                                2 => DiagnosticSeverity::WARNING,
                                3 => DiagnosticSeverity::INFORMATION,
                                4 => DiagnosticSeverity::HINT,
                                _ => DiagnosticSeverity::INFORMATION,
                            }),
                            code: None,
                            code_description: None,
                            source: Some("bsl-target".to_string()),
                            message: d.message,
                            related_information: None,
                            tags: None,
                            data: None,
                        });
                    }
                }
                Err(e) => error!("target diagnostics failed: {}", e),
            }
        }

        // Отправляем объединённые диагностики
        self.client
            .publish_diagnostics(uri.clone(), all_diagnostics, Some(version))
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

            // TODO: Более сложная логика применения инкрементальных изменений
            // Пока используем последнее изменение как полное
            changes
                .last()
                .map(|c| c.text.clone())
                .unwrap_or(existing_text)
        };

        // Кешируем текст
        self.documents
            .write()
            .await
            .insert(uri.clone(), updated_text.clone());
        let _ = (version, changes); // не используем, пока нет инкрементального анализатора в target

        // Базовые диагностики (пусто)
        let base_diagnostics: Result<Vec<Diagnostic>, anyhow::Error> = Ok(Vec::new());

        let mut all_diagnostics: Vec<Diagnostic> = base_diagnostics.unwrap_or_else(|e| {
            error!("Failed to incrementally analyze document {}: {}", uri, e);
            Vec::new()
        });

        // Берём актуальный текст документа
        let documents = self.documents.read().await;
        if let Some(text) = documents.get(&uri) {
            match self
                .central
                .lsp_interface()
                .analyze_text_for_diagnostics(&uri.to_string(), text)
                .await
            {
                Ok(diags) => {
                    for d in diags {
                        all_diagnostics.push(Diagnostic {
                            range: Range {
                                start: Position {
                                    line: d.range.start.line,
                                    character: d.range.start.character,
                                },
                                end: Position {
                                    line: d.range.end.line,
                                    character: d.range.end.character,
                                },
                            },
                            severity: Some(match d.severity {
                                1 => DiagnosticSeverity::ERROR,
                                2 => DiagnosticSeverity::WARNING,
                                3 => DiagnosticSeverity::INFORMATION,
                                4 => DiagnosticSeverity::HINT,
                                _ => DiagnosticSeverity::INFORMATION,
                            }),
                            code: None,
                            code_description: None,
                            source: Some("bsl-target".to_string()),
                            message: d.message,
                            related_information: None,
                            tags: None,
                            data: None,
                        });
                    }
                }
                Err(e) => error!("target diagnostics failed: {}", e),
            }
        }

        // Отправляем обновленные диагностики
        self.client
            .publish_diagnostics(uri.clone(), all_diagnostics, Some(version))
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
        // Target-only path
        // Берём текущий текст документа, чтобы вычислить префикс
        let documents = self.documents.read().await;
        let text = match documents.get(&uri) {
            Some(text) => text,
            None => return Ok(None),
        };
        let prefix = self.get_completion_prefix(text, position);
        let req = bsl_gradual_types::unified::presentation::LspCompletionRequest {
            file_path: uri.to_string(),
            line: position.line,
            column: position.character,
            prefix,
            trigger_character: None,
        };
        match self
            .central
            .lsp_interface()
            .handle_completion_request(req)
            .await
        {
            Ok(resp) => {
                let items: Vec<CompletionItem> = resp
                    .items
                    .into_iter()
                    .map(|it| CompletionItem {
                        label: it.label,
                        kind: None,
                        detail: it.detail,
                        documentation: it.documentation.map(|doc| {
                            Documentation::MarkupContent(MarkupContent {
                                kind: MarkupKind::Markdown,
                                value: doc,
                            })
                        }),
                        ..Default::default()
                    })
                    .collect();
                Ok(Some(CompletionResponse::Array(items)))
            }
            Err(e) => {
                error!("target completion failed: {}", e);
                Ok(None)
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
        // Target-only path
        let documents = self.documents.read().await;
        let text = match documents.get(&uri) {
            Some(text) => text,
            None => return Ok(None),
        };
        let expr = self.get_completion_prefix(text, position);
        let req = bsl_gradual_types::unified::presentation::LspHoverRequest {
            file_path: uri.to_string(),
            line: position.line,
            column: position.character,
            expression: expr,
        };
        match self.central.lsp_interface().handle_hover_request(req).await {
            Ok(Some(hr)) => {
                let value = hr.contents.join("\n\n");
                Ok(Some(Hover {
                    contents: HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value,
                    }),
                    range: None,
                }))
            }
            Ok(None) => Ok(None),
            Err(e) => {
                error!("target hover failed: {}", e);
                Ok(None)
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Настраиваем логирование
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("bsl_gradual_types=debug".parse()?)
                .add_directive("tower_lsp=info".parse()?),
        )
        .init();

    info!("Starting BSL Language Server");

    // Параметры запуска (без движка)
    let _args = Args::parse();
    // Инициализируем центральную систему (target-only)
    let cs =
        Arc::new(CentralTypeSystem::initialize_with_config(CentralSystemConfig::default()).await?);

    // Создаём stdin/stdout для коммуникации с клиентом
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    // Создаём LSP сервис (с выбранным движком)
    let central_clone = cs.clone();
    let (service, socket) =
        LspService::new(move |client| BslLanguageServer::new(client, central_clone.clone()));

    // Запускаем сервер
    Server::new(stdin, stdout, socket).serve(service).await;

    Ok(())
}
