//! Hardcoded platform types for MVP
//! 
//! TODO: Replace this module with proper platform documentation parser
//! This is a temporary solution to bootstrap the type system.
//! See: src/adapters/platform_docs.rs for future implementation

use std::collections::HashMap;
use crate::core::types::{
    Certainty, ConcreteType, PlatformType, Method, Parameter,
    ResolutionResult, ResolutionSource, TypeResolution, ResolutionMetadata,
};

/// Platform global objects that are always available
/// TODO: Generate this from platform documentation
pub fn get_platform_globals() -> HashMap<String, TypeResolution> {
    let mut globals = HashMap::new();
    
    // Справочники (Catalogs manager)
    globals.insert("Справочники".to_string(), create_catalogs_manager());
    globals.insert("Catalogs".to_string(), create_catalogs_manager());
    
    // Документы (Documents manager)
    globals.insert("Документы".to_string(), create_documents_manager());
    globals.insert("Documents".to_string(), create_documents_manager());
    
    // Перечисления (Enums manager)
    globals.insert("Перечисления".to_string(), create_enums_manager());
    globals.insert("Enums".to_string(), create_enums_manager());
    
    // РегистрыСведений (Information registers manager)
    globals.insert("РегистрыСведений".to_string(), create_info_registers_manager());
    globals.insert("InformationRegisters".to_string(), create_info_registers_manager());
    
    // TODO: Add more global objects:
    // - РегистрыНакопления (AccumulationRegisters)
    // - ПланыВидовХарактеристик (ChartsOfCharacteristicTypes)
    // - ПланыСчетов (ChartsOfAccounts)
    // - Константы (Constants)
    // - Отчеты (Reports)
    // - Обработки (DataProcessors)
    
    globals
}

fn create_catalogs_manager() -> TypeResolution {
    // TODO: This is a simplified version, real manager has more methods
    let platform_type = PlatformType {
        name: "CatalogsManager".to_string(),
        methods: vec![
            Method {
                name: "НайтиПоНаименованию".to_string(),
                parameters: vec![
                    Parameter {
                        name: "Наименование".to_string(),
                        type_: Some("Строка".to_string()),
                        optional: false,
                    },
                ],
                return_type: Some("СправочникСсылка".to_string()),
            },
            Method {
                name: "НайтиПоКоду".to_string(),
                parameters: vec![
                    Parameter {
                        name: "Код".to_string(),
                        type_: Some("Строка,Число".to_string()),
                        optional: false,
                    },
                ],
                return_type: Some("СправочникСсылка".to_string()),
            },
        ],
        properties: vec![
            // Properties are dynamic - they're actual catalog names from configuration
            // This will be populated from ConfigParser
        ],
    };
    
    TypeResolution {
        certainty: Certainty::Known,
        result: ResolutionResult::Concrete(ConcreteType::Platform(platform_type)),
        source: ResolutionSource::Static,
        metadata: ResolutionMetadata {
            file: Some("platform:globals".to_string()),
            line: None,
            column: None,
            notes: vec!["Hardcoded platform type - TODO: replace with parser".to_string()],
        },
        active_facet: None, // Platform globals don't have facets
        available_facets: vec![],
    }
}

fn create_documents_manager() -> TypeResolution {
    // TODO: Add actual methods for documents manager
    let platform_type = PlatformType {
        name: "DocumentsManager".to_string(),
        methods: vec![
            Method {
                name: "НайтиПоНомеру".to_string(),
                parameters: vec![
                    Parameter {
                        name: "Номер".to_string(),
                        type_: Some("Строка".to_string()),
                        optional: false,
                    },
                    Parameter {
                        name: "Дата".to_string(),
                        type_: Some("Дата".to_string()),
                        optional: true,
                    },
                ],
                return_type: Some("ДокументСсылка".to_string()),
            },
        ],
        properties: vec![],
    };
    
    TypeResolution {
        certainty: Certainty::Known,
        result: ResolutionResult::Concrete(ConcreteType::Platform(platform_type)),
        source: ResolutionSource::Static,
        metadata: ResolutionMetadata {
            file: Some("platform:globals".to_string()),
            line: None,
            column: None,
            notes: vec!["Hardcoded platform type - TODO: replace with parser".to_string()],
        },
        active_facet: None, // Platform globals don't have facets
        available_facets: vec![],
    }
}

fn create_enums_manager() -> TypeResolution {
    let platform_type = PlatformType {
        name: "EnumsManager".to_string(),
        methods: vec![],
        properties: vec![], // Will be populated from configuration
    };
    
    TypeResolution {
        certainty: Certainty::Known,
        result: ResolutionResult::Concrete(ConcreteType::Platform(platform_type)),
        source: ResolutionSource::Static,
        metadata: ResolutionMetadata {
            file: Some("platform:globals".to_string()),
            line: None,
            column: None,
            notes: vec!["Hardcoded platform type - TODO: replace with parser".to_string()],
        },
        active_facet: None, // Platform globals don't have facets
        available_facets: vec![],
    }
}

fn create_info_registers_manager() -> TypeResolution {
    // TODO: Add actual methods for information registers
    let platform_type = PlatformType {
        name: "InformationRegistersManager".to_string(),
        methods: vec![
            Method {
                name: "СоздатьМенеджерЗаписи".to_string(),
                parameters: vec![],
                return_type: Some("РегистрСведенийМенеджерЗаписи".to_string()),
            },
            Method {
                name: "СоздатьНаборЗаписей".to_string(),
                parameters: vec![],
                return_type: Some("РегистрСведенийНаборЗаписей".to_string()),
            },
        ],
        properties: vec![], // Will be populated from configuration
    };
    
    TypeResolution {
        certainty: Certainty::Known,
        result: ResolutionResult::Concrete(ConcreteType::Platform(platform_type)),
        source: ResolutionSource::Static,
        metadata: ResolutionMetadata {
            file: Some("platform:globals".to_string()),
            line: None,
            column: None,
            notes: vec!["Hardcoded platform type - TODO: replace with parser".to_string()],
        },
        active_facet: None, // Platform globals don't have facets
        available_facets: vec![],
    }
}

/// Basic platform primitive types
/// TODO: Replace with comprehensive type definitions from documentation
pub fn get_primitive_types() -> HashMap<String, TypeResolution> {
    let mut types = HashMap::new();
    
    // Basic types
    for (name, eng_name) in &[
        ("Строка", "String"),
        ("Число", "Number"),
        ("Булево", "Boolean"),
        ("Дата", "Date"),
        ("Неопределено", "Undefined"),
        ("Null", "Null"),
        ("Тип", "Type"),
    ] {
        let platform_type = PlatformType {
            name: eng_name.to_string(),
            methods: vec![],
            properties: vec![],
        };
        
        types.insert(name.to_string(), TypeResolution {
            certainty: Certainty::Known,
            result: ResolutionResult::Concrete(ConcreteType::Platform(platform_type)),
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata {
                file: Some("platform:primitives".to_string()),
                line: None,
                column: None,
                notes: vec!["Hardcoded primitive type".to_string()],
            },
            active_facet: None,
            available_facets: vec![],
        });
    }
    
    types
}

/// Common collection types
/// TODO: Add actual methods and properties from platform documentation
pub fn get_collection_types() -> HashMap<String, TypeResolution> {
    let mut types = HashMap::new();
    
    // Массив (Array)
    let array_type = PlatformType {
        name: "Array".to_string(),
        methods: vec![
            Method {
                name: "Добавить".to_string(),
                parameters: vec![
                    Parameter {
                        name: "Значение".to_string(),
                        type_: None, // Any type
                        optional: false,
                    },
                ],
                return_type: None,
            },
            Method {
                name: "Количество".to_string(),
                parameters: vec![],
                return_type: Some("Число".to_string()),
            },
            Method {
                name: "Очистить".to_string(),
                parameters: vec![],
                return_type: None,
            },
        ],
        properties: vec![],
    };
    
    types.insert("Массив".to_string(), TypeResolution {
        certainty: Certainty::Known,
        result: ResolutionResult::Concrete(ConcreteType::Platform(array_type)),
        source: ResolutionSource::Static,
        metadata: ResolutionMetadata {
            file: Some("platform:collections".to_string()),
            line: None,
            column: None,
            notes: vec!["Hardcoded collection type".to_string()],
        },
        active_facet: None,
        available_facets: vec![],
    });
    
    // TODO: Add more collections:
    // - Структура (Structure)
    // - Соответствие (Map)
    // - СписокЗначений (ValueList)
    // - ТаблицаЗначений (ValueTable)
    
    types
}