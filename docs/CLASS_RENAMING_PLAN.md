# 🔄 Class Renaming Plan для Simplified Architecture

## 🎯 **ЦЕЛЬ**: Привести все названия классов в соответствие с `docs/simplified_architecture.md`

---

## 📋 **CRITICAL RENAMINGS**

### **🔥 Priority 1: Architecture-Breaking Changes**

#### **1. TypeSystemService Location Fix**
```rust
// ПРОБЛЕМА: TypeSystemService в System Layer, а должен быть в Application Layer

// ТЕКУЩЕЕ СОСТОЯНИЕ:
// src/system/system_coordinator.rs
pub struct TypeSystemService { ... }

// ТРЕБУЕМОЕ СОСТОЯНИЕ:
// src/application/type_system_service.rs  
pub struct TypeSystemService { ... }

// ДЕЙСТВИЯ:
// 1. Создать src/application/type_system_service.rs
// 2. Переместить TypeSystemService из SystemCoordinator
// 3. SystemCoordinator получает TypeSystemService через DI
// 4. Обновить все импорты
```

#### **2. SystemCoordinator Architecture Fix**
```rust
// ТЕКУЩЕЕ СОСТОЯНИЕ:
impl SystemCoordinator {
    pub fn type_service(&self) -> Arc<TypeSystemService> {
        self.type_service.clone() // WRONG: должен быть в Application Layer
    }
}

// ТРЕБУЕМОЕ СОСТОЯНИЕ:  
impl SystemCoordinator {
    pub fn application_layer(&self) -> Arc<ApplicationLayer> {
        // SystemCoordinator создает и управляет Application Layer
        Arc::new(ApplicationLayer::new(
            self.cache.clone(),
            self.parser.clone(), 
            self.observability.clone(),
            // Domain Layer components
            self.type_resolver.clone(),
            self.repository.clone(),
        ))
    }
}

// НОВЫЙ Application Layer:
// src/application/mod.rs
pub struct ApplicationLayer {
    type_system_service: Arc<TypeSystemService>,
}
```

---

### **🌐 Priority 2: Presentation Layer Renamings**

#### **1. BslLanguageServer → LSPServer**
```bash
# ФАЙЛ: src/bin/lsp_server.rs

# БЫЛО:
struct BslLanguageServer {
    client: Client,
    documents: Arc<RwLock<HashMap<Url, String>>>,
    type_service: Arc<TypeSystemService>,
}

# СТАНЕТ:
struct LSPServer {
    client: Client,
    documents: Arc<RwLock<HashMap<Url, String>>>,
    application: Arc<ApplicationLayer>,
}

impl LanguageServer for LSPServer { ... } # было: BslLanguageServer
```

#### **2. WebInterface Enhancement**
```bash
# НОВЫЙ ФАЙЛ: src/bin/web_server.rs

pub struct WebInterface {
    application: Arc<ApplicationLayer>,
    server_config: WebServerConfig,
}

impl WebInterface {
    pub async fn start(&self, port: u16) -> Result<()> {
        // Simple HTML dashboard
        // Type visualization  
        // REST API endpoints
    }
    
    // Handlers для REST API:
    async fn handle_search(&self, query: String) -> Result<SearchResponse> {
        self.application.type_system_service.web_search(&query).await
    }
    
    async fn handle_type_info(&self, type_name: String) -> Result<TypeInfoResponse> {
        self.application.type_system_service.get_type_info(&type_name).await
    }
}
```

#### **3. CLI Tools Unification**
```bash
# НОВЫЙ ФАЙЛ: src/bin/cli_tool.rs

pub struct CLITool {
    application: Arc<ApplicationLayer>,
    config: CLIConfig,
}

impl CLITool {
    // Unified CLI interface объединяющий:
    pub async fn analyze_file(&self, path: &str) -> Result<()> {
        // Из type_check.rs
    }
    
    pub async fn batch_analysis(&self, paths: &[&str]) -> Result<()> {
        // Новая функциональность
    }
    
    pub async fn start_web_server(&self, port: u16) -> Result<()> {
        // Из bsl-web-server.rs
    }
    
    pub async fn interactive_mode(&self) -> Result<()> {
        // Новая функциональность
    }
}

# ОБЪЕДИНЯЕТ:
# - src/bin/type_check.rs → CLITool::analyze_file()
# - src/bin/bsl-web-server.rs → CLITool::start_web_server() 
# - Новые CLI команды → CLITool::*()
```

---

### **🧠 Priority 3: Domain Layer Renamings**

#### **1. BasicTypeResolver → TypeResolver**
```rust
// ФАЙЛ: src/domain/resolution_service.rs

// БЫЛО:
pub struct BasicTypeResolver {
    platform_types: HashMap<String, PlatformType>,
    config_types: HashMap<String, ConfigurationType>,
}

impl TypeResolver for BasicTypeResolver { ... }

// СТАНЕТ:
pub struct TypeResolver {
    platform_types: HashMap<String, PlatformType>,
    config_types: HashMap<String, ConfigurationType>,
}

impl TypeResolverTrait for TypeResolver { ... } # trait переименован

# ОБНОВИТЬ ВСЕ ИМПОРТЫ:
# use crate::domain::TypeResolver; # было: BasicTypeResolver
```

#### **2. TypeRepository - уже правильно**
```rust
// ✅ УЖЕ КОРРЕКТНО:
// src/domain/repository.rs

pub trait TypeRepository: Send + Sync { ... }
pub struct InMemoryTypeRepository { ... }

impl TypeRepository for InMemoryTypeRepository { ... }

# НЕ ТРЕБУЕТ ИЗМЕНЕНИЙ - соответствует документации
```

---

### **💾 Priority 4: Data Layer Renamings**

#### **1. PlatformTypesRepository → PlatformTypes**
```bash
# ПЕРЕИМЕНОВАНИЕ ФАЙЛА:
# src/data/loaders/platform_types_repository.rs → src/data/platform_types.rs

# ПЕРЕИМЕНОВАНИЕ КЛАССА:
# БЫЛО:
pub struct PlatformTypesRepository {
    parser: SyntaxHelperParser,
    database: Option<SyntaxHelperDatabase>, 
    type_index: Option<TypeIndex>,
}

# СТАНЕТ:
pub struct PlatformTypes {
    parser: SyntaxHelperParser,
    database: Option<SyntaxHelperDatabase>,
    type_index: Option<TypeIndex>,
}

# ОБНОВИТЬ ВСЕ МЕТОДЫ:
impl PlatformTypes { # было: PlatformTypesRepository
    pub fn new() -> Self { ... }
    pub fn load_from_directory<P: AsRef<Path>>(&mut self, path: P) -> Result<()> { ... }
    pub fn get_platform_globals(&self) -> HashMap<String, TypeResolution> { ... }
    # ... все остальные методы остаются
}
```

#### **2. ConfigurationGuidedParser → ConfigData**
```bash
# ПЕРЕИМЕНОВАНИЕ ФАЙЛА:
# src/data/loaders/config_parser_guided_discovery.rs → src/data/config_data.rs

# ПЕРЕИМЕНОВАНИЕ КЛАССА:
# БЫЛО:
pub struct ConfigurationGuidedParser {
    config_path: String,
    discovered_metadata: Vec<DiscoveredMetadata>,
}

# СТАНЕТ:
pub struct ConfigData {
    config_path: String,
    discovered_metadata: Vec<DiscoveredMetadata>,
}

# ОБНОВИТЬ ВСЕ МЕТОДЫ:
impl ConfigData { # было: ConfigurationGuidedParser
    pub fn new(config_path: &str) -> Self { ... }
    pub fn parse_with_configuration_guide(&mut self) -> Result<Vec<TypeResolution>> { ... }
    # ... все остальные методы остаются
}
```

---

## 🔧 **TECHNICAL IMPLEMENTATION PLAN**

### **Phase 1: Preparation (Pre-renaming)**
```bash
# 1. Создать backup текущего состояния
git checkout -b backup/before-simplified-architecture-renaming

# 2. Создать feature branch  
git checkout -b feature/simplified-architecture-renaming

# 3. Создать новые файлы для Application Layer
mkdir -p src/application
touch src/application/mod.rs
touch src/application/type_system_service.rs
touch src/application/application_layer.rs
```

### **Phase 2: Critical Architecture Changes**
```rust
// src/application/application_layer.rs - НОВЫЙ
pub struct ApplicationLayer {
    type_system_service: Arc<TypeSystemService>,
}

impl ApplicationLayer {
    pub fn new(
        cache: Arc<AnalysisCache>,
        parser: Arc<ParserCoordinator>,
        observability: Arc<BasicObservability>,
        type_resolver: Arc<TypeResolver>,
        repository: Arc<dyn TypeRepository>,
    ) -> Self {
        let type_system_service = Arc::new(TypeSystemService::new(
            type_resolver, cache, parser
        ));
        
        Self { type_system_service }
    }
    
    pub fn type_system_service(&self) -> Arc<TypeSystemService> {
        self.type_system_service.clone()
    }
}

// src/application/type_system_service.rs - ПЕРЕМЕСТИТЬ из SystemCoordinator
// (Весь код TypeSystemService перемещается сюда)
```

### **Phase 3: SystemCoordinator Refactoring**
```rust
// src/system/system_coordinator.rs - ИЗМЕНИТЬ
pub struct SystemCoordinator {
    // === SYSTEM COMPONENTS ===
    cache: Arc<AnalysisCache>,
    parser: Arc<ParserCoordinator>, 
    observability: Arc<BasicObservability>,
    
    // === APPLICATION LAYER ===
    application: Arc<ApplicationLayer>, # БЫЛО: type_service
    
    // === DOMAIN LAYER ===  
    type_resolver: Arc<TypeResolver>, # БЫЛО: TypeResolutionService
    repository: Arc<dyn TypeRepository>,
}

impl SystemCoordinator {
    pub fn new() -> Self {
        // 1. System components
        let cache = Arc::new(AnalysisCache::new(1000));
        let parser = Arc::new(ParserCoordinator::with_fallback());
        let observability = Arc::new(BasicObservability::default());
        
        // 2. Domain components
        let repository: Arc<dyn TypeRepository> = Arc::new(InMemoryTypeRepository::new());
        let type_resolver = Arc::new(TypeResolver::new(repository.clone())); # ПЕРЕИМЕНОВАН
        
        // 3. Application layer
        let application = Arc::new(ApplicationLayer::new(
            cache.clone(),
            parser.clone(),
            observability.clone(),
            type_resolver.clone(),
            repository.clone(),
        ));
        
        Self { cache, parser, observability, application, type_resolver, repository }
    }
    
    // НОВЫЙ API:
    pub fn application(&self) -> Arc<ApplicationLayer> {
        self.application.clone()
    }
    
    // DEPRECATED - для обратной совместимости:
    #[deprecated(note = "Use application().type_system_service() instead")]
    pub fn type_service(&self) -> Arc<TypeSystemService> {
        self.application.type_system_service()
    }
}
```

### **Phase 4: Presentation Layer Updates**
```rust
// src/bin/lsp_server.rs - ОБНОВИТЬ
struct LSPServer { # ПЕРЕИМЕНОВАН из BslLanguageServer
    client: Client,
    documents: Arc<RwLock<HashMap<Url, String>>>,
    application: Arc<ApplicationLayer>, # ИЗМЕНЕН тип
}

impl LSPServer {
    fn new(client: Client, application: Arc<ApplicationLayer>) -> Self {
        Self {
            client,
            documents: Arc::new(RwLock::new(HashMap::new())),
            application,
        }
    }
}

impl LanguageServer for LSPServer { # ОБНОВЛЕН trait impl
    async fn hover(&self, params: HoverParams) -> JsonRpcResult<Option<Hover>> {
        // БЫЛО: self.type_service.get_hover_info(...)
        // СТАНЕТ: self.application.type_system_service().get_hover_info(...)
    }
    
    async fn completion(&self, params: CompletionParams) -> JsonRpcResult<Option<CompletionResponse>> {
        // БЫЛО: self.type_service.get_completion(...)
        // СТАНЕТ: self.application.type_system_service().get_completion(...)
    }
}

// main() function:
#[tokio::main] 
async fn main() -> Result<()> {
    let coordinator = Arc::new(SystemCoordinator::new());
    let application = coordinator.application(); # НОВЫЙ API
    
    let (service, socket) = LspService::new(move |client| LSPServer::new(client, application.clone()));
    # ПЕРЕИМЕНОВАН: BslLanguageServer → LSPServer
    
    Server::new(stdin, stdout, socket).serve(service).await;
    Ok(())
}
```

### **Phase 5: Domain & Data Layer Renamings**
```rust
// src/domain/resolution_service.rs
pub struct TypeResolver { # ПЕРЕИМЕНОВАН из BasicTypeResolver
    platform_types: HashMap<String, PlatformType>,
    config_types: HashMap<String, ConfigurationType>,
}

// Trait тоже переименовывается для consistency:
pub trait TypeResolverTrait { # ПЕРЕИМЕНОВАН из TypeResolver
    fn resolve(&self, expression: &str, context: Option<&Context>) -> TypeResolution;
    fn get_completions(&self, position: &Position) -> Vec<Completion>;
    fn check_types(&self, ast: &AST) -> Vec<Diagnostic>;
}

impl TypeResolverTrait for TypeResolver { # ОБНОВЛЕН impl
    // ... implementation остается той же
}

// src/data/platform_types.rs - ПЕРЕИМЕНОВАННЫЙ ФАЙЛ
pub struct PlatformTypes { # ПЕРЕИМЕНОВАН из PlatformTypesRepository
    parser: SyntaxHelperParser,
    database: Option<SyntaxHelperDatabase>,
    type_index: Option<TypeIndex>,
}

// src/data/config_data.rs - ПЕРЕИМЕНОВАННЫЙ ФАЙЛ  
pub struct ConfigData { # ПЕРЕИМЕНОВАН из ConfigurationGuidedParser
    config_path: String,
    discovered_metadata: Vec<DiscoveredMetadata>,
}
```

---

## 🧪 **TESTING STRATEGY**

### **Integration Tests для новой архитектуры:**
```rust
// tests/simplified_architecture_integration_test.rs - НОВЫЙ ФАЙЛ
#[tokio::test]
async fn test_simplified_architecture_flow() {
    // 1. System Layer
    let coordinator = SystemCoordinator::new();
    
    // 2. Application Layer  
    let application = coordinator.application();
    let type_service = application.type_system_service();
    
    // 3. Presentation Layer
    let lsp_server = LSPServer::new(mock_client, application.clone());
    let web_interface = WebInterface::new(application.clone());
    let cli_tool = CLITool::new(application.clone());
    
    // 4. Test unified API
    assert!(type_service.lsp_completion("test", 1, 1).await.is_ok());
    assert!(type_service.web_search("test").await.is_ok());
    assert!(type_service.cli_analyze("test.bsl").await.is_ok());
    
    // 5. Test architecture compliance
    assert_eq!(coordinator.health_status().status, "healthy");
}

#[tokio::test] 
async fn test_renamed_components() {
    // Test TypeResolver (бывший BasicTypeResolver)
    let resolver = TypeResolver::new();
    assert!(resolver.resolve("Строка", None).certainty.is_known());
    
    // Test PlatformTypes (бывший PlatformTypesRepository)  
    let platform_types = PlatformTypes::new();
    assert!(!platform_types.get_platform_globals().is_empty());
    
    // Test ConfigData (бывший ConfigurationGuidedParser)
    let config_data = ConfigData::new("test/config.xml");
    assert!(config_data.parse_with_configuration_guide().is_ok());
}
```

### **Unit Tests Updates:**
```rust
// Обновить все существующие тесты:

// БЫЛО:
use crate::domain::resolution_service::BasicTypeResolver;
let resolver = BasicTypeResolver::new();

// СТАНЕТ:  
use crate::domain::TypeResolver;
let resolver = TypeResolver::new();

// БЫЛО:
use crate::data::loaders::platform_types_repository::PlatformTypesRepository;
let repo = PlatformTypesRepository::new();

// СТАНЕТ:
use crate::data::PlatformTypes;
let platform_types = PlatformTypes::new();
```

---

## 📋 **IMPORT UPDATES CHECKLIST**

### **Файлы требующие обновления импортов:**

1. **System Layer:**
   - `src/system/system_coordinator.rs` ✅ Основные изменения
   - `src/system/simple_cache.rs` ✅ Без изменений
   - `src/system/parser_coordinator.rs` ✅ Без изменений  
   - `src/system/basic_observability.rs` ✅ Без изменений

2. **Application Layer (НОВЫЙ):**
   - `src/application/mod.rs` ✅ Новый файл
   - `src/application/type_system_service.rs` ✅ Перемещен
   - `src/application/application_layer.rs` ✅ Новый файл

3. **Presentation Layer:**
   - `src/bin/lsp_server.rs` ⚠️ Обновить импорты  
   - `src/bin/web_server.rs` ✅ Новый файл
   - `src/bin/cli_tool.rs` ✅ Новый файл
   - `src/presentation/adapters.rs` ⚠️ Обновить импорты

4. **Domain Layer:**
   - `src/domain/resolution_service.rs` ⚠️ Переименования
   - `src/domain/repository.rs` ✅ Без изменений

5. **Data Layer:**  
   - `src/data/platform_types.rs` ⚠️ Переименован файл
   - `src/data/config_data.rs` ⚠️ Переименован файл

6. **Tests:**
   - `tests/*_test.rs` ⚠️ Все тесты требуют обновления импортов

---

## 🚨 **BREAKING CHANGES WARNING**

### **Public API Changes:**
```rust
// BREAKING: TypeSystemService location change
// OLD:
use bsl_gradual_types::system::SystemCoordinator;
let coordinator = SystemCoordinator::new();
let type_service = coordinator.type_service(); // DEPRECATED

// NEW:  
use bsl_gradual_types::system::SystemCoordinator;
let coordinator = SystemCoordinator::new();
let application = coordinator.application();
let type_service = application.type_system_service(); // НОВЫЙ API

// BREAKING: Component renames
// OLD:
use bsl_gradual_types::domain::resolution_service::BasicTypeResolver;
use bsl_gradual_types::data::loaders::platform_types_repository::PlatformTypesRepository;

// NEW:
use bsl_gradual_types::domain::TypeResolver;  
use bsl_gradual_types::data::PlatformTypes;
```

### **Migration Path для пользователей:**
```rust
// src/migration.rs - НОВЫЙ ФАЙЛ с helpers
pub mod compatibility {
    // Deprecated aliases для backward compatibility
    
    #[deprecated(note = "Use TypeResolver instead")]
    pub type BasicTypeResolver = crate::domain::TypeResolver;
    
    #[deprecated(note = "Use PlatformTypes instead")]
    pub type PlatformTypesRepository = crate::data::PlatformTypes;
    
    #[deprecated(note = "Use ConfigData instead")]
    pub type ConfigurationGuidedParser = crate::data::ConfigData;
    
    #[deprecated(note = "Use coordinator.application().type_system_service() instead")]
    pub fn get_type_service(coordinator: &SystemCoordinator) -> Arc<TypeSystemService> {
        coordinator.application().type_system_service()
    }
}
```

---

## ✅ **COMPLETION CHECKLIST**

### **Phase 1: Architecture (Critical)**
- [ ] TypeSystemService перемещен в Application Layer
- [ ] ApplicationLayer создан и интегрирован
- [ ] SystemCoordinator рефакторен для использования Application Layer
- [ ] Все связи между слоями соответствуют диаграмме

### **Phase 2: Presentation Layer**  
- [ ] BslLanguageServer → LSPServer
- [ ] WebInterface полностью реализован  
- [ ] CLITool создан и объединяет CLI функциональность
- [ ] Все Presentation компоненты используют Application Layer

### **Phase 3: Domain Layer**
- [ ] BasicTypeResolver → TypeResolver
- [ ] TypeResolverTrait создан для consistency
- [ ] Все импорты обновлены

### **Phase 4: Data Layer**
- [ ] PlatformTypesRepository → PlatformTypes
- [ ] ConfigurationGuidedParser → ConfigData  
- [ ] Файлы переименованы и перемещены
- [ ] Все импорты обновлены

### **Phase 5: Testing & Documentation**
- [ ] Все тесты обновлены и проходят
- [ ] Integration tests для новой архитектуры
- [ ] Migration guide создан  
- [ ] API documentation обновлена
- [ ] Breaking changes задокументированы

### **Phase 6: Cleanup**
- [ ] Deprecated код удален (после grace period)
- [ ] Legacy файлы удалены
- [ ] Backward compatibility helpers протестированы
- [ ] Performance benchmarks подтверждают отсутствие regression

---

## 🎯 **FINAL VALIDATION**

После завершения всех переименований проверить:

1. **Architecture compliance**: 100% соответствие `simplified_architecture.md`
2. **Test coverage**: Все тесты проходят с новыми именами  
3. **Performance**: Отсутствие degradation
4. **Documentation**: Полная актуализация
5. **Breaking changes**: Четко задокументированы с migration path

**Expected result**: Чистая, простая архитектура с интуитивными названиями классов! 🚀
