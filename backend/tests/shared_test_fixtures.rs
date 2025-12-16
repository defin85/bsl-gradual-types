//! Shared test fixtures для интеграционных тестов.
//!
//! ПРОБЛЕМА: Каждый тест вызывал create_test_service(), который парсит
//! syntax_helper (52K+ файлов) за ~10 секунд. С 39 тестами = ~6.5 минут.
//!
//! РЕШЕНИЕ: LazyLock инициализирует TypeSystemService ОДИН раз,
//! все тесты используют shared instance. Время тестов: ~15-20 секунд.

use bsl_backend::application::TypeSystemService;
use bsl_backend::data::adapters::converters::convert_syntax_helper_to_raw;
use bsl_backend::data::loaders::progress::ProgressUpdate;
use bsl_backend::data::loaders::syntax_helper::SyntaxHelperLoader;
use bsl_backend::data::loaders::SyntaxHelperSource;
use bsl_backend::system::system_coordinator::SystemCoordinator;
use bsl_backend::system::{AnalysisCache, IrCache, ParserCoordinator};
use bsl_shared::domain::repository::{InMemoryTypeRepository, TypeRepository};
use bsl_shared::domain::SignatureSourceRegistry;
use bsl_shared::engine::AnalysisEngine;
use bsl_shared::TypeResolver;
use std::sync::{Arc, LazyLock};

/// Shared TypeSystemService для всех тестов.
/// Инициализируется ОДИН раз при первом доступе (LazyLock).
/// Thread-safe: LazyLock гарантирует однократную инициализацию.
pub static SHARED_TEST_SERVICE: LazyLock<TypeSystemService> = LazyLock::new(|| {
    create_test_service_internal()
});

/// Shared repository для тестов, которым нужен доступ к репозиторию напрямую.
pub static SHARED_REPOSITORY: LazyLock<Arc<InMemoryTypeRepository>> = LazyLock::new(|| {
    create_repository_internal()
});

/// Shared SystemCoordinator с конфигурацией для тестов табличных частей.
/// Инициализируется ОДИН раз при первом доступе (LazyLock).
/// Использует синхронную инициализацию (start_with_paths_blocking) чтобы
/// избежать конфликта с tokio runtime в async тестах.
#[allow(dead_code)]
pub static SHARED_CONFIG_COORDINATOR: LazyLock<SystemCoordinator> = LazyLock::new(|| {
    let coordinator = SystemCoordinator::new();
    let config_path = std::path::Path::new("../examples/conf/conf_test");
    coordinator
        .start_with_paths_blocking(None, Some(config_path), None)
        .expect("Failed to start coordinator with config");
    coordinator
});

/// Получить shared SystemCoordinator с конфигурацией для тестов.
#[allow(dead_code)]
pub fn get_config_coordinator() -> &'static SystemCoordinator {
    &SHARED_CONFIG_COORDINATOR
}

/// Получить shared TypeSystemService для тестов.
/// Использует LazyLock - инициализация происходит только при первом вызове.
pub fn get_test_service() -> &'static TypeSystemService {
    &SHARED_TEST_SERVICE
}

/// Получить shared repository для тестов.
pub fn get_test_repository() -> Arc<InMemoryTypeRepository> {
    SHARED_REPOSITORY.clone()
}

/// Внутренняя функция создания TypeSystemService.
/// Вызывается ОДИН раз через LazyLock.
fn create_test_service_internal() -> TypeSystemService {
    // 1. Парсим синтаксис-помощник
    let mut parser = SyntaxHelperLoader::new();
    parser
        .parse_directory("../examples/syntax_helper", None::<fn(ProgressUpdate)>)
        .expect("Failed to parse syntax helper");

    let db = parser.export_database();

    // 2. Конвертируем в raw типы
    let parsed_types = convert_syntax_helper_to_raw(&db);

    // 3. Создаём репозиторий и загружаем типы
    let repository_impl = Arc::new(InMemoryTypeRepository::new());
    let platform_types_clone = parsed_types.clone();

    repository_impl
        .load_types(parsed_types)
        .expect("Failed to load types");

    // 4. Заполняем SignatureIndex из syntax_helper
    let index = SignatureSourceRegistry::new()
        .register(SyntaxHelperSource::new(platform_types_clone))
        .build();
    repository_impl.set_signature_index(index);

    // 5. Применяем GenericInfo для типов-коллекций
    bsl_backend::data::loaders::apply_generic_info_to_repository(repository_impl.as_ref());

    // 6. Приводим к trait object
    let repository = repository_impl.clone() as Arc<dyn TypeRepository>;

    // 7. Создаём остальные компоненты
    let resolver = Arc::new(TypeResolver::new(repository.clone()));
    let analysis_engine = Arc::new(AnalysisEngine::new(resolver.clone(), repository.clone()));
    let cache = Arc::new(AnalysisCache::new(100));
    let ir_cache = Arc::new(IrCache::new(50));
    let parser = Arc::new(ParserCoordinator::new_with_resolver(repository.clone(), resolver));

    let service = TypeSystemService::new(analysis_engine, cache, parser, ir_cache);

    // 8. Инициализируем сервис
    service
        .initialize()
        .expect("Failed to initialize TypeSystemService");

    service
}

/// Внутренняя функция создания repository.
fn create_repository_internal() -> Arc<InMemoryTypeRepository> {
    let mut parser = SyntaxHelperLoader::new();
    parser
        .parse_directory("../examples/syntax_helper", None::<fn(ProgressUpdate)>)
        .expect("Failed to parse syntax helper");

    let db = parser.export_database();
    let parsed_types = convert_syntax_helper_to_raw(&db);

    let repository_impl = Arc::new(InMemoryTypeRepository::new());
    let platform_types_clone = parsed_types.clone();

    repository_impl
        .load_types(parsed_types)
        .expect("Failed to load types");

    let index = SignatureSourceRegistry::new()
        .register(SyntaxHelperSource::new(platform_types_clone))
        .build();
    repository_impl.set_signature_index(index);

    bsl_backend::data::loaders::apply_generic_info_to_repository(repository_impl.as_ref());

    repository_impl
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shared_service_initialization() {
        // Проверяем что shared service инициализируется
        let _service = get_test_service();
        // Если дошли сюда без паники - всё работает
        assert!(true, "Shared service initialized successfully");
    }

    #[test]
    fn test_shared_repository_initialization() {
        // Проверяем что shared repository инициализируется
        let repo = get_test_repository();
        // Если дошли сюда без паники - всё работает
        assert!(repo.get_signature_index_clone().find_method("Массив", "Добавить").is_some());
    }
}
