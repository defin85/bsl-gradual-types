//! Code Location — определение контекста выполнения по пути к файлу
//!
//! Этот модуль предоставляет систему определения типа модуля и контекста выполнения
//! на основе пути к файлу в конфигурации 1С.
//!
//! # Типы модулей
//!
//! - **CommonModule** - общий модуль с настраиваемыми контекстами выполнения
//! - **ObjectModule** - модуль объекта метаданных (всегда серверный)
//! - **ManagerModule** - модуль менеджера (всегда серверный)
//! - **FormModule** - модуль формы (контекст зависит от директивы)
//! - **RecordSetModule** - модуль набора записей (всегда серверный)
//!
//! # Examples
//!
//! ```
//! use std::path::Path;
//! use bsl_shared::domain::code_location::CodeLocation;
//!
//! let path = Path::new("Catalogs/Контрагенты/Ext/ObjectModule.bsl");
//! let location = CodeLocation::determine_from_path(path).unwrap();
//! assert!(location.can_call_database_methods(None));
//! ```

use std::path::{Path, PathBuf};
use anyhow::{anyhow, Result};

/// Контекст выполнения кода в 1С (из types.rs)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ExecutionContext {
    /// Серверный контекст (выполняется на сервере)
    Server,
    /// Контекст управляемого клиентского приложения
    ClientManaged,
    /// Контекст обычного клиентского приложения
    ClientOrdinary,
    /// Контекст внешнего соединения
    ExternalConnection,
}

/// Директива компилятора для управления контекстом выполнения
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CompilerDirective {
    /// &НаСервере - выполняется на сервере
    OnServer,
    /// &НаСервереБезКонтекста - на сервере без контекста формы
    OnServerNoContext,
    /// &НаКлиенте - выполняется на клиенте
    OnClient,
    /// &НаКлиентеНаСервереБезКонтекста - универсальный код
    OnClientOnServerNoContext,
    /// Нет директивы
    Unknown,
}

/// Расположение кода в конфигурации 1С
///
/// Определяет тип модуля и контекст выполнения на основе пути к файлу.
#[derive(Debug, Clone)]
pub struct CodeLocation {
    /// Путь к файлу модуля
    pub file_path: PathBuf,
    /// Тип модуля
    pub module_type: ModuleType,
    /// Контекст метаданных (если применимо)
    pub metadata_context: Option<MetadataContext>,
}

/// Тип модуля в конфигурации 1С
#[derive(Debug, Clone, PartialEq)]
pub enum ModuleType {
    /// Общий модуль с настраиваемыми контекстами
    CommonModule {
        /// Имя модуля
        name: String,
        /// Доступные контексты выполнения
        contexts: Vec<ExecutionContext>,
    },
    /// Модуль объекта метаданных (всегда серверный)
    ObjectModule {
        /// Тип владельца (например, "Catalog.Контрагенты")
        owner_type: String,
    },
    /// Модуль менеджера (всегда серверный)
    ManagerModule {
        /// Тип владельца
        owner_type: String,
    },
    /// Модуль формы (контекст зависит от директивы)
    FormModule {
        /// Имя формы
        form_name: String,
        /// Тип владельца
        owner_type: String,
    },
    /// Модуль набора записей (всегда серверный)
    RecordSetModule {
        /// Тип владельца
        owner_type: String,
    },
    /// Неизвестный тип модуля
    Unknown,
}

/// Контекст метаданных для модуля
#[derive(Debug, Clone, PartialEq)]
pub struct MetadataContext {
    /// Полный тип объекта (например, "Catalog.Контрагенты")
    pub object_type: String,
    /// Имя объекта (например, "Контрагенты")
    pub object_name: String,
}

impl CodeLocation {
    /// Определить CodeLocation по пути к файлу
    ///
    /// # Паттерны путей:
    ///
    /// - `CommonModules/ИмяМодуля/Ext/Module.bsl` → CommonModule
    /// - `Catalogs/ИмяОбъекта/Ext/ObjectModule.bsl` → ObjectModule
    /// - `Catalogs/ИмяОбъекта/Ext/ManagerModule.bsl` → ManagerModule
    /// - `Documents/ИмяОбъекта/Forms/ИмяФормы/Ext/Module.bsl` → FormModule
    /// - `InformationRegisters/ИмяРегистра/Ext/RecordSetModule.bsl` → RecordSetModule
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::Path;
    /// use bsl_shared::domain::code_location::CodeLocation;
    ///
    /// let path = Path::new("Catalogs/Контрагенты/Ext/ObjectModule.bsl");
    /// let loc = CodeLocation::determine_from_path(path).unwrap();
    /// ```
    pub fn determine_from_path(path: &Path) -> Result<Self> {
        let path_str = path.to_string_lossy();

        // CommonModule: CommonModules/ИмяМодуля/Ext/Module.bsl
        if path_str.contains("CommonModules") && path_str.ends_with("Module.bsl") {
            return Self::parse_common_module_path(path);
        }

        // ObjectModule: */Ext/ObjectModule.bsl
        if path_str.ends_with("ObjectModule.bsl") {
            return Self::parse_object_module_path(path);
        }

        // ManagerModule: */Ext/ManagerModule.bsl
        if path_str.ends_with("ManagerModule.bsl") {
            return Self::parse_manager_module_path(path);
        }

        // FormModule: */Forms/*/Ext/Module.bsl
        if path_str.contains("Forms") && path_str.ends_with("Module.bsl") {
            return Self::parse_form_module_path(path);
        }

        // RecordSetModule: */Ext/RecordSetModule.bsl
        if path_str.ends_with("RecordSetModule.bsl") {
            return Self::parse_record_set_module_path(path);
        }

        // Неизвестный тип модуля
        Ok(Self {
            file_path: path.to_path_buf(),
            module_type: ModuleType::Unknown,
            metadata_context: None,
        })
    }

    /// Парсинг пути к общему модулю
    ///
    /// Путь: `CommonModules/ИмяМодуля/Ext/Module.bsl`
    fn parse_common_module_path(path: &Path) -> Result<Self> {
        let components: Vec<_> = path.components().collect();

        // Извлекаем имя модуля (компонент перед "Ext")
        let module_name = components
            .iter()
            .rev()
            .nth(2) // Пропускаем "Ext" и "Module.bsl"
            .and_then(|c| c.as_os_str().to_str())
            .ok_or_else(|| anyhow!("Cannot extract module name from path: {:?}", path))?;

        Ok(Self {
            file_path: path.to_path_buf(),
            module_type: ModuleType::CommonModule {
                name: module_name.to_string(),
                contexts: Vec::new(), // Заполняется из метаданных конфигурации
            },
            metadata_context: None,
        })
    }

    /// Парсинг пути к модулю объекта
    ///
    /// Путь: `Catalogs/Контрагенты/Ext/ObjectModule.bsl`
    fn parse_object_module_path(path: &Path) -> Result<Self> {
        let (object_type, object_name) = Self::extract_metadata_info(path)?;

        Ok(Self {
            file_path: path.to_path_buf(),
            module_type: ModuleType::ObjectModule {
                owner_type: format!("{}.{}", object_type, object_name),
            },
            metadata_context: Some(MetadataContext {
                object_type: format!("{}.{}", object_type, object_name),
                object_name,
            }),
        })
    }

    /// Парсинг пути к модулю менеджера
    ///
    /// Путь: `Catalogs/Контрагенты/Ext/ManagerModule.bsl`
    fn parse_manager_module_path(path: &Path) -> Result<Self> {
        let (object_type, object_name) = Self::extract_metadata_info(path)?;

        Ok(Self {
            file_path: path.to_path_buf(),
            module_type: ModuleType::ManagerModule {
                owner_type: format!("{}.{}", object_type, object_name),
            },
            metadata_context: Some(MetadataContext {
                object_type: format!("{}.{}", object_type, object_name),
                object_name,
            }),
        })
    }

    /// Парсинг пути к модулю формы
    ///
    /// Путь: `Catalogs/Контрагенты/Forms/ФормаЭлемента/Ext/Module.bsl`
    fn parse_form_module_path(path: &Path) -> Result<Self> {
        let components: Vec<_> = path.components().collect();

        // Извлекаем имя формы (компонент перед "Ext")
        let form_name = components
            .iter()
            .rev()
            .nth(2) // Пропускаем "Ext" и "Module.bsl"
            .and_then(|c| c.as_os_str().to_str())
            .ok_or_else(|| anyhow!("Cannot extract form name from path: {:?}", path))?;

        // Извлекаем тип объекта и имя (до "Forms")
        let (object_type, object_name) = Self::extract_metadata_info_before_forms(path)?;

        Ok(Self {
            file_path: path.to_path_buf(),
            module_type: ModuleType::FormModule {
                form_name: form_name.to_string(),
                owner_type: format!("{}.{}", object_type, object_name),
            },
            metadata_context: Some(MetadataContext {
                object_type: format!("{}.{}", object_type, object_name),
                object_name,
            }),
        })
    }

    /// Парсинг пути к модулю набора записей
    ///
    /// Путь: `InformationRegisters/РегистрСведений/Ext/RecordSetModule.bsl`
    fn parse_record_set_module_path(path: &Path) -> Result<Self> {
        let (object_type, object_name) = Self::extract_metadata_info(path)?;

        Ok(Self {
            file_path: path.to_path_buf(),
            module_type: ModuleType::RecordSetModule {
                owner_type: format!("{}.{}", object_type, object_name),
            },
            metadata_context: Some(MetadataContext {
                object_type: format!("{}.{}", object_type, object_name),
                object_name,
            }),
        })
    }

    /// Извлечь информацию о типе объекта и имени из пути
    ///
    /// Путь: `Catalogs/Контрагенты/Ext/...`
    /// Возвращает: ("Catalog", "Контрагенты")
    fn extract_metadata_info(path: &Path) -> Result<(String, String)> {
        let components: Vec<_> = path.components().collect();

        // Ищем компонент "Ext" и берём два компонента перед ним
        let ext_index = components
            .iter()
            .position(|c| c.as_os_str().to_str() == Some("Ext"))
            .ok_or_else(|| anyhow!("Cannot find 'Ext' in path: {:?}", path))?;

        if ext_index < 2 {
            return Err(anyhow!("Invalid path structure: {:?}", path));
        }

        let object_name = components[ext_index - 1]
            .as_os_str()
            .to_str()
            .ok_or_else(|| anyhow!("Invalid object name in path: {:?}", path))?;

        let object_type_plural = components[ext_index - 2]
            .as_os_str()
            .to_str()
            .ok_or_else(|| anyhow!("Invalid object type in path: {:?}", path))?;

        // Конвертируем множественное число в единственное
        // "Catalogs" -> "Catalog", "Documents" -> "Document"
        let object_type = object_type_plural.trim_end_matches('s');

        Ok((object_type.to_string(), object_name.to_string()))
    }

    /// Извлечь информацию о типе объекта и имени из пути с формами
    ///
    /// Путь: `Catalogs/Контрагенты/Forms/ФормаЭлемента/...`
    /// Возвращает: ("Catalog", "Контрагенты")
    fn extract_metadata_info_before_forms(path: &Path) -> Result<(String, String)> {
        let components: Vec<_> = path.components().collect();

        // Ищем компонент "Forms"
        let forms_index = components
            .iter()
            .position(|c| c.as_os_str().to_str() == Some("Forms"))
            .ok_or_else(|| anyhow!("Cannot find 'Forms' in path: {:?}", path))?;

        if forms_index < 2 {
            return Err(anyhow!("Invalid path structure before Forms: {:?}", path));
        }

        let object_name = components[forms_index - 1]
            .as_os_str()
            .to_str()
            .ok_or_else(|| anyhow!("Invalid object name in path: {:?}", path))?;

        let object_type_plural = components[forms_index - 2]
            .as_os_str()
            .to_str()
            .ok_or_else(|| anyhow!("Invalid object type in path: {:?}", path))?;

        let object_type = object_type_plural.trim_end_matches('s');

        Ok((object_type.to_string(), object_name.to_string()))
    }

    /// Проверить, можно ли вызывать методы БД в текущем контексте
    ///
    /// # Правила:
    ///
    /// - **ObjectModule, ManagerModule, RecordSetModule**: Всегда `true` (серверные модули)
    /// - **CommonModule**: Зависит от свойств модуля (Server context)
    /// - **FormModule**: Зависит от директивы компилятора (&НаСервере)
    /// - **Unknown**: `false`
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::Path;
    /// use bsl_shared::domain::code_location::{CodeLocation, CompilerDirective};
    ///
    /// let path = Path::new("Catalogs/Контрагенты/Ext/ObjectModule.bsl");
    /// let loc = CodeLocation::determine_from_path(path).unwrap();
    /// assert!(loc.can_call_database_methods(None));
    /// ```
    pub fn can_call_database_methods(&self, directive: Option<&CompilerDirective>) -> bool {
        match &self.module_type {
            // Серверные модули объектов - всегда имеют доступ к БД
            ModuleType::ObjectModule { .. }
            | ModuleType::ManagerModule { .. }
            | ModuleType::RecordSetModule { .. } => true,

            // Общий модуль - проверяем контексты
            ModuleType::CommonModule { contexts, .. } => {
                contexts.contains(&ExecutionContext::Server)
            }

            // Модуль формы - зависит от директивы
            ModuleType::FormModule { .. } => {
                matches!(directive, Some(CompilerDirective::OnServer))
            }

            // Неизвестный тип - нет доступа
            ModuleType::Unknown => false,
        }
    }

    /// Получить имя модуля (если доступно)
    pub fn get_module_name(&self) -> Option<&str> {
        match &self.module_type {
            ModuleType::CommonModule { name, .. } => Some(name),
            ModuleType::FormModule { form_name, .. } => Some(form_name),
            _ => None,
        }
    }

    /// Получить тип владельца модуля (если доступно)
    pub fn get_owner_type(&self) -> Option<&str> {
        match &self.module_type {
            ModuleType::ObjectModule { owner_type }
            | ModuleType::ManagerModule { owner_type }
            | ModuleType::FormModule { owner_type, .. }
            | ModuleType::RecordSetModule { owner_type } => Some(owner_type),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_determine_common_module_location() {
        let path = PathBuf::from("CommonModules/ОбщийМодуль1/Ext/Module.bsl");
        let loc = CodeLocation::determine_from_path(&path).unwrap();

        match loc.module_type {
            ModuleType::CommonModule { ref name, .. } => {
                assert_eq!(name, "ОбщийМодуль1");
            }
            _ => panic!("Expected CommonModule, got {:?}", loc.module_type),
        }

        assert!(loc.metadata_context.is_none());
    }

    #[test]
    fn test_determine_object_module_location() {
        let path = PathBuf::from("Catalogs/Контрагенты/Ext/ObjectModule.bsl");
        let loc = CodeLocation::determine_from_path(&path).unwrap();

        match loc.module_type {
            ModuleType::ObjectModule { ref owner_type } => {
                assert_eq!(owner_type, "Catalog.Контрагенты");
            }
            _ => panic!("Expected ObjectModule, got {:?}", loc.module_type),
        }

        assert!(loc.metadata_context.is_some());
        let ctx = loc.metadata_context.unwrap();
        assert_eq!(ctx.object_name, "Контрагенты");
    }

    #[test]
    fn test_determine_manager_module_location() {
        let path = PathBuf::from("Documents/ЗаказНаряд/Ext/ManagerModule.bsl");
        let loc = CodeLocation::determine_from_path(&path).unwrap();

        match loc.module_type {
            ModuleType::ManagerModule { ref owner_type } => {
                assert_eq!(owner_type, "Document.ЗаказНаряд");
            }
            _ => panic!("Expected ManagerModule, got {:?}", loc.module_type),
        }
    }

    #[test]
    fn test_determine_form_module_location() {
        let path = PathBuf::from("Catalogs/Контрагенты/Forms/ФормаЭлемента/Ext/Module.bsl");
        let loc = CodeLocation::determine_from_path(&path).unwrap();

        match loc.module_type {
            ModuleType::FormModule {
                ref form_name,
                ref owner_type,
            } => {
                assert_eq!(form_name, "ФормаЭлемента");
                assert_eq!(owner_type, "Catalog.Контрагенты");
            }
            _ => panic!("Expected FormModule, got {:?}", loc.module_type),
        }
    }

    #[test]
    fn test_determine_record_set_module_location() {
        let path = PathBuf::from("InformationRegisters/РегистрСведений/Ext/RecordSetModule.bsl");
        let loc = CodeLocation::determine_from_path(&path).unwrap();

        match loc.module_type {
            ModuleType::RecordSetModule { ref owner_type } => {
                assert_eq!(owner_type, "InformationRegister.РегистрСведений");
            }
            _ => panic!("Expected RecordSetModule, got {:?}", loc.module_type),
        }
    }

    #[test]
    fn test_can_call_database_methods_object_module() {
        let path = PathBuf::from("Catalogs/Контрагенты/Ext/ObjectModule.bsl");
        let loc = CodeLocation::determine_from_path(&path).unwrap();

        // ObjectModule всегда имеет доступ к БД
        assert!(loc.can_call_database_methods(None));
    }

    #[test]
    fn test_can_call_database_methods_form_module() {
        let path = PathBuf::from("Catalogs/Контрагенты/Forms/ФормаЭлемента/Ext/Module.bsl");
        let loc = CodeLocation::determine_from_path(&path).unwrap();

        // FormModule без директивы - нет доступа
        assert!(!loc.can_call_database_methods(None));

        // FormModule с &НаСервере - есть доступ
        assert!(loc.can_call_database_methods(Some(&CompilerDirective::OnServer)));

        // FormModule с &НаКлиенте - нет доступа
        assert!(!loc.can_call_database_methods(Some(&CompilerDirective::OnClient)));
    }

    #[test]
    fn test_unknown_module_type() {
        let path = PathBuf::from("SomeUnknown/Path/File.bsl");
        let loc = CodeLocation::determine_from_path(&path).unwrap();

        assert!(matches!(loc.module_type, ModuleType::Unknown));
        assert!(!loc.can_call_database_methods(None));
    }

    #[test]
    fn test_get_module_name() {
        let path = PathBuf::from("CommonModules/ОбщийМодуль1/Ext/Module.bsl");
        let loc = CodeLocation::determine_from_path(&path).unwrap();
        assert_eq!(loc.get_module_name(), Some("ОбщийМодуль1"));
    }

    #[test]
    fn test_get_owner_type() {
        let path = PathBuf::from("Catalogs/Контрагенты/Ext/ObjectModule.bsl");
        let loc = CodeLocation::determine_from_path(&path).unwrap();
        assert_eq!(loc.get_owner_type(), Some("Catalog.Контрагенты"));
    }
}
