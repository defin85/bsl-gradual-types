//! System Coordinator - упрощенная замена CentralTypeSystem
//!
//! Единая точка координации всех компонентов системы типов согласно Simple Architecture.
//! Координирует только System Layer компоненты.
//!
//! # Модули
//!
//! - `coordinator` - основная структура SystemCoordinator и core методы
//! - `lifecycle` - инициализация системы и загрузка типов платформы
//! - `config_loader` - загрузка метаданных конфигураций
//! - `types` - вспомогательные типы (ошибки, результаты)

mod config_loader;
mod coordinator;
mod lifecycle;
mod types;

// Реэкспорты публичного API
pub use coordinator::SystemCoordinator;
pub use types::{
    CacheClearReport, CacheScope, CacheStatsReport, CacheToggleResult, ConfigIndexCache,
    DiskCacheStatsReport, DomainBundle, LoadMetadataResult, ObjectKey, StartupError, SymbolInfo,
};

#[cfg(test)]
mod tests {
    use bsl_shared::domain::repository::InMemoryTypeRepository;
    use bsl_shared::TypeRepository;
    use std::sync::Arc;

    /// Вспомогательная функция для создания тестового репозитория с инициализированными конструкторами
    fn create_test_repository() -> Arc<InMemoryTypeRepository> {
        use bsl_shared::domain::types::{RawDataSource, RawTypeData};

        let repo = Arc::new(InMemoryTypeRepository::new());

        // Загружаем базовые типы-коллекции
        let platform_types = vec![
            RawTypeData {
                name: "Массив".to_string(),
                english_name: "Array".to_string(),
                category: "Универсальные коллекции значений".to_string(),
                source: RawDataSource::Platform,
                ..Default::default()
            },
            RawTypeData {
                name: "Соответствие".to_string(),
                english_name: "Map".to_string(),
                category: "Универсальные коллекции значений".to_string(),
                source: RawDataSource::Platform,
                ..Default::default()
            },
            RawTypeData {
                name: "СписокЗначений".to_string(),
                english_name: "ValueList".to_string(),
                category: "Универсальные коллекции значений".to_string(),
                source: RawDataSource::Platform,
                ..Default::default()
            },
            RawTypeData {
                name: "ТабличнаяЧасть".to_string(),
                english_name: "TabularSection".to_string(),
                category: "Универсальные коллекции значений".to_string(),
                source: RawDataSource::Platform,
                ..Default::default()
            },
        ];

        repo.load_types(platform_types).unwrap();

        // SignatureIndex в тесте не инициализируем: источником правды остаётся Syntax Helper.

        // Применяем GenericInfo
        crate::data::loaders::apply_generic_info_to_repository(repo.as_ref());

        repo
    }

    #[test]
    fn test_constructor_resolution_via_repository() {
        use bsl_shared::domain::resolver::TypeResolver;

        let repo = create_test_repository();
        let _resolver = TypeResolver::new(repo.clone());

        // Проверяем что можно резолвить конструктор через TypeResolver
        // Для этого нам нужен SignatureIndex из репозитория
        //
        // Примечание: TypeResolver.resolve_constructor() требует SignatureIndex,
        // который хранится в репозитории. Проверяем интеграцию.

        // Пока просто проверяем что репозиторий создан успешно
        // TODO: Добавить полноценный тест с resolve_constructor после интеграции
        assert!(
            repo.find_type("Массив").is_some(),
            "Тип Массив должен существовать в репозитории"
        );
    }
}
