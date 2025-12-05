//! TypeMetadataLookup - мост между TypeResolution и RawTypeData
//!
//! Этот модуль предоставляет сервис для получения полной документации типа
//! на основе результата статического анализа (TypeResolution).

use crate::domain::repository::TypeRepository;
use crate::domain::signature_index::MethodSignature;
use crate::domain::types::{
    ConcreteType, FacetKind, GenericType, MetadataKind, RawMethodData, RawParamData,
    RawPropertyData, RawTypeData, ResolutionResult, TypeResolution,
};
use crate::utils::string_utils::levenshtein_distance;
use std::sync::Arc;

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

        // Приоритет 1 - Lazy lookup через active_facet (для конфигурационных типов)
        if let Some(facet) = resolution.active_facet {
            if let Some(facet_methods) = self.get_facet_methods(resolution, facet) {
                return facet_methods;
            }
        }

        // Приоритет 2 - Нормализованное имя типа через SignatureIndex
        if let Some(name) = self.normalize_type_name(resolution) {
            let sig_methods = self.repository.get_methods_from_signature_index(&name);
            if !sig_methods.is_empty() {
                tracing::trace!(
                    "get_methods('{}') → found {} methods in signature_index",
                    name,
                    sig_methods.len()
                );
                return sig_methods
                    .into_iter()
                    .map(Self::method_signature_to_raw)
                    .collect();
            }
        }

        // Приоритет 3 - Извлекаем имя типа для fallback поиска
        if let Some(name) = self.extract_type_name(resolution) {
            // Сначала пробуем SignatureIndex с извлечённым именем
            let sig_methods = self.repository.get_methods_from_signature_index(&name);
            if !sig_methods.is_empty() {
                tracing::trace!(
                    "get_methods('{}') → found {} methods in signature_index (extracted name)",
                    name,
                    sig_methods.len()
                );
                return sig_methods
                    .into_iter()
                    .map(Self::method_signature_to_raw)
                    .collect();
            }

            // Fallback: для типов не в SignatureIndex (примитивные, тестовые)
            if let Some(raw) = self.repository.find_type(&name) {
                tracing::trace!(
                    "get_methods('{}') → fallback to raw types ({} methods)",
                    name,
                    raw.methods.len()
                );
                return raw.methods.clone();
            }

            // Fallback для фасетных типов: извлекаем базовый тип
            if let Some(base_type) = super::facet_utils::extract_base_facet_type(&name) {
                // Сначала пробуем SignatureIndex
                let sig_methods = self.repository.get_methods_from_signature_index(base_type);
                if !sig_methods.is_empty() {
                    tracing::trace!(
                        "get_methods('{}') → found {} methods via base type '{}' in signature_index",
                        name,
                        sig_methods.len(),
                        base_type
                    );
                    return sig_methods
                        .into_iter()
                        .map(Self::method_signature_to_raw)
                        .collect();
                }

                // Затем raw types
                if let Some(raw) = self.repository.find_type(base_type) {
                    tracing::trace!(
                        "get_methods('{}') → fallback via base type '{}' ({} methods)",
                        name,
                        base_type,
                        raw.methods.len()
                    );
                    return raw.methods.clone();
                }
            }
        }

        vec![]
    }

    /// Нормализует имя типа для поиска в SignatureIndex
    ///
    /// Учитывает active_facet для построения имени платформенного типа.
    ///
    /// # Возвращает
    /// * `Some(String)` - нормализованное имя типа для SignatureIndex
    /// * `None` - если тип не поддерживается
    fn normalize_type_name(&self, resolution: &TypeResolution) -> Option<String> {
        // 1. Если есть active_facet → строим platform facet type name
        if let Some(facet) = resolution.active_facet {
            if let Some(metadata_kind) = self.extract_metadata_kind(resolution) {
                if let Some(platform_name) = Self::get_platform_facet_type(metadata_kind, facet) {
                    return Some(platform_name.to_string());
                }
            }
        }

        // 2. Fallback на extract_type_name
        self.extract_type_name(resolution)
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
        if raw
            .methods
            .iter()
            .any(|m| m.name == member_name || m.english_name == member_name)
        {
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
                    Some(format!("{}.{}", config.kind.to_prefix(), config.name))
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
            ResolutionResult::Intersection(types) => types.first().and_then(|t| {
                self.extract_type_name(&TypeResolution {
                    result: ResolutionResult::Concrete(t.clone()),
                    ..resolution.clone()
                })
            }),
            // Generic - используем базовый тип
            ResolutionResult::Generic(gen) => Some(gen.base_type.clone()),
            // Nullable - распаковываем внутренний тип
            ResolutionResult::Nullable(inner) => self.extract_type_name(&TypeResolution {
                result: ResolutionResult::Concrete(inner.as_ref().clone()),
                ..resolution.clone()
            }),
        }
    }

    /// Конвертирует MethodSignature из signature_index в RawMethodData
    ///
    /// Это необходимо для обратной совместимости с существующим API,
    /// который возвращает Vec<RawMethodData>.
    fn method_signature_to_raw(sig: MethodSignature) -> RawMethodData {
        RawMethodData {
            name: sig.name,
            english_name: String::new(), // SignatureIndex не хранит english_name
            return_type: sig.return_type.unwrap_or_default(),
            params: sig
                .params
                .into_iter()
                .map(|p| RawParamData {
                    name: p.name,
                    param_type: p.type_name.unwrap_or_default(),
                    is_optional: p.is_optional,
                    default_value: p.default_value,
                })
                .collect(),
            description: None,
            is_deprecated: false,
            is_constructor: false,
            context_requirements: Some(sig.context_requirements),
            return_facet: sig.return_facet,
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
        let base_methods = self
            .repository
            .find_type(&generic_type.base_type)
            .map(|raw| raw.methods.clone())
            .unwrap_or_default();

        tracing::trace!("  📋 Найдено {} методов базового типа", base_methods.len());

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
            }
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
        use FacetKind::*;
        use MetadataKind::*;

        // ВАЖНО: имена типов должны содержать placeholder как в Syntax Helper
        // Например: "СправочникМенеджер.<Имя справочника>" вместо "СправочникМенеджер"
        match (kind, facet) {
            // Documents mapping
            (Document, Manager) => Some("ДокументМенеджер.<Имя документа>"),
            (Document, Object) => Some("ДокументОбъект.<Имя документа>"),
            (Document, Reference) => Some("ДокументСсылка.<Имя документа>"),
            (Document, Selection) => Some("ДокументВыборка.<Имя документа>"),
            (Document, List) => Some("ДокументСписок.<Имя документа>"),

            // Catalogs mapping
            (Catalog, Manager) => Some("СправочникМенеджер.<Имя справочника>"),
            (Catalog, Object) => Some("СправочникОбъект.<Имя справочника>"),
            (Catalog, Reference) => Some("СправочникСсылка.<Имя справочника>"),
            (Catalog, Selection) => Some("СправочникВыборка.<Имя справочника>"),
            (Catalog, List) => Some("СправочникСписок.<Имя справочника>"),

            // Enums mapping
            (Enum, Manager) => Some("ПеречислениеМенеджер.<Имя перечисления>"),
            (Enum, Reference) => Some("ПеречислениеСсылка.<Имя перечисления>"),

            // Information Registers mapping
            (InformationRegister, Manager) => Some("РегистрСведенийМенеджер.<Имя регистра сведений>"),
            (InformationRegister, Collection) => Some("РегистрСведенийНаборЗаписей.<Имя регистра сведений>"),
            (InformationRegister, Selection) => Some("РегистрСведенийВыборка.<Имя регистра сведений>"),

            // Accumulation Registers mapping
            (AccumulationRegister, Manager) => Some("РегистрНакопленияМенеджер.<Имя регистра накопления>"),
            (AccumulationRegister, Collection) => Some("РегистрНакопленияНаборЗаписей.<Имя регистра накопления>"),
            (AccumulationRegister, Selection) => Some("РегистрНакопленияВыборка.<Имя регистра накопления>"),

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
            ResolutionResult::Concrete(ConcreteType::Configuration(cfg)) => Some(cfg.kind),
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
        facet: FacetKind,
    ) -> Option<Vec<RawMethodData>> {
        // 1. Извлекаем MetadataKind
        let metadata_kind = self.extract_metadata_kind(resolution)?;

        // 2. Получаем имя платформенного типа через mapping
        let platform_type_name = Self::get_platform_facet_type(metadata_kind, facet)?;

        // 3. ✅ ПРИОРИТЕТ: Сначала ищем в signature_index (обогащённые данные)
        let sig_methods = self.repository.get_methods_from_signature_index(platform_type_name);
        if !sig_methods.is_empty() {
            tracing::trace!(
                "get_facet_methods('{}') → found {} methods in signature_index",
                platform_type_name,
                sig_methods.len()
            );
            return Some(
                sig_methods
                    .into_iter()
                    .map(Self::method_signature_to_raw)
                    .collect(),
            );
        }

        // Fallback: ищем в raw types
        // Сначала пробуем точное имя с placeholder ("ДокументМенеджер.<Имя документа>")
        if let Some(platform_type) = self.repository.find_type(platform_type_name) {
            tracing::trace!(
                "get_facet_methods('{}') → fallback to raw types ({} methods)",
                platform_type_name,
                platform_type.methods.len()
            );
            return Some(platform_type.methods.clone());
        }

        // Если не найдено, пробуем извлечь базовый тип ("ДокументМенеджер")
        // Это нужно для тестов, которые создают типы без placeholder
        if let Some(base_type_name) = super::facet_utils::extract_base_facet_type(platform_type_name)
        {
            // Сначала пробуем SignatureIndex с базовым типом
            let sig_methods = self.repository.get_methods_from_signature_index(base_type_name);
            if !sig_methods.is_empty() {
                tracing::trace!(
                    "get_facet_methods('{}') → found {} methods via base type '{}' in signature_index",
                    platform_type_name,
                    sig_methods.len(),
                    base_type_name
                );
                return Some(
                    sig_methods
                        .into_iter()
                        .map(Self::method_signature_to_raw)
                        .collect(),
                );
            }

            // Затем fallback на raw types
            if let Some(platform_type) = self.repository.find_type(base_type_name) {
                tracing::trace!(
                    "get_facet_methods('{}') → fallback to raw types via base type '{}' ({} methods)",
                    platform_type_name,
                    base_type_name,
                    platform_type.methods.len()
                );
                return Some(platform_type.methods.clone());
            }
        }

        // Тип не найден ни с placeholder, ни без него
        None
    }

    // === Milestone 3.16: MetadataLookup API ===

    /// Проверяет существование объекта метаданных указанного вида
    ///
    /// # Параметры
    ///
    /// * `kind` - вид метаданных (Catalog, Document, etc.)
    /// * `name` - имя объекта без префикса (например, "Контрагенты")
    ///
    /// # Возвращает
    ///
    /// `true` если объект найден в репозитории
    ///
    /// # Примеры
    ///
    /// ```ignore
    /// let lookup = TypeMetadataLookup::new(repository);
    ///
    /// // Проверяем существующий справочник
    /// assert!(lookup.exists_metadata_object(MetadataKind::Catalog, "Контрагенты"));
    ///
    /// // Проверяем несуществующий справочник
    /// assert!(!lookup.exists_metadata_object(MetadataKind::Catalog, "НесуществующийСправочник"));
    /// ```
    pub fn exists_metadata_object(&self, kind: MetadataKind, name: &str) -> bool {
        let full_type_name = format!("{}.{}", kind.to_prefix(), name);
        self.repository.find_type(&full_type_name).is_some()
    }

    /// Возвращает похожие имена объектов метаданных (fuzzy matching)
    ///
    /// Использует алгоритм Левенштейна для поиска похожих имён.
    /// Полезно для диагностических сообщений с предложениями исправлений.
    ///
    /// # Параметры
    ///
    /// * `kind` - вид метаданных (Catalog, Document, etc.)
    /// * `name` - имя для поиска похожих
    /// * `max_suggestions` - максимальное количество предложений
    ///
    /// # Алгоритм
    ///
    /// 1. Получает все объекты указанного вида
    /// 2. Вычисляет расстояние Левенштейна для каждого
    /// 3. Фильтрует по порогу (distance <= max(len/2, 3))
    /// 4. Сортирует по расстоянию (меньше = лучше)
    /// 5. Возвращает топ-N результатов
    ///
    /// # Примеры
    ///
    /// ```ignore
    /// let lookup = TypeMetadataLookup::new(repository);
    ///
    /// // Опечатка: "Контрогенты" вместо "Контрагенты"
    /// let suggestions = lookup.suggest_similar_names(
    ///     MetadataKind::Catalog,
    ///     "Контрогенты",
    ///     3
    /// );
    /// // → ["Контрагенты"]
    /// ```
    pub fn suggest_similar_names(
        &self,
        kind: MetadataKind,
        name: &str,
        max_suggestions: usize,
    ) -> Vec<String> {
        let all_objects = self.repository.get_metadata_objects_by_kind(kind);

        let mut candidates: Vec<(String, usize)> = all_objects
            .into_iter()
            .filter_map(|obj_name| {
                let distance = levenshtein_distance(name, &obj_name);
                // Порог: до половины длины имени, но минимум 3
                let threshold = (name.chars().count() / 2).max(3);
                if distance <= threshold {
                    Some((obj_name, distance))
                } else {
                    None
                }
            })
            .collect();

        // Сортируем по расстоянию (меньше = лучше совпадение)
        candidates.sort_by_key(|(_, dist)| *dist);

        candidates
            .into_iter()
            .take(max_suggestions)
            .map(|(n, _)| n)
            .collect()
    }

    /// Проверяет, загружена ли конфигурация в репозиторий
    ///
    /// # Возвращает
    ///
    /// `true` если есть хотя бы один конфигурационный тип
    ///
    /// # Использование
    ///
    /// Полезно для определения, нужно ли выполнять валидацию
    /// объектов метаданных или просто пропустить проверку.
    ///
    /// # Примеры
    ///
    /// ```ignore
    /// let lookup = TypeMetadataLookup::new(repository);
    ///
    /// if lookup.is_configuration_loaded() {
    ///     // Выполняем валидацию объектов метаданных
    ///     if !lookup.exists_metadata_object(kind, name) {
    ///         // Генерируем ошибку
    ///     }
    /// } else {
    ///     // Пропускаем валидацию - конфигурация не загружена
    /// }
    /// ```
    pub fn is_configuration_loaded(&self) -> bool {
        let stats = self.repository.get_stats();
        stats.configuration_types > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::repository::InMemoryTypeRepository;
    use crate::domain::types::{
        Certainty, FacetKind, GenericType, PlatformType, RawDataSource, RawParamData,
        ResolutionMetadata, ResolutionSource, TabularRowType,
    };

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
                    context_requirements: None,
                    return_facet: None,
                },
                RawMethodData {
                    name: "Количество".to_string(),
                    english_name: "Count".to_string(),
                    return_type: "Число".to_string(),
                    params: vec![],
                    description: None,
                    is_deprecated: false,
                    is_constructor: false,
                    context_requirements: None,
                    return_facet: None,
                },
            ],
            properties: vec![],
            facets: vec![FacetKind::Collection],
            kind: None,
            attributes: vec![],
            tabular_sections: vec![],
            enum_values: vec![],
            generic_info: None,
            module_paths: None,
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
                    return_type: "T".to_string(), // ← Generic!
                    params: vec![],
                    description: None,
                    is_deprecated: false,
                    is_constructor: false,
                    context_requirements: None,
                    return_facet: None,
                },
                RawMethodData {
                    name: "Получить".to_string(),
                    english_name: "Get".to_string(),
                    return_type: "T".to_string(), // ← Generic!
                    params: vec![RawParamData {
                        name: "Индекс".to_string(),
                        param_type: "Число".to_string(),
                        is_optional: false,
                        default_value: None,
                    }],
                    description: None,
                    is_deprecated: false,
                    is_constructor: false,
                    context_requirements: None,
                    return_facet: None,
                },
                RawMethodData {
                    name: "Количество".to_string(),
                    english_name: "Count".to_string(),
                    return_type: "Число".to_string(), // НЕ Generic
                    params: vec![],
                    description: None,
                    is_deprecated: false,
                    is_constructor: false,
                    context_requirements: None,
                    return_facet: None,
                },
                RawMethodData {
                    name: "Индекс".to_string(),
                    english_name: "IndexOf".to_string(),
                    return_type: "Число".to_string(),
                    params: vec![RawParamData {
                        name: "Строка".to_string(),
                        param_type: "T".to_string(), // ← Generic параметр!
                        is_optional: false,
                        default_value: None,
                    }],
                    description: None,
                    is_deprecated: false,
                    is_constructor: false,
                    context_requirements: None,
                    return_facet: None,
                },
            ],
            properties: vec![],
            kind: None,
            attributes: vec![],
            tabular_sections: vec![],
            enum_values: vec![],
            generic_info: None,
            module_paths: None,
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

    // === Тесты для Milestone 3.16: MetadataLookup API ===

    /// Создаёт репозиторий с конфигурационными типами для тестирования
    fn create_test_repository_with_config_types() -> Arc<InMemoryTypeRepository> {
        use crate::domain::types::RawDataSource;

        let repo = Arc::new(InMemoryTypeRepository::new());

        // Создаём тестовые справочники
        let catalog1 = RawTypeData {
            name: "Справочники.Контрагенты".to_string(),
            english_name: "Catalogs.Contractors".to_string(),
            description: "Справочник контрагентов".to_string(),
            category: "Справочники".to_string(),
            source: RawDataSource::Configuration,
            methods: vec![],
            properties: vec![],
            facets: vec![FacetKind::Manager, FacetKind::Object, FacetKind::Reference],
            kind: Some(MetadataKind::Catalog),
            attributes: vec![],
            tabular_sections: vec![],
            enum_values: vec![],
            generic_info: None,
            module_paths: None,
        };

        let catalog2 = RawTypeData {
            name: "Справочники.Номенклатура".to_string(),
            english_name: "Catalogs.Products".to_string(),
            description: "Справочник номенклатуры".to_string(),
            category: "Справочники".to_string(),
            source: RawDataSource::Configuration,
            methods: vec![],
            properties: vec![],
            facets: vec![FacetKind::Manager, FacetKind::Object, FacetKind::Reference],
            kind: Some(MetadataKind::Catalog),
            attributes: vec![],
            tabular_sections: vec![],
            enum_values: vec![],
            generic_info: None,
            module_paths: None,
        };

        let catalog3 = RawTypeData {
            name: "Справочники.Склады".to_string(),
            english_name: "Catalogs.Warehouses".to_string(),
            description: "Справочник складов".to_string(),
            category: "Справочники".to_string(),
            source: RawDataSource::Configuration,
            methods: vec![],
            properties: vec![],
            facets: vec![FacetKind::Manager, FacetKind::Object, FacetKind::Reference],
            kind: Some(MetadataKind::Catalog),
            attributes: vec![],
            tabular_sections: vec![],
            enum_values: vec![],
            generic_info: None,
            module_paths: None,
        };

        // Создаём тестовый документ
        let document = RawTypeData {
            name: "Документы.ЗаказПокупателя".to_string(),
            english_name: "Documents.CustomerOrder".to_string(),
            description: "Заказ покупателя".to_string(),
            category: "Документы".to_string(),
            source: RawDataSource::Configuration,
            methods: vec![],
            properties: vec![],
            facets: vec![FacetKind::Manager, FacetKind::Object, FacetKind::Reference],
            kind: Some(MetadataKind::Document),
            attributes: vec![],
            tabular_sections: vec![],
            enum_values: vec![],
            generic_info: None,
            module_paths: None,
        };

        repo.load_types(vec![catalog1, catalog2, catalog3, document])
            .unwrap();
        repo
    }

    #[test]
    fn test_exists_metadata_object_found() {
        let repo = create_test_repository_with_config_types();
        let lookup = TypeMetadataLookup::new(repo);

        assert!(lookup.exists_metadata_object(MetadataKind::Catalog, "Контрагенты"));
        assert!(lookup.exists_metadata_object(MetadataKind::Catalog, "Номенклатура"));
        assert!(lookup.exists_metadata_object(MetadataKind::Document, "ЗаказПокупателя"));
    }

    #[test]
    fn test_exists_metadata_object_not_found() {
        let repo = create_test_repository_with_config_types();
        let lookup = TypeMetadataLookup::new(repo);

        assert!(!lookup.exists_metadata_object(MetadataKind::Catalog, "НесуществующийСправочник"));
        assert!(!lookup.exists_metadata_object(MetadataKind::Document, "НесуществующийДокумент"));
        // Неправильный вид метаданных
        assert!(!lookup.exists_metadata_object(MetadataKind::Document, "Контрагенты"));
    }

    #[test]
    fn test_suggest_similar_names_typo() {
        let repo = create_test_repository_with_config_types();
        let lookup = TypeMetadataLookup::new(repo);

        // Опечатка: "Контрогенты" вместо "Контрагенты"
        let suggestions =
            lookup.suggest_similar_names(MetadataKind::Catalog, "Контрогенты", 3);

        assert!(!suggestions.is_empty());
        assert!(suggestions.contains(&"Контрагенты".to_string()));
    }

    #[test]
    fn test_suggest_similar_names_no_match() {
        let repo = create_test_repository_with_config_types();
        let lookup = TypeMetadataLookup::new(repo);

        // Совсем непохожее имя
        let suggestions =
            lookup.suggest_similar_names(MetadataKind::Catalog, "АбсолютноДругоеИмя", 3);

        // Должен вернуть пустой вектор - слишком большое расстояние
        assert!(suggestions.is_empty());
    }

    #[test]
    fn test_suggest_similar_names_sorting() {
        let repo = create_test_repository_with_config_types();
        let lookup = TypeMetadataLookup::new(repo);

        // "Склад" близко к "Склады" (1 операция)
        let suggestions = lookup.suggest_similar_names(MetadataKind::Catalog, "Склад", 3);

        assert!(!suggestions.is_empty());
        // Склады должен быть в списке (расстояние = 1)
        assert!(suggestions.contains(&"Склады".to_string()));
    }

    #[test]
    fn test_suggest_similar_names_max_limit() {
        let repo = create_test_repository_with_config_types();
        let lookup = TypeMetadataLookup::new(repo);

        // Ограничение на количество предложений
        let suggestions = lookup.suggest_similar_names(MetadataKind::Catalog, "Н", 1);

        assert!(suggestions.len() <= 1);
    }

    #[test]
    fn test_is_configuration_loaded_true() {
        let repo = create_test_repository_with_config_types();
        let lookup = TypeMetadataLookup::new(repo);

        assert!(lookup.is_configuration_loaded());
    }

    #[test]
    fn test_is_configuration_loaded_false() {
        // Репозиторий только с платформенными типами
        let repo = create_test_repository();
        let lookup = TypeMetadataLookup::new(repo);

        assert!(!lookup.is_configuration_loaded());
    }

    #[test]
    fn test_get_metadata_objects_by_kind() {
        let repo = create_test_repository_with_config_types();

        let catalogs = repo.get_metadata_objects_by_kind(MetadataKind::Catalog);
        assert_eq!(catalogs.len(), 3);
        assert!(catalogs.contains(&"Контрагенты".to_string()));
        assert!(catalogs.contains(&"Номенклатура".to_string()));
        assert!(catalogs.contains(&"Склады".to_string()));

        let documents = repo.get_metadata_objects_by_kind(MetadataKind::Document);
        assert_eq!(documents.len(), 1);
        assert!(documents.contains(&"ЗаказПокупателя".to_string()));

        // Пустой результат для несуществующего вида
        let enums = repo.get_metadata_objects_by_kind(MetadataKind::Enum);
        assert!(enums.is_empty());
    }

    // === Тесты для приоритета signature_index над raw types ===

    #[test]
    fn test_get_methods_prefers_signature_index() {
        use crate::domain::signature_index::{MethodSignature, SignatureSource, ContextRequirements};
        use crate::domain::types::ParameterInfo;

        let repo = Arc::new(InMemoryTypeRepository::new());

        // 1. Создаём тип с методом БЕЗ return_type (как из syntax_helper)
        let manager_type = RawTypeData {
            name: "СправочникМенеджер.<Имя справочника>".to_string(),
            english_name: "CatalogManager".to_string(),
            description: "Менеджер справочника".to_string(),
            category: "Справочники".to_string(),
            source: RawDataSource::Platform,
            methods: vec![
                RawMethodData {
                    name: "НайтиПоКоду".to_string(),
                    english_name: "FindByCode".to_string(),
                    return_type: "".to_string(), // Пустой return_type в raw data!
                    params: vec![],
                    description: None,
                    is_deprecated: false,
                    is_constructor: false,
                    context_requirements: None,
                    return_facet: None,
                },
            ],
            properties: vec![],
            facets: vec![FacetKind::Manager],
            kind: None,
            attributes: vec![],
            tabular_sections: vec![],
            enum_values: vec![],
            generic_info: None,
            module_paths: None,
        };

        repo.load_types(vec![manager_type]).unwrap();

        // 2. Добавляем метод в signature_index С return_type (как из platform_types.rs)
        repo.populate_signature_index(|index| {
            let sig = MethodSignature::new(
                "НайтиПоКоду".to_string(),
                Some("СправочникМенеджер.<Имя справочника>".to_string()),
                vec![
                    ParameterInfo {
                        name: "Код".to_string(),
                        type_name: Some("Число | Строка".to_string()),
                        is_optional: false,
                        default_value: None,
                        description: None,
                    },
                ],
                Some("СправочникСсылка".to_string()), // Корректный return_type!
                SignatureSource::Platform,
                Some(FacetKind::Reference),
                ContextRequirements::Universal,
            );
            index.add_platform_method("СправочникМенеджер.<Имя справочника>".to_string(), sig);
        });

        // 3. Проверяем через TypeMetadataLookup
        let lookup = TypeMetadataLookup::new(repo.clone());
        let resolution = create_test_resolution("СправочникМенеджер.<Имя справочника>");

        let methods = lookup.get_methods(&resolution);

        // Должен найти 1 метод
        assert_eq!(methods.len(), 1, "Should find 1 method");

        let method = &methods[0];
        assert_eq!(method.name, "НайтиПоКоду");

        // ГЛАВНАЯ ПРОВЕРКА: return_type должен быть из signature_index, не из raw data!
        assert_eq!(
            method.return_type, "СправочникСсылка",
            "return_type should come from signature_index, not raw data"
        );

        // Проверяем параметры тоже из signature_index
        assert_eq!(method.params.len(), 1, "Should have 1 param from signature_index");
        assert_eq!(method.params[0].name, "Код");
        assert_eq!(method.params[0].param_type, "Число | Строка");

        // Проверяем return_facet
        assert_eq!(method.return_facet, Some(FacetKind::Reference));
    }

    #[test]
    fn test_get_methods_fallback_to_raw_when_no_signature_index() {
        let repo = Arc::new(InMemoryTypeRepository::new());

        // Создаём тип ТОЛЬКО в raw types (без signature_index)
        let simple_type = RawTypeData {
            name: "ПростойТип".to_string(),
            english_name: "SimpleType".to_string(),
            description: "Тип без signature_index".to_string(),
            category: "Тестовые".to_string(),
            source: RawDataSource::Platform,
            methods: vec![
                RawMethodData {
                    name: "Метод1".to_string(),
                    english_name: "Method1".to_string(),
                    return_type: "Строка".to_string(),
                    params: vec![],
                    description: None,
                    is_deprecated: false,
                    is_constructor: false,
                    context_requirements: None,
                    return_facet: None,
                },
            ],
            properties: vec![],
            facets: vec![],
            kind: None,
            attributes: vec![],
            tabular_sections: vec![],
            enum_values: vec![],
            generic_info: None,
            module_paths: None,
        };

        repo.load_types(vec![simple_type]).unwrap();

        // НЕ добавляем в signature_index

        let lookup = TypeMetadataLookup::new(repo.clone());
        let resolution = create_test_resolution("ПростойТип");

        let methods = lookup.get_methods(&resolution);

        // Должен найти метод из raw types (fallback)
        assert_eq!(methods.len(), 1);
        assert_eq!(methods[0].name, "Метод1");
        assert_eq!(methods[0].return_type, "Строка");
    }
}
