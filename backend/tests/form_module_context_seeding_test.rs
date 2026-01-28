//! Интеграционный тест: контекст модуля формы (FormModule) засевается в SymbolTable.
//!
//! Проверяем сценарии из roadmap F5:
//! - `Объект.Работы` → `ДанныеФормыКоллекция<...>`
//! - `Работы` (реквизит формы) → `ДанныеФормыКоллекция<...>`
//! - `Элементы.Работы` → UI-тип таблицы формы

use std::path::PathBuf;
use std::sync::Arc;

use bsl_analysis_v2::AstToIrConverter;
use bsl_backend::data::loaders::config_metadata_parser::ConfigurationDiscovery;
use bsl_backend::parsing::bsl::ast::{Expression, Program, Span as AstSpan, Statement};
use bsl_shared::domain::repository::{InMemoryTypeRepository, TypeRepository};
use bsl_shared::domain::signature_index::SignatureIndex;
use bsl_shared::ir::SemanticNodeKind;

fn test_config_path() -> PathBuf {
    let backend_root = std::env::current_dir().expect("Failed to get current dir");
    let workspace_root = backend_root.parent().expect("Failed to get workspace root");
    workspace_root
        .join("examples")
        .join("conf")
        .join("conf_test")
}

#[test]
fn test_seed_form_module_symbols_into_ir() {
    // 1) Загружаем метаданные документа и генерируем синтетические типы форм
    let discovery = ConfigurationDiscovery::new(test_config_path(), false);
    let configs = discovery
        .discover_all_configurations()
        .expect("Failed to discover configurations");
    let first = &configs[0];

    let metadata = discovery
        .discover_metadata_in_configuration(first, None::<fn(_)>)
        .expect("Failed to discover metadata");

    let doc = metadata
        .iter()
        .find(|m| m.name == "ЗаказНаряды" && m.object_type_raw == "Document")
        .expect("Should find Document.ЗаказНаряды");

    let raw_types = doc.to_raw_type_data_with_forms(None);
    let repo = Arc::new(InMemoryTypeRepository::new());
    repo.load_types(raw_types).expect("Failed to load types");

    // 2) Минимальная программа: читаем переменные/свойства из контекста формы
    let ast = Program {
        statements: vec![
            // a = Работы;
            Statement::Assignment {
                target: Expression::Identifier {
                    name: "a".to_string(),
                    span: AstSpan::stub(),
                },
                value: Expression::Identifier {
                    name: "Работы".to_string(),
                    span: AstSpan::stub(),
                },
                span: AstSpan::stub(),
            },
            // b = Объект.Работы;
            Statement::Assignment {
                target: Expression::Identifier {
                    name: "b".to_string(),
                    span: AstSpan::stub(),
                },
                value: Expression::PropertyAccess {
                    object: Box::new(Expression::Identifier {
                        name: "Объект".to_string(),
                        span: AstSpan::stub(),
                    }),
                    property: "Работы".to_string(),
                    span: AstSpan::stub(),
                },
                span: AstSpan::stub(),
            },
            // c = Элементы.Работы;
            Statement::Assignment {
                target: Expression::Identifier {
                    name: "c".to_string(),
                    span: AstSpan::stub(),
                },
                value: Expression::PropertyAccess {
                    object: Box::new(Expression::Identifier {
                        name: "Элементы".to_string(),
                        span: AstSpan::stub(),
                    }),
                    property: "Работы".to_string(),
                    span: AstSpan::stub(),
                },
                span: AstSpan::stub(),
            },
            // d = Элементы.Номер;
            Statement::Assignment {
                target: Expression::Identifier {
                    name: "d".to_string(),
                    span: AstSpan::stub(),
                },
                value: Expression::PropertyAccess {
                    object: Box::new(Expression::Identifier {
                        name: "Элементы".to_string(),
                        span: AstSpan::stub(),
                    }),
                    property: "Номер".to_string(),
                    span: AstSpan::stub(),
                },
                span: AstSpan::stub(),
            },
            // e = Элементы.Страницы;
            Statement::Assignment {
                target: Expression::Identifier {
                    name: "e".to_string(),
                    span: AstSpan::stub(),
                },
                value: Expression::PropertyAccess {
                    object: Box::new(Expression::Identifier {
                        name: "Элементы".to_string(),
                        span: AstSpan::stub(),
                    }),
                    property: "Страницы".to_string(),
                    span: AstSpan::stub(),
                },
                span: AstSpan::stub(),
            },
            // f = Элементы.ГруппаРаботы;
            Statement::Assignment {
                target: Expression::Identifier {
                    name: "f".to_string(),
                    span: AstSpan::stub(),
                },
                value: Expression::PropertyAccess {
                    object: Box::new(Expression::Identifier {
                        name: "Элементы".to_string(),
                        span: AstSpan::stub(),
                    }),
                    property: "ГруппаРаботы".to_string(),
                    span: AstSpan::stub(),
                },
                span: AstSpan::stub(),
            },
        ],
    };

    // 3) Важно: file_path должен распознаться как FormModule
    let form_module_path = "Documents/ЗаказНаряды/Forms/ФормаДокумента/Ext/Form/Module.bsl";

    let ir = AstToIrConverter::convert(
        ast,
        "a = Работы; b = Объект.Работы; c = Элементы.Работы; d = Элементы.Номер; e = Элементы.Страницы; f = Элементы.ГруппаРаботы;".to_string(),
        form_module_path.to_string(),
        repo as Arc<dyn TypeRepository>,
        SignatureIndex::new(),
    )
    .expect("Failed to convert AST -> IR");

    let mut got = std::collections::HashMap::new();
    for node in &ir.nodes {
        if let SemanticNodeKind::Assignment {
            variable,
            value_type,
            ..
        } = &node.kind
        {
            got.insert(variable.clone(), value_type.type_name().to_string());
        }
    }

    assert_eq!(
        got.get("a").map(|s| s.as_str()),
        Some("ДанныеФормыКоллекция<СтрокаРаботы>"),
        "Ожидаем, что реквизит формы Работы засеивается как ДанныеФормыКоллекция<СтрокаРаботы>"
    );
    assert_eq!(
        got.get("b").map(|s| s.as_str()),
        Some("ДанныеФормыКоллекция<СтрокаРаботы>"),
        "Ожидаем, что Объект.Работы резолвится как ДанныеФормыКоллекция<СтрокаРаботы>"
    );
    assert_eq!(
        got.get("c").map(|s| s.as_str()),
        Some("ТаблицаФормы"),
        "Ожидаем, что Элементы.Работы резолвится как UI-тип таблицы формы"
    );
    assert_eq!(
        got.get("d").map(|s| s.as_str()),
        Some("ПолеФормы"),
        "Ожидаем, что Элементы.Номер резолвится как UI-тип поля формы"
    );
    assert_eq!(
        got.get("e").map(|s| s.as_str()),
        Some("ГруппаФормы"),
        "Ожидаем, что Элементы.Страницы резолвится как UI-тип группы формы"
    );
    assert_eq!(
        got.get("f").map(|s| s.as_str()),
        Some("ГруппаФормы"),
        "Ожидаем, что Элементы.ГруппаРаботы резолвится как UI-тип группы формы"
    );
}
