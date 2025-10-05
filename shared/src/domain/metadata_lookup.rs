//! TypeMetadataLookup - мост между TypeResolution и RawTypeData
//!
//! Этот модуль предоставляет сервис для получения полной документации типа
//! на основе результата статического анализа (TypeResolution).

use std::sync::Arc;
use crate::domain::repository::TypeRepository;
use crate::domain::types::{
    TypeResolution, RawTypeData, RawMethodData, RawPropertyData,
    ResolutionResult, ConcreteType, MetadataKind,
};

/// Сервис для получения метаданных типа по TypeResolution
///
/// # Назначение
///
/// TypeMetadataLookup является мостом между двумя концепциями:
/// - **TypeResolution** - результат статического анализа (что мы вывели о типе)
/// - **RawTypeData** - полная документация типа (что мы знаем из справки)
///
/// # Разделение ответственностей
///
/// - TypeResolution содержит только результат анализа (легковесный Value Object)
/// - RawTypeData содержит полную документацию (Single Source of Truth в Repository)
/// - TypeMetadataLookup предоставляет явный способ получить документацию по результату анализа
///
/// # Примеры использования
///
/// ```rust
/// use bsl_gradual_types::domain::metadata_lookup::TypeMetadataLookup;
/// use bsl_gradual_types::domain::repository::TypeRepository;
/// use std::sync::Arc;
///
/// // Получить методы для TypeResolution
/// let lookup = TypeMetadataLookup::new(repository);
/// let resolution = resolver.resolve_expression_sync("Массив");
/// let methods = lookup.get_methods(&resolution);
///
/// // Проверить существование метода
/// if !lookup.has_member(&resolution, "НеСуществующийМетод") {
///     println!("Метод не найден!");
/// }
///
/// // Получить полную RawTypeData
/// if let Some(raw_type) = lookup.get_raw_type(&resolution) {
///     println!("Описание: {}", raw_type.description);
/// }
/// ```
#[derive(Clone)]
pub struct TypeMetadataLookup {
    repository: Arc<dyn TypeRepository>,
}

impl TypeMetadataLookup {
    /// Создать новый экземпляр TypeMetadataLookup
    ///
    /// # Параметры
    ///
    /// * `repository` - хранилище типов с RawTypeData
    pub fn new(repository: Arc<dyn TypeRepository>) -> Self {
        Self { repository }
    }

    /// Получить полную RawTypeData для TypeResolution
    ///
    /// # Параметры
    ///
    /// * `resolution` - результат статического анализа типа
    ///
    /// # Возвращает
    ///
    /// `Some(RawTypeData)` если тип найден в repository, иначе `None`
    ///
    /// # Примеры
    ///
    /// ```rust
    /// let resolution = resolver.resolve_expression_sync("ТаблицаЗначений");
    /// if let Some(raw_type) = lookup.get_raw_type(&resolution) {
    ///     println!("Категория: {}", raw_type.category);
    ///     println!("Методов: {}", raw_type.methods.len());
    /// }
    /// ```
    pub fn get_raw_type(&self, resolution: &TypeResolution) -> Option<RawTypeData> {
        let type_name = self.extract_type_name(resolution)?;
        self.repository.find_type(&type_name)
    }

    /// Получить методы для TypeResolution
    ///
    /// # Параметры
    ///
    /// * `resolution` - результат статического анализа типа
    ///
    /// # Возвращает
    ///
    /// Вектор методов типа или пустой вектор если тип не найден
    ///
    /// # Примеры
    ///
    /// ```rust
    /// let resolution = resolver.resolve_expression_sync("Массив");
    /// let methods = lookup.get_methods(&resolution);
    /// for method in methods {
    ///     println!("Метод: {} -> {}", method.name, method.return_type);
    /// }
    /// ```
    pub fn get_methods(&self, resolution: &TypeResolution) -> Vec<RawMethodData> {
        let type_name = self.extract_type_name(resolution);

        let raw_type = type_name.and_then(|name| {
            self.repository.find_type(&name)
        });

        raw_type.map(|raw| raw.methods).unwrap_or_default()
    }

    /// Получить свойства для TypeResolution
    ///
    /// # Параметры
    ///
    /// * `resolution` - результат статического анализа типа
    ///
    /// # Возвращает
    ///
    /// Вектор свойств типа или пустой вектор если тип не найден
    ///
    /// # Примеры
    ///
    /// ```rust
    /// let resolution = resolver.resolve_expression_sync("HTTPСоединение");
    /// let properties = lookup.get_properties(&resolution);
    /// for prop in properties {
    ///     println!("Свойство: {} ({})", prop.name, prop.prop_type);
    /// }
    /// ```
    pub fn get_properties(&self, resolution: &TypeResolution) -> Vec<RawPropertyData> {
        self.get_raw_type(resolution)
            .map(|raw| raw.properties)
            .unwrap_or_default()
    }

    /// Проверить существование метода или свойства у типа
    ///
    /// # Параметры
    ///
    /// * `resolution` - результат статического анализа типа
    /// * `member_name` - имя метода или свойства для проверки
    ///
    /// # Возвращает
    ///
    /// `true` если метод/свойство существует, `false` если нет или тип не найден
    ///
    /// # Примеры
    ///
    /// ```rust
    /// let resolution = resolver.resolve_expression_sync("ТаблицаЗначений");
    ///
    /// // Проверяем существующий метод
    /// assert!(lookup.has_member(&resolution, "Добавить"));
    ///
    /// // Проверяем несуществующий метод
    /// assert!(!lookup.has_member(&resolution, "НеСуществующийМетод"));
    /// ```
    ///
    /// # Использование для валидации
    ///
    /// ```rust
    /// // В TypeValidator
    /// if !metadata_lookup.has_member(&resolution, "Записать") {
    ///     return Some(TypeErrorKind::NonExistentProperty {
    ///         object_type: format!("{:?}", resolution.result),
    ///         property_name: "Записать".to_string(),
    ///     });
    /// }
    /// ```
    pub fn has_member(&self, resolution: &TypeResolution, member_name: &str) -> bool {
        let raw = match self.get_raw_type(resolution) {
            Some(r) => r,
            None => return false,
        };

        // Проверяем методы
        if raw.methods.iter().any(|m| m.name == member_name || m.english_name == member_name) {
            return true;
        }

        // Проверяем свойства
        if raw.properties.iter().any(|p| p.name == member_name) {
            return true;
        }

        false
    }

    /// Получить описание типа
    ///
    /// # Параметры
    ///
    /// * `resolution` - результат статического анализа типа
    ///
    /// # Возвращает
    ///
    /// Описание типа или пустую строку если тип не найден
    pub fn get_description(&self, resolution: &TypeResolution) -> String {
        self.get_raw_type(resolution)
            .map(|raw| raw.description)
            .unwrap_or_default()
    }

    /// Получить категорию типа
    ///
    /// # Параметры
    ///
    /// * `resolution` - результат статического анализа типа
    ///
    /// # Возвращает
    ///
    /// Категорию типа или пустую строку если тип не найден
    pub fn get_category(&self, resolution: &TypeResolution) -> String {
        self.get_raw_type(resolution)
            .map(|raw| raw.category)
            .unwrap_or_default()
    }

    /// Извлечь имя типа из TypeResolution
    ///
    /// # Параметры
    ///
    /// * `resolution` - результат статического анализа типа
    ///
    /// # Возвращает
    ///
    /// `Some(String)` с именем типа, `None` если тип не поддерживается
    ///
    /// # Поддерживаемые типы
    ///
    /// - **Platform** типы: `Массив`, `ТаблицаЗначений`, `Строка`
    /// - **Configuration** типы: `Справочники.Контрагенты`, `Документы.Заказ`
    /// - **Primitive** и **Special** типы пока не поддерживаются (нет RawTypeData)
    fn extract_type_name(&self, resolution: &TypeResolution) -> Option<String> {
        match &resolution.result {
            ResolutionResult::Concrete(concrete) => match concrete {
                ConcreteType::Platform(platform) => {
                    // Для платформенных типов используем имя напрямую
                    Some(platform.name.clone())
                }
                ConcreteType::Configuration(config) => {
                    // Для конфигурации формируем полное имя
                    // Например: "Справочники.Контрагенты"
                    Some(format!("{}.{}",
                        self.metadata_kind_to_prefix(&config.kind),
                        config.name
                    ))
                }
                // Primitive и Special типы не имеют RawTypeData в repository
                ConcreteType::Primitive(_) | ConcreteType::Special(_) => None,
                // GlobalFunction может иметь документацию
                ConcreteType::GlobalFunction(func) => Some(func.name.clone()),
            },
            // Union и Dynamic типы не имеют прямого соответствия в RawTypeData
            ResolutionResult::Union(_) | ResolutionResult::Dynamic => None,
            // Intersection - берём первый тип
            ResolutionResult::Intersection(types) => {
                types.first().and_then(|t| self.extract_type_name(&TypeResolution {
                    result: ResolutionResult::Concrete(t.clone()),
                    ..resolution.clone()
                }))
            },
            // Generic - используем базовый тип
            ResolutionResult::Generic(gen) => Some(gen.base_type.clone()),
            // Nullable - распаковываем внутренний тип
            ResolutionResult::Nullable(inner) => {
                self.extract_type_name(&TypeResolution {
                    result: ResolutionResult::Concrete(inner.as_ref().clone()),
                    ..resolution.clone()
                })
            },
        }
    }

    /// Преобразовать MetadataKind в префикс имени типа
    ///
    /// # Примеры
    ///
    /// - `Catalog` -> `"Справочники"`
    /// - `Document` -> `"Документы"`
    fn metadata_kind_to_prefix(&self, kind: &MetadataKind) -> &'static str {
        match kind {
            MetadataKind::Catalog => "Справочники",
            MetadataKind::Document => "Документы",
            MetadataKind::Enum => "Перечисления",
            MetadataKind::Report => "Отчеты",
            MetadataKind::DataProcessor => "Обработки",
            MetadataKind::Register => "РегистрыСведений",
            MetadataKind::ChartOfAccounts => "ПланыСчетов",
            MetadataKind::ChartOfCharacteristicTypes => "ПланыВидовХарактеристик",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::types::{
        PlatformType, RawDataSource, FacetKind,
        Certainty, ResolutionSource, ResolutionMetadata,
    };
    use crate::domain::repository::InMemoryTypeRepository;

    fn create_test_repository() -> Arc<InMemoryTypeRepository> {
        let repo = Arc::new(InMemoryTypeRepository::new());

        // Создаем тестовый тип "Массив" с методами
        let array_type = RawTypeData {
            name: "Массив".to_string(),
            english_name: "Array".to_string(),
            description: "Коллекция элементов".to_string(),
            category: "Типы коллекций".to_string(),
            source: RawDataSource::Platform,
            methods: vec![
                RawMethodData {
                    name: "Добавить".to_string(),
                    english_name: "Add".to_string(),
                    return_type: "".to_string(),
                    params: vec![],
                },
                RawMethodData {
                    name: "Количество".to_string(),
                    english_name: "Count".to_string(),
                    return_type: "Число".to_string(),
                    params: vec![],
                },
            ],
            properties: vec![],
            facets: vec![FacetKind::Collection],
            kind: None,
            attributes: vec![],
            tabular_sections: vec![],
            enum_values: vec![],
        };

        repo.load_types(vec![array_type]).unwrap();
        repo
    }

    fn create_test_resolution(type_name: &str) -> TypeResolution {
        TypeResolution {
            certainty: Certainty::Known,
            result: ResolutionResult::Concrete(ConcreteType::Platform(PlatformType {
                name: type_name.to_string(),
            })),
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: None,
            available_facets: vec![],
        }
    }

    #[test]
    fn test_get_raw_type() {
        let repo = create_test_repository();
        let lookup = TypeMetadataLookup::new(repo.clone());
        let resolution = create_test_resolution("Массив");

        let raw_type = lookup.get_raw_type(&resolution);
        assert!(raw_type.is_some());
        let raw = raw_type.unwrap();
        assert_eq!(raw.name, "Массив");
        assert_eq!(raw.english_name, "Array");
    }

    #[test]
    fn test_get_methods() {
        let repo = create_test_repository();
        let lookup = TypeMetadataLookup::new(repo.clone());
        let resolution = create_test_resolution("Массив");

        let methods = lookup.get_methods(&resolution);
        assert_eq!(methods.len(), 2);
        assert_eq!(methods[0].name, "Добавить");
        assert_eq!(methods[1].name, "Количество");
    }

    #[test]
    fn test_has_member_existing() {
        let repo = create_test_repository();
        let lookup = TypeMetadataLookup::new(repo.clone());
        let resolution = create_test_resolution("Массив");

        assert!(lookup.has_member(&resolution, "Добавить"));
        assert!(lookup.has_member(&resolution, "Количество"));
        // Проверка английского имени
        assert!(lookup.has_member(&resolution, "Add"));
    }

    #[test]
    fn test_has_member_nonexistent() {
        let repo = create_test_repository();
        let lookup = TypeMetadataLookup::new(repo.clone());
        let resolution = create_test_resolution("Массив");

        assert!(!lookup.has_member(&resolution, "НеСуществующийМетод"));
    }

    #[test]
    fn test_get_description() {
        let repo = create_test_repository();
        let lookup = TypeMetadataLookup::new(repo.clone());
        let resolution = create_test_resolution("Массив");

        let description = lookup.get_description(&resolution);
        assert_eq!(description, "Коллекция элементов");
    }

    #[test]
    fn test_unknown_type_returns_empty() {
        let repo = create_test_repository();
        let lookup = TypeMetadataLookup::new(repo.clone());
        let resolution = create_test_resolution("НеСуществующийТип");

        assert!(lookup.get_raw_type(&resolution).is_none());
        assert!(lookup.get_methods(&resolution).is_empty());
        assert!(!lookup.has_member(&resolution, "Метод"));
        assert_eq!(lookup.get_description(&resolution), "");
    }
}
