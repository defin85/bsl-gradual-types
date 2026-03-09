use super::*;
use tree_sitter::Parser as TreeSitterParser;

fn parse_backend_tree_for_test(text: &str) -> Arc<Tree> {
    let mut parser = TreeSitterParser::new();
    parser
        .set_language(&tree_sitter_bsl::LANGUAGE.into())
        .expect("tree-sitter-bsl language");
    Arc::new(
        parser
            .parse(text, None)
            .expect("tree-sitter parse for snapshot"),
    )
}

fn normalize_json(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            let mut entries: Vec<(String, serde_json::Value)> =
                std::mem::take(map).into_iter().collect();
            entries.sort_by(|(a, _), (b, _)| a.cmp(b));

            let mut sorted = serde_json::Map::new();
            for (key, mut value) in entries {
                normalize_json(&mut value);
                sorted.insert(key, value);
            }
            *map = sorted;
        }
        serde_json::Value::Array(items) => {
            for item in items {
                normalize_json(item);
            }
        }
        _ => {}
    }
}

fn parse_snapshot_for_test(
    file_id: FileId,
    file_version: i32,
    text: &str,
    changed_ranges: Vec<ParseChangedRange>,
    incremental: bool,
    fallback_reason: Option<&str>,
) -> ParseSnapshot {
    ParseSnapshot {
        file_id,
        file_version,
        parse_result: Arc::new(
            bsl_syntax::parse(text, &ParseOptions::default()).expect("snapshot parse"),
        ),
        line_index: Arc::new(LineIndex::new(text)),
        backend_tree: parse_backend_tree_for_test(text),
        changed_ranges: Arc::new(changed_ranges),
        produced_at_millis: 0,
        backend_tree_hash: 0,
        incremental,
        fallback_reason: fallback_reason.map(Arc::from),
    }
}

#[test]
fn compute_index_fetch_wait_ms_subtracts_parse_and_build_time() {
    assert_eq!(compute_index_fetch_wait_ms(156_207, 0, 97), 156_110);
    assert_eq!(compute_index_fetch_wait_ms(10, 8, 5), 0);
}

#[test]
fn compute_index_fetch_salsa_event_edges_ms_tracks_pre_and_post_edges() {
    assert_eq!(
        compute_index_fetch_salsa_event_edges_ms(200, Some(40), Some(170)),
        (40, 30)
    );
    assert_eq!(
        compute_index_fetch_salsa_event_edges_ms(200, None, None),
        (200, 0)
    );
    assert_eq!(
        compute_index_fetch_salsa_event_edges_ms(200, Some(250), Some(10)),
        (200, 0)
    );
}

#[test]
fn compute_index_fetch_inside_salsa_window_ms_excludes_pre_and_post_edges() {
    assert_eq!(compute_index_fetch_inside_salsa_window_ms(200, 40, 30), 130);
    assert_eq!(compute_index_fetch_inside_salsa_window_ms(200, 200, 0), 0);
    assert_eq!(compute_index_fetch_inside_salsa_window_ms(200, 150, 100), 0);
}

#[test]
fn compute_index_fetch_event_delta_ms_returns_zero_when_marker_is_missing() {
    assert_eq!(
        compute_index_fetch_event_delta_ms(200, Some(30), Some(150)),
        120
    );
    assert_eq!(
        compute_index_fetch_event_delta_ms(200, Some(250), Some(300)),
        0
    );
    assert_eq!(compute_index_fetch_event_delta_ms(200, None, Some(150)), 0);
    assert_eq!(compute_index_fetch_event_delta_ms(200, Some(30), None), 0);
}

#[test]
fn compute_first_type_index_timeline_snapshot_tracks_preceding_gap_markers() {
    let snapshot = compute_first_type_index_timeline_snapshot(
        &[
            SalsaEventTimelineEvent {
                elapsed_ms: 0,
                kind: SalsaEventTimelineEventKind::WillCheckCancellation,
            },
            SalsaEventTimelineEvent {
                elapsed_ms: 7,
                kind: SalsaEventTimelineEventKind::WillExecute(SalsaEventKeyKind::ParseResult),
            },
            SalsaEventTimelineEvent {
                elapsed_ms: 8,
                kind: SalsaEventTimelineEventKind::WillCheckCancellation,
            },
            SalsaEventTimelineEvent {
                elapsed_ms: 120,
                kind: SalsaEventTimelineEventKind::WillExecute(SalsaEventKeyKind::TypeIndex),
            },
            SalsaEventTimelineEvent {
                elapsed_ms: 121,
                kind: SalsaEventTimelineEventKind::WillCheckCancellation,
            },
        ],
        200,
    );

    assert_eq!(
        snapshot.first_will_execute_type_index_elapsed_ms,
        Some(120),
        "first type_index marker must be captured"
    );
    assert_eq!(
        snapshot.last_event_before_first_will_execute_type_index_elapsed_ms,
        Some(8),
        "last event before first type_index must be captured"
    );
    assert_eq!(
        snapshot.last_will_check_before_first_will_execute_type_index_elapsed_ms,
        Some(8),
        "last WillCheck before first type_index must be captured"
    );
    assert_eq!(
        snapshot.last_will_execute_parse_result_before_first_will_execute_type_index_elapsed_ms,
        Some(7),
        "last parse_result before first type_index must be captured"
    );
    assert_eq!(
        snapshot.events_before_first_will_execute_type_index_total,
        3
    );
    assert_eq!(
        snapshot.will_check_before_first_will_execute_type_index_total,
        2
    );
    assert_eq!(
        snapshot.will_execute_parse_result_before_first_will_execute_type_index_total,
        1
    );
    assert_eq!(snapshot.first_will_execute_type_index_seen_total, 1);
}

#[test]
fn compute_first_type_index_timeline_snapshot_handles_absent_type_index() {
    let snapshot = compute_first_type_index_timeline_snapshot(
        &[SalsaEventTimelineEvent {
            elapsed_ms: 55,
            kind: SalsaEventTimelineEventKind::WillCheckCancellation,
        }],
        100,
    );
    assert_eq!(snapshot.first_will_execute_type_index_elapsed_ms, None);
    assert_eq!(snapshot.first_will_execute_type_index_seen_total, 0);
    assert_eq!(
        snapshot.events_before_first_will_execute_type_index_total,
        1
    );
    assert_eq!(
        snapshot.will_check_before_first_will_execute_type_index_total,
        1
    );
}

#[test]
fn compute_index_fetch_block_on_other_total_subtracts_known_query_kinds() {
    assert_eq!(compute_index_fetch_key_kind_other_total(7, 3, 2), 2);
    assert_eq!(compute_index_fetch_key_kind_other_total(2, 3, 2), 0);
}

#[test]
fn revision_to_u64_parses_debug_representation() {
    let host = AnalysisHostV2::default();
    assert!(current_revision_u64(&host.db) >= 1);
}

#[test]
fn format_salsa_event_timeline_handles_disabled_and_empty_states() {
    let disabled = SalsaEventTimelineSnapshot::default();
    assert_eq!(format_salsa_event_timeline(&disabled), "disabled");

    let empty = SalsaEventTimelineSnapshot {
        event_capture_limit: 32,
        ..SalsaEventTimelineSnapshot::default()
    };
    assert_eq!(format_salsa_event_timeline(&empty), "empty");
}

#[test]
fn format_salsa_event_timeline_renders_elapsed_and_delta_per_event() {
    let timeline = SalsaEventTimelineSnapshot {
        event_capture_limit: 32,
        event_total: 3,
        event_truncated: false,
        events: vec![
            SalsaEventTimelineEvent {
                elapsed_ms: 11,
                kind: SalsaEventTimelineEventKind::WillCheckCancellation,
            },
            SalsaEventTimelineEvent {
                elapsed_ms: 35,
                kind: SalsaEventTimelineEventKind::WillExecute(SalsaEventKeyKind::TypeIndex),
            },
            SalsaEventTimelineEvent {
                elapsed_ms: 38,
                kind: SalsaEventTimelineEventKind::DidSetCancellationFlag,
            },
        ],
        ..SalsaEventTimelineSnapshot::default()
    };

    assert_eq!(
            format_salsa_event_timeline(&timeline),
            "11ms:+11:will_check_cancellation|35ms:+24:will_execute(type_index)|38ms:+3:did_set_cancellation_flag"
        );
}

#[test]
fn file_text_and_version_update_after_set_file() {
    let mut host = AnalysisHostV2::default();
    let file_id = FileId(1);

    host.apply_change(Change::SetFile {
        file_id,
        text: Arc::from("abc"),
        version: 1,
        path: Arc::from("test.bsl"),
    });

    {
        let analysis = host.analysis();
        assert_eq!(analysis.file_text(file_id).unwrap().as_deref(), Some("abc"));
        assert_eq!(analysis.file_version(file_id).unwrap(), Some(1));
        assert_eq!(analysis.file_text_len(file_id).unwrap(), Some(3));
    }

    host.apply_change(Change::SetFile {
        file_id,
        text: Arc::from("abcd"),
        version: 2,
        path: Arc::from("test.bsl"),
    });

    {
        let analysis = host.analysis();
        assert_eq!(
            analysis.file_text(file_id).unwrap().as_deref(),
            Some("abcd")
        );
        assert_eq!(analysis.file_version(file_id).unwrap(), Some(2));
        assert_eq!(analysis.file_text_len(file_id).unwrap(), Some(4));
    }
}

#[test]
fn remove_file_makes_queries_return_none() {
    let mut host = AnalysisHostV2::default();
    let file_id = FileId(1);

    host.apply_change(Change::SetFile {
        file_id,
        text: Arc::from("abc"),
        version: 1,
        path: Arc::from("test.bsl"),
    });
    host.apply_change(Change::RemoveFile { file_id });

    let analysis = host.analysis();
    assert_eq!(analysis.file_text(file_id).unwrap(), None);
    assert_eq!(analysis.file_version(file_id).unwrap(), None);
    assert_eq!(analysis.file_text_len(file_id).unwrap(), None);
}

#[test]
fn deps_and_settings_ids_are_read_from_snapshot() {
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    let host = Arc::new(Mutex::new(AnalysisHostV2::default()));
    host.lock().unwrap().apply_change(Change::SetDepsSnapshot {
        deps_id: DepsSnapshotId::from_hash("deps-a"),
        deps: Arc::new(SemanticDeps {
            repository: Arc::new(InMemoryTypeRepository::new()),
            signature_index: SignatureIndex::new(),
            resolver: None,
            platform_signatures_loaded: false,
        }),
    });
    host.lock()
        .unwrap()
        .apply_change(Change::SetSettingsSnapshot {
            settings_id: SettingsId::from_hash("settings-a"),
            diagnostics_detail_level: DetailLevel::Full,
        });

    let analysis_a = host.lock().unwrap().snapshot();
    assert_eq!(analysis_a.deps_id().unwrap().as_str(), "deps-a");
    assert_eq!(analysis_a.settings_id().unwrap().as_str(), "settings-a");

    let (locked_tx, locked_rx) = std::sync::mpsc::channel::<()>();
    let (done_tx, done_rx) = std::sync::mpsc::channel::<()>();
    let host_for_update = host.clone();
    let update_thread = std::thread::spawn(move || {
        let mut host = host_for_update.lock().unwrap();
        locked_tx.send(()).unwrap();
        host.apply_change(Change::SetDepsSnapshot {
            deps_id: DepsSnapshotId::from_hash("deps-b"),
            deps: Arc::new(SemanticDeps {
                repository: Arc::new(InMemoryTypeRepository::new()),
                signature_index: SignatureIndex::new(),
                resolver: None,
                platform_signatures_loaded: false,
            }),
        });
        host.apply_change(Change::SetSettingsSnapshot {
            settings_id: SettingsId::from_hash("settings-b"),
            diagnostics_detail_level: DetailLevel::Full,
        });
        done_tx.send(()).unwrap();
    });

    locked_rx.recv_timeout(Duration::from_secs(1)).unwrap();

    assert!(done_rx.recv_timeout(Duration::from_millis(200)).is_err());
    assert_eq!(analysis_a.deps_id().unwrap().as_str(), "deps-a");
    assert_eq!(analysis_a.settings_id().unwrap().as_str(), "settings-a");

    drop(analysis_a);
    done_rx.recv_timeout(Duration::from_secs(1)).unwrap();
    update_thread.join().unwrap();

    let analysis_b = host.lock().unwrap().snapshot();
    assert_eq!(analysis_b.deps_id().unwrap().as_str(), "deps-b");
    assert_eq!(analysis_b.settings_id().unwrap().as_str(), "settings-b");
}

#[test]
fn line_index_and_positioning_are_read_from_snapshot() {
    let mut host = AnalysisHostV2::default();
    let file_id = FileId(1);

    host.apply_change(Change::SetFile {
        file_id,
        text: Arc::from("abc\ndef"),
        version: 1,
        path: Arc::from("test.bsl"),
    });

    let analysis = host.snapshot();
    let index = analysis.line_index(file_id).unwrap().unwrap();
    assert_eq!(index.line_count(), 2);

    assert_eq!(
        analysis
            .utf16_position_to_byte_offset(file_id, 0, 999)
            .unwrap(),
        Some(3)
    );
    assert_eq!(
        analysis.utf16_position_to_point(file_id, 0, 999).unwrap(),
        Some((0, 3))
    );
}

#[test]
fn set_file_with_snapshot_uses_snapshot_parse_result_and_line_index() {
    let mut host = AnalysisHostV2::default();
    let file_id = FileId(7);
    let text: Arc<str> = Arc::from("Процедура Тест()\nКонецПроцедуры");
    let parsed = Arc::new(
        bsl_syntax::parse(text.as_ref(), &ParseOptions::default()).expect("snapshot parse"),
    );
    let index = Arc::new(LineIndex::new(text.as_ref()));
    let snapshot = ParseSnapshot {
        file_id,
        file_version: 3,
        parse_result: parsed.clone(),
        line_index: index.clone(),
        backend_tree: parse_backend_tree_for_test(text.as_ref()),
        changed_ranges: Arc::new(Vec::new()),
        produced_at_millis: 0,
        backend_tree_hash: 0,
        incremental: false,
        fallback_reason: None,
    };

    host.apply_change(Change::SetFileWithSnapshot {
        file_id,
        text: text.clone(),
        version: 3,
        path: Arc::from("snapshot-test.bsl"),
        parse_snapshot: snapshot,
    });

    let analysis = host.snapshot();
    let parse_result = analysis.parse_result(file_id).unwrap().unwrap();
    let line_index = analysis.line_index(file_id).unwrap().unwrap();

    assert!(Arc::ptr_eq(&parsed, &parse_result));
    assert!(Arc::ptr_eq(&index, &line_index));
}

#[test]
fn set_file_with_snapshot_ignores_mismatched_snapshot_version() {
    let mut host = AnalysisHostV2::default();
    let file_id = FileId(8);
    let text: Arc<str> = Arc::from("Процедура Тест()\nКонецПроцедуры");
    let snapshot_parsed = Arc::new(
        bsl_syntax::parse(text.as_ref(), &ParseOptions::default()).expect("snapshot parse"),
    );
    let snapshot = ParseSnapshot {
        file_id,
        file_version: 99,
        parse_result: snapshot_parsed.clone(),
        line_index: Arc::new(LineIndex::new(text.as_ref())),
        backend_tree: parse_backend_tree_for_test(text.as_ref()),
        changed_ranges: Arc::new(Vec::new()),
        produced_at_millis: 0,
        backend_tree_hash: 0,
        incremental: true,
        fallback_reason: Some(Arc::from("version_mismatch")),
    };

    host.apply_change(Change::SetFileWithSnapshot {
        file_id,
        text,
        version: 1,
        path: Arc::from("snapshot-mismatch.bsl"),
        parse_snapshot: snapshot,
    });

    let analysis = host.snapshot();
    let parsed = analysis.parse_result(file_id).unwrap().unwrap();
    assert!(
        !Arc::ptr_eq(&parsed, &snapshot_parsed),
        "snapshot with mismatched version must not be used"
    );
}

#[test]
fn parse_result_recomputes_when_file_text_changes() {
    let mut host = AnalysisHostV2::default();
    let file_id = FileId(1);

    host.apply_change(Change::SetFile {
        file_id,
        text: Arc::from("Procedure Test()\nEndProcedure"),
        version: 1,
        path: Arc::from("test.bsl"),
    });

    let parsed_a = {
        let analysis = host.snapshot();
        analysis.parse_result(file_id).unwrap().unwrap()
    };

    host.apply_change(Change::SetFile {
        file_id,
        text: Arc::from("Procedure Test(\nEndProcedure"),
        version: 2,
        path: Arc::from("test.bsl"),
    });

    let parsed_b = {
        let analysis = host.snapshot();
        analysis.parse_result(file_id).unwrap().unwrap()
    };

    assert!(!Arc::ptr_eq(&parsed_a, &parsed_b));
    assert!(parsed_a.syntax_errors.is_empty());
    assert!(!parsed_b.syntax_errors.is_empty());
}

#[test]
fn parse_result_recomputes_when_settings_id_changes() {
    let mut host = AnalysisHostV2::default();
    let file_id = FileId(1);

    host.apply_change(Change::SetFile {
        file_id,
        text: Arc::from("Procedure Test()\nEndProcedure"),
        version: 1,
        path: Arc::from("test.bsl"),
    });

    let parsed_a = {
        let analysis = host.snapshot();
        analysis.parse_result(file_id).unwrap().unwrap()
    };

    host.apply_change(Change::SetSettingsSnapshot {
        settings_id: SettingsId::from_hash("settings-b"),
        diagnostics_detail_level: DetailLevel::Full,
    });

    let parsed_b = {
        let analysis = host.snapshot();
        analysis.parse_result(file_id).unwrap().unwrap()
    };

    assert!(!Arc::ptr_eq(&parsed_a, &parsed_b));
}

#[test]
fn remove_file_makes_parse_result_return_none() {
    let mut host = AnalysisHostV2::default();
    let file_id = FileId(1);

    host.apply_change(Change::SetFile {
        file_id,
        text: Arc::from("Procedure Test()\nEndProcedure"),
        version: 1,
        path: Arc::from("test.bsl"),
    });
    host.apply_change(Change::RemoveFile { file_id });

    let analysis = host.snapshot();
    assert!(analysis.parse_result(file_id).unwrap().is_none());
}

#[test]
fn ir_recomputes_when_file_text_changes() {
    let mut host = AnalysisHostV2::default();
    let file_id = FileId(1);

    host.apply_change(Change::SetFile {
        file_id,
        text: Arc::from("Procedure Test()\nEndProcedure"),
        version: 1,
        path: Arc::from("test.bsl"),
    });

    let ir_a = {
        let analysis = host.snapshot();
        analysis.ir(file_id).unwrap().unwrap()
    };

    host.apply_change(Change::SetFile {
        file_id,
        text: Arc::from("Procedure Test(\nEndProcedure"),
        version: 2,
        path: Arc::from("test.bsl"),
    });

    let ir_b = {
        let analysis = host.snapshot();
        analysis.ir(file_id).unwrap().unwrap()
    };

    assert!(!Arc::ptr_eq(&ir_a, &ir_b));
}

#[test]
fn ir_reuses_previous_version_for_tail_whitespace_append_snapshot() {
    let mut host = AnalysisHostV2::default();
    let file_id = FileId(11);
    let text_v1: Arc<str> = Arc::from("Procedure Test()\n    x = 1;\nEndProcedure");

    host.apply_change(Change::SetFile {
        file_id,
        text: text_v1.clone(),
        version: 1,
        path: Arc::from("tail-whitespace.bsl"),
    });

    let ir_v1 = host.analysis().ir(file_id).unwrap().unwrap();

    let text_v2 = Arc::<str>::from(format!("{}\n", text_v1.as_ref()));
    let old_len = text_v1.len() as u32;
    host.apply_change(Change::SetFileWithSnapshot {
        file_id,
        text: text_v2.clone(),
        version: 2,
        path: Arc::from("tail-whitespace.bsl"),
        parse_snapshot: parse_snapshot_for_test(
            file_id,
            2,
            text_v2.as_ref(),
            vec![ParseChangedRange {
                start_byte: old_len,
                old_end_byte: old_len,
                new_end_byte: text_v2.len() as u32,
            }],
            true,
            None,
        ),
    });

    let ir_v2 = host.analysis().ir(file_id).unwrap().unwrap();
    assert!(
        Arc::ptr_eq(&ir_v1, &ir_v2),
        "tail whitespace append must reuse previous IR"
    );
}

#[test]
fn ir_does_not_reuse_previous_version_for_non_tail_snapshot_change() {
    let mut host = AnalysisHostV2::default();
    let file_id = FileId(12);
    let text_v1: Arc<str> = Arc::from("Procedure Test()\n    x = 1;\nEndProcedure");

    host.apply_change(Change::SetFile {
        file_id,
        text: text_v1.clone(),
        version: 1,
        path: Arc::from("non-tail-change.bsl"),
    });

    let ir_v1 = host.analysis().ir(file_id).unwrap().unwrap();

    let edit_start = text_v1.find('1').expect("edit marker") as u32;
    let text_v2: Arc<str> = Arc::from(text_v1.replacen('1', "2", 1));
    host.apply_change(Change::SetFileWithSnapshot {
        file_id,
        text: text_v2.clone(),
        version: 2,
        path: Arc::from("non-tail-change.bsl"),
        parse_snapshot: parse_snapshot_for_test(
            file_id,
            2,
            text_v2.as_ref(),
            vec![ParseChangedRange {
                start_byte: edit_start,
                old_end_byte: edit_start + 1,
                new_end_byte: edit_start + 1,
            }],
            true,
            None,
        ),
    });

    let ir_v2 = host.analysis().ir(file_id).unwrap().unwrap();
    assert!(
        !Arc::ptr_eq(&ir_v1, &ir_v2),
        "non-tail change must trigger full IR recompute"
    );
}

fn build_large_burst_module(marker: u32) -> String {
    let mut text = String::from("Процедура СтрессТест()\n");
    text.push_str("    ЛокМассив = Новый Массив;\n");
    for idx in 0..800_u32 {
        text.push_str(&format!("    ЛокПер{idx} = {idx};\n"));
    }
    text.push_str(&format!("    Маркер = {marker};\n"));
    text.push_str("    ЛокМассив.НесуществующийМетод();\n");
    text.push_str("КонецПроцедуры\n");
    text
}

#[test]
fn large_module_snapshot_edit_burst_preserves_semantic_diagnostics_parity() {
    let file_id = FileId(77);
    let path: Arc<str> = Arc::from("large-burst-parity.bsl");
    let mut host_snapshot = AnalysisHostV2::default();
    let mut host_full = AnalysisHostV2::default();
    let mut current_text = build_large_burst_module(0);

    host_snapshot.apply_change(Change::SetFile {
        file_id,
        text: Arc::from(current_text.clone()),
        version: 1,
        path: path.clone(),
    });
    host_full.apply_change(Change::SetFile {
        file_id,
        text: Arc::from(current_text.clone()),
        version: 1,
        path: path.clone(),
    });

    for step in 1..=16_i32 {
        let previous_marker = format!("    Маркер = {};", step - 1);
        let next_marker = format!("    Маркер = {step};");
        let start = current_text
            .find(&previous_marker)
            .expect("marker from previous step");
        let old_end = start + previous_marker.len();
        let updated_text = current_text.replacen(&previous_marker, &next_marker, 1);
        let new_end = start + next_marker.len();
        let version = step + 1;

        host_snapshot.apply_change(Change::SetFileWithSnapshot {
            file_id,
            text: Arc::from(updated_text.clone()),
            version,
            path: path.clone(),
            parse_snapshot: parse_snapshot_for_test(
                file_id,
                version,
                updated_text.as_ref(),
                vec![ParseChangedRange {
                    start_byte: start as u32,
                    old_end_byte: old_end as u32,
                    new_end_byte: new_end as u32,
                }],
                true,
                None,
            ),
        });
        host_full.apply_change(Change::SetFile {
            file_id,
            text: Arc::from(updated_text.clone()),
            version,
            path: path.clone(),
        });

        let snapshot_analysis = host_snapshot.snapshot();
        let full_analysis = host_full.snapshot();

        let syntax_snapshot = snapshot_analysis
            .syntax_diagnostics(file_id)
            .unwrap()
            .unwrap();
        let syntax_full = full_analysis.syntax_diagnostics(file_id).unwrap().unwrap();
        let mut syntax_snapshot_json =
            serde_json::to_value(syntax_snapshot.as_ref()).expect("serialize snapshot syntax");
        let mut syntax_full_json =
            serde_json::to_value(syntax_full.as_ref()).expect("serialize full syntax");
        normalize_json(&mut syntax_snapshot_json);
        normalize_json(&mut syntax_full_json);
        assert_eq!(
            syntax_snapshot_json, syntax_full_json,
            "syntax diagnostics drift at burst step {step}"
        );

        let semantic_snapshot = snapshot_analysis
            .semantic_diagnostics(file_id)
            .unwrap()
            .unwrap();
        let semantic_full = full_analysis
            .semantic_diagnostics(file_id)
            .unwrap()
            .unwrap();
        let mut semantic_snapshot_json = serde_json::to_value(semantic_snapshot.as_ref())
            .expect("serialize snapshot semantic diagnostics");
        let mut semantic_full_json = serde_json::to_value(semantic_full.as_ref())
            .expect("serialize full semantic diagnostics");
        normalize_json(&mut semantic_snapshot_json);
        normalize_json(&mut semantic_full_json);
        assert_eq!(
            semantic_snapshot_json, semantic_full_json,
            "semantic diagnostics drift at burst step {step}"
        );

        current_text = updated_text;
    }
}

#[test]
fn ir_recomputes_when_deps_id_changes() {
    use salsa::Setter;

    let mut host = AnalysisHostV2::default();
    let file_id = FileId(1);

    host.apply_change(Change::SetFile {
        file_id,
        text: Arc::from("Procedure Test()\nEndProcedure"),
        version: 1,
        path: Arc::from("test.bsl"),
    });

    let ir_a = {
        let analysis = host.snapshot();
        analysis.ir(file_id).unwrap().unwrap()
    };

    host.deps
        .set_id(&mut host.db)
        .to(DepsSnapshotId::from_hash("deps-b"));

    let ir_b = {
        let analysis = host.snapshot();
        analysis.ir(file_id).unwrap().unwrap()
    };

    assert!(!Arc::ptr_eq(&ir_a, &ir_b));
}

#[test]
fn ir_recomputes_when_settings_id_changes() {
    let mut host = AnalysisHostV2::default();
    let file_id = FileId(1);

    host.apply_change(Change::SetFile {
        file_id,
        text: Arc::from("Procedure Test()\nEndProcedure"),
        version: 1,
        path: Arc::from("test.bsl"),
    });

    let ir_a = {
        let analysis = host.snapshot();
        analysis.ir(file_id).unwrap().unwrap()
    };

    host.apply_change(Change::SetSettingsSnapshot {
        settings_id: SettingsId::from_hash("settings-b"),
        diagnostics_detail_level: DetailLevel::Full,
    });

    let ir_b = {
        let analysis = host.snapshot();
        analysis.ir(file_id).unwrap().unwrap()
    };

    assert!(!Arc::ptr_eq(&ir_a, &ir_b));
}

#[test]
fn ir_is_deterministic_for_same_input_across_hosts() {
    let file_id = FileId(1);
    let text: Arc<str> = Arc::from(
        "Procedure Test()\n\
             x = 1;\n\
             x = x + 1;\n\
             EndProcedure",
    );
    let path: Arc<str> = Arc::from("test.bsl");

    let program_a = {
        let mut host = AnalysisHostV2::default();
        host.apply_change(Change::SetFile {
            file_id,
            text: text.clone(),
            version: 1,
            path: path.clone(),
        });
        let analysis = host.snapshot();
        analysis.ir(file_id).unwrap().unwrap()
    };

    let program_b = {
        let mut host = AnalysisHostV2::default();
        host.apply_change(Change::SetFile {
            file_id,
            text: text.clone(),
            version: 1,
            path: path.clone(),
        });
        let analysis = host.snapshot();
        analysis.ir(file_id).unwrap().unwrap()
    };

    let mut json_a = serde_json::to_value(&*program_a).expect("serialize SemanticProgram");
    let mut json_b = serde_json::to_value(&*program_b).expect("serialize SemanticProgram");
    normalize_json(&mut json_a);
    normalize_json(&mut json_b);
    assert_eq!(json_a, json_b);
}

#[test]
fn remove_file_makes_ir_return_none() {
    let mut host = AnalysisHostV2::default();
    let file_id = FileId(1);

    host.apply_change(Change::SetFile {
        file_id,
        text: Arc::from("Procedure Test()\nEndProcedure"),
        version: 1,
        path: Arc::from("test.bsl"),
    });
    host.apply_change(Change::RemoveFile { file_id });

    let analysis = host.snapshot();
    assert!(analysis.ir(file_id).unwrap().is_none());
}

#[test]
fn syntax_diagnostics_are_read_from_parse_result() {
    let file_id = FileId(1);

    let syntax_a = {
        let mut host = AnalysisHostV2::default();
        host.apply_change(Change::SetFile {
            file_id,
            text: Arc::from("Procedure Test(\nEndProcedure"),
            version: 1,
            path: Arc::from("test.bsl"),
        });
        let analysis = host.snapshot();
        analysis.syntax_diagnostics(file_id).unwrap().unwrap()
    };

    let syntax_b = {
        let mut host = AnalysisHostV2::default();
        host.apply_change(Change::SetFile {
            file_id,
            text: Arc::from("Procedure Test(\nEndProcedure"),
            version: 1,
            path: Arc::from("test.bsl"),
        });
        let analysis = host.snapshot();
        analysis.syntax_diagnostics(file_id).unwrap().unwrap()
    };

    assert!(!syntax_a.is_empty());
    let json_a = serde_json::to_string(&*syntax_a).expect("serialize syntax diagnostics");
    let json_b = serde_json::to_string(&*syntax_b).expect("serialize syntax diagnostics");
    assert_eq!(json_a, json_b);
}

#[test]
fn syntax_diagnostics_observability_mode_uses_version_bound_parse_snapshot() {
    let file_id = FileId(1);
    let path: Arc<str> = Arc::from("test.bsl");
    let text: Arc<str> = Arc::from("Procedure Test()\nEndProcedure");

    let reused_mode = {
        let mut host = AnalysisHostV2::default();
        host.apply_change(Change::SetFileWithSnapshot {
            file_id,
            text: text.clone(),
            version: 1,
            path: path.clone(),
            parse_snapshot: parse_snapshot_for_test(
                file_id,
                1,
                text.as_ref(),
                Vec::new(),
                true,
                None,
            ),
        });
        let analysis = host.snapshot();
        analysis
            .syntax_diagnostics_observability_mode(file_id)
            .unwrap()
            .expect("mode for snapshot-backed file")
    };
    assert_eq!(reused_mode, "reused");

    let incremental_mode = {
        let mut host = AnalysisHostV2::default();
        host.apply_change(Change::SetFileWithSnapshot {
            file_id,
            text: text.clone(),
            version: 1,
            path: path.clone(),
            parse_snapshot: parse_snapshot_for_test(
                file_id,
                1,
                text.as_ref(),
                vec![ParseChangedRange {
                    start_byte: 0,
                    old_end_byte: 0,
                    new_end_byte: 1,
                }],
                true,
                None,
            ),
        });
        let analysis = host.snapshot();
        analysis
            .syntax_diagnostics_observability_mode(file_id)
            .unwrap()
            .expect("mode for incremental snapshot")
    };
    assert_eq!(incremental_mode, "incremental");

    let full_mode = {
        let mut host = AnalysisHostV2::default();
        host.apply_change(Change::SetFileWithSnapshot {
            file_id,
            text: text.clone(),
            version: 1,
            path: path.clone(),
            parse_snapshot: parse_snapshot_for_test(
                file_id,
                1,
                text.as_ref(),
                Vec::new(),
                false,
                None,
            ),
        });
        let analysis = host.snapshot();
        analysis
            .syntax_diagnostics_observability_mode(file_id)
            .unwrap()
            .expect("mode for full snapshot")
    };
    assert_eq!(full_mode, "full");

    let other_mode = {
        let mut host = AnalysisHostV2::default();
        host.apply_change(Change::SetFile {
            file_id,
            text,
            version: 2,
            path,
        });
        let analysis = host.snapshot();
        analysis
            .syntax_diagnostics_observability_mode(file_id)
            .unwrap()
            .expect("mode for file without snapshot")
    };
    assert_eq!(other_mode, "other");
}

#[test]
fn semantic_diagnostics_skip_when_syntax_errors_present() {
    let mut host = AnalysisHostV2::default();
    let file_id = FileId(1);

    host.apply_change(Change::SetFile {
        file_id,
        text: Arc::from("Procedure Test(\nEndProcedure"),
        version: 1,
        path: Arc::from("test.bsl"),
    });

    let analysis = host.snapshot();
    let semantic = analysis.semantic_diagnostics(file_id).unwrap().unwrap();
    assert!(semantic.is_empty());
}

#[test]
fn semantic_diagnostics_depend_on_deps_id() {
    let mut host = AnalysisHostV2::default();
    let file_id = FileId(1);

    host.apply_change(Change::SetFile {
        file_id,
        text: Arc::from("Procedure Test()\nEndProcedure"),
        version: 1,
        path: Arc::from("test.bsl"),
    });

    let repository = Arc::new(InMemoryTypeRepository::new()) as Arc<dyn TypeRepository>;
    let platform_signatures_loaded = repository.platform_docs_loaded();
    let deps = Arc::new(SemanticDeps {
        signature_index: repository.get_signature_index_clone(),
        resolver: Some(Arc::new(TypeResolver::new(repository.clone()))),
        repository,
        platform_signatures_loaded,
    });

    host.apply_change(Change::SetDepsSnapshot {
        deps_id: DepsSnapshotId::from_hash("deps-a"),
        deps: deps.clone(),
    });
    let diagnostics_a = host
        .analysis()
        .semantic_diagnostics(file_id)
        .unwrap()
        .unwrap();

    host.apply_change(Change::SetDepsSnapshot {
        deps_id: DepsSnapshotId::from_hash("deps-b"),
        deps,
    });
    let diagnostics_b = host
        .analysis()
        .semantic_diagnostics(file_id)
        .unwrap()
        .unwrap();

    assert!(
        !Arc::ptr_eq(&diagnostics_a, &diagnostics_b),
        "semantic diagnostics should be recomputed when deps_id changes"
    );
}

#[test]
fn semantic_diagnostics_respect_diagnostics_detail_level() {
    let mut host = AnalysisHostV2::default();
    let file_id = FileId(1);

    host.apply_change(Change::SetFile {
        file_id,
        text: Arc::from(
            "Procedure Test()\n\
                 x = 1;\n\
                 x.UnknownMethod();\n\
                 EndProcedure",
        ),
        version: 1,
        path: Arc::from("test.bsl"),
    });

    host.apply_change(Change::SetSettingsSnapshot {
        settings_id: SettingsId::from_hash("settings-compact"),
        diagnostics_detail_level: DetailLevel::Compact,
    });
    let compact = host
        .analysis()
        .semantic_diagnostics(file_id)
        .unwrap()
        .unwrap();
    assert!(!compact.is_empty());

    host.apply_change(Change::SetSettingsSnapshot {
        settings_id: SettingsId::from_hash("settings-detailed"),
        diagnostics_detail_level: DetailLevel::Detailed,
    });
    let detailed = host
        .analysis()
        .semantic_diagnostics(file_id)
        .unwrap()
        .unwrap();
    assert_eq!(compact.len(), detailed.len());
    assert_ne!(compact[0].message, detailed[0].message);
}

#[test]
fn semantic_diagnostics_do_not_include_flow_sensitive_null_safety_by_default() {
    let mut host = AnalysisHostV2::default();
    let file_id = FileId(1);

    host.apply_change(Change::SetFile {
        file_id,
        text: Arc::from(
            "Procedure Test()\n\
                 x = Null;\n\
                 x.Method();\n\
                 EndProcedure",
        ),
        version: 1,
        path: Arc::from("test.bsl"),
    });

    let repository = Arc::new(InMemoryTypeRepository::new()) as Arc<dyn TypeRepository>;
    let platform_signatures_loaded = repository.platform_docs_loaded();
    let deps = Arc::new(SemanticDeps {
        signature_index: repository.get_signature_index_clone(),
        resolver: Some(Arc::new(TypeResolver::new(repository.clone()))),
        repository,
        platform_signatures_loaded,
    });

    host.apply_change(Change::SetDepsSnapshot {
        deps_id: DepsSnapshotId::from_hash("deps"),
        deps,
    });

    let base = host
        .analysis()
        .semantic_diagnostics(file_id)
        .unwrap()
        .unwrap();

    assert!(
        base.iter().all(|d| !d.message.contains("может быть Null")),
        "base diagnostics unexpectedly contain flow-sensitive null-safety: {:?}",
        base
    );

    let flow = host
        .analysis()
        .semantic_diagnostics_flow_sensitive(file_id)
        .unwrap()
        .unwrap();

    assert!(
        flow.iter().any(|d| d.message.contains("может быть Null")),
        "flow-sensitive diagnostics should contain null-safety warning: {:?}",
        flow
    );
}

#[test]
fn semantic_diagnostics_use_current_file_text_even_with_stale_parse_snapshot_program() {
    let mut host = AnalysisHostV2::default();
    let file_id = FileId(310);
    let text: Arc<str> = Arc::from(
        "Procedure Test()\n\
             x = 1;\n\
             x.UnknownMethod();\n\
             EndProcedure",
    );

    host.apply_change(Change::SetDepsSnapshot {
        deps_id: DepsSnapshotId::from_hash("deps-semantic-snapshot-mismatch"),
        deps: default_semantic_deps(),
    });
    host.apply_change(Change::SetFileWithSnapshot {
        file_id,
        text: text.clone(),
        version: 1,
        path: Arc::from("semantic-snapshot-mismatch.bsl"),
        parse_snapshot: parse_snapshot_for_test(
            file_id,
            1,
            "Procedure Safe()\nEndProcedure",
            Vec::new(),
            true,
            None,
        ),
    });

    let diagnostics = host
        .analysis()
        .semantic_diagnostics(file_id)
        .unwrap()
        .unwrap();

    assert!(
        diagnostics
            .iter()
            .any(|diag| diag.message.contains("UnknownMethod")),
        "semantic diagnostics must follow current file text, got: {:?}",
        diagnostics
    );
}

#[test]
fn flow_sensitive_diagnostics_use_current_file_text_even_with_stale_parse_snapshot_program() {
    let mut host = AnalysisHostV2::default();
    let file_id = FileId(311);
    let text: Arc<str> = Arc::from(
        "Procedure Test()\n\
             x = Null;\n\
             x.Method();\n\
             EndProcedure",
    );

    host.apply_change(Change::SetDepsSnapshot {
        deps_id: DepsSnapshotId::from_hash("deps-flow-snapshot-mismatch"),
        deps: default_semantic_deps(),
    });
    host.apply_change(Change::SetFileWithSnapshot {
        file_id,
        text,
        version: 1,
        path: Arc::from("flow-semantic-snapshot-mismatch.bsl"),
        parse_snapshot: parse_snapshot_for_test(
            file_id,
            1,
            "Procedure Safe()\nEndProcedure",
            Vec::new(),
            true,
            None,
        ),
    });

    let diagnostics = host
        .analysis()
        .semantic_diagnostics_flow_sensitive(file_id)
        .unwrap()
        .unwrap();

    assert!(
        diagnostics
            .iter()
            .any(|diag| diag.message.contains("может быть Null")),
        "flow-sensitive diagnostics must follow current file text, got: {:?}",
        diagnostics
    );
}

fn marker_offset(text: &str, marker: &str) -> u32 {
    text.find(marker)
        .unwrap_or_else(|| panic!("marker `{marker}` not found"))
        .min(u32::MAX as usize) as u32
}

fn marker_tail_offset(text: &str, marker: &str) -> u32 {
    text.find(marker)
        .map(|idx| idx + marker.len() - 1)
        .unwrap_or_else(|| panic!("marker `{marker}` not found"))
        .min(u32::MAX as usize) as u32
}

fn default_semantic_deps() -> Arc<SemanticDeps> {
    let repository = Arc::new(InMemoryTypeRepository::new()) as Arc<dyn TypeRepository>;
    let platform_signatures_loaded = repository.platform_docs_loaded();
    Arc::new(SemanticDeps {
        signature_index: repository.get_signature_index_clone(),
        resolver: Some(Arc::new(TypeResolver::new(repository.clone()))),
        repository,
        platform_signatures_loaded,
    })
}

fn universal_collection_semantic_deps() -> Arc<SemanticDeps> {
    let repository_impl = Arc::new(InMemoryTypeRepository::new());
    repository_impl
        .load_types(vec![
            bsl_shared::domain::types::RawTypeData {
                name: "Соответствие".to_string(),
                source: bsl_shared::domain::types::RawDataSource::Platform,
                ..Default::default()
            },
            bsl_shared::domain::types::RawTypeData {
                name: "Структура".to_string(),
                source: bsl_shared::domain::types::RawDataSource::Platform,
                ..Default::default()
            },
            bsl_shared::domain::types::RawTypeData {
                name: "ТаблицаЗначений".to_string(),
                source: bsl_shared::domain::types::RawDataSource::Platform,
                properties: vec![bsl_shared::domain::types::RawPropertyData {
                    name: "Колонки".to_string(),
                    prop_type: "КоллекцияКолонокТаблицыЗначений".to_string(),
                    is_readonly: false,
                }],
                ..Default::default()
            },
            bsl_shared::domain::types::RawTypeData {
                name: "КоллекцияКолонокТаблицыЗначений".to_string(),
                source: bsl_shared::domain::types::RawDataSource::Platform,
                ..Default::default()
            },
            bsl_shared::domain::types::RawTypeData {
                name: "СтрокаТаблицыЗначений".to_string(),
                source: bsl_shared::domain::types::RawDataSource::Platform,
                ..Default::default()
            },
            bsl_shared::domain::types::RawTypeData {
                name: "ОписаниеТипов".to_string(),
                source: bsl_shared::domain::types::RawDataSource::Platform,
                ..Default::default()
            },
        ])
        .expect("load universal collection types");

    let repository =
        repository_impl.clone() as Arc<dyn bsl_shared::domain::repository::TypeRepository>;
    Arc::new(SemanticDeps {
        signature_index: SignatureIndex::new(),
        resolver: Some(Arc::new(TypeResolver::new(repository.clone()))),
        repository,
        platform_signatures_loaded: true,
    })
}

#[test]
fn serve_only_exact_hit_matches_legacy_for_same_snapshot() {
    let mut host = AnalysisHostV2::default();
    let file_id = FileId(201);
    let text: Arc<str> = Arc::from(
        "Procedure Test()\n\
             x = 1;\n\
             x = x + 1;\n\
             EndProcedure",
    );

    host.apply_change(Change::SetFileWithSnapshot {
        file_id,
        text: text.clone(),
        version: 1,
        path: Arc::from("serve-only-exact-hit.bsl"),
        parse_snapshot: parse_snapshot_for_test(file_id, 1, text.as_ref(), Vec::new(), true, None),
    });

    let analysis = host.snapshot();
    let probe = marker_offset(text.as_ref(), "x = x + 1;");
    let legacy = analysis
        .type_at_byte_offset(file_id, probe)
        .expect("legacy type lookup");
    let precompute = analysis
        .precompute_type_index_for_file(file_id, Some(1), 7)
        .expect("precompute");
    assert_eq!(
        precompute.reason_code,
        TypeIndexPrecomputeReasonCode::TypeIndexPrecomputeExactStored
    );
    assert_eq!(precompute.file_version, Some(1));

    let serve_only = analysis
        .type_at_byte_offset_serve_only_profiled(file_id, probe)
        .expect("serve-only lookup");
    assert_eq!(
        serve_only.serve_reason_code,
        TypeIndexServeReasonCode::TypeIndexExactHit
    );
    assert_eq!(serve_only.resolution, legacy);
}

#[test]
fn serve_only_matches_legacy_for_universal_collections_in_complete_snapshot() {
    let mut host = AnalysisHostV2::default();
    let file_id = FileId(208);
    let text: Arc<str> = Arc::from(
        "Процедура Тест()\n\
             Map = Новый Соответствие;\n\
             Map.Вставить(\"k\", Новый ТаблицаЗначений);\n\
             S = Новый Структура;\n\
             S.Вставить(\"Идентификатор\", \"A-01\");\n\
             ТЗ = Новый ТаблицаЗначений;\n\
             ТЗ.Колонки.Добавить(\"Идентификатор\", Новый ОписаниеТипов(\"Строка\"));\n\
             Стр = ТЗ.Добавить();\n\
             ЗначДляMap = Map[\"k\"];\n\
             ЗначДляСтруктуры = S;\n\
             ЗначДляСтроки = Стр;\n\
             КонецПроцедуры",
    );

    host.apply_change(Change::SetDepsSnapshot {
        deps_id: DepsSnapshotId::from_hash("deps-universal-complete"),
        deps: universal_collection_semantic_deps(),
    });
    host.apply_change(Change::SetFileWithSnapshot {
        file_id,
        text: text.clone(),
        version: 1,
        path: Arc::from("serve-only-universal-complete.bsl"),
        parse_snapshot: parse_snapshot_for_test(file_id, 1, text.as_ref(), Vec::new(), true, None),
    });

    let analysis = host.snapshot();
    let precompute = analysis
        .precompute_type_index_for_file(file_id, Some(1), 0)
        .expect("precompute universal complete");
    assert_eq!(
        precompute.reason_code,
        TypeIndexPrecomputeReasonCode::TypeIndexPrecomputeExactStored
    );

    for (label, probe) in [
        ("map", marker_tail_offset(text.as_ref(), "Map[\"k\"]")),
        (
            "structure",
            marker_tail_offset(text.as_ref(), "ЗначДляСтруктуры = S"),
        ),
        (
            "row",
            marker_tail_offset(text.as_ref(), "ЗначДляСтроки = Стр"),
        ),
    ] {
        let legacy = analysis
            .type_at_byte_offset(file_id, probe)
            .unwrap_or_else(|_| panic!("legacy type lookup for {label}"))
            .unwrap_or_else(|| panic!("legacy type result for {label}"));
        let serve_only = analysis
            .type_at_byte_offset_serve_only_profiled(file_id, probe)
            .unwrap_or_else(|_| panic!("serve-only lookup for {label}"));
        assert_eq!(
            serve_only.serve_reason_code,
            TypeIndexServeReasonCode::TypeIndexExactHit,
            "serve-only must hit exact artifact for {label}"
        );
        assert_eq!(
            serve_only.resolution,
            Some(legacy),
            "serve-only must match legacy resolution for {label}"
        );
    }
}

#[test]
fn serve_only_matches_legacy_for_universal_collections_with_incomplete_member_access() {
    let mut host = AnalysisHostV2::default();
    let file_id = FileId(209);
    let text: Arc<str> = Arc::from(
        "Процедура Тест()\n\
             Map = Новый Соответствие;\n\
             Map.Вставить(\"k\", Новый ТаблицаЗначений);\n\
             S = Новый Структура;\n\
             S.Вставить(\"Идентификатор\", \"A-01\");\n\
             ТЗ = Новый ТаблицаЗначений;\n\
             ТЗ.Колонки.Добавить(\"Идентификатор\", Новый ОписаниеТипов(\"Строка\"));\n\
             Стр = ТЗ.Добавить();\n\
             ДляCompletionMap = Map[\"k\"].\n\
             ДляCompletionСтруктуры = S.\n\
             ДляCompletionСтроки = Стр.\n\
             КонецПроцедуры",
    );

    host.apply_change(Change::SetDepsSnapshot {
        deps_id: DepsSnapshotId::from_hash("deps-universal-incomplete"),
        deps: universal_collection_semantic_deps(),
    });
    host.apply_change(Change::SetFileWithSnapshot {
        file_id,
        text: text.clone(),
        version: 1,
        path: Arc::from("serve-only-universal-incomplete.bsl"),
        parse_snapshot: parse_snapshot_for_test(file_id, 1, text.as_ref(), Vec::new(), true, None),
    });

    let analysis = host.snapshot();
    let precompute = analysis
        .precompute_type_index_for_file(file_id, Some(1), 0)
        .expect("precompute universal incomplete");
    assert_eq!(
        precompute.reason_code,
        TypeIndexPrecomputeReasonCode::TypeIndexPrecomputeExactStored
    );

    for (label, probe) in [
        ("map", marker_tail_offset(text.as_ref(), "Map[\"k\"]")),
        (
            "structure",
            marker_tail_offset(text.as_ref(), "ДляCompletionСтруктуры = S"),
        ),
        (
            "row",
            marker_tail_offset(text.as_ref(), "ДляCompletionСтроки = Стр"),
        ),
    ] {
        let legacy = analysis
            .type_at_byte_offset(file_id, probe)
            .unwrap_or_else(|_| panic!("legacy type lookup for {label}"))
            .unwrap_or_else(|| panic!("legacy type result for {label}"));
        let serve_only = analysis
            .type_at_byte_offset_serve_only_profiled(file_id, probe)
            .unwrap_or_else(|_| panic!("serve-only lookup for {label}"));
        assert_eq!(
            serve_only.serve_reason_code,
            TypeIndexServeReasonCode::TypeIndexExactHit,
            "serve-only must stay exact for incomplete member access on {label}"
        );
        assert_eq!(
            serve_only.resolution,
            Some(legacy),
            "serve-only must match legacy resolution for incomplete member access on {label}"
        );
    }
}

#[test]
fn serve_only_matches_legacy_for_short_map_index_incomplete_member_access() {
    let mut host = AnalysisHostV2::default();
    let file_id = FileId(210);
    let text: Arc<str> = Arc::from(
        "Процедура Тест()\n\
             Map = Новый Соответствие;\n\
             Map.Вставить(\"k\", Новый ТаблицаЗначений);\n\
             ДляCompletion = Map[\"k\"].\n\
             КонецПроцедуры",
    );

    host.apply_change(Change::SetDepsSnapshot {
        deps_id: DepsSnapshotId::from_hash("deps-universal-short-map-incomplete"),
        deps: universal_collection_semantic_deps(),
    });
    host.apply_change(Change::SetFileWithSnapshot {
        file_id,
        text: text.clone(),
        version: 1,
        path: Arc::from("serve-only-universal-short-map-incomplete.bsl"),
        parse_snapshot: parse_snapshot_for_test(file_id, 1, text.as_ref(), Vec::new(), true, None),
    });

    let analysis = host.snapshot();
    let probe = marker_tail_offset(text.as_ref(), "Map[\"k\"]");
    let legacy = analysis
        .type_at_byte_offset(file_id, probe)
        .expect("legacy type lookup")
        .expect("legacy type result");
    let precompute = analysis
        .precompute_type_index_for_file(file_id, Some(1), 0)
        .expect("precompute short map incomplete");
    assert_eq!(
        precompute.reason_code,
        TypeIndexPrecomputeReasonCode::TypeIndexPrecomputeExactStored
    );

    let serve_only = analysis
        .type_at_byte_offset_serve_only_profiled(file_id, probe)
        .expect("serve-only lookup");
    assert_eq!(
        serve_only.serve_reason_code,
        TypeIndexServeReasonCode::TypeIndexExactHit,
        "serve-only must stay exact for short map incomplete member access"
    );
    assert_eq!(
        legacy.type_name(),
        "ТаблицаЗначений",
        "legacy type lookup must keep exact map value type on short incomplete fixture"
    );
    assert_eq!(
        serve_only.resolution,
        Some(legacy),
        "serve-only must match legacy resolution on short map incomplete fixture"
    );
}

#[test]
fn universal_collection_snapshot_switch_does_not_leak_schema_effects() {
    let mut host = AnalysisHostV2::default();
    let file_id = FileId(211);
    let text_v1: Arc<str> = Arc::from(
        "Процедура Тест()\n\
             Map = Новый Соответствие;\n\
             Map.Вставить(\"k\", Новый ТаблицаЗначений);\n\
             S = Новый Структура;\n\
             S.Вставить(\"Идентификатор\", \"A-01\");\n\
             ТЗ = Новый ТаблицаЗначений;\n\
             ТЗ.Колонки.Добавить(\"Идентификатор\", Новый ОписаниеТипов(\"Строка\"));\n\
             Стр = ТЗ.Добавить();\n\
             ЗначДляMap = Map[\"k\"];\n\
             ЗначДляСтруктуры = S;\n\
             ЗначДляСтроки = Стр;\n\
             КонецПроцедуры",
    );
    let text_v2: Arc<str> = Arc::from(
        "Процедура Тест()\n\
             Map = Новый Соответствие;\n\
             S = Новый Структура;\n\
             ТЗ = Новый ТаблицаЗначений;\n\
             Стр = ТЗ.Добавить();\n\
             ЗначДляMap = Map[\"k\"];\n\
             ЗначДляСтруктуры = S;\n\
             ЗначДляСтроки = Стр;\n\
             КонецПроцедуры",
    );

    host.apply_change(Change::SetDepsSnapshot {
        deps_id: DepsSnapshotId::from_hash("deps-universal-switch"),
        deps: universal_collection_semantic_deps(),
    });
    host.apply_change(Change::SetFileWithSnapshot {
        file_id,
        text: text_v1.clone(),
        version: 1,
        path: Arc::from("serve-only-universal-switch-v1.bsl"),
        parse_snapshot: parse_snapshot_for_test(
            file_id,
            1,
            text_v1.as_ref(),
            Vec::new(),
            true,
            None,
        ),
    });
    {
        let analysis_v1 = host.snapshot();
        let precompute = analysis_v1
            .precompute_type_index_for_file(file_id, Some(1), 0)
            .expect("precompute universal switch v1");
        assert_eq!(
            precompute.reason_code,
            TypeIndexPrecomputeReasonCode::TypeIndexPrecomputeExactStored
        );
    }

    host.apply_change(Change::SetFileWithSnapshot {
        file_id,
        text: text_v2.clone(),
        version: 2,
        path: Arc::from("serve-only-universal-switch-v2.bsl"),
        parse_snapshot: parse_snapshot_for_test(
            file_id,
            2,
            text_v2.as_ref(),
            Vec::new(),
            true,
            None,
        ),
    });

    let analysis_v2 = host.snapshot();
    let precompute = analysis_v2
        .precompute_type_index_for_file(file_id, Some(2), 0)
        .expect("precompute universal switch v2");
    assert_eq!(
        precompute.reason_code,
        TypeIndexPrecomputeReasonCode::TypeIndexPrecomputeExactStored
    );

    let map_probe = marker_tail_offset(text_v2.as_ref(), "Map[\"k\"]");
    let map_legacy = analysis_v2
        .type_at_byte_offset(file_id, map_probe)
        .expect("legacy map lookup")
        .expect("legacy map result");
    let map_serve_only = analysis_v2
        .type_at_byte_offset_serve_only_profiled(file_id, map_probe)
        .expect("serve-only map lookup");
    assert_eq!(
        map_serve_only.serve_reason_code,
        TypeIndexServeReasonCode::TypeIndexExactHit
    );
    assert_eq!(map_serve_only.resolution, Some(map_legacy.clone()));
    assert_eq!(
        map_legacy.type_name(),
        "Произвольный",
        "new snapshot without map effects must not leak previous value specialization"
    );

    let structure_probe = marker_tail_offset(text_v2.as_ref(), "ЗначДляСтруктуры = S");
    let structure_legacy = analysis_v2
        .type_at_byte_offset(file_id, structure_probe)
        .expect("legacy structure lookup")
        .expect("legacy structure result");
    let structure_serve_only = analysis_v2
        .type_at_byte_offset_serve_only_profiled(file_id, structure_probe)
        .expect("serve-only structure lookup");
    assert_eq!(
        structure_serve_only.serve_reason_code,
        TypeIndexServeReasonCode::TypeIndexExactHit
    );
    let structure_resolution = structure_serve_only
        .resolution
        .expect("serve-only structure resolution");
    assert_eq!(structure_resolution, structure_legacy);
    assert!(
        structure_resolution
            .find_structural_member("идентификатор")
            .is_none(),
        "new snapshot without inserts must not leak previous structure field"
    );

    let row_probe = marker_tail_offset(text_v2.as_ref(), "ЗначДляСтроки = Стр");
    let row_legacy = analysis_v2
        .type_at_byte_offset(file_id, row_probe)
        .expect("legacy row lookup")
        .expect("legacy row result");
    let row_serve_only = analysis_v2
        .type_at_byte_offset_serve_only_profiled(file_id, row_probe)
        .expect("serve-only row lookup");
    assert_eq!(
        row_serve_only.serve_reason_code,
        TypeIndexServeReasonCode::TypeIndexExactHit
    );
    let row_resolution = row_serve_only
        .resolution
        .expect("serve-only row resolution");
    assert_eq!(row_resolution, row_legacy);
    assert!(
        row_resolution
            .find_structural_member("идентификатор")
            .is_none(),
        "new snapshot without column effects must not leak previous typed-row column"
    );
}

#[test]
fn serve_only_fails_closed_for_new_version_without_exact_artifact() {
    let mut host = AnalysisHostV2::default();
    let file_id = FileId(202);
    let text_v1: Arc<str> = Arc::from(
        "Procedure Test()\n\
             x = 1;\n\
             x = x + 1;\n\
             EndProcedure",
    );
    let text_v2: Arc<str> = Arc::from(
        "Procedure Test()\n\
             x = 2;\n\
             x = x + 1;\n\
             EndProcedure",
    );

    host.apply_change(Change::SetFileWithSnapshot {
        file_id,
        text: text_v1.clone(),
        version: 1,
        path: Arc::from("serve-only-stale-v1.bsl"),
        parse_snapshot: parse_snapshot_for_test(
            file_id,
            1,
            text_v1.as_ref(),
            Vec::new(),
            true,
            None,
        ),
    });
    {
        let analysis_v1 = host.snapshot();
        let precompute = analysis_v1
            .precompute_type_index_for_file(file_id, Some(1), 0)
            .expect("precompute v1");
        assert_eq!(
            precompute.reason_code,
            TypeIndexPrecomputeReasonCode::TypeIndexPrecomputeExactStored
        );
    }

    host.apply_change(Change::SetFile {
        file_id,
        text: text_v2.clone(),
        version: 2,
        path: Arc::from("serve-only-stale-v2.bsl"),
    });

    let analysis_v2 = host.snapshot();
    let probe = marker_offset(text_v2.as_ref(), "x = x + 1;");
    let serve_only = analysis_v2
        .type_at_byte_offset_serve_only_profiled(file_id, probe)
        .expect("serve-only stale lookup");
    assert_eq!(
        serve_only.serve_reason_code,
        TypeIndexServeReasonCode::TypeIndexFallbackUnavailable
    );
    assert!(
        serve_only.resolution.is_none(),
        "serve-only must fail closed instead of serving stale artifact"
    );
}

#[test]
fn serve_only_fails_closed_for_snapshot_fallback_artifact() {
    let mut host = AnalysisHostV2::default();
    let file_id = FileId(203);
    let text: Arc<str> = Arc::from(
        "Procedure Test()\n\
             x = 1;\n\
             x = x + 1;\n\
             EndProcedure",
    );

    host.apply_change(Change::SetFileWithSnapshot {
        file_id,
        text: text.clone(),
        version: 1,
        path: Arc::from("serve-only-degraded.bsl"),
        parse_snapshot: parse_snapshot_for_test(
            file_id,
            1,
            text.as_ref(),
            vec![ParseChangedRange {
                start_byte: 0,
                old_end_byte: 0,
                new_end_byte: 0,
            }],
            true,
            Some("incremental_fallback"),
        ),
    });

    let analysis = host.snapshot();
    analysis
        .precompute_type_index_for_file(file_id, Some(1), 0)
        .expect("precompute degraded artifact");
    let probe = marker_offset(text.as_ref(), "x = x + 1;");
    let serve_only = analysis
        .type_at_byte_offset_serve_only_profiled(file_id, probe)
        .expect("serve-only degraded lookup");
    assert_eq!(
        serve_only.serve_reason_code,
        TypeIndexServeReasonCode::TypeIndexFallbackUnavailable
    );
    assert!(
        serve_only.resolution.is_none(),
        "serve-only must fail closed instead of serving degraded artifact"
    );
}

#[test]
fn serve_only_fallback_unavailable_on_miss_without_sync_compute() {
    let mut host = AnalysisHostV2::default();
    let file_id = FileId(204);
    let text: Arc<str> = Arc::from(
        "Procedure Test()\n\
             x = 1;\n\
             x = x + 1;\n\
             EndProcedure",
    );

    host.apply_change(Change::SetFile {
        file_id,
        text: text.clone(),
        version: 1,
        path: Arc::from("serve-only-miss.bsl"),
    });

    let analysis = host.snapshot();
    let probe = marker_offset(text.as_ref(), "x = x + 1;");
    let serve_only = analysis
        .type_at_byte_offset_serve_only_profiled(file_id, probe)
        .expect("serve-only miss");
    assert_eq!(
        serve_only.serve_reason_code,
        TypeIndexServeReasonCode::TypeIndexFallbackUnavailable
    );
    assert!(
        serve_only.resolution.is_none(),
        "cache miss must not return synthetic on-demand resolution"
    );
    assert_eq!(
        serve_only.profile.index_fetch_will_execute_type_index_total, 0,
        "serve-only miss must not execute salsa type_index query"
    );
    assert_eq!(
        serve_only
            .profile
            .index_fetch_will_execute_parse_result_total,
        0,
        "serve-only miss must not execute salsa parse_result query"
    );
}

#[test]
fn precompute_returns_superseded_when_expected_version_is_stale() {
    let mut host = AnalysisHostV2::default();
    let file_id = FileId(205);

    host.apply_change(Change::SetFile {
        file_id,
        text: Arc::from("Procedure Test()\nEndProcedure"),
        version: 1,
        path: Arc::from("precompute-superseded-v1.bsl"),
    });
    host.apply_change(Change::SetFile {
        file_id,
        text: Arc::from("Procedure Test()\nx = 1;\nEndProcedure"),
        version: 2,
        path: Arc::from("precompute-superseded-v2.bsl"),
    });

    let analysis = host.snapshot();
    let precompute = analysis
        .precompute_type_index_for_file(file_id, Some(1), 11)
        .expect("precompute superseded");
    assert_eq!(
        precompute.reason_code,
        TypeIndexPrecomputeReasonCode::TypeIndexPrecomputeSuperseded
    );
    assert_eq!(precompute.file_version, Some(2));
    assert_eq!(precompute.stats.queue_wait_ms, 11);
}

#[test]
fn deps_and_settings_switch_invalidate_type_index_artifacts() {
    let mut host = AnalysisHostV2::default();
    let file_id = FileId(206);
    let text: Arc<str> = Arc::from(
        "Procedure Test()\n\
             x = 1;\n\
             x = x + 1;\n\
             EndProcedure",
    );

    host.apply_change(Change::SetFileWithSnapshot {
        file_id,
        text: text.clone(),
        version: 1,
        path: Arc::from("invalidation.bsl"),
        parse_snapshot: parse_snapshot_for_test(file_id, 1, text.as_ref(), Vec::new(), true, None),
    });
    let probe = marker_offset(text.as_ref(), "x = x + 1;");
    {
        let analysis_before = host.snapshot();
        analysis_before
            .precompute_type_index_for_file(file_id, Some(1), 0)
            .expect("precompute before invalidation");
        let exact_before = analysis_before
            .type_at_byte_offset_serve_only_profiled(file_id, probe)
            .expect("serve before invalidation");
        assert_eq!(
            exact_before.serve_reason_code,
            TypeIndexServeReasonCode::TypeIndexExactHit
        );
    }

    host.apply_change(Change::SetDepsSnapshot {
        deps_id: DepsSnapshotId::from_hash("deps-new"),
        deps: default_semantic_deps(),
    });
    let after_deps = host
        .snapshot()
        .type_at_byte_offset_serve_only_profiled(file_id, probe)
        .expect("serve after deps switch");
    assert_eq!(
        after_deps.serve_reason_code,
        TypeIndexServeReasonCode::TypeIndexFallbackUnavailable
    );

    host.apply_change(Change::SetSettingsSnapshot {
        settings_id: SettingsId::from_hash("settings-new"),
        diagnostics_detail_level: DetailLevel::Detailed,
    });
    let after_settings = host
        .snapshot()
        .type_at_byte_offset_serve_only_profiled(file_id, probe)
        .expect("serve after settings switch");
    assert_eq!(
        after_settings.serve_reason_code,
        TypeIndexServeReasonCode::TypeIndexFallbackUnavailable
    );
}

#[test]
fn type_index_reason_code_strings_match_contract() {
    assert_eq!(
        TypeIndexPrecomputeReasonCode::TypeIndexPrecomputeQueueSaturated.as_str(),
        "type_index_precompute_queue_saturated"
    );
    assert_eq!(
        TypeIndexArtifactReasonCode::TypeIndexArtifactInvalidatedDeps.as_str(),
        "type_index_artifact_invalidated_deps"
    );
    assert_eq!(
        TypeIndexArtifactReasonCode::TypeIndexArtifactInvalidatedSettings.as_str(),
        "type_index_artifact_invalidated_settings"
    );
    assert_eq!(
        TypeIndexArtifactReasonCode::TypeIndexArtifactEvictedGlobalGuard.as_str(),
        "type_index_artifact_evicted_global_guard"
    );
    assert_eq!(
        TypeIndexArtifactReasonCode::TypeIndexArtifactEvictedPerFileWindow.as_str(),
        "type_index_artifact_evicted_per_file_window"
    );
}

#[test]
fn apply_change_reports_type_index_invalidation_effects() {
    let mut host = AnalysisHostV2::default();
    let file_id = FileId(207);
    let text: Arc<str> = Arc::from("Procedure Test()\n    x = 1;\nEndProcedure");

    host.apply_change(Change::SetFileWithSnapshot {
        file_id,
        text: text.clone(),
        version: 1,
        path: Arc::from("effects-invalidation.bsl"),
        parse_snapshot: parse_snapshot_for_test(file_id, 1, text.as_ref(), Vec::new(), true, None),
    });
    host.snapshot()
        .precompute_type_index_for_file(file_id, Some(1), 0)
        .expect("precompute before deps invalidation");

    let deps_effects = host.apply_change(Change::SetDepsSnapshot {
        deps_id: DepsSnapshotId::from_hash("deps-effects-new"),
        deps: default_semantic_deps(),
    });
    assert!(
        deps_effects.invalidated_deps_total > 0,
        "deps invalidation should report removed artifacts"
    );

    host.snapshot()
        .precompute_type_index_for_file(file_id, Some(1), 0)
        .expect("precompute before settings invalidation");
    let settings_effects = host.apply_change(Change::SetSettingsSnapshot {
        settings_id: SettingsId::from_hash("settings-effects-new"),
        diagnostics_detail_level: DetailLevel::Detailed,
    });
    assert!(
        settings_effects.invalidated_settings_total > 0,
        "settings invalidation should report removed artifacts"
    );
}

#[test]
fn cancellable_propagates_panics() {
    let result = std::panic::catch_unwind(|| {
        let _: Cancellable<()> = cancellable(|| panic!("test panic"));
    });
    assert!(result.is_err());
}
