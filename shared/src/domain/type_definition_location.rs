//! Местоположение определения типа для Go To Definition
//!
//! НЕ путать с CodeLocation (контекст выполнения кода)!
//! TypeDefinitionLocation указывает где ОПРЕДЕЛЁН тип, а не где используется.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Местоположение определения типа
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TypeDefinitionLocation {
    /// Платформенный тип — определён в платформе 1С
    /// Нет исходного файла, только ссылка на документацию
    Platform {
        type_name: String,
        /// URI для Syntax Helper документации (опционально)
        docs_uri: Option<String>,
    },

    /// Конфигурационный тип — определён в метаданных конфигурации
    Configuration {
        /// Путь к файлу метаданных (.xml)
        metadata_path: PathBuf,
        /// Пути к модулям (если есть)
        module_paths: ModulePaths,
    },

    /// Пользовательский тип — определён в BSL коде
    UserDefined {
        file_path: PathBuf,
        /// Позиция определения
        start_line: u32,
        start_column: u32,
        end_line: u32,
        end_column: u32,
    },

    /// Примитивный тип — встроен в язык, нет определения
    Primitive,
}

/// Пути к модулям конфигурационного типа
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ModulePaths {
    /// Модуль объекта (Ext/ObjectModule.bsl)
    pub object_module: Option<PathBuf>,
    /// Модуль менеджера (Ext/ManagerModule.bsl)
    pub manager_module: Option<PathBuf>,
    /// Модуль набора записей (Ext/RecordSetModule.bsl)
    pub recordset_module: Option<PathBuf>,
}

impl TypeDefinitionLocation {
    /// Создать location для платформенного типа
    pub fn platform(type_name: &str) -> Self {
        Self::Platform {
            type_name: type_name.to_string(),
            docs_uri: Some(format!("bsl://docs/{}", type_name)),
        }
    }

    /// Создать location для платформенного типа с custom docs URI
    pub fn platform_with_docs(type_name: &str, docs_uri: &str) -> Self {
        Self::Platform {
            type_name: type_name.to_string(),
            docs_uri: Some(docs_uri.to_string()),
        }
    }

    /// Создать location для конфигурационного типа
    pub fn configuration(metadata_path: PathBuf) -> Self {
        Self::Configuration {
            metadata_path,
            module_paths: ModulePaths::default(),
        }
    }

    /// Создать location для конфигурационного типа с путями к модулям
    pub fn configuration_with_modules(metadata_path: PathBuf, module_paths: ModulePaths) -> Self {
        Self::Configuration {
            metadata_path,
            module_paths,
        }
    }

    /// Создать location для пользовательского типа
    pub fn user_defined(
        file_path: PathBuf,
        start_line: u32,
        start_column: u32,
        end_line: u32,
        end_column: u32,
    ) -> Self {
        Self::UserDefined {
            file_path,
            start_line,
            start_column,
            end_line,
            end_column,
        }
    }

    /// Создать location для примитива
    pub fn primitive() -> Self {
        Self::Primitive
    }

    /// Получить основной путь для навигации (если есть)
    pub fn primary_path(&self) -> Option<&PathBuf> {
        match self {
            Self::Configuration {
                module_paths,
                metadata_path,
                ..
            } => {
                // Приоритет: object_module > manager_module > metadata_path
                module_paths
                    .object_module
                    .as_ref()
                    .or(module_paths.manager_module.as_ref())
                    .or(Some(metadata_path))
            }
            Self::UserDefined { file_path, .. } => Some(file_path),
            _ => None,
        }
    }

    /// Проверить, есть ли навигируемый путь
    pub fn is_navigable(&self) -> bool {
        match self {
            Self::Configuration { .. } | Self::UserDefined { .. } => true,
            Self::Platform { docs_uri, .. } => docs_uri.is_some(),
            Self::Primitive => false,
        }
    }

    /// Получить docs URI для платформенного типа
    pub fn docs_uri(&self) -> Option<&str> {
        match self {
            Self::Platform { docs_uri, .. } => docs_uri.as_deref(),
            _ => None,
        }
    }
}

impl ModulePaths {
    /// Создать пустые пути
    pub fn new() -> Self {
        Self::default()
    }

    /// Установить путь к модулю объекта
    pub fn with_object_module(mut self, path: PathBuf) -> Self {
        self.object_module = Some(path);
        self
    }

    /// Установить путь к модулю менеджера
    pub fn with_manager_module(mut self, path: PathBuf) -> Self {
        self.manager_module = Some(path);
        self
    }

    /// Установить путь к модулю набора записей
    pub fn with_recordset_module(mut self, path: PathBuf) -> Self {
        self.recordset_module = Some(path);
        self
    }

    /// Проверить наличие хотя бы одного модуля
    pub fn has_any_module(&self) -> bool {
        self.object_module.is_some()
            || self.manager_module.is_some()
            || self.recordset_module.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_location() {
        let loc = TypeDefinitionLocation::platform("Массив");

        assert!(matches!(loc, TypeDefinitionLocation::Platform { .. }));
        assert_eq!(loc.docs_uri(), Some("bsl://docs/Массив"));
        assert!(!loc.is_navigable() || loc.docs_uri().is_some());
    }

    #[test]
    fn test_configuration_location_primary_path() {
        let metadata_path = PathBuf::from("Catalogs/Контрагенты.xml");
        let object_module = PathBuf::from("Catalogs/Контрагенты/Ext/ObjectModule.bsl");

        let module_paths = ModulePaths::new().with_object_module(object_module.clone());

        let loc =
            TypeDefinitionLocation::configuration_with_modules(metadata_path.clone(), module_paths);

        // primary_path должен вернуть object_module (приоритет выше)
        assert_eq!(loc.primary_path(), Some(&object_module));
        assert!(loc.is_navigable());
    }

    #[test]
    fn test_configuration_location_fallback_to_metadata() {
        let metadata_path = PathBuf::from("Catalogs/Контрагенты.xml");

        let loc = TypeDefinitionLocation::configuration(metadata_path.clone());

        // Без модулей primary_path должен вернуть metadata_path
        assert_eq!(loc.primary_path(), Some(&metadata_path));
    }

    #[test]
    fn test_user_defined_location() {
        let file_path = PathBuf::from("src/module.bsl");
        let loc = TypeDefinitionLocation::user_defined(file_path.clone(), 10, 5, 10, 20);

        assert_eq!(loc.primary_path(), Some(&file_path));
        assert!(loc.is_navigable());
    }

    #[test]
    fn test_primitive_not_navigable() {
        let loc = TypeDefinitionLocation::primitive();

        assert_eq!(loc.primary_path(), None);
        assert!(!loc.is_navigable());
    }

    #[test]
    fn test_module_paths_builder() {
        let paths = ModulePaths::new()
            .with_object_module(PathBuf::from("obj.bsl"))
            .with_manager_module(PathBuf::from("mgr.bsl"));

        assert!(paths.has_any_module());
        assert!(paths.object_module.is_some());
        assert!(paths.manager_module.is_some());
        assert!(paths.recordset_module.is_none());
    }

    #[test]
    fn test_platform_with_custom_docs() {
        let loc =
            TypeDefinitionLocation::platform_with_docs("Массив", "https://docs.1c.ru/array.html");

        assert_eq!(loc.docs_uri(), Some("https://docs.1c.ru/array.html"));
    }

    #[test]
    fn test_configuration_module_priority() {
        let metadata_path = PathBuf::from("meta.xml");
        let object_module = PathBuf::from("obj.bsl");
        let manager_module = PathBuf::from("mgr.bsl");

        // Только manager_module
        let paths1 = ModulePaths::new().with_manager_module(manager_module.clone());
        let loc1 = TypeDefinitionLocation::configuration_with_modules(metadata_path.clone(), paths1);
        assert_eq!(loc1.primary_path(), Some(&manager_module));

        // object_module + manager_module -> object_module имеет приоритет
        let paths2 = ModulePaths::new()
            .with_manager_module(manager_module.clone())
            .with_object_module(object_module.clone());
        let loc2 = TypeDefinitionLocation::configuration_with_modules(metadata_path.clone(), paths2);
        assert_eq!(loc2.primary_path(), Some(&object_module));
    }

    #[test]
    fn test_user_defined_position() {
        let loc = TypeDefinitionLocation::user_defined(
            PathBuf::from("test.bsl"),
            10,
            5,
            15,
            20,
        );

        if let TypeDefinitionLocation::UserDefined {
            file_path,
            start_line,
            start_column,
            end_line,
            end_column,
        } = loc
        {
            assert_eq!(file_path, PathBuf::from("test.bsl"));
            assert_eq!(start_line, 10);
            assert_eq!(start_column, 5);
            assert_eq!(end_line, 15);
            assert_eq!(end_column, 20);
        } else {
            panic!("Expected UserDefined location");
        }
    }
}
