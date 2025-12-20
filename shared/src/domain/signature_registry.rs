//! Registry паттерн для источников данных SignatureIndex
//!
//! Обеспечивает декларативную регистрацию источников типов.
//! Milestone 3.x: Рефакторинг для предотвращения забытых источников.

use super::signature_index::SignatureIndex;
use super::type_id::TypeId;
use super::types::RawTypeData;

/// Источник данных для SignatureIndex
///
/// Каждый источник предоставляет типы платформы или конфигурации
/// для наполнения SignatureIndex методами.
pub trait SignatureDataSource: Send + Sync {
    /// Имя источника (для логирования и отладки)
    fn name(&self) -> &str;

    /// Приоритет применения (меньше = раньше)
    ///
    /// Источники применяются в порядке возрастания приоритета.
    /// Это важно для merge-логики: более поздние источники
    /// могут дополнять (но не перезаписывать) данные ранних.
    fn priority(&self) -> u32;

    /// Загрузить данные типов
    fn load(&self) -> Vec<RawTypeData>;
}

/// Реестр источников SignatureIndex
///
/// Позволяет декларативно регистрировать источники типов
/// и собирать SignatureIndex из всех зарегистрированных источников.
///
/// # Пример
/// ```ignore
/// let index = SignatureSourceRegistry::new()
///     .register(SyntaxHelperSource::new(platform_types))
///     .build();
/// ```
pub struct SignatureSourceRegistry {
    sources: Vec<Box<dyn SignatureDataSource>>,
}

impl SignatureSourceRegistry {
    /// Создать пустой реестр
    pub fn new() -> Self {
        Self { sources: vec![] }
    }

    /// Зарегистрировать источник данных (builder pattern)
    ///
    /// # Arguments
    /// * `source` - Реализация SignatureDataSource
    ///
    /// # Returns
    /// Self для цепочки вызовов
    pub fn register<S: SignatureDataSource + 'static>(mut self, source: S) -> Self {
        self.sources.push(Box::new(source));
        self
    }

    /// Собрать SignatureIndex из всех зарегистрированных источников
    ///
    /// Источники обрабатываются в порядке приоритета (меньше = раньше).
    /// Методы добавляются через `add_platform_method`, который поддерживает merge-логику:
    /// - Если метод уже существует, обновляются только пустые поля
    /// - return_type, return_facet, context_requirements, params могут быть дополнены
    pub fn build(&self) -> SignatureIndex {
        let mut index = SignatureIndex::new();

        // Сортируем по приоритету
        let mut sorted: Vec<_> = self.sources.iter().collect();
        sorted.sort_by_key(|s| s.priority());

        for source in sorted {
            tracing::debug!(
                "Loading SignatureIndex source: {} (priority={})",
                source.name(),
                source.priority()
            );
            let types = source.load();

            // Заполняем методами из типов
            for platform_type in &types {
                for method in &platform_type.methods {
                    if method.is_constructor {
                        let constructor =
                            raw_method_to_constructor(method, &platform_type.name, platform_type);
                        index.add_constructor(TypeId::new(&platform_type.name), constructor);
                        continue;
                    }

                    let signature = raw_method_to_signature(method, &platform_type.name);
                    let type_id = TypeId::new(&platform_type.name);
                    index.add_platform_method(type_id, signature.clone());

                    // Также добавляем под базовым именем для фасетных типов с placeholder
                    if let Some(base_type) =
                        super::facet_utils::extract_placeholder_base_type(&platform_type.name)
                    {
                        index.add_platform_method(TypeId::new(base_type), signature);
                    }
                }
            }
        }

        // Встроенные методы не инициализируем: источником правды является Syntax Helper.

        index
    }

    /// Получить количество зарегистрированных источников
    pub fn source_count(&self) -> usize {
        self.sources.len()
    }
}

impl Default for SignatureSourceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ==================== Helper Functions ====================

use super::signature_index::{ContextRequirements, MethodSignature, SignatureSource};
use super::types::{FacetKind, ParameterInfo, RawMethodData};

/// Конвертирует RawMethodData в MethodSignature с поддержкой facets
///
/// # Arguments
/// * `method` - Метод из RawTypeData
/// * `owner_type` - Имя типа-владельца
pub fn raw_method_to_signature(method: &RawMethodData, owner_type: &str) -> MethodSignature {
    let params = method
        .params
        .iter()
        .map(|p| ParameterInfo {
            name: p.name.clone(),
            type_name: Some(p.param_type.clone()),
            is_optional: p.is_optional,
            default_value: p.default_value.clone(),
            description: None,
        })
        .collect();

    // Определяем facet и context requirements на основе имени метода и возвращаемого типа
    let (return_facet, context_requirements) = infer_method_metadata(method);

    MethodSignature::new(
        method.name.clone(),
        Some(owner_type.to_string()),
        params,
        if method.return_type.is_empty() {
            None
        } else {
            Some(method.return_type.clone())
        },
        SignatureSource::Platform,
        return_facet,
        context_requirements,
    )
}

pub fn raw_method_to_constructor(
    method: &RawMethodData,
    owner_type: &str,
    owner_raw: &RawTypeData,
) -> super::signature_index::ConstructorSignature {
    let params = method
        .params
        .iter()
        .map(|p| ParameterInfo {
            name: p.name.clone(),
            type_name: Some(p.param_type.clone()),
            is_optional: p.is_optional,
            default_value: p.default_value.clone(),
            description: None,
        })
        .collect();

    let (is_collection, generic_params_count) = match &owner_raw.generic_info {
        Some(info) => (true, info.type_param_count),
        None => {
            let is_collection = owner_raw.collection_item_type.is_some();
            let generic_params_count = if is_collection { 1 } else { 0 };
            (is_collection, generic_params_count)
        }
    };

    super::signature_index::ConstructorSignature {
        type_name: owner_type.to_string(),
        params,
        facet: None,
        source: SignatureSource::Platform,
        is_collection,
        generic_params_count,
    }
}

/// Выводит метаданные метода (facet и context) на основе его сигнатуры
///
/// # Правила inference
/// - Методы создания (СоздатьЭлемент, СоздатьДокумент, СоздатьГруппу) -> Object, ServerOnly
/// - Методы поиска (НайтиПо*, Выбрать) -> Reference/Selection, ServerOnly
/// - Методы записи (Записать, Провести, ОтменитьПроведение) -> void, ServerOnly
/// - Методы конвертации (ПолучитьОбъект) -> Object, ServerOnly
/// - Методы проверки (Пустая, ПустаяСсылка) -> Reference/Boolean, Universal
fn infer_method_metadata(method: &RawMethodData) -> (Option<FacetKind>, ContextRequirements) {
    // Если метаданные уже указаны в method, используем их
    if let Some(facet) = method.return_facet {
        let context = method.context_requirements.unwrap_or_default();
        return (Some(facet), context);
    }

    let method_name = method.name.to_lowercase();
    let return_type = method.return_type.to_lowercase();

    // Методы создания объектов -> Object, ServerOnly
    if method_name.starts_with("создать")
        || method_name.starts_with("create")
        || method_name == "скопировать"
        || method_name == "copy"
    {
        return (Some(FacetKind::Object), ContextRequirements::ServerOnly);
    }

    // Методы поиска -> Reference, ServerOnly
    if method_name.starts_with("найтипо")
        || method_name.starts_with("findby")
        || method_name == "найти"
        || method_name == "find"
    {
        return (Some(FacetKind::Reference), ContextRequirements::ServerOnly);
    }

    // Методы выборки -> Selection, ServerOnly
    if method_name == "выбрать" || method_name == "select" {
        return (Some(FacetKind::Selection), ContextRequirements::ServerOnly);
    }

    // Методы записи -> ServerOnly (без facet - void)
    if method_name == "записать"
        || method_name == "write"
        || method_name == "провести"
        || method_name == "post"
        || method_name == "отменитьпроведение"
        || method_name == "unpost"
        || method_name == "удалить"
        || method_name == "delete"
    {
        return (None, ContextRequirements::ServerOnly);
    }

    // Методы получения объекта -> Object, ServerOnly
    if method_name == "получитьобъект" || method_name == "getobject" {
        return (Some(FacetKind::Object), ContextRequirements::ServerOnly);
    }

    // Методы проверки ссылки -> Reference, Universal
    if method_name == "пустая"
        || method_name == "isempty"
        || method_name == "пустаяссылка"
        || method_name == "emptyref"
    {
        return (Some(FacetKind::Reference), ContextRequirements::Universal);
    }

    // Методы менеджера без явного типа возврата
    if return_type.contains("менеджер") || return_type.contains("manager") {
        return (Some(FacetKind::Manager), ContextRequirements::Universal);
    }

    // По умолчанию - Universal context, без facet
    (None, ContextRequirements::Universal)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Тестовый источник данных
    struct TestSource {
        name: String,
        priority: u32,
        types: Vec<RawTypeData>,
    }

    impl SignatureDataSource for TestSource {
        fn name(&self) -> &str {
            &self.name
        }

        fn priority(&self) -> u32 {
            self.priority
        }

        fn load(&self) -> Vec<RawTypeData> {
            self.types.clone()
        }
    }

    #[test]
    fn test_registry_empty() {
        let registry = SignatureSourceRegistry::new();
        assert_eq!(registry.source_count(), 0);

        let index = registry.build();
        assert!(index.find_constructor("Массив").is_none());
    }

    #[test]
    fn test_registry_single_source() {
        use crate::domain::types::{RawDataSource, RawMethodData, RawParamData};

        let types = vec![RawTypeData {
            name: "Массив".to_string(),
            methods: vec![RawMethodData {
                name: "Добавить".to_string(),
                return_type: "".to_string(),
                params: vec![RawParamData {
                    name: "Значение".to_string(),
                    param_type: "Произвольный".to_string(),
                    is_optional: false,
                    default_value: None,
                }],
                ..Default::default()
            }],
            source: RawDataSource::Platform,
            ..Default::default()
        }];

        let source = TestSource {
            name: "Test".to_string(),
            priority: 10,
            types,
        };

        let index = SignatureSourceRegistry::new().register(source).build();

        // Проверяем что метод добавлен
        let method = index.find_method("Массив", "Добавить");
        assert!(method.is_some());
        assert_eq!(method.unwrap().name, "Добавить");
    }

    #[test]
    fn test_registry_priority_order() {
        use crate::domain::types::{RawDataSource, RawMethodData};

        // Источник с приоритетом 20 (добавится позже)
        let types1 = vec![RawTypeData {
            name: "ТестТип".to_string(),
            methods: vec![RawMethodData {
                name: "Метод1".to_string(),
                return_type: "Строка".to_string(), // С return type
                ..Default::default()
            }],
            source: RawDataSource::Platform,
            ..Default::default()
        }];

        // Источник с приоритетом 10 (добавится раньше)
        let types2 = vec![RawTypeData {
            name: "ТестТип".to_string(),
            methods: vec![RawMethodData {
                name: "Метод1".to_string(),
                return_type: "".to_string(), // Без return type
                ..Default::default()
            }],
            source: RawDataSource::Platform,
            ..Default::default()
        }];

        let source1 = TestSource {
            name: "Later".to_string(),
            priority: 20,
            types: types1,
        };

        let source2 = TestSource {
            name: "Earlier".to_string(),
            priority: 10,
            types: types2,
        };

        // Регистрируем в обратном порядке (source1 первым)
        // но source2 должен обработаться раньше из-за приоритета
        let index = SignatureSourceRegistry::new()
            .register(source1)
            .register(source2)
            .build();

        let method = index.find_method("ТестТип", "Метод1");
        assert!(method.is_some());
        // Должен быть return_type из source1 (приоритет 20),
        // т.к. он обогатил метод из source2 (приоритет 10)
        assert_eq!(method.unwrap().return_type, Some("Строка".to_string()));
    }

    #[test]
    fn test_extract_placeholder_base_type_via_facet_utils() {
        use super::super::facet_utils::extract_placeholder_base_type;

        // Стандартный формат
        assert_eq!(
            extract_placeholder_base_type("СправочникМенеджер.<Имя справочника>"),
            Some("СправочникМенеджер")
        );
        assert_eq!(
            extract_placeholder_base_type("ДокументОбъект.<Имя документа>"),
            Some("ДокументОбъект")
        );

        // HTML-encoded формат
        assert_eq!(
            extract_placeholder_base_type("СправочникМенеджер.&lt;Имя справочника&gt;"),
            Some("СправочникМенеджер")
        );
        assert_eq!(
            extract_placeholder_base_type("ДокументОбъект.&lt;Имя документа&gt;"),
            Some("ДокументОбъект")
        );

        // Не-фасетные типы
        assert_eq!(extract_placeholder_base_type("Массив"), None);
        assert_eq!(extract_placeholder_base_type("СправочникМенеджер"), None);
    }

    #[test]
    fn test_infer_method_metadata_create() {
        let method = RawMethodData {
            name: "СоздатьЭлемент".to_string(),
            return_type: "СправочникОбъект".to_string(),
            ..Default::default()
        };

        let (facet, context) = infer_method_metadata(&method);
        assert_eq!(facet, Some(FacetKind::Object));
        assert_eq!(context, ContextRequirements::ServerOnly);
    }

    #[test]
    fn test_infer_method_metadata_find() {
        let method = RawMethodData {
            name: "НайтиПоКоду".to_string(),
            return_type: "СправочникСсылка".to_string(),
            ..Default::default()
        };

        let (facet, context) = infer_method_metadata(&method);
        assert_eq!(facet, Some(FacetKind::Reference));
        assert_eq!(context, ContextRequirements::ServerOnly);
    }

    #[test]
    fn test_infer_method_metadata_write() {
        let method = RawMethodData {
            name: "Записать".to_string(),
            return_type: "".to_string(),
            ..Default::default()
        };

        let (facet, context) = infer_method_metadata(&method);
        assert_eq!(facet, None);
        assert_eq!(context, ContextRequirements::ServerOnly);
    }
}
