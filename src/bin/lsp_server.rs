//! LSP Server for BSL Gradual Type System

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result as JsonRpcResult;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};
use tracing::{error, info};

use bsl_gradual_types::core::platform_resolver::{PlatformTypeResolver, CompletionItem as BslCompletion};
use bsl_gradual_types::core::lsp_enhanced::{DocumentManager, DiagnosticsConverter};

/// BSL Language Server backend
struct BslLanguageServer {
    client: Client,
    documents: Arc<RwLock<HashMap<Url, String>>>,
    resolver: Arc<RwLock<PlatformTypeResolver>>,
    document_manager: Arc<DocumentManager>,
}

impl BslLanguageServer {
    fn new(client: Client) -> Self {
        Self {
            client,
            documents: Arc::new(RwLock::new(HashMap::new())),
            resolver: Arc::new(RwLock::new(PlatformTypeResolver::new())),
            document_manager: Arc::new(DocumentManager::new()),
        }
    }
    
    /// Конвертирует наши внутренние completion items в LSP формат
    fn convert_completions(&self, items: Vec<BslCompletion>) -> Vec<CompletionItem> {
        items.into_iter().map(|item| {
            CompletionItem {
                label: item.label,
                kind: Some(match item.kind {
                    bsl_gradual_types::core::platform_resolver::CompletionKind::Global => {
                        CompletionItemKind::MODULE
                    }
                    bsl_gradual_types::core::platform_resolver::CompletionKind::Catalog => {
                        CompletionItemKind::CLASS
                    }
                    bsl_gradual_types::core::platform_resolver::CompletionKind::Document => {
                        CompletionItemKind::CLASS
                    }
                    bsl_gradual_types::core::platform_resolver::CompletionKind::Enum => {
                        CompletionItemKind::ENUM
                    }
                    bsl_gradual_types::core::platform_resolver::CompletionKind::Method => {
                        CompletionItemKind::METHOD
                    }
                    bsl_gradual_types::core::platform_resolver::CompletionKind::Property => {
                        CompletionItemKind::PROPERTY
                    }
                    bsl_gradual_types::core::platform_resolver::CompletionKind::GlobalFunction => {
                        CompletionItemKind::FUNCTION
                    }
                    bsl_gradual_types::core::platform_resolver::CompletionKind::Variable => {
                        CompletionItemKind::VARIABLE
                    }
                    bsl_gradual_types::core::platform_resolver::CompletionKind::Function => {
                        CompletionItemKind::FUNCTION
                    }
                }),
                detail: item.detail,
                documentation: item.documentation.map(|doc| {
                    Documentation::MarkupContent(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: doc,
                    })
                }),
                ..Default::default()
            }
        }).collect()
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
                    if !ch.is_alphanumeric() && 
                       ch != '.' && 
                       ch != '_' && 
                       !('а'..='я').contains(&ch) && 
                       !('А'..='Я').contains(&ch) {
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
        
        // Если указан путь к конфигурации, загружаем её
        if let Some(root) = params.root_uri {
            if let Ok(path) = root.to_file_path() {
                let config_path = path.join("src").join("cf");
                if config_path.exists() {
                    info!("Found configuration at: {:?}", config_path);
                    
                    // Пытаемся загрузить конфигурацию
                    match PlatformTypeResolver::with_config(config_path.to_str().unwrap()) {
                        Ok(resolver) => {
                            let mut res = self.resolver.write().await;
                            *res = resolver;
                            info!("Configuration loaded successfully");
                        }
                        Err(e) => {
                            error!("Failed to load configuration: {}", e);
                        }
                    }
                }
            }
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
                    }
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
                    }
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
        
        // Обновляем старое хранилище для совместимости
        self.documents.write().await.insert(uri.clone(), text.clone());
        
        // Используем новый document manager для анализа
        match self.document_manager.update_document(
            uri.to_string(),
            text,
            version,
            vec![], // Нет изменений при открытии
        ).await {
            Ok(diagnostics) => {
                // Отправляем диагностики клиенту
                self.client.publish_diagnostics(uri.clone(), diagnostics, Some(version)).await;
                
                self.client
                    .log_message(MessageType::INFO, format!("Opened and analyzed document: {}", uri))
                    .await;
            }
            Err(e) => {
                error!("Failed to analyze document {}: {}", uri, e);
                self.client
                    .log_message(MessageType::ERROR, format!("Analysis failed for {}: {}", uri, e))
                    .await;
            }
        }
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
            let existing_text = self.documents.read().await.get(&uri).cloned().unwrap_or_default();
            
            // TODO: Более сложная логика применения инкрементальных изменений
            // Пока используем последнее изменение как полное
            changes.last().map(|c| c.text.clone()).unwrap_or(existing_text)
        };
        
        // Обновляем старое хранилище
        self.documents.write().await.insert(uri.clone(), updated_text.clone());
        
        // Используем новый document manager с инкрементальным анализом
        match self.document_manager.update_document(
            uri.to_string(),
            updated_text,
            version,
            changes,
        ).await {
            Ok(diagnostics) => {
                // Отправляем обновленные диагностики
                self.client.publish_diagnostics(uri.clone(), diagnostics, Some(version)).await;
            }
            Err(e) => {
                error!("Failed to incrementally analyze document {}: {}", uri, e);
            }
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        self.documents.write().await.remove(&uri);
        
        self.client
            .log_message(MessageType::INFO, format!("Closed document: {}", uri))
            .await;
    }

    async fn completion(&self, params: CompletionParams) -> JsonRpcResult<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        
        let documents = self.documents.read().await;
        let text = match documents.get(&uri) {
            Some(text) => text,
            None => return Ok(None),
        };
        
        // Получаем префикс для автодополнения
        let prefix = self.get_completion_prefix(text, position);
        
        info!("Enhanced completion requested for prefix: '{}'", prefix);
        
        // Получаем enhanced предложения с типами
        let resolver = self.resolver.read().await;
        let enhanced_completions = self.document_manager.get_completions(
            &uri.to_string(),
            position,
            &prefix,
            &resolver,
        ).await;
        
        // Конвертируем в LSP формат
        let items = self.convert_completions(enhanced_completions);
        
        info!("Returning {} completion items", items.len());
        
        Ok(Some(CompletionResponse::Array(items)))
    }

    async fn hover(&self, params: HoverParams) -> JsonRpcResult<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;
        
        info!("Enhanced hover requested at {}:{}", position.line, position.character);
        
        // Получаем enhanced hover информацию
        if let Some(hover_text) = self.document_manager.get_enhanced_hover(
            &uri.to_string(),
            position,
        ).await {
            Ok(Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: hover_text,
                }),
                range: None,
            }))
        } else {
            // Fallback к старому поведению
            let documents = self.documents.read().await;
            let text = match documents.get(&uri) {
                Some(text) => text,
                None => return Ok(None),
            };
            
            let word = self.get_completion_prefix(text, position);
            
            if word.is_empty() {
                return Ok(None);
            }
            
            let mut resolver = self.resolver.write().await;
            let resolution = resolver.resolve_expression(&word);
            
            let hover_text = if resolution.is_resolved() {
                format!(
                    "**{}**\n\n*Тип:* {:?}\n\n*Уверенность:* {:?}\n\n*(legacy resolver)*",
                    word, resolution.result, resolution.certainty
                )
            } else {
                format!("**{}**\n\n*Тип не определён*", word)
            };
            
            Ok(Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: hover_text,
                }),
                range: None,
            }))
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
                .add_directive("tower_lsp=info".parse()?)
        )
        .init();
    
    info!("Starting BSL Language Server");
    
    // Создаём stdin/stdout для коммуникации с клиентом
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    
    // Создаём LSP сервис
    let (service, socket) = LspService::new(BslLanguageServer::new);
    
    // Запускаем сервер
    Server::new(stdin, stdout, socket).serve(service).await;
    
    Ok(())
}