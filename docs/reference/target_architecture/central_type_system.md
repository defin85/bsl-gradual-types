# Центральная система: контракт и ответственность

Центральная система координирует слои, владеет репозиторием типов, инстанцирует доменные и прикладные сервисы и предоставляет стабильные интерфейсы для LSP/Web/CLI.

## Контракт
```rust
pub struct CentralTypeSystem {
    // Data
    repository: Arc<dyn TypeRepository>,

    // Domain
    resolution_service: Arc<TypeResolutionService>,

    // Application
    lsp_service: Arc<LspTypeService>,
    web_service: Arc<WebTypeService>,
    analysis_service: Arc<AnalysisTypeService>,

    // Infra
    cache: Arc<TypeCache>,
    metrics: Arc<TypeMetrics>,
}

impl CentralTypeSystem {
    pub async fn initialize(config: CentralSystemConfig) -> Result<Self> { /* загрузка данных и сборка слоёв */ }

    pub fn lsp_interface(&self) -> LspInterface { /* адаптер для LSP */ }
    pub fn web_interface(&self) -> WebInterface { /* адаптер для HTTP */ }
    pub fn cli_interface(&self) -> CliInterface { /* адаптер для CLI */ }

    pub async fn reload_data(&self) -> Result<()> { /* горячая перезагрузка */ }
    pub fn health_check(&self) -> HealthStatus { /* состояние */ }
}
```

## Инициализация
1) Парсеры источников → сырые данные типов
2) Сохранение в `TypeRepository`
3) Создание `TypeResolutionService` и подключение резолверов
4) Создание прикладных сервисов (LSP/Web/CLI)
5) Построение интерфейсов (адаптеров)

## Ответственность
- Не содержит бизнес‑логики резолвинга — это задача Domain
- Инкапсулирует кэширование и метрики
- Предоставляет единые фабрики интерфейсов

## Минимальные интерфейсы
```rust
trait LspInterface {
    async fn resolve_expression(&self, code: &str, pos: Position) -> TypeResolution;
    async fn completions(&self, prefix: &str) -> Vec<CompletionItem>;
    async fn hover(&self, expr: &str) -> Option<HoverInfo>;
}

trait WebInterface {
    async fn type_hierarchy(&self) -> TypeHierarchy;
    async fn search(&self, query: &str) -> Vec<TypeSearchResult>;
    async fn type_details(&self, name: &str) -> TypeDocumentation;
}

trait CliInterface {
    async fn analyze_project(&self, path: &Path) -> ProjectAnalysis;
    async fn check_types(&self, files: &[Path]) -> Vec<Diagnostic>;
}
```

## Метрики
- Время инициализации по слоям
- Размер хранилища типов и кешей
- Латентность LSP/Web операций

