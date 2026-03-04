use super::*;

pub trait TypeRepository: Send + Sync {
    /// Установить флаг загрузки документации платформы (Syntax Helper)
    fn set_platform_docs_loaded(&self, loaded: bool);

    /// Проверить, загружена ли документация платформы (Syntax Helper)
    fn platform_docs_loaded(&self) -> bool;

    /// Загрузить типы в репозиторий
    fn load_types(&self, types: Vec<RawTypeData>) -> Result<()>;

    /// Обновить или добавить типы в репозиторий (по имени типа)
    fn upsert_types(&self, types: Vec<RawTypeData>) -> Result<()>;

    /// Удалить типы из репозитория по именам (регистронезависимо)
    fn remove_types(&self, type_names: &[String]) -> Result<usize>;

    /// Получить все типы из репозитория
    fn get_all_types(&self) -> Vec<RawTypeData>;

    /// Найти тип по имени
    fn find_type(&self, name: &str) -> Option<RawTypeData>;

    /// Получить статистику
    fn get_stats(&self) -> RepositoryStats;

    /// Валидировать сигнатуру метода
    ///
    /// Проверяет соответствие типов параметров и возвращаемого значения
    fn validate_method_signature(
        &self,
        owner_type: &str,
        method_name: &str,
        actual_params: &[ParameterInfo],
        actual_return_type: Option<&str>,
    ) -> SignatureValidationResult;

    /// Найти сигнатуру метода (для SignatureHelp)
    ///
    /// Ищет сигнатуру метода в индексе платформенных и конфигурационных типов
    fn find_method_signature(
        &self,
        owner_type: Option<&str>,
        method_name: &str,
    ) -> Option<MethodSignature>;

    /// Найти конструктор для указанного типа
    ///
    /// # Arguments
    /// * `type_name` - Имя типа (регистронезависимо)
    ///
    /// # Returns
    /// * `Some(ConstructorSignature)` - если конструктор найден
    /// * `None` - если конструктор не найден или произошла ошибка
    fn find_constructor(&self, type_name: &str) -> Option<ConstructorSignature>;

    /// Получить клон SignatureIndex для валидации (Milestone 3.10)
    ///
    /// Возвращает клон индекса сигнатур для использования в валидаторах
    fn get_signature_index_clone(&self) -> SignatureIndex;

    /// Добавить конфигурационный метод в SignatureIndex (экспорт из модулей конфигурации)
    ///
    /// Используется после загрузки конфигурации, чтобы методы из `*.bsl` модулей
    /// участвовали в hover/validation/signatureHelp.
    fn add_config_method_signature(&self, owner_type: &str, method: MethodSignature);

    /// Добавить глобальную функцию в SignatureIndex (например, из global common module)
    fn add_global_function_signature(&self, function_name: &str, method: MethodSignature);

    /// Удалить конфигурационные методы по именам (регистронезависимо)
    fn remove_config_method_signatures(&self, owner_type: &str, method_names: &[String]);

    /// Удалить глобальные функции по именам (регистронезависимо)
    fn remove_global_function_signatures(&self, function_names: &[String]);

    /// Добавить location определения конфигурационного метода (для Go To Definition на метод)
    fn add_config_method_definition_location(
        &self,
        owner_type: &str,
        method_name: &str,
        location: TypeDefinitionLocation,
    );

    /// Добавить location определения глобальной функции (например, из global common module)
    fn add_global_function_definition_location(
        &self,
        function_name: &str,
        location: TypeDefinitionLocation,
    );

    /// Удалить locations для конфигурационных методов по именам (регистронезависимо)
    fn remove_config_method_definition_locations(&self, owner_type: &str, method_names: &[String]);

    /// Удалить locations для глобальных функций по именам (регистронезависимо)
    fn remove_global_function_definition_locations(&self, function_names: &[String]);

    /// Найти location определения метода/функции (case-insensitive)
    ///
    /// `owner_type=None` означает глобальную функцию.
    fn find_method_definition_location(
        &self,
        owner_type: Option<&str>,
        method_name: &str,
    ) -> Option<TypeDefinitionLocation>;

    /// Получить все locations определений методов/функций (для переноса в snapshot’ы).
    ///
    /// Формат:
    /// - `(Some(owner_type), method_name, location)` для методов конфигурации
    /// - `(None, function_name, location)` для глобальных функций
    ///
    /// Важно: возвращает **клон** данных.
    fn get_method_definition_locations_clone(
        &self,
    ) -> Vec<(Option<String>, String, TypeDefinitionLocation)>;

    /// Получить все объекты метаданных указанного вида (Milestone 3.16)
    ///
    /// Возвращает имена объектов без префикса (например, "Контрагенты" вместо "Справочники.Контрагенты")
    ///
    /// # Параметры
    ///
    /// * `kind` - вид метаданных (Catalog, Document, etc.)
    ///
    /// # Возвращает
    ///
    /// Вектор имён объектов метаданных
    ///
    /// # Примеры
    ///
    /// ```rust,no_run
    /// # use bsl_repository::TypeRepository;
    /// # use bsl_types::types::MetadataKind;
    /// # let repository: &dyn TypeRepository = todo!();
    /// let catalogs = repository.get_metadata_objects_by_kind(MetadataKind::Catalog);
    /// // → ["Контрагенты", "Номенклатура", "Склады", ...]
    /// # let _ = catalogs;
    /// ```
    fn get_metadata_objects_by_kind(&self, kind: MetadataKind) -> Vec<String>;

    /// Найти метод по имени типа и имени метода из signature_index
    ///
    /// Использует обогащённые сигнатуры из signature_index, где return_type корректный.
    /// Поддерживает фасетные типы с fallback к базовому типу.
    ///
    /// # Параметры
    ///
    /// * `owner_type` - имя типа-владельца (например, "СправочникМенеджер.Контрагенты")
    /// * `method_name` - имя метода
    ///
    /// # Возвращает
    ///
    /// `Some(MethodSignature)` если метод найден, иначе `None`
    fn find_method(&self, owner_type: Option<&str>, method_name: &str) -> Option<MethodSignature>;

    /// Получить все методы для указанного типа из signature_index
    ///
    /// Возвращает методы с обогащёнными сигнатурами (включая return_type).
    /// Поддерживает фасетные типы с fallback к базовому типу.
    ///
    /// # Параметры
    ///
    /// * `type_name` - имя типа (например, "СправочникМенеджер" или "СправочникМенеджер.Контрагенты")
    ///
    /// # Возвращает
    ///
    /// Вектор сигнатур методов (клонированных)
    fn get_methods_from_signature_index(&self, type_name: &str) -> Vec<MethodSignature>;

    /// Установить GenericInfo для типа (Milestone 3.x: Унификация источников данных)
    ///
    /// Применяет GenericInfo к существующему типу в репозитории.
    /// Используется для добавления inference metadata к типам из syntax_helper.
    ///
    /// # Параметры
    ///
    /// * `type_name` - имя типа (например, "Массив", "Соответствие")
    /// * `generic_info` - метаданные для Generic inference
    ///
    /// # Возвращает
    ///
    /// `true` если тип найден и GenericInfo установлен, `false` если тип не найден
    fn set_generic_info(&self, type_name: &str, generic_info: GenericInfo) -> bool;
}
