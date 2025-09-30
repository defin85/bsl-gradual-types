//! LSP Server for BSL Gradual Type System - MIGRATED TO Clean Architecture

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result as JsonRpcResult;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};
use tracing::{error, info};

use clap::Parser;

// ✅ ИСПРАВЛЕНО: Clean Architecture - используем Application Layer
use bsl_backend::application::TypeSystemService;
use bsl_backend::system::SystemCoordinator;

// ✅ ИСПРАВЛЕНО: временные структуры удалены, используем TypeSystemService API

#[derive(Parser, Debug)]
#[command(name = "lsp-server")]
#[command(about = "BSL Language Server (Clean Architecture)", long_about = None)]
#[allow(dead_code)]
struct Args {}

/// BSL Language Server backend - CLEAN ARCHITECTURE
struct BslLanguageServer {
    client: Client,
    documents: Arc<RwLock<HashMap<Url, String>>>,
    // ✅ ИСПРАВЛЕНО: используем Application Layer вместо System Layer
    type_service: Arc<TypeSystemService>,
}

impl BslLanguageServer {
    fn new(client: Client, type_service: Arc<TypeSystemService>) -> Self {
        Self {
            client,
            documents: Arc::new(RwLock::new(HashMap::new())),
            // ✅ ИСПРАВЛЕНО: используем TypeSystemService напрямую
            type_service,
        }
    }

    // ✅ ИСПРАВЛЕНО: удален неиспользуемый get_completion_prefix метод
}

#[tower_lsp::async_trait]
impl LanguageServer for BslLanguageServer {
    async fn initialize(&self, _params: InitializeParams) -> JsonRpcResult<InitializeResult> {
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

        // ✅ ИСПРАВЛЕНО: диагностики через TypeSystemService
        let file_path = uri.to_file_path().map_err(|e| {
            error!("Failed to convert URI to file path: {:?}", e);
        });

        let diagnostics = match file_path {
            Ok(path) => {
                let path_str = path.to_string_lossy();
                info!("🔍 Анализируем файл: {}", path_str);

                // Анализируем файл через Application Layer
                match self.type_service.analyze_file(&path_str).await {
                    Ok(analysis) => {
                        info!("✅ Анализ файла {} успешно завершён", path_str);
                        // Создаём информационную диагностику об успешном анализе
                        vec![Diagnostic {
                            range: Range::new(Position::new(0, 0), Position::new(0, 1)),
                            severity: Some(DiagnosticSeverity::INFORMATION),
                            message: format!(
                                "✅ BSL файл проанализирован успешно ({})",
                                analysis.file_path
                            ),
                            source: Some("bsl-gradual-types".to_string()),
                            ..Default::default()
                        }]
                    }
                    Err(e) => {
                        error!("Failed to analyze document {}: {}", uri, e);
                        // Создаём диагностику об ошибке анализа
                        vec![Diagnostic {
                            range: Range::new(Position::new(0, 0), Position::new(0, 1)),
                            severity: Some(DiagnosticSeverity::ERROR),
                            message: format!("❌ Ошибка анализа BSL файла: {}", e),
                            source: Some("bsl-gradual-types".to_string()),
                            ..Default::default()
                        }]
                    }
                }
            }
            Err(_) => {
                // Создаём диагностику об ошибке пути
                vec![Diagnostic {
                    range: Range::new(Position::new(0, 0), Position::new(0, 1)),
                    severity: Some(DiagnosticSeverity::ERROR),
                    message: "❌ Невозможно получить путь к файлу".to_string(),
                    source: Some("bsl-gradual-types".to_string()),
                    ..Default::default()
                }]
            }
        };

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
        // TODO: интегрировать с analyze_file для получения реальных диагностик
        let all_diagnostics: Vec<Diagnostic> = Vec::new();

        // Берём актуальный текст документа
        let documents = self.documents.read().await;
        if let Some(_text) = documents.get(&uri) {
            // Пока что возвращаем пустые диагностики
            // TODO: интегрировать с analyze_file для получения реальных диагностик
            info!("🔍 Обновление диагностики файла: {}", uri.path());
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
            .type_service
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

        // Получаем информацию о символе через TypeSystemService
        match self
            .type_service
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

    info!("Starting BSL Language Server - Clean Architecture");

    // Параметры запуска
    let _args = Args::parse();

    // ✅ ИСПРАВЛЕНО: SystemCoordinator как IoC Container
    let coordinator = Arc::new(SystemCoordinator::new());

    // ✅ Инициализируем SystemCoordinator с Domain Layer
    coordinator.start().await.map_err(|e| anyhow::anyhow!("Failed to start coordinator: {}", e))?;

    // ✅ ИСПРАВЛЕНО: создаем TypeSystemService через SystemCoordinator согласно новой архитектуре
    let type_service = coordinator.type_service()
        .ok_or_else(|| anyhow::anyhow!("Failed to create TypeSystemService: AnalysisEngine not initialized"))?;

    // Initialize the type service
    type_service.initialize()?;

    // Создаём stdin/stdout для коммуникации с клиентом
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    // ✅ ИСПРАВЛЕНО: передаем TypeSystemService в LSP Server
    let (service, socket) =
        LspService::new(move |client| BslLanguageServer::new(client, type_service.clone()));

    // Запускаем сервер
    Server::new(stdin, stdout, socket).serve(service).await;

    Ok(())
}
