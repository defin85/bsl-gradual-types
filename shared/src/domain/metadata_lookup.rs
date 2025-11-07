//! TypeMetadataLookup - мост между TypeResolution и RawTypeData
//!
//! Этот модуль предоставляет сервис для получения полной документации типа
//! на основе результата статического анализа (TypeResolution).

use std::sync::Arc;
use crate::domain::repository::TypeRepository;
use crate::domain::types::{
    TypeResolution, RawTypeData, RawMethodData, RawPropertyData,
    ResolutionResult, ConcreteType, GenericType, MetadataKind, FacetKind,
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
/// ```ignore
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
    /// ```ignore
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
    /// ```ignore
    /// let resolution = resolver.resolve_expression_sync("Массив");
    /// let methods = lookup.get_methods(&resolution);
    /// for method in methods {
    ///     println!("Метод: {} -> {}", method.name, method.return_type);
    /// }
    /// ```
    pub fn get_methods(&self, resolution: &TypeResolution) -> Vec<RawMethodData> {
        // Специальная обработка для Generic типов (СОХРАНИТЬ существующую логику!)
        if let ResolutionResult::Generic(generic_type) = &resolution.result {
            return self.get_methods_for_generic(generic_type);
        }
        
        // ✅ НОВОЕ: Приоритет 1 - Lazy lookup через active_facet
        if let Some(facet) = resolution.active_facet {
            if let Some(facet_methods) = self.get_facet_methods(resolution, facet) {
                // Нашли методы платформенного типа для фасета
                return facet_methods;
            }
            // Если lookup не удался, продолжаем с fallback
        }
        
        // Fallback: старая логика для совместимости
        // Используется для:
        // - Примитивных типов (Строка, Число)
        // - Типов без активного фасета
        // - Случаев когда платформенный тип не загружен
        let type_name = self.extract_type_name(resolution);
        let raw_type = type_name.and_then(|name| self.repository.find_type(&name));
        raw_type.map(|raw| raw.methods.clone()).unwrap_or_default()
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
    /// ```ignore
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
    /// ```ignore
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
    /// ```ignore
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
    #[allow(clippy::only_used_in_recursion)]
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
                        config.kind.to_prefix(),
                        config.name
                    ))
                }
                // Primitive и Special типы не имеют RawTypeData в repository
                ConcreteType::Primitive(_) | ConcreteType::Special(_) => None,
                // GlobalFunction может иметь документацию
                ConcreteType::GlobalFunction(func) => Some(func.name.clone()),
                ConcreteType::TabularRow(tr) => Some(tr.get_full_name()),
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

    /// Возвращает методы для Generic типа с подстановкой типовых параметров
    ///
    /// # Примеры
    /// ```ignore
    /// Generic: ТабличнаяЧасть<СтрокаРаботы>
    /// Метод: Добавить() → T
    /// Результат: Добавить() → СтрокаРаботы
    /// ```
    fn get_methods_for_generic(&self, generic_type: &GenericType) -> Vec<RawMethodData> {
        tracing::debug!(
            "🔍 Получение методов для Generic типа: {}",
            generic_type.base_type
        );

        // 1. Получаем методы базового типа (например, "ТабличнаяЧасть")
        let base_methods = self.repository
            .find_type(&generic_type.base_type)
            .map(|raw| raw.methods.clone())
            .unwrap_or_default();

        tracing::trace!(
            "  📋 Найдено {} методов базового типа",
            base_methods.len()
        );

        // 2. Если есть типовой параметр (например, СтрокаРаботы)
        if let Some(param_type) = generic_type.type_params.first() {
            // Форматируем имя типового параметра
            let param_type_name = self.format_concrete_type(param_type);

            tracing::trace!(
                "  🔄 Подстановка типового параметра: T → {}",
                param_type_name
            );

            // 3. Подставляем конкретный тип вместо "T" в методах
            base_methods
                .into_iter()
                .map(|mut method| {
                    // Подменяем "T" на конкретный тип в return_type
                    if method.return_type == "T" {
                        method.return_type = param_type_name.clone();
                        tracing::trace!(
                            "    ✅ Метод {}: return_type T → {}",
                            method.name,
                            param_type_name
                        );
                    }

                    // Подменяем "T" в типах параметров
                    for param in &mut method.params {
                        if param.param_type == "T" {
                            param.param_type = param_type_name.clone();
                            tracing::trace!(
                                "      ✅ Параметр {}: тип T → {}",
                                param.name,
                                param_type_name
                            );
                        }
                    }

                    method
                })
                .collect()
        } else {
            // Нет типовых параметров → возвращаем методы как есть
            tracing::warn!(
                "  ⚠️ Generic тип {} не имеет параметров",
                generic_type.base_type
            );
            base_methods
        }
    }

    /// Форматирует ConcreteType в строку для отображения
    ///
    /// # Примеры
    /// - `Platform(Строка)` → `"Строка"`
    /// - `Configuration(Справочники.Контрагенты)` → `"Справочники.Контрагенты"`
    /// - `TabularRow(СтрокаРаботы)` → `"СтрокаРаботы"`
    fn format_concrete_type(&self, concrete: &ConcreteType) -> String {
        match concrete {
            ConcreteType::Platform(pt) => pt.name.clone(),
            ConcreteType::Configuration(ct) => {
                // Формируем полное имя: "Справочники.Контрагенты"
                format!("{}.{}", ct.kind.to_prefix(), ct.name)
            },
            ConcreteType::Primitive(prim) => format!("{:?}", prim),
            ConcreteType::Special(spec) => format!("{:?}", spec),
            ConcreteType::GlobalFunction(gf) => gf.name.clone(),
            ConcreteType::TabularRow(tr) => tr.get_full_name(),
        }
    }

    /// Определяет имя платформенного типа на основе вида метаданных и активного фасета
    /// 
    /// # Mapping таблица:
    /// 
    /// | MetadataKind | FacetKind  | Platform Type Name     |
    /// |-------------|------------|------------------------|
    /// | Document    | Manager    | ДокументМенеджер       |
    /// | Document    | Object     | ДокументОбъект         |
    /// | Document    | Reference  | ДокументСсылка         |
    /// | Document    | Selection  | ДокументВыборка        |
    /// | Document    | List       | ДокументСписок         |
    /// | Catalog     | Manager    | СправочникМенеджер     |
    /// | Catalog     | Object     | СправочникОбъект       |
    /// | Catalog     | Reference  | СправочникСсылка       |
    /// | Catalog     | Selection  | СправочникВыборка      |
    /// | Catalog     | List       | СправочникСписок       |
    /// 
    /// # Возвращает
    /// 
    /// * `Some(&'static str)` - имя платформенного типа для поддерживаемой комбинации
    /// * `None` - для неподдерживаемых комбинаций (Enums, Registers пока не реализованы)
    /// 
    fn get_platform_facet_type(kind: MetadataKind, facet: FacetKind) -> Option<&'static str> {
        use MetadataKind::*;
        use FacetKind::*;
        
        match (kind, facet) {
            // Documents mapping
            (Document, Manager)   => Some("ДокументМенеджер"),
            (Document, Object)    => Some("ДокументОбъект"),
            (Document, Reference) => Some("ДокументСсылка"),
            (Document, Selection) => Some("ДокументВыборка"),
            (Document, List)      => Some("ДокументСписок"),
            
            // Catalogs mapping
            (Catalog, Manager)    => Some("СправочникМенеджер"),
            (Catalog, Object)     => Some("СправочникОбъект"),
            (Catalog, Reference)  => Some("СправочникСсылка"),
            (Catalog, Selection)  => Some("СправочникВыборка"),
            (Catalog, List)       => Some("СправочникСписок"),
            
            // TODO: Будущие расширения
            // (Enum, Manager) => Some("ПеречислениеМенеджер"),
            // (InformationRegister, Manager) => Some("РегистрСведенийМенеджер"),
            
            // Неподдерживаемые комбинации
            _ => None,
        }
    }

    /// Извлекает MetadataKind из TypeResolution
    /// 
    /// # Возвращает
    /// 
    /// * `Some(MetadataKind)` - для конфигурационных типов (Документы, Справочники)
    /// * `None` - для примитивных и других не-конфигурационных типов
    /// 
    fn extract_metadata_kind(&self, resolution: &TypeResolution) -> Option<MetadataKind> {
        match &resolution.result {
            ResolutionResult::Concrete(ConcreteType::Configuration(cfg)) => {
                Some(cfg.kind)
            },
            _ => None,
        }
    }

    /// Выполняет lazy lookup методов для конкретного фасета
    /// 
    /// # Алгоритм
    /// 
    /// 1. Извлекает MetadataKind из resolution
    /// 2. Определяет имя платформенного типа через mapping
    /// 3. Ищет платформенный тип в репозитории
    /// 4. Возвращает его методы
    /// 
    /// # Edge cases
    /// 
    /// - Если resolution не содержит ConfigurationType → None
    /// - Если mapping не найден для комбинации → None
    /// - Если платформенный тип не загружен → None
    /// - Если методы пусты → Some(vec![])
    /// 
    /// # Примеры
    /// 
    /// ```ignore
    /// // Документы.ЗаказНаряды + Manager фасет
    /// let methods = lookup.get_facet_methods(&resolution, FacetKind::Manager);
    /// // → Ищет "ДокументМенеджер" → Возвращает 12 методов
    /// ```
    /// 
    fn get_facet_methods(
        &self, 
        resolution: &TypeResolution, 
        facet: FacetKind
    ) -> Option<Vec<RawMethodData>> {
        // 1. Извлекаем MetadataKind
        let metadata_kind = self.extract_metadata_kind(resolution)?;
        
        // 2. Получаем имя платформенного типа через mapping
        let platform_type_name = Self::get_platform_facet_type(metadata_kind, facet)?;
        
        // 3. Ищем платформенный тип в репозитории
        let platform_type = self.repository.find_type(platform_type_name)?;
        
        // 4. Возвращаем клонированные методы
        // Важно: возвращаем Some даже для пустого Vec, чтобы отличать
        // "тип найден, но методов нет" от "тип не найден"
        Some(platform_type.methods.clone())
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::types::{
        PlatformType, RawDataSource, FacetKind,
        Certainty, ResolutionSource, ResolutionMetadata,
        TabularRowType, GenericType, RawParamData,
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
                    description: None,
                    is_deprecated: false,
                    is_constructor: false,
                },
                RawMethodData {
                    name: "Количество".to_string(),
                    english_name: "Count".to_string(),
                    return_type: "Число".to_string(),
                    params: vec![],
                    description: None,
                    is_deprecated: false,
                    is_constructor: false,
                },
            ],
            properties: vec![],
            facets: vec![FacetKind::Collection],
            kind: None,
            attributes: vec![],
            tabular_sections: vec![],
            enum_values: vec![],
            generic_info: None,
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

    // === Тесты для Generic типов ===

    /// Вспомогательная функция для создания тестового репозитория с Generic типами
    fn create_test_repository_with_generic_types() -> Arc<InMemoryTypeRepository> {
        let repo = Arc::new(InMemoryTypeRepository::new());

        // Создаём платформенный тип "ТабличнаяЧасть" с Generic методами
        let tabular_type = RawTypeData {
            name: "ТабличнаяЧасть".to_string(),
            english_name: "TabularSection".to_string(),
            category: "PlatformType".to_string(),
            description: "Табличная часть с Generic методами".to_string(),
            source: RawDataSource::Platform,
            facets: vec![FacetKind::Collection],
            methods: vec![
                RawMethodData {
                    name: "Добавить".to_string(),
                    english_name: "Add".to_string(),
                    return_type: "T".to_string(),  // ← Generic!
                    params: vec![],
                    description: None,
                    is_deprecated: false,
                    is_constructor: false,
                },
                RawMethodData {
                    name: "Получить".to_string(),
                    english_name: "Get".to_string(),
                    return_type: "T".to_string(),  // ← Generic!
                    params: vec![
                        RawParamData {
                            name: "Индекс".to_string(),
                            param_type: "Число".to_string(),
                            is_optional: false,
                            default_value: None,
                        },
                    ],
                    description: None,
                    is_deprecated: false,
                    is_constructor: false,
                },
                RawMethodData {
                    name: "Количество".to_string(),
                    english_name: "Count".to_string(),
                    return_type: "Число".to_string(),  // НЕ Generic
                    params: vec![],
                    description: None,
                    is_deprecated: false,
                    is_constructor: false,
                },
                RawMethodData {
                    name: "Индекс".to_string(),
                    english_name: "IndexOf".to_string(),
                    return_type: "Число".to_string(),
                    params: vec![
                        RawParamData {
                            name: "Строка".to_string(),
                            param_type: "T".to_string(),  // ← Generic параметр!
                            is_optional: false,
                            default_value: None,
                        },
                    ],
                    description: None,
                    is_deprecated: false,
                    is_constructor: false,
                },
            ],
            properties: vec![],
            kind: None,
            attributes: vec![],
            tabular_sections: vec![],
            enum_values: vec![],
            generic_info: None,
        };

        repo.load_types(vec![tabular_type]).unwrap();
        repo
    }

    #[test]
    fn test_generic_method_return_type_substitution() {
        let repo = create_test_repository_with_generic_types();
        let lookup = TypeMetadataLookup::new(repo.clone());

        // Создаём Generic тип: ТабличнаяЧасть<СтрокаРаботы>
        let row_type = TabularRowType::new(
            "Документы.ЗаказНаряды".to_string(),
            "Работы".to_string(),
            vec![],
        );

        let generic_type = GenericType {
            base_type: "ТабличнаяЧасть".to_string(),
            type_params: vec![ConcreteType::TabularRow(row_type)],
        };

        let resolution = TypeResolution {
            result: ResolutionResult::Generic(generic_type),
            certainty: Certainty::Known,
            source: ResolutionSource::Static,
            active_facet: Some(FacetKind::Collection),
            metadata: ResolutionMetadata::default(),
            available_facets: vec![],
        };

        // Получаем методы
        let methods = lookup.get_methods(&resolution);

        // Проверяем метод "Добавить": return_type должен быть "СтрокаРаботы"
        let add_method = methods.iter().find(|m| m.name == "Добавить").unwrap();
        assert_eq!(add_method.return_type, "СтрокаРаботы");

        // Проверяем метод "Получить": return_type должен быть "СтрокаРаботы"
        let get_method = methods.iter().find(|m| m.name == "Получить").unwrap();
        assert_eq!(get_method.return_type, "СтрокаРаботы");
    }

    #[test]
    fn test_generic_method_param_type_substitution() {
        let repo = create_test_repository_with_generic_types();
        let lookup = TypeMetadataLookup::new(repo.clone());

        // Создаём Generic тип
        let row_type = TabularRowType::new(
            "Документы.ЗаказНаряды".to_string(),
            "Работы".to_string(),
            vec![],
        );

        let generic_type = GenericType {
            base_type: "ТабличнаяЧасть".to_string(),
            type_params: vec![ConcreteType::TabularRow(row_type)],
        };

        let resolution = TypeResolution {
            result: ResolutionResult::Generic(generic_type),
            certainty: Certainty::Known,
            source: ResolutionSource::Static,
            active_facet: Some(FacetKind::Collection),
            metadata: ResolutionMetadata::default(),
            available_facets: vec![],
        };

        let methods = lookup.get_methods(&resolution);

        // Проверяем метод "Индекс": параметр "Строка" должен иметь тип "СтрокаРаботы"
        let index_method = methods.iter().find(|m| m.name == "Индекс").unwrap();
        assert_eq!(index_method.params.len(), 1);
        assert_eq!(index_method.params[0].name, "Строка");
        assert_eq!(index_method.params[0].param_type, "СтрокаРаботы");
    }

    #[test]
    fn test_non_generic_methods_unchanged() {
        let repo = create_test_repository_with_generic_types();
        let lookup = TypeMetadataLookup::new(repo.clone());

        let row_type = TabularRowType::new(
            "Документы.ЗаказНаряды".to_string(),
            "Работы".to_string(),
            vec![],
        );

        let generic_type = GenericType {
            base_type: "ТабличнаяЧасть".to_string(),
            type_params: vec![ConcreteType::TabularRow(row_type)],
        };

        let resolution = TypeResolution {
            result: ResolutionResult::Generic(generic_type),
            certainty: Certainty::Known,
            source: ResolutionSource::Static,
            active_facet: Some(FacetKind::Collection),
            metadata: ResolutionMetadata::default(),
            available_facets: vec![],
        };

        let methods = lookup.get_methods(&resolution);

        // Проверяем метод "Количество": return_type должен остаться "Число"
        let count_method = methods.iter().find(|m| m.name == "Количество").unwrap();
        assert_eq!(count_method.return_type, "Число");
    }

    #[test]
    fn test_all_methods_returned() {
        let repo = create_test_repository_with_generic_types();
        let lookup = TypeMetadataLookup::new(repo.clone());

        let row_type = TabularRowType::new(
            "Документы.ЗаказНаряды".to_string(),
            "Работы".to_string(),
            vec![],
        );

        let generic_type = GenericType {
            base_type: "ТабличнаяЧасть".to_string(),
            type_params: vec![ConcreteType::TabularRow(row_type)],
        };

        let resolution = TypeResolution {
            result: ResolutionResult::Generic(generic_type),
            certainty: Certainty::Known,
            source: ResolutionSource::Static,
            active_facet: Some(FacetKind::Collection),
            metadata: ResolutionMetadata::default(),
            available_facets: vec![],
        };

        let methods = lookup.get_methods(&resolution);

        // Должны вернуться все 4 метода
        assert_eq!(methods.len(), 4);

        let method_names: Vec<_> = methods.iter().map(|m| m.name.as_str()).collect();
        assert!(method_names.contains(&"Добавить"));
        assert!(method_names.contains(&"Получить"));
        assert!(method_names.contains(&"Количество"));
        assert!(method_names.contains(&"Индекс"));
    }
}
