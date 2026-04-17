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
    use std::sync::mpsc;
    use std::sync::{Arc, Barrier, Mutex, OnceLock};
    use std::time::Duration;

    fn lock_parse_snapshot_test_env() -> std::sync::MutexGuard<'static, ()> {
        static TEST_ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        TEST_ENV_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    struct EnvVarGuard {
        key: &'static str,
        previous: Option<String>,
    }

    impl EnvVarGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let previous = std::env::var(key).ok();
            std::env::set_var(key, value);
            Self { key, previous }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            if let Some(previous) = &self.previous {
                std::env::set_var(self.key, previous);
            } else {
                std::env::remove_var(self.key);
            }
        }
    }

    #[test]
    fn incremental_snapshot_matches_full_parse_result() {
        let _env_lock = lock_parse_snapshot_test_env();
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
        let _env_lock = lock_parse_snapshot_test_env();
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
    fn incremental_snapshot_reports_fallback_reason_when_edit_base_is_stale() {
        let _env_lock = lock_parse_snapshot_test_env();
        let parser = ParserCoordinator::with_fallback();
        let file_path = PathBuf::from("snapshot-stale-base.bsl");

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
                "Процедура Тест()\n    x = 3;\nКонецПроцедуры".to_string(),
                vec![TextEdit {
                    start_line: 1,
                    start_utf16_column: 8,
                    old_end_line: 1,
                    old_end_utf16_column: 9,
                    new_text: "2".to_string(),
                }],
            )
            .expect("fallback parse report");
        assert!(!report.incremental);
        assert_eq!(
            report.fallback_reason.as_deref(),
            Some("edits_do_not_match_new_content")
        );
    }

    #[test]
    fn incremental_snapshot_reports_fallback_reason_when_incremental_parser_rejects() {
        let _env_lock = lock_parse_snapshot_test_env();
        let _force_guard = EnvVarGuard::set("BSL_TEST_FORCE_INCREMENTAL_PARSE_FAILURE", "1");

        let parser = ParserCoordinator::with_fallback();
        let file_path = PathBuf::from("snapshot-incremental-reject.bsl");

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
                vec![TextEdit {
                    start_line: 1,
                    start_utf16_column: 8,
                    old_end_line: 1,
                    old_end_utf16_column: 9,
                    new_text: "2".to_string(),
                }],
            )
            .expect("fallback parse report");
        assert!(!report.incremental);
        assert_eq!(
            report.fallback_reason.as_deref(),
            Some("incremental_parse_failed")
        );
    }

    #[test]
    fn incremental_snapshot_reports_fallback_reason_when_adapter_conversion_fails() {
        let _env_lock = lock_parse_snapshot_test_env();
        let _force_guard = EnvVarGuard::set("BSL_TEST_FORCE_INCREMENTAL_ADAPTER_ERROR", "1");

        let parser = ParserCoordinator::with_fallback();
        let file_path = PathBuf::from("snapshot-incremental-adapter-error.bsl");
        let base = "Процедура Тест()\n    Знач = 1;\nКонецПроцедуры".to_string();
        let updated = "Процедура Тест()\n    Знач = 2;\nКонецПроцедуры".to_string();

        parser
            .parse_incremental_with_report(file_path.clone(), base, Vec::new())
            .expect("seed snapshot");

        let report = parser
            .parse_incremental_with_report(
                file_path,
                updated.clone(),
                vec![TextEdit {
                    start_line: 1,
                    start_utf16_column: 11,
                    old_end_line: 1,
                    old_end_utf16_column: 12,
                    new_text: "2".to_string(),
                }],
            )
            .expect("fallback parse report");

        assert!(!report.incremental);
        assert_eq!(
            report.fallback_reason.as_deref(),
            Some("incremental_parse_failed")
        );

        let full = ParserCoordinator::with_fallback()
            .parse(&updated)
            .expect("full parse");
        let fallback_json =
            serde_json::to_string(&report.parse_result).expect("serialize fallback parse");
        let full_json = serde_json::to_string(&full).expect("serialize full parse");
        assert_eq!(fallback_json, full_json);
    }

    #[test]
    fn incremental_snapshot_reports_fallback_reason_when_input_edit_conversion_fails() {
        let _env_lock = lock_parse_snapshot_test_env();
        let parser = ParserCoordinator::with_fallback();
        let file_path = PathBuf::from("snapshot-input-edit-conversion-failure.bsl");

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
                vec![TextEdit {
                    start_line: 99,
                    start_utf16_column: 0,
                    old_end_line: 99,
                    old_end_utf16_column: 0,
                    new_text: "2".to_string(),
                }],
            )
            .expect("fallback parse report");
        assert!(!report.incremental);
        assert_eq!(
            report.fallback_reason.as_deref(),
            Some("input_edit_conversion_failed")
        );
    }

    #[test]
    fn incremental_snapshot_handles_edit_burst_without_drift() {
        let _env_lock = lock_parse_snapshot_test_env();
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

    #[test]
    fn same_text_concurrent_parse_reports_share_single_full_parse() {
        let _env_lock = lock_parse_snapshot_test_env();
        let _delay_guard = EnvVarGuard::set("BSL_TEST_PARSE_SNAPSHOT_FULL_PARSE_DELAY_MS", "150");
        reset_parse_snapshot_full_parse_attempts_for_test();

        let parser = Arc::new(ParserCoordinator::with_fallback());
        let barrier = Arc::new(Barrier::new(3));
        let file_path = PathBuf::from("snapshot-singleflight.bsl");
        let text = "Процедура Тест()\n    x = 1;\nКонецПроцедуры".to_string();

        let spawn_parse = |parser: Arc<ParserCoordinator>, barrier: Arc<Barrier>| {
            let file_path = file_path.clone();
            let text = text.clone();
            std::thread::spawn(move || {
                barrier.wait();
                parser.parse_incremental_with_report(file_path, text, Vec::new())
            })
        };

        let first = spawn_parse(Arc::clone(&parser), Arc::clone(&barrier));
        let second = spawn_parse(parser, Arc::clone(&barrier));
        barrier.wait();

        let first = first
            .join()
            .expect("first concurrent parse thread")
            .expect("first parse report");
        let second = second
            .join()
            .expect("second concurrent parse thread")
            .expect("second parse report");

        assert!(!first.incremental, "first concurrent parse must stay full");
        assert!(
            !second.incremental,
            "second concurrent parse must reuse the full parse"
        );
        assert_eq!(
            first.fallback_reason.as_deref(),
            Some("no_previous_tree"),
            "first parse must record initial cold-parse reason"
        );
        assert_eq!(
            second.fallback_reason.as_deref(),
            Some("no_previous_tree"),
            "coalesced follower must preserve canonical cold-parse attribution"
        );
        assert_eq!(
            get_parse_snapshot_full_parse_attempts_for_test(),
            1,
            "concurrent identical parse reports must pay a single full parse"
        );
    }

    fn build_large_callable_fixture(statement_count: usize) -> String {
        let mut source = String::from("Процедура Тест()\n");
        for index in 0..statement_count {
            source.push_str(&format!("    Значение{} = {};\n", index, index));
        }
        source.push_str("КонецПроцедуры");
        source
    }

    fn build_large_ascii_callable_fixture(statement_count: usize) -> String {
        let mut source = String::from("Процедура Test()\n");
        for index in 0..statement_count {
            source.push_str(&format!("    Value{} = {};\n", index, index));
        }
        source.push_str("КонецПроцедуры");
        source
    }

    fn replace_first_occurrence_edit(
        source: &str,
        needle: &str,
        replacement: &str,
    ) -> (String, TextEdit) {
        let start_byte = source.find(needle).expect("needle must exist");
        let old_end_byte = start_byte + needle.len();
        let updated = source.replacen(needle, replacement, 1);
        let line_index = crate::system::positioning::LineIndex::new(source);
        let start_point = line_index.byte_offset_to_point(source, start_byte);
        let old_end_point = line_index.byte_offset_to_point(source, old_end_byte);
        let start_line = start_point.row as u32;
        let old_end_line = old_end_point.row as u32;
        let start_utf16_column =
            line_index.byte_column_to_utf16(source, start_point.row, start_point.column);
        let old_end_utf16_column =
            line_index.byte_column_to_utf16(source, old_end_point.row, old_end_point.column);
        (
            updated,
            TextEdit {
                start_line,
                start_utf16_column,
                old_end_line,
                old_end_utf16_column,
                new_text: replacement.to_string(),
            },
        )
    }

    #[test]
    fn exact_lowering_reuse_plan_reuses_unchanged_routine_body_windows_for_local_edit() {
        let _env_lock = lock_parse_snapshot_test_env();
        let parser = ParserCoordinator::with_fallback();
        let file_path = PathBuf::from("snapshot-routine-body-reuse-plan.bsl");
        let base = build_large_ascii_callable_fixture(12);

        parser
            .parse_incremental_with_report(file_path.clone(), base.clone(), Vec::new())
            .expect("seed snapshot");

        let (updated, edit) = replace_first_occurrence_edit(&base, "Value7 = 7", "Value7 = 700");
        let (old_tree, old_source, _) = parser
            .tree_cache
            .get(&file_path)
            .expect("seeded tree cache entry");
        let cancellation_flag = AtomicBool::new(false);
        let (new_tree, changed_ranges) = parser
            .tree_sitter
            .parse_incremental_tree_only_with_cancellation(
                &updated,
                Some(old_tree.as_ref()),
                vec![edit],
                &old_source,
                &cancellation_flag,
            )
            .expect("incremental tree");
        let mut lowering_attribution = ParseSnapshotProgramLoweringAttribution::default();

        let plan = parser
            .build_exact_lowering_reuse_plan(
                &old_source,
                &new_tree,
                &changed_ranges,
                &mut lowering_attribution,
            )
            .expect("routine-body reuse plan");
        assert_eq!(plan.outcome, LoweringReusePlanOutcome::RoutineBodyReuse);
        assert_eq!(plan.top_level_nodes.len(), 1);

        let LoweringReuseNodePlan::RebuildRoutineBody(body_plan) = &plan.top_level_nodes[0] else {
            panic!(
                "expected routine body reuse plan, got {:?}",
                plan.top_level_nodes
            );
        };
        assert_eq!(body_plan.original_body_len, 12);
        assert_eq!(body_plan.reused_body_prefix.len(), 7);
        assert_eq!(body_plan.reused_body_suffix.len(), 4);
    }

    #[test]
    fn exact_lowering_reuse_plan_reuses_p55_style_local_assignment_window() {
        let _env_lock = lock_parse_snapshot_test_env();
        let parser = ParserCoordinator::with_fallback();
        let file_path = PathBuf::from("snapshot-routine-body-reuse-p55-shape.bsl");
        let base = r#"Функция ПолучитьПараметрыРаботыПользователяДляИсходящегоЭлектронногоПисьма(УчетнаяЗаписьЭлектроннойПочты, ФорматСообщения,ДляНового) Экспорт
	СтруктураВозврата = Новый Структура;
	СтруктураВозврата.Вставить("Подпись", Неопределено);
	СтруктураВозврата.Вставить("УведомитьОДоставке", Ложь);
	СтруктураВозврата.Вставить("УведомитьОПрочтении", Ложь);
	СтруктураВозврата.Вставить("ОтображатьТелоИсходногоПисьма", Ложь);
	СтруктураВозврата.Вставить("ВключатьТелоИсходногоПисьма", Истина);
	Подпись = ОтправкаПочтовыхСообщенийПереопределяемый.ПодписьПисьма();
	ПодписьПростойТекст            = Взаимодействия.ПолучитьОбычныйТекстИзHTML(Подпись);
	ПодписьФорматированныйДокумент = Новый ФорматированныйДокумент();
	ПодписьФорматированныйДокумент.УстановитьHTML(Подпись, Новый Структура);
	Если ФорматСообщения = Перечисления.СпособыРедактированияЭлектронныхПисем.ОбычныйТекст Тогда
		СтруктураВозврата.Подпись = Символы.ПС + Символы.ПС + ПодписьПростойТекст;
	Иначе
		ФорматированныйДокумент = ПодписьФорматированныйДокумент;
		ФорматированныйДокумент.Вставить(ФорматированныйДокумент.ПолучитьЗакладкуНачала(), , ТипЭлементаФорматированногоДокумента.ПереводСтроки);
		ФорматированныйДокумент.Вставить(ФорматированныйДокумент.ПолучитьЗакладкуНачала(), , ТипЭлементаФорматированногоДокумента.ПереводСтроки);
		СтруктураВозврата.Подпись = ФорматированныйДокумент;
	КонецЕсли;
	Возврат СтруктураВозврата;
КонецФункции"#.to_string();

        parser
            .parse_incremental_with_report(file_path.clone(), base.clone(), Vec::new())
            .expect("seed snapshot");

        let (updated, edit) = replace_first_occurrence_edit(
            &base,
            "СтруктураВозврата = Новый Структура",
            "СтруктураВозврата = НеобъявленнаяПеременная",
        );
        let (old_tree, old_source, _) = parser
            .tree_cache
            .get(&file_path)
            .expect("seeded tree cache entry");
        let cancellation_flag = AtomicBool::new(false);
        let (new_tree, changed_ranges) = parser
            .tree_sitter
            .parse_incremental_tree_only_with_cancellation(
                &updated,
                Some(old_tree.as_ref()),
                vec![edit],
                &old_source,
                &cancellation_flag,
            )
            .expect("incremental tree");
        let mut lowering_attribution = ParseSnapshotProgramLoweringAttribution::default();

        let plan = parser
            .build_exact_lowering_reuse_plan(
                &old_source,
                &new_tree,
                &changed_ranges,
                &mut lowering_attribution,
            )
            .expect("p55-style local assignment must qualify for routine-body reuse");
        assert_eq!(plan.outcome, LoweringReusePlanOutcome::RoutineBodyReuse);

        let LoweringReuseNodePlan::RebuildRoutineBody(body_plan) = &plan.top_level_nodes[0] else {
            panic!(
                "expected routine-body reuse plan, got {:?}",
                plan.top_level_nodes
            );
        };
        assert_eq!(body_plan.reused_body_prefix.len(), 0);
        assert!(
            body_plan.reused_body_suffix.len() >= 5,
            "expected a large unchanged suffix after the first assignment edit"
        );
    }

    #[test]
    fn exact_lowering_reuse_plan_reuses_callable_body_siblings_around_changed_if_region() {
        let _env_lock = lock_parse_snapshot_test_env();
        let parser = ParserCoordinator::with_fallback();
        let file_path = PathBuf::from("snapshot-routine-body-reuse-if-region.bsl");
        let base = r#"Процедура ПриСозданииНаСервере(Параметры) Экспорт
	НадписьСчет = НСтр("ru = 'Указывается в табличной части'");
	ДополнительныеПараметры = Неопределено;
	СобытияФормИС.ПриСозданииНаСервере(ЭтотОбъект, Отказ, СтандартнаяОбработка, ДополнительныеПараметры);
	ШтрихкодированиеИС.ИнициализироватьКэшМаркируемойПродукции(ЭтотОбъект);
	Если Параметры.Свойство("ДобавитьНоменклатуру") Тогда
		ТаблицаТовары = Новый ТаблицаЗначений;
		ТаблицаТовары.Колонки.Добавить("Номенклатура");
		НоваяСтрока = ТаблицаТовары.Добавить();
		НоваяСтрока.Номенклатура = Параметры.ДобавитьНоменклатуру;
		СтруктураВозврата = Новый Структура;
		СтруктураВозврата.Вставить("АдресПодобраннойНоменклатурыВХранилище", ПоместитьВоВременноеХранилище(ТаблицаТовары));
		ОбработкаВыбораПодборВставкаИзБуфераНаСервере(СтруктураВозврата, "Услуги");
	КонецЕсли;
	УправлениеПанельюПодсказки.ПриСозданииНаСервере(ЭтотОбъект);
	Если ОбщегоНазначения.ПодсистемаСуществует("ИнтеграцияС1СДокументооборотом") Тогда
		МодульИнтеграцияС1СДокументооборотБазоваяФункциональность = ОбщегоНазначения.ОбщийМодуль("ИнтеграцияС1СДокументооборотБазоваяФункциональность");
		МодульИнтеграцияС1СДокументооборотБазоваяФункциональность.ПриСозданииНаСервере(ЭтотОбъект, Элементы.ГруппаГлобальныеКоманды);
	КонецЕсли;
	МобильныйКлиентАдаптацияФормы();
КонецПроцедуры"#.to_string();

        parser
            .parse_incremental_with_report(file_path.clone(), base.clone(), Vec::new())
            .expect("seed snapshot");

        let (updated, edit) = replace_first_occurrence_edit(
            &base,
            "СтруктураВозврата = Новый Структура",
            "СтруктураВозврата = НеобъявленнаяПеременная",
        );
        let (old_tree, old_source, _) = parser
            .tree_cache
            .get(&file_path)
            .expect("seeded tree cache entry");
        let cancellation_flag = AtomicBool::new(false);
        let (new_tree, changed_ranges) = parser
            .tree_sitter
            .parse_incremental_tree_only_with_cancellation(
                &updated,
                Some(old_tree.as_ref()),
                vec![edit],
                &old_source,
                &cancellation_flag,
            )
            .expect("incremental tree");
        let mut lowering_attribution = ParseSnapshotProgramLoweringAttribution::default();

        let plan = parser
            .build_exact_lowering_reuse_plan(
                &old_source,
                &new_tree,
                &changed_ranges,
                &mut lowering_attribution,
            )
            .expect("edit inside a top-level if-region must still qualify for body-local reuse");
        assert_eq!(plan.outcome, LoweringReusePlanOutcome::RoutineBodyReuse);

        let LoweringReuseNodePlan::RebuildRoutineBody(body_plan) = &plan.top_level_nodes[0] else {
            panic!(
                "expected routine-body reuse plan, got {:?}",
                plan.top_level_nodes
            );
        };
        assert!(
            body_plan.reused_body_prefix.len() >= 4,
            "expected unchanged statements before the affected if region to stay reusable"
        );
        assert!(
            body_plan.reused_body_suffix.len() >= 3,
            "expected unchanged statements after the affected if region to stay reusable"
        );
    }

    #[test]
    fn exact_lowering_reuse_plan_fails_closed_for_local_var_declaration_edit() {
        let _env_lock = lock_parse_snapshot_test_env();
        let parser = ParserCoordinator::with_fallback();
        let file_path = PathBuf::from("snapshot-routine-body-reuse-var-decl.bsl");
        let base =
            "Процедура Test()\n    Перем Local;\n    Value0 = 0;\n    Value1 = 1;\nКонецПроцедуры"
                .to_string();

        parser
            .parse_incremental_with_report(file_path.clone(), base.clone(), Vec::new())
            .expect("seed snapshot");

        let (updated, edit) = replace_first_occurrence_edit(&base, "Перем Local", "Перем Local2");
        let (old_tree, old_source, _) = parser
            .tree_cache
            .get(&file_path)
            .expect("seeded tree cache entry");
        let cancellation_flag = AtomicBool::new(false);
        let (new_tree, changed_ranges) = parser
            .tree_sitter
            .parse_incremental_tree_only_with_cancellation(
                &updated,
                Some(old_tree.as_ref()),
                vec![edit],
                &old_source,
                &cancellation_flag,
            )
            .expect("incremental tree");
        let mut lowering_attribution = ParseSnapshotProgramLoweringAttribution::default();

        assert!(
            parser
                .build_exact_lowering_reuse_plan(
                    &old_source,
                    &new_tree,
                    &changed_ranges,
                    &mut lowering_attribution,
                )
                .is_none(),
            "var-declaration edit must fail closed instead of reusing lowered body windows"
        );
    }

    #[test]
    fn exact_lowering_reuse_plan_fails_closed_for_edit_inside_try_region() {
        let _env_lock = lock_parse_snapshot_test_env();
        let parser = ParserCoordinator::with_fallback();
        let file_path = PathBuf::from("snapshot-routine-body-reuse-try-region.bsl");
        let base = r#"Процедура Test()
	Сообщить("before");
	Попытка
		Значение = 1;
	Исключение
		Сообщить("error");
	КонецПопытки;
	Сообщить("after");
КонецПроцедуры"#
            .to_string();

        parser
            .parse_incremental_with_report(file_path.clone(), base.clone(), Vec::new())
            .expect("seed snapshot");

        let (updated, edit) = replace_first_occurrence_edit(&base, "Значение = 1", "Значение = 2");
        let (old_tree, old_source, _) = parser
            .tree_cache
            .get(&file_path)
            .expect("seeded tree cache entry");
        let cancellation_flag = AtomicBool::new(false);
        let (new_tree, changed_ranges) = parser
            .tree_sitter
            .parse_incremental_tree_only_with_cancellation(
                &updated,
                Some(old_tree.as_ref()),
                vec![edit],
                &old_source,
                &cancellation_flag,
            )
            .expect("incremental tree");
        let mut lowering_attribution = ParseSnapshotProgramLoweringAttribution::default();

        assert!(
            parser
                .build_exact_lowering_reuse_plan(
                    &old_source,
                    &new_tree,
                    &changed_ranges,
                    &mut lowering_attribution,
                )
                .is_none(),
            "try/except body edit must stay fail-closed until exception-region boundaries are proven sound"
        );
    }

    #[test]
    fn exact_ready_snapshot_reuse_path_matches_full_parse_for_top_level_edit() {
        let _env_lock = lock_parse_snapshot_test_env();
        let parser = ParserCoordinator::with_fallback();
        let file_path = PathBuf::from("snapshot-top-level-reuse-parity.bsl");
        let base = "Процедура Alpha()\n    Сообщить(\"alpha\");\nКонецПроцедуры\n\nПроцедура Beta()\n    Сообщить(\"beta\");\nКонецПроцедуры\n\nПроцедура Gamma()\n    Сообщить(\"gamma\");\nКонецПроцедуры\n"
            .to_string();

        parser
            .parse_incremental_with_report(file_path.clone(), base.clone(), Vec::new())
            .expect("seed snapshot");

        let (updated, edit) =
            replace_first_occurrence_edit(&base, "Процедура Beta()", "Процедура BetaRenamed()");
        let report = parser
            .parse_incremental_with_report_with_cancellation_and_options(
                file_path,
                updated.clone(),
                vec![edit],
                &AtomicBool::new(false),
                ParseSnapshotExecutionOptions {
                    save_critical_initial: false,
                    save_critical_requested: None,
                    reused_program_prefix: None,
                    lowering_reuse_plan: None,
                    lowering_reuse_summary: None,
                    lowering_reuse_attribution: None,
                    exact_ready_snapshot_control_callback: None,
                    progress_callback: None,
                    core_build_progress_callback: None,
                    assembly_progress_callback: None,
                },
            )
            .expect("exact-path incremental report");

        assert!(report.incremental);
        assert!(report.fallback_reason.is_none());
        assert_eq!(
            report.program_lowering_summary.reuse_outcome,
            ParseSnapshotProgramLoweringReuseOutcome::TopLevelReuse
        );
        assert_eq!(report.program_lowering_summary.reused_window_count, 2);
        assert_eq!(report.program_lowering_summary.rebuilt_window_count, 1);
        assert_eq!(
            report
                .program_lowering_summary
                .fully_reused_top_level_node_count,
            2
        );
        assert_eq!(
            report
                .program_lowering_summary
                .fully_rebuilt_top_level_node_count,
            1
        );
        assert!(report.program_lowering_summary.reused_lowering_units > 0);
        assert!(report.program_lowering_summary.rebuilt_lowering_units > 0);

        let full = ParserCoordinator::with_fallback()
            .parse(&updated)
            .expect("full parse");
        let incremental_json =
            serde_json::to_string(&report.parse_result).expect("serialize incremental parse");
        let full_json = serde_json::to_string(&full).expect("serialize full parse");
        assert_eq!(incremental_json, full_json);
    }

    #[test]
    fn exact_ready_snapshot_reuse_path_matches_full_parse_for_local_body_edit() {
        let _env_lock = lock_parse_snapshot_test_env();
        let parser = ParserCoordinator::with_fallback();
        let file_path = PathBuf::from("snapshot-routine-body-reuse-parity.bsl");
        let base = build_large_ascii_callable_fixture(24);

        parser
            .parse_incremental_with_report(file_path.clone(), base.clone(), Vec::new())
            .expect("seed snapshot");

        let (updated, edit) =
            replace_first_occurrence_edit(&base, "Value12 = 12", "Value12 = 1200");
        let report = parser
            .parse_incremental_with_report_with_cancellation_and_options(
                file_path,
                updated.clone(),
                vec![edit],
                &AtomicBool::new(false),
                ParseSnapshotExecutionOptions {
                    save_critical_initial: false,
                    save_critical_requested: None,
                    reused_program_prefix: None,
                    lowering_reuse_plan: None,
                    lowering_reuse_summary: None,
                    lowering_reuse_attribution: None,
                    exact_ready_snapshot_control_callback: None,
                    progress_callback: None,
                    core_build_progress_callback: None,
                    assembly_progress_callback: None,
                },
            )
            .expect("exact-path incremental report");

        assert!(report.incremental);
        assert!(report.fallback_reason.is_none());

        let full = ParserCoordinator::with_fallback()
            .parse(&updated)
            .expect("full parse");
        let incremental_json =
            serde_json::to_string(&report.parse_result).expect("serialize incremental parse");
        let full_json = serde_json::to_string(&full).expect("serialize full parse");
        assert_eq!(incremental_json, full_json);
    }

    #[test]
    fn exact_ready_snapshot_reuse_path_reports_program_lowering_summary_for_local_body_edit() {
        let _env_lock = lock_parse_snapshot_test_env();
        let parser = ParserCoordinator::with_fallback();
        let file_path = PathBuf::from("snapshot-routine-body-reuse-summary.bsl");
        let base = build_large_ascii_callable_fixture(24);

        parser
            .parse_incremental_with_report(file_path.clone(), base.clone(), Vec::new())
            .expect("seed snapshot");

        let (updated, edit) =
            replace_first_occurrence_edit(&base, "Value12 = 12", "Value12 = 1200");
        let report = parser
            .parse_incremental_with_report_with_cancellation_and_options(
                file_path,
                updated,
                vec![edit],
                &AtomicBool::new(false),
                ParseSnapshotExecutionOptions {
                    save_critical_initial: false,
                    save_critical_requested: None,
                    reused_program_prefix: None,
                    lowering_reuse_plan: None,
                    lowering_reuse_summary: None,
                    lowering_reuse_attribution: None,
                    exact_ready_snapshot_control_callback: None,
                    progress_callback: None,
                    core_build_progress_callback: None,
                    assembly_progress_callback: None,
                },
            )
            .expect("exact-path incremental report");

        assert_eq!(
            report.program_lowering_summary.reuse_outcome,
            ParseSnapshotProgramLoweringReuseOutcome::RoutineBodyReuse
        );
        assert!(report.program_lowering_summary.reused_lowering_units > 0);
        assert!(report.program_lowering_summary.rebuilt_lowering_units > 0);
        assert_eq!(report.program_lowering_summary.reused_window_count, 2);
        assert_eq!(report.program_lowering_summary.rebuilt_window_count, 1);
        assert!(
            report
                .program_lowering_summary
                .largest_rebuilt_window_lowering_units
                > 0
        );
    }

    #[test]
    fn exact_ready_snapshot_reuse_path_matches_full_parse_for_if_region_body_edit() {
        let _env_lock = lock_parse_snapshot_test_env();
        let parser = ParserCoordinator::with_fallback();
        let file_path = PathBuf::from("snapshot-routine-body-if-region-reuse-parity.bsl");
        let base = r#"Процедура ПриСозданииНаСервере(Параметры) Экспорт
	НадписьСчет = НСтр("ru = 'Указывается в табличной части'");
	ДополнительныеПараметры = Неопределено;
	СобытияФормИС.ПриСозданииНаСервере(ЭтотОбъект, Отказ, СтандартнаяОбработка, ДополнительныеПараметры);
	ШтрихкодированиеИС.ИнициализироватьКэшМаркируемойПродукции(ЭтотОбъект);
	Если Параметры.Свойство("ДобавитьНоменклатуру") Тогда
		ТаблицаТовары = Новый ТаблицаЗначений;
		ТаблицаТовары.Колонки.Добавить("Номенклатура");
		НоваяСтрока = ТаблицаТовары.Добавить();
		НоваяСтрока.Номенклатура = Параметры.ДобавитьНоменклатуру;
		СтруктураВозврата = Новый Структура;
		СтруктураВозврата.Вставить("АдресПодобраннойНоменклатурыВХранилище", ПоместитьВоВременноеХранилище(ТаблицаТовары));
		ОбработкаВыбораПодборВставкаИзБуфераНаСервере(СтруктураВозврата, "Услуги");
	КонецЕсли;
	УправлениеПанельюПодсказки.ПриСозданииНаСервере(ЭтотОбъект);
	Если ОбщегоНазначения.ПодсистемаСуществует("ИнтеграцияС1СДокументооборотом") Тогда
		МодульИнтеграцияС1СДокументооборотБазоваяФункциональность = ОбщегоНазначения.ОбщийМодуль("ИнтеграцияС1СДокументооборотБазоваяФункциональность");
		МодульИнтеграцияС1СДокументооборотБазоваяФункциональность.ПриСозданииНаСервере(ЭтотОбъект, Элементы.ГруппаГлобальныеКоманды);
	КонецЕсли;
	МобильныйКлиентАдаптацияФормы();
КонецПроцедуры"#.to_string();

        parser
            .parse_incremental_with_report(file_path.clone(), base.clone(), Vec::new())
            .expect("seed snapshot");

        let (updated, edit) = replace_first_occurrence_edit(
            &base,
            "СтруктураВозврата = Новый Структура",
            "СтруктураВозврата = НеобъявленнаяПеременная",
        );
        let report = parser
            .parse_incremental_with_report_with_cancellation_and_options(
                file_path,
                updated.clone(),
                vec![edit],
                &AtomicBool::new(false),
                ParseSnapshotExecutionOptions {
                    save_critical_initial: false,
                    save_critical_requested: None,
                    reused_program_prefix: None,
                    lowering_reuse_plan: None,
                    lowering_reuse_summary: None,
                    lowering_reuse_attribution: None,
                    exact_ready_snapshot_control_callback: None,
                    progress_callback: None,
                    core_build_progress_callback: None,
                    assembly_progress_callback: None,
                },
            )
            .expect("exact-path incremental report");

        assert!(report.incremental);
        assert!(report.fallback_reason.is_none());
        assert_eq!(
            report.program_lowering_summary.reuse_outcome,
            ParseSnapshotProgramLoweringReuseOutcome::RoutineBodyReuse
        );
        assert!(report.program_lowering_summary.reused_lowering_units > 0);
        assert!(report.program_lowering_summary.rebuilt_lowering_units > 0);
        assert!(
            report
                .program_lowering_summary
                .routine_body_reuse_node_count
                >= 1
        );

        let full = ParserCoordinator::with_fallback()
            .parse(&updated)
            .expect("full parse");
        let incremental_json =
            serde_json::to_string(&report.parse_result).expect("serialize incremental parse");
        let full_json = serde_json::to_string(&full).expect("serialize full parse");
        assert_eq!(incremental_json, full_json);
    }

    #[test]
    fn exact_ready_snapshot_reuse_path_fails_closed_for_try_region_body_edit() {
        let _env_lock = lock_parse_snapshot_test_env();
        let parser = ParserCoordinator::with_fallback();
        let file_path = PathBuf::from("snapshot-routine-body-try-region-reuse-parity.bsl");
        let base = r#"Процедура Test()
	Сообщить("before");
	Попытка
		Значение = 1;
	Исключение
		Сообщить("error");
	КонецПопытки;
	Сообщить("after");
КонецПроцедуры"#
            .to_string();

        parser
            .parse_incremental_with_report(file_path.clone(), base.clone(), Vec::new())
            .expect("seed snapshot");

        let (updated, edit) = replace_first_occurrence_edit(&base, "Значение = 1", "Значение = 2");
        let report = parser
            .parse_incremental_with_report_with_cancellation_and_options(
                file_path,
                updated.clone(),
                vec![edit],
                &AtomicBool::new(false),
                ParseSnapshotExecutionOptions {
                    save_critical_initial: false,
                    save_critical_requested: None,
                    reused_program_prefix: None,
                    lowering_reuse_plan: None,
                    lowering_reuse_summary: None,
                    lowering_reuse_attribution: None,
                    exact_ready_snapshot_control_callback: None,
                    progress_callback: None,
                    core_build_progress_callback: None,
                    assembly_progress_callback: None,
                },
            )
            .expect("exact-path incremental report");

        assert!(report.incremental);
        assert!(report.fallback_reason.is_none());
        assert_eq!(
            report.program_lowering_summary.reuse_outcome,
            ParseSnapshotProgramLoweringReuseOutcome::FullRebuild
        );

        let full = ParserCoordinator::with_fallback()
            .parse(&updated)
            .expect("full parse");
        let incremental_json =
            serde_json::to_string(&report.parse_result).expect("serialize incremental parse");
        let full_json = serde_json::to_string(&full).expect("serialize full parse");
        assert_eq!(incremental_json, full_json);
    }

    #[test]
    fn exact_program_lowering_reuse_kill_switch_disables_reuse_plan() {
        let _env_lock = lock_parse_snapshot_test_env();
        {
            let _reuse_guard = EnvVarGuard::set(
                "BSL_INTELLISENSE_V2_EXACT_PROGRAM_LOWERING_REUSE_ENABLED",
                "0",
            );
            global_runtime_config().reload_env_bootstrap_from_env();

            let parser = ParserCoordinator::with_fallback();
            let file_path = PathBuf::from("snapshot-routine-body-reuse-disabled.bsl");
            let base = build_large_ascii_callable_fixture(24);

            parser
                .parse_incremental_with_report(file_path.clone(), base.clone(), Vec::new())
                .expect("seed snapshot");

            let (updated, edit) =
                replace_first_occurrence_edit(&base, "Value12 = 12", "Value12 = 1200");
            let (old_tree, old_source, _) = parser
                .tree_cache
                .get(&file_path)
                .expect("seeded tree cache entry");
            let cancellation_flag = AtomicBool::new(false);
            let (new_tree, changed_ranges) = parser
                .tree_sitter
                .parse_incremental_tree_only_with_cancellation(
                    &updated,
                    Some(old_tree.as_ref()),
                    vec![edit.clone()],
                    &old_source,
                    &cancellation_flag,
                )
                .expect("incremental tree");
            let mut lowering_attribution = ParseSnapshotProgramLoweringAttribution::default();
            assert!(
                parser
                    .build_exact_lowering_reuse_plan(
                        &old_source,
                        &new_tree,
                        &changed_ranges,
                        &mut lowering_attribution,
                    )
                    .is_none(),
                "runtime kill switch must disable exact lowering reuse planning"
            );

            let report = parser
                .parse_incremental_with_report_with_cancellation_and_options(
                    file_path,
                    updated,
                    vec![edit],
                    &AtomicBool::new(false),
                    ParseSnapshotExecutionOptions {
                        save_critical_initial: false,
                        save_critical_requested: None,
                        reused_program_prefix: None,
                        lowering_reuse_plan: None,
                        lowering_reuse_summary: None,
                        lowering_reuse_attribution: None,
                        exact_ready_snapshot_control_callback: None,
                        progress_callback: None,
                        core_build_progress_callback: None,
                        assembly_progress_callback: None,
                    },
                )
                .expect("incremental report with kill switch disabled");

            assert!(report.incremental);
            assert!(report.fallback_reason.is_none());
            assert_eq!(
                report.program_lowering_summary.reuse_outcome,
                ParseSnapshotProgramLoweringReuseOutcome::FullRebuild
            );
            assert_eq!(report.program_lowering_summary.reused_lowering_units, 0);
            assert!(report.program_lowering_summary.rebuilt_lowering_units > 0);
        }
        global_runtime_config().reload_env_bootstrap_from_env();
    }

    #[test]
    fn save_critical_requested_during_reused_program_lowering_returns_before_packaging_checkpoint()
    {
        let _env_lock = lock_parse_snapshot_test_env();
        let _conversion_delay_guard = EnvVarGuard::set(
            "BSL_TEST_PARSE_SNAPSHOT_PROGRAM_CONVERSION_PROGRESS_DELAY_MS",
            "40",
        );

        let parser = Arc::new(ParserCoordinator::with_fallback());
        let file_path = PathBuf::from("snapshot-save-critical-during-reused-lowering.bsl");
        let base = build_large_ascii_callable_fixture(512);

        parser
            .parse_incremental_with_report(file_path.clone(), base.clone(), Vec::new())
            .expect("seed snapshot");

        let (updated, edit) =
            replace_first_occurrence_edit(&base, "Value256 = 256", "Value256 = 1256");
        let (old_tree, old_source, _) = parser
            .tree_cache
            .get(&file_path)
            .expect("seeded tree cache entry");
        let cancellation_flag = AtomicBool::new(false);
        let (new_tree, changed_ranges) = parser
            .tree_sitter
            .parse_incremental_tree_only_with_cancellation(
                &updated,
                Some(old_tree.as_ref()),
                vec![edit.clone()],
                &old_source,
                &cancellation_flag,
            )
            .expect("incremental tree");
        let mut lowering_attribution = ParseSnapshotProgramLoweringAttribution::default();
        let lowering_reuse_plan = parser
            .build_exact_lowering_reuse_plan(
                &old_source,
                &new_tree,
                &changed_ranges,
                &mut lowering_attribution,
            )
            .expect("local edit must produce lowering reuse plan");
        assert_eq!(
            lowering_reuse_plan.outcome,
            LoweringReusePlanOutcome::RoutineBodyReuse,
            "local body edit must exercise bounded routine-body reuse"
        );

        let cancellation_flag = Arc::new(AtomicBool::new(false));
        let save_critical_requested = Arc::new(AtomicBool::new(false));
        let checkpoints = Arc::new(Mutex::new(Vec::new()));
        let (entered_program_lowering_tx, entered_program_lowering_rx) = mpsc::channel();

        let parse_thread = {
            let parser = Arc::clone(&parser);
            let checkpoints = Arc::clone(&checkpoints);
            let cancellation_flag = Arc::clone(&cancellation_flag);
            let save_critical_requested = Arc::clone(&save_critical_requested);
            let file_path = file_path.clone();
            let updated = updated.clone();
            std::thread::spawn(move || {
                let assembly_progress = |checkpoint: ParseSnapshotAssemblyCheckpoint| {
                    checkpoints
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner())
                        .push(checkpoint);
                    if checkpoint == ParseSnapshotAssemblyCheckpoint::ProgramLowering {
                        let _ = entered_program_lowering_tx.send(());
                    }
                };
                let options = ParseSnapshotExecutionOptions {
                    save_critical_initial: false,
                    save_critical_requested: Some(save_critical_requested.as_ref()),
                    reused_program_prefix: None,
                    lowering_reuse_plan: None,
                    lowering_reuse_summary: None,
                    lowering_reuse_attribution: None,
                    exact_ready_snapshot_control_callback: None,
                    progress_callback: None,
                    core_build_progress_callback: None,
                    assembly_progress_callback: Some(&assembly_progress),
                };
                parser.parse_incremental_with_report_with_cancellation_and_options(
                    file_path,
                    updated,
                    vec![edit],
                    cancellation_flag.as_ref(),
                    options,
                )
            })
        };

        entered_program_lowering_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("reused exact path must enter program lowering");
        std::thread::sleep(Duration::from_millis(100));
        save_critical_requested.store(true, Ordering::SeqCst);

        let report = parse_thread
            .join()
            .expect("parse thread join")
            .expect("save-critical reuse parse report");

        assert!(report.incremental);
        assert!(report.fallback_reason.is_none());
        assert!(
            report.parse_exec_subphases.deferred_syntax_error_assembly,
            "save-critical promotion inside reused lowering must defer syntax-error assembly"
        );

        let checkpoints = checkpoints
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        assert!(
            checkpoints.contains(&ParseSnapshotAssemblyCheckpoint::ProgramLowering),
            "expected lowering checkpoint trace, got: {checkpoints:?}"
        );
        assert!(
            !checkpoints.contains(&ParseSnapshotAssemblyCheckpoint::PublishableArtifactPackaging),
            "save-critical promotion during reused lowering must return before packaging checkpoint: {checkpoints:?}"
        );
        assert!(
            !checkpoints.contains(&ParseSnapshotAssemblyCheckpoint::SyntaxErrorCollection),
            "save-critical promotion during reused lowering must return before syntax-error collection: {checkpoints:?}"
        );
    }

    #[test]
    fn exact_ready_control_callback_can_cancel_during_reused_program_lowering() {
        let _env_lock = lock_parse_snapshot_test_env();
        let _conversion_delay_guard = EnvVarGuard::set(
            "BSL_TEST_PARSE_SNAPSHOT_PROGRAM_CONVERSION_PROGRESS_DELAY_MS",
            "40",
        );

        let parser = Arc::new(ParserCoordinator::with_fallback());
        let file_path = PathBuf::from("snapshot-cancel-during-reused-lowering.bsl");
        let base = build_large_ascii_callable_fixture(512);

        parser
            .parse_incremental_with_report(file_path.clone(), base.clone(), Vec::new())
            .expect("seed snapshot");

        let (updated, edit) =
            replace_first_occurrence_edit(&base, "Value256 = 256", "Value256 = 1256");
        let (old_tree, old_source, _) = parser
            .tree_cache
            .get(&file_path)
            .expect("seeded tree cache entry");
        let cancellation_flag = AtomicBool::new(false);
        let (new_tree, changed_ranges) = parser
            .tree_sitter
            .parse_incremental_tree_only_with_cancellation(
                &updated,
                Some(old_tree.as_ref()),
                vec![edit.clone()],
                &old_source,
                &cancellation_flag,
            )
            .expect("incremental tree");
        let mut lowering_attribution = ParseSnapshotProgramLoweringAttribution::default();
        let lowering_reuse_plan = parser
            .build_exact_lowering_reuse_plan(
                &old_source,
                &new_tree,
                &changed_ranges,
                &mut lowering_attribution,
            )
            .expect("local edit must produce lowering reuse plan");
        assert_eq!(
            lowering_reuse_plan.outcome,
            LoweringReusePlanOutcome::RoutineBodyReuse,
            "local body edit must exercise bounded routine-body reuse"
        );

        let cancellation_flag = Arc::new(AtomicBool::new(false));
        let cancel_on_checkpoint = Arc::new(AtomicBool::new(false));
        let checkpoints = Arc::new(Mutex::new(Vec::new()));
        let (entered_program_lowering_tx, entered_program_lowering_rx) = mpsc::channel();

        let parse_thread = {
            let parser = Arc::clone(&parser);
            let checkpoints = Arc::clone(&checkpoints);
            let cancellation_flag = Arc::clone(&cancellation_flag);
            let cancel_on_checkpoint = Arc::clone(&cancel_on_checkpoint);
            let file_path = file_path.clone();
            let updated = updated.clone();
            std::thread::spawn(move || {
                let assembly_progress = |checkpoint: ParseSnapshotAssemblyCheckpoint| {
                    checkpoints
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner())
                        .push(checkpoint);
                    if checkpoint == ParseSnapshotAssemblyCheckpoint::ProgramLowering {
                        let _ = entered_program_lowering_tx.send(());
                    }
                };
                let control = || {
                    if cancel_on_checkpoint.load(Ordering::SeqCst) {
                        ParseSnapshotExactReadyControl::Cancel
                    } else {
                        ParseSnapshotExactReadyControl::Continue
                    }
                };
                let options = ParseSnapshotExecutionOptions {
                    save_critical_initial: false,
                    save_critical_requested: None,
                    reused_program_prefix: None,
                    lowering_reuse_plan: None,
                    lowering_reuse_summary: None,
                    lowering_reuse_attribution: None,
                    exact_ready_snapshot_control_callback: Some(&control),
                    progress_callback: None,
                    core_build_progress_callback: None,
                    assembly_progress_callback: Some(&assembly_progress),
                };
                parser.parse_incremental_with_report_with_cancellation_and_options(
                    file_path,
                    updated,
                    vec![edit],
                    cancellation_flag.as_ref(),
                    options,
                )
            })
        };

        entered_program_lowering_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("reused exact path must enter program lowering");
        std::thread::sleep(Duration::from_millis(100));
        cancel_on_checkpoint.store(true, Ordering::SeqCst);

        let error = parse_thread
            .join()
            .expect("parse thread join")
            .expect_err("control callback must cancel reused lowering parse");
        assert!(
            is_parse_cancelled_error(&error),
            "reused lowering control cancel must surface as parse cancellation, got: {error}"
        );

        let checkpoints = checkpoints
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        assert!(
            checkpoints.contains(&ParseSnapshotAssemblyCheckpoint::ProgramLowering),
            "expected lowering checkpoint trace, got: {checkpoints:?}"
        );
        assert!(
            !checkpoints.contains(&ParseSnapshotAssemblyCheckpoint::PublishableArtifactPackaging),
            "cancellation during reused lowering must not advance into packaging: {checkpoints:?}"
        );
    }

    #[test]
    fn save_critical_requested_during_program_lowering_returns_before_packaging_checkpoint() {
        let _env_lock = lock_parse_snapshot_test_env();
        let _conversion_delay_guard = EnvVarGuard::set(
            "BSL_TEST_PARSE_SNAPSHOT_PROGRAM_CONVERSION_PROGRESS_DELAY_MS",
            "40",
        );

        let parser = Arc::new(ParserCoordinator::with_fallback());
        let file_path = PathBuf::from("snapshot-save-critical-during-lowering.bsl");
        // Keep lowering in flight across multiple observer checkpoints so this
        // regression proves bounded save-critical promotion instead of racing a
        // nearly-finished conversion on fast CPUs.
        let text = build_large_callable_fixture(512);

        parser
            .parse_incremental_with_report(file_path.clone(), text.clone(), Vec::new())
            .expect("seed snapshot");

        let cancellation_flag = Arc::new(AtomicBool::new(false));
        let save_critical_requested = Arc::new(AtomicBool::new(false));
        let checkpoints = Arc::new(Mutex::new(Vec::new()));
        let (entered_program_lowering_tx, entered_program_lowering_rx) = mpsc::channel();

        let parse_thread = {
            let parser = Arc::clone(&parser);
            let checkpoints = Arc::clone(&checkpoints);
            let cancellation_flag = Arc::clone(&cancellation_flag);
            let save_critical_requested = Arc::clone(&save_critical_requested);
            let file_path = file_path.clone();
            let text = text.clone();
            std::thread::spawn(move || {
                let assembly_progress = |checkpoint: ParseSnapshotAssemblyCheckpoint| {
                    checkpoints
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner())
                        .push(checkpoint);
                    if checkpoint == ParseSnapshotAssemblyCheckpoint::ProgramLowering {
                        let _ = entered_program_lowering_tx.send(());
                    }
                };
                let options = ParseSnapshotExecutionOptions {
                    save_critical_initial: false,
                    save_critical_requested: Some(save_critical_requested.as_ref()),
                    reused_program_prefix: None,
                    lowering_reuse_plan: None,
                    lowering_reuse_summary: None,
                    lowering_reuse_attribution: None,
                    exact_ready_snapshot_control_callback: None,
                    progress_callback: None,
                    core_build_progress_callback: None,
                    assembly_progress_callback: Some(&assembly_progress),
                };
                parser.parse_full_with_report_with_cancellation_and_options(
                    file_path,
                    text,
                    PARSE_SNAPSHOT_FALLBACK_NO_PREVIOUS_TREE,
                    cancellation_flag.as_ref(),
                    options,
                )
            })
        };

        entered_program_lowering_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("exact ready snapshot assembly must enter program lowering");
        std::thread::sleep(Duration::from_millis(100));
        save_critical_requested.store(true, Ordering::SeqCst);

        let report = parse_thread
            .join()
            .expect("parse thread join")
            .expect("save-critical parse report");

        assert!(
            report.parse_exec_subphases.deferred_syntax_error_assembly,
            "save-critical promotion inside program lowering must defer syntax-error assembly"
        );

        let checkpoints = checkpoints
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        assert!(
            checkpoints.contains(&ParseSnapshotAssemblyCheckpoint::ProgramLowering),
            "expected lowering checkpoint trace, got: {checkpoints:?}"
        );
        assert!(
            !checkpoints.contains(&ParseSnapshotAssemblyCheckpoint::PublishableArtifactPackaging),
            "save-critical promotion during lowering must return before packaging checkpoint: {checkpoints:?}"
        );
        assert!(
            !checkpoints.contains(&ParseSnapshotAssemblyCheckpoint::SyntaxErrorCollection),
            "save-critical promotion during lowering must return before syntax-error collection: {checkpoints:?}"
        );
    }

    #[test]
    fn exact_ready_control_callback_can_cancel_during_program_lowering() {
        let _env_lock = lock_parse_snapshot_test_env();
        let _conversion_delay_guard = EnvVarGuard::set(
            "BSL_TEST_PARSE_SNAPSHOT_PROGRAM_CONVERSION_PROGRESS_DELAY_MS",
            "40",
        );

        let parser = Arc::new(ParserCoordinator::with_fallback());
        let file_path = PathBuf::from("snapshot-cancel-during-lowering.bsl");
        let text = build_large_callable_fixture(512);

        parser
            .parse_incremental_with_report(file_path.clone(), text.clone(), Vec::new())
            .expect("seed snapshot");

        let cancellation_flag = Arc::new(AtomicBool::new(false));
        let cancel_on_checkpoint = Arc::new(AtomicBool::new(false));
        let checkpoints = Arc::new(Mutex::new(Vec::new()));
        let (entered_program_lowering_tx, entered_program_lowering_rx) = mpsc::channel();

        let parse_thread = {
            let parser = Arc::clone(&parser);
            let checkpoints = Arc::clone(&checkpoints);
            let cancellation_flag = Arc::clone(&cancellation_flag);
            let cancel_on_checkpoint = Arc::clone(&cancel_on_checkpoint);
            let file_path = file_path.clone();
            let text = text.clone();
            std::thread::spawn(move || {
                let assembly_progress = |checkpoint: ParseSnapshotAssemblyCheckpoint| {
                    checkpoints
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner())
                        .push(checkpoint);
                    if checkpoint == ParseSnapshotAssemblyCheckpoint::ProgramLowering {
                        let _ = entered_program_lowering_tx.send(());
                    }
                };
                let control = || {
                    if cancel_on_checkpoint.load(Ordering::SeqCst) {
                        ParseSnapshotExactReadyControl::Cancel
                    } else {
                        ParseSnapshotExactReadyControl::Continue
                    }
                };
                let options = ParseSnapshotExecutionOptions {
                    save_critical_initial: false,
                    save_critical_requested: None,
                    reused_program_prefix: None,
                    lowering_reuse_plan: None,
                    lowering_reuse_summary: None,
                    lowering_reuse_attribution: None,
                    exact_ready_snapshot_control_callback: Some(&control),
                    progress_callback: None,
                    core_build_progress_callback: None,
                    assembly_progress_callback: Some(&assembly_progress),
                };
                parser.parse_full_with_report_with_cancellation_and_options(
                    file_path,
                    text,
                    PARSE_SNAPSHOT_FALLBACK_NO_PREVIOUS_TREE,
                    cancellation_flag.as_ref(),
                    options,
                )
            })
        };

        entered_program_lowering_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("exact ready snapshot assembly must enter program lowering");
        std::thread::sleep(Duration::from_millis(100));
        cancel_on_checkpoint.store(true, Ordering::SeqCst);

        let error = parse_thread
            .join()
            .expect("parse thread join")
            .expect_err("control callback must cancel bounded lowering parse");
        assert!(
            is_parse_cancelled_error(&error),
            "bounded lowering control cancel must surface as parse cancellation, got: {error}"
        );

        let checkpoints = checkpoints
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        assert!(
            checkpoints.contains(&ParseSnapshotAssemblyCheckpoint::ProgramLowering),
            "expected lowering checkpoint trace, got: {checkpoints:?}"
        );
        assert!(
            !checkpoints.contains(&ParseSnapshotAssemblyCheckpoint::PublishableArtifactPackaging),
            "cancellation during lowering must not advance into packaging: {checkpoints:?}"
        );
    }
}
