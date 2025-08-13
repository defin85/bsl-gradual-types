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

/// BSL Language Server backend
struct BslLanguageServer {
    client: Client,
    documents: Arc<RwLock<HashMap<Url, String>>>,
    resolver: Arc<RwLock<PlatformTypeResolver>>,
}

impl BslLanguageServer {
    fn new(client: Client) -> Self {
        Self {
            client,
            documents: Arc::new(RwLock::new(HashMap::new())),
            resolver: Arc::new(RwLock::new(PlatformTypeResolver::new())),
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
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![".".to_string()]),
                    ..Default::default()
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
    }

    async fn shutdown(&self) -> JsonRpcResult<()> {
        info!("Shutting down BSL Language Server");
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;
        
        self.documents.write().await.insert(uri.clone(), text);
        
        self.client
            .log_message(MessageType::INFO, format!("Opened document: {}", uri))
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        
        // Обновляем содержимое документа
        if let Some(change) = params.content_changes.first() {
            self.documents.write().await.insert(uri.clone(), change.text.clone());
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
        
        info!("Completion requested for prefix: '{}'", prefix);
        
        // Получаем предложения от резолвера
        let resolver = self.resolver.read().await;
        let completions = resolver.get_completions(&prefix);
        
        // Конвертируем в LSP формат
        let items = self.convert_completions(completions);
        
        Ok(Some(CompletionResponse::Array(items)))
    }

    async fn hover(&self, params: HoverParams) -> JsonRpcResult<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;
        
        let documents = self.documents.read().await;
        let text = match documents.get(&uri) {
            Some(text) => text,
            None => return Ok(None),
        };
        
        // Получаем слово под курсором
        let word = self.get_completion_prefix(text, position);
        
        if word.is_empty() {
            return Ok(None);
        }
        
        // Пытаемся разрешить тип
        let mut resolver = self.resolver.write().await;
        let resolution = resolver.resolve_expression(&word);
        
        // Формируем hover информацию
        let hover_text = if resolution.is_resolved() {
            format!(
                "**{}**\n\n*Тип:* {:?}\n\n*Уверенность:* {:?}",
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