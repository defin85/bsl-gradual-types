use super::*;

#[cfg(test)]
mod comparison_notes {
    //! Сравнение архитектур парсинга
    //!
    //! Old Complex (UnifiedParserCoordinator):
    //! - Strategy pattern с 3+ парсерами
    //! - TreeSitterStrategy + SyntaxHelperStrategy + RegexFallback
    //! - Parser selection logic
    //! - ~300+ LOC
    //!
    //! Current (ParserCoordinator после Milestone 2.8):
    //! - Только Tree-sitter (regex legacy удалён)
    //! - Инкрементальный парсинг + кэширование AST/Tree
    //! - ~200 LOC
    //!
    //! Результат: Упрощение архитектуры + качество анализа
}

#[cfg(test)]
mod symbol_index_tests {
    use super::*;
    use crate::parsing::bsl::ast::{Expression, Program, Statement};
    use bsl_shared::ir::Span;

    fn has_symbol(items: &[IndexItem], name: &str, kind: SymbolKind, scope: SymbolScope) -> bool {
        items.iter().any(|item| {
            item.name == name
                && item.kind == IndexItemKind::Symbol(kind)
                && item.scope == Some(scope)
        })
    }

    #[test]
    fn collect_symbol_items_from_program() {
        let span = Span::new(1, 2);
        let expr = Expression::Number { value: 1.0, span };
        let program = Program {
            statements: vec![
                Statement::VarDeclaration {
                    name: "x".to_string(),
                    type_hint: None,
                    span,
                },
                Statement::Assignment {
                    target: Expression::Identifier {
                        name: "assigned_top".to_string(),
                        span,
                    },
                    value: expr.clone(),
                    span,
                },
                Statement::FunctionDecl {
                    name: "Func".to_string(),
                    params: vec!["a".to_string(), "b".to_string()],
                    body: vec![
                        Statement::VarDeclaration {
                            name: "y".to_string(),
                            type_hint: None,
                            span,
                        },
                        Statement::Assignment {
                            target: Expression::Identifier {
                                name: "assigned_local".to_string(),
                                span,
                            },
                            value: expr.clone(),
                            span,
                        },
                        Statement::Assignment {
                            target: Expression::PropertyAccess {
                                object: Box::new(Expression::Identifier {
                                    name: "obj".to_string(),
                                    span,
                                }),
                                property: "field".to_string(),
                                span,
                            },
                            value: expr.clone(),
                            span,
                        },
                        Statement::For {
                            variable: "i".to_string(),
                            start: expr.clone(),
                            end: expr.clone(),
                            body: Vec::new(),
                            span,
                        },
                    ],
                    compiler_directive: None,
                    is_export: false,
                    span,
                },
                Statement::ProcedureDecl {
                    name: "Proc".to_string(),
                    params: vec!["p".to_string()],
                    body: vec![Statement::ForEach {
                        variable: "item".to_string(),
                        collection: expr.clone(),
                        body: Vec::new(),
                        span,
                    }],
                    compiler_directive: None,
                    is_export: true,
                    span,
                },
                Statement::If {
                    condition: expr.clone(),
                    then_body: vec![Statement::VarDeclaration {
                        name: "z".to_string(),
                        type_hint: None,
                        span,
                    }],
                    else_body: Some(vec![Statement::VarDeclaration {
                        name: "w".to_string(),
                        type_hint: None,
                        span,
                    }]),
                    span,
                },
            ],
        };

        let items = collect_symbol_items(&program, "file:///test.bsl");

        assert!(has_symbol(
            &items,
            "x",
            SymbolKind::Variable,
            SymbolScope::Module
        ));
        assert!(has_symbol(
            &items,
            "assigned_top",
            SymbolKind::Variable,
            SymbolScope::Module
        ));
        assert!(has_symbol(
            &items,
            "Func",
            SymbolKind::Function,
            SymbolScope::Module
        ));
        assert!(has_symbol(
            &items,
            "a",
            SymbolKind::Parameter,
            SymbolScope::Local
        ));
        assert!(has_symbol(
            &items,
            "b",
            SymbolKind::Parameter,
            SymbolScope::Local
        ));
        assert!(has_symbol(
            &items,
            "y",
            SymbolKind::Variable,
            SymbolScope::Local
        ));
        assert!(has_symbol(
            &items,
            "assigned_local",
            SymbolKind::Variable,
            SymbolScope::Local
        ));
        assert!(!has_symbol(
            &items,
            "field",
            SymbolKind::Variable,
            SymbolScope::Local
        ));
        assert!(has_symbol(
            &items,
            "i",
            SymbolKind::Variable,
            SymbolScope::Local
        ));
        assert!(has_symbol(
            &items,
            "Proc",
            SymbolKind::Procedure,
            SymbolScope::Module
        ));
        assert!(has_symbol(
            &items,
            "p",
            SymbolKind::Parameter,
            SymbolScope::Local
        ));
        assert!(has_symbol(
            &items,
            "item",
            SymbolKind::Variable,
            SymbolScope::Local
        ));
        assert!(has_symbol(
            &items,
            "z",
            SymbolKind::Variable,
            SymbolScope::Local
        ));
        assert!(has_symbol(
            &items,
            "w",
            SymbolKind::Variable,
            SymbolScope::Local
        ));
    }

    #[test]
    fn update_symbol_index_on_parse() {
        let parser = ParserCoordinator::with_fallback();
        let index = Arc::new(IntellisenseIndexStore::new("cfg", "platform"));
        parser.set_intellisense_index(index.clone());

        let code = r#"Перем x;
Процедура Test(p)
    Неявная = 1;
    Перем y;
КонецПроцедуры"#;
        let file_path = "test.bsl";

        let result = parser.parse_with_cache_for_file(code, file_path);
        assert!(result.is_ok());

        let uri = path_to_uri(Path::new(file_path));
        let snapshot = index.snapshot();
        let items = snapshot
            .symbol_index
            .get(&uri)
            .expect("symbols missing")
            .as_ref();

        assert!(has_symbol(
            items,
            "x",
            SymbolKind::Variable,
            SymbolScope::Module
        ));
        assert!(has_symbol(
            items,
            "Test",
            SymbolKind::Procedure,
            SymbolScope::Module
        ));
        assert!(has_symbol(
            items,
            "p",
            SymbolKind::Parameter,
            SymbolScope::Local
        ));
        assert!(has_symbol(
            items,
            "Неявная",
            SymbolKind::Variable,
            SymbolScope::Local
        ));
        assert!(has_symbol(
            items,
            "y",
            SymbolKind::Variable,
            SymbolScope::Local
        ));
    }
}

#[cfg(test)]
mod parse_snapshot_tests {
    use super::*;

    #[test]
    fn incremental_snapshot_matches_full_parse_result() {
        let parser = ParserCoordinator::with_fallback();
        let file_path = PathBuf::from("snapshot-parity.bsl");
        let base = "Процедура Тест()\n    x = 1;\nКонецПроцедуры".to_string();
        let updated = "Процедура Тест()\n    x = 2;\nКонецПроцедуры".to_string();

        let seed = parser
            .parse_incremental_with_report(file_path.clone(), base, Vec::new())
            .expect("seed snapshot");
        assert!(!seed.incremental);

        let report = parser
            .parse_incremental_with_report(
                file_path,
                updated.clone(),
                vec![TextEdit {
                    start_line: 1,
                    start_utf16_column: 0,
                    old_end_line: 1,
                    old_end_utf16_column: 10,
                    new_text: "    x = 2;".to_string(),
                }],
            )
            .expect("incremental report");

        assert!(report.incremental);
        assert!(report.fallback_reason.is_none());
        assert!(!report.changed_ranges.is_empty());

        let full = ParserCoordinator::with_fallback()
            .parse(&updated)
            .expect("full parse");
        let incremental_json =
            serde_json::to_string(&report.parse_result).expect("serialize incremental parse");
        let full_json = serde_json::to_string(&full).expect("serialize full parse");
        assert_eq!(incremental_json, full_json);
    }

    #[test]
    fn incremental_snapshot_reports_fallback_reason_when_edits_missing() {
        let parser = ParserCoordinator::with_fallback();
        let file_path = PathBuf::from("snapshot-fallback.bsl");

        parser
            .parse_incremental_with_report(
                file_path.clone(),
                "Процедура Тест()\n    x = 1;\nКонецПроцедуры".to_string(),
                Vec::new(),
            )
            .expect("seed snapshot");

        let report = parser
            .parse_incremental_with_report(
                file_path,
                "Процедура Тест()\n    x = 2;\nКонецПроцедуры".to_string(),
                Vec::new(),
            )
            .expect("fallback parse report");
        assert!(!report.incremental);
        assert_eq!(report.fallback_reason.as_deref(), Some("no_edits_provided"));
    }

    #[test]
    fn incremental_snapshot_handles_edit_burst_without_drift() {
        let parser = ParserCoordinator::with_fallback();
        let file_path = PathBuf::from("snapshot-burst.bsl");
        let initial_text = "Процедура Тест()\n    x = 0;\nКонецПроцедуры".to_string();

        parser
            .parse_incremental_with_report(file_path.clone(), initial_text, Vec::new())
            .expect("seed snapshot");

        for step in 1..=32_u8 {
            let next_digit = char::from(b'0' + (step % 10));
            let updated = format!("Процедура Тест()\n    x = {};\nКонецПроцедуры", next_digit);

            let report = parser
                .parse_incremental_with_report(
                    file_path.clone(),
                    updated.clone(),
                    vec![TextEdit {
                        start_line: 1,
                        start_utf16_column: 8,
                        old_end_line: 1,
                        old_end_utf16_column: 9,
                        new_text: next_digit.to_string(),
                    }],
                )
                .expect("incremental burst parse");

            assert!(report.incremental, "step {step} should stay incremental");
            assert!(
                report.fallback_reason.is_none(),
                "step {step} must not fallback"
            );
            assert_eq!(report.changed_ranges.len(), 1);
        }
    }
}
