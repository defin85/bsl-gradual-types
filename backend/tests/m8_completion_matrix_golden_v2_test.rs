//! Golden matrix tests for IntelliSense completion completeness (M8).

mod intellisense_testkit;
mod support;

use std::path::PathBuf;
use std::sync::{Arc, LazyLock};

use bsl_analysis_v2::{
    AnalysisHostV2, AnalysisV2, Change as ChangeV2, FileId as V2FileId, SettingsId,
};
use bsl_backend::application::get_completion_with_semantic_program_snapshot_v2;
use bsl_backend::system::DepsBundleV2;
use bsl_shared::domain::resolver::TypeResolver;
use bsl_shared::domain::TypeMetadataLookup;
use bsl_shared::formatting::DetailLevel;

fn workspace_root() -> PathBuf {
    let backend_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    backend_root.parent().expect("workspace root").to_path_buf()
}

static FIXTURE_DEPS: LazyLock<Arc<DepsBundleV2>> = LazyLock::new(|| {
    let root = workspace_root();
    let syntax_helper = root.join("examples").join("syntax_helper");
    let config_root = root.join("examples").join("conf").join("conf_test");
    support::deps_bundle_v2_for_paths(
        Some(syntax_helper.as_path()),
        Some(config_root.as_path()),
        Some("8.3.25"),
    )
});

#[derive(Debug, Clone)]
struct MatrixCase {
    id: &'static str,
    expression_form: &'static str,
    source: &'static str,
    receiver_expr: &'static str,
    typed_prefix: &'static str,
    expected_top_n: &'static [&'static str],
    top_n: usize,
}

fn matrix_cases() -> Vec<MatrixCase> {
    vec![
        MatrixCase {
            id: "m8_metadata_documents",
            expression_form: "id.",
            source: "metadata",
            receiver_expr: "Документы",
            typed_prefix: "Документы.",
            expected_top_n: &["ЗаказНаряды"],
            top_n: 25,
        },
        MatrixCase {
            id: "m8_metadata_catalogs",
            expression_form: "id.",
            source: "metadata",
            receiver_expr: "Справочники",
            typed_prefix: "Справочники.",
            expected_top_n: &["Контрагенты"],
            top_n: 25,
        },
        MatrixCase {
            id: "m8_metadata_doc_manager",
            expression_form: "id.",
            source: "metadata",
            receiver_expr: "Документы.ЗаказНаряды",
            typed_prefix: "Документы.ЗаказНаряды.",
            expected_top_n: &["СоздатьДокумент"],
            top_n: 25,
        },
        MatrixCase {
            id: "m8_metadata_doc_object_call",
            expression_form: "call().",
            source: "metadata",
            receiver_expr: "Документы.ЗаказНаряды.СоздатьДокумент()",
            typed_prefix: "Документы.ЗаказНаряды.СоздатьДокумент().",
            expected_top_n: &["ПолучитьСсылкуНового", "Работы"],
            top_n: 35,
        },
        MatrixCase {
            id: "m8_metadata_tabular_section_chain",
            expression_form: "цепочка",
            source: "metadata",
            receiver_expr: "Документы.ЗаказНаряды.СоздатьДокумент().Работы",
            typed_prefix: "Документы.ЗаказНаряды.СоздатьДокумент().Работы.",
            expected_top_n: &["Добавить"],
            top_n: 35,
        },
        MatrixCase {
            id: "m8_metadata_doc_ref_chain",
            expression_form: "цепочка",
            source: "metadata",
            receiver_expr: "Документы.ЗаказНаряды.СоздатьДокумент().ПолучитьСсылкуНового()",
            typed_prefix: "Документы.ЗаказНаряды.СоздатьДокумент().ПолучитьСсылкуНового().",
            expected_top_n: &["ПолучитьОбъект"],
            top_n: 35,
        },
        MatrixCase {
            id: "m8_stdlib_new_value_table",
            expression_form: "call().",
            source: "stdlib",
            receiver_expr: "Новый ТаблицаЗначений()",
            typed_prefix: "Новый ТаблицаЗначений().",
            expected_top_n: &["Колонки"],
            top_n: 35,
        },
        MatrixCase {
            id: "m8_stdlib_value_table_columns",
            expression_form: "цепочка",
            source: "stdlib",
            receiver_expr: "Новый ТаблицаЗначений().Колонки",
            typed_prefix: "Новый ТаблицаЗначений().Колонки.",
            expected_top_n: &["Добавить"],
            top_n: 35,
        },
        MatrixCase {
            id: "m8_stdlib_index_after_call",
            expression_form: "[]",
            source: "stdlib",
            receiver_expr: "Новый Массив()[0]",
            typed_prefix: "Новый Массив()[0].",
            expected_top_n: &["Добавить"],
            top_n: 25,
        },
        MatrixCase {
            id: "m8_stdlib_parens_receiver",
            expression_form: "()",
            source: "stdlib",
            receiver_expr: "(Новый Массив())",
            typed_prefix: "(Новый Массив()).",
            expected_top_n: &["Добавить"],
            top_n: 25,
        },
        MatrixCase {
            id: "m8_stdlib_conditional_receiver",
            expression_form: "?()",
            source: "stdlib",
            receiver_expr: "?(Истина, Новый Массив(), Новый Массив())",
            typed_prefix: "?(Истина, Новый Массив(), Новый Массив()).",
            expected_top_n: &["Добавить"],
            top_n: 25,
        },
        MatrixCase {
            id: "m8_stdlib_choice_receiver",
            expression_form: "Выбор",
            source: "stdlib",
            receiver_expr:
                "(Выбор Когда Истина Тогда Новый Массив() Иначе Новый Массив() КонецВыбора)",
            typed_prefix:
                "(Выбор Когда Истина Тогда Новый Массив() Иначе Новый Массив() КонецВыбора).",
            expected_top_n: &["Добавить"],
            top_n: 25,
        },
    ]
}

fn build_analysis_and_ir(
    deps_bundle: &DepsBundleV2,
    file_path: &str,
    code: &str,
) -> (AnalysisV2, Arc<bsl_shared::ir::SemanticProgram>) {
    let mut host = AnalysisHostV2::default();
    host.apply_change(ChangeV2::SetDepsSnapshot {
        deps_id: deps_bundle.deps_id.clone(),
        deps: deps_bundle.semantic_deps.clone(),
    });
    host.apply_change(ChangeV2::SetSettingsSnapshot {
        settings_id: SettingsId::from_hash("m8-completion-matrix"),
        diagnostics_detail_level: DetailLevel::Full,
    });
    host.apply_change(ChangeV2::SetFile {
        file_id: V2FileId(1),
        text: Arc::from(code.to_string()),
        version: 0,
        path: Arc::from(file_path.to_string()),
    });

    let analysis = host.analysis();
    analysis
        .precompute_type_index_for_file(V2FileId(1), None, 0)
        .expect("type index precompute");
    let ir_program = analysis.ir(V2FileId(1)).ok().flatten().expect("ir");
    (analysis, ir_program)
}

#[tokio::test]
async fn m8_completion_matrix_golden_v2() {
    let deps_bundle = FIXTURE_DEPS.clone();
    let resolver: Arc<TypeResolver> =
        deps_bundle
            .semantic_deps
            .resolver
            .clone()
            .unwrap_or_else(|| {
                Arc::new(TypeResolver::new(
                    deps_bundle.semantic_deps.repository.clone(),
                ))
            });
    let metadata_lookup = TypeMetadataLookup::new(deps_bundle.semantic_deps.repository.clone());
    let index_snapshot = deps_bundle.index_snapshot.as_ref();

    let file_path = "m8_completion_matrix_v2.bsl";
    let file_uri = Some("file:///m8_completion_matrix_v2.bsl");
    let mut snapshots: Vec<serde_json::Value> = Vec::new();

    for case in matrix_cases() {
        let content = format!(
            "Процедура M8()\n    __tmp = {} X;\nКонецПроцедуры\n",
            case.typed_prefix
        );
        let (line, column) =
            intellisense_testkit::find_marker_position(&content, case.typed_prefix);
        let (analysis, ir_program) =
            build_analysis_and_ir(deps_bundle.as_ref(), file_path, &content);
        if case.id == "m8_stdlib_parens_receiver" {
            assert!(
                analysis
                    .current_type_index_serve_only_ready(V2FileId(1))
                    .expect("serve-only readiness"),
                "{} must have exact serve-only artifact",
                case.id
            );
            let probe = content
                .find("Новый Массив()")
                .map(|idx| idx + "Новый Массив()".len() - 1)
                .expect("parens probe") as u32;
            let profiled = analysis
                .type_at_byte_offset_serve_only_profiled(V2FileId(1), probe)
                .expect("parens serve-only lookup");
            assert_eq!(
                profiled
                    .resolution
                    .as_ref()
                    .map(bsl_shared::domain::TypeResolution::type_name),
                Some("Массив<Неопределено>".to_string()),
                "{} must materialize exact owner type before completion",
                case.id
            );
        }
        if case.id == "m8_stdlib_choice_receiver" {
            assert!(
                analysis
                    .current_type_index_serve_only_ready(V2FileId(1))
                    .expect("serve-only readiness"),
                "{} must have exact serve-only artifact",
                case.id
            );
            let probes: Vec<u32> = content
                .match_indices("Новый Массив()")
                .map(|(idx, _)| (idx + "Новый Массив()".len() - 1) as u32)
                .collect();
            assert_eq!(
                probes.len(),
                2,
                "{} must contain two branch probes",
                case.id
            );
            for probe in probes {
                let profiled = analysis
                    .type_at_byte_offset_serve_only_profiled(V2FileId(1), probe)
                    .expect("choice branch serve-only lookup");
                assert_eq!(
                    profiled
                        .resolution
                        .as_ref()
                        .map(bsl_shared::domain::TypeResolution::type_name),
                    Some("Массив<Неопределено>".to_string()),
                    "{} must materialize branch owner type before completion",
                    case.id
                );
            }
        }
        let owner_hint = support::completion_owner_hint_for_position(
            &analysis,
            V2FileId(1),
            &content,
            line,
            column,
        );
        match case.id {
            "m8_metadata_documents" => assert_eq!(
                owner_hint
                    .as_ref()
                    .map(bsl_shared::domain::TypeResolution::type_name),
                Some("Документы".to_string()),
                "{} must surface canonical owner hint before completion",
                case.id
            ),
            "m8_stdlib_parens_receiver" => assert_eq!(
                owner_hint
                    .as_ref()
                    .map(bsl_shared::domain::TypeResolution::type_name),
                Some("Массив<Неопределено>".to_string()),
                "{} must surface canonical owner hint before completion",
                case.id
            ),
            "m8_stdlib_choice_receiver" => assert_eq!(
                owner_hint
                    .as_ref()
                    .map(bsl_shared::domain::TypeResolution::type_name),
                Some("Массив<Неопределено>".to_string()),
                "{} must surface canonical owner hint before completion",
                case.id
            ),
            _ => {}
        }

        let result = get_completion_with_semantic_program_snapshot_v2(
            &content,
            line,
            column,
            file_uri,
            index_snapshot,
            &metadata_lookup,
            file_path,
            resolver.as_ref(),
            ir_program,
            owner_hint,
            false,
        )
        .await
        .expect("completion ok");

        let items: Vec<bsl_shared::domain::CompletionItem> =
            result.items.into_iter().map(|c| c.item).collect();
        let snapshot_completion = intellisense_testkit::completion_snapshot_domain_top_n(
            &items,
            result.is_incomplete,
            case.top_n,
        );

        let labels: Vec<&str> = items
            .iter()
            .take(case.top_n)
            .map(|item| item.label.as_str())
            .collect();
        for expected in case.expected_top_n {
            assert!(
                labels.contains(expected),
                "{}: expected '{}' in top-{}, labels={:?}",
                case.id,
                expected,
                case.top_n,
                labels
            );
        }

        let receiver_type = resolver
            .resolve_expression_sync(case.receiver_expr)
            .type_name();
        snapshots.push(serde_json::json!({
            "case": case.id,
            "expressionForm": case.expression_form,
            "source": case.source,
            "receiverExpr": case.receiver_expr,
            "receiverType": receiver_type,
            "expectedTopN": case.expected_top_n,
            "completion": snapshot_completion,
        }));
    }

    let snapshot = serde_json::json!({
        "schema": "m8_completion_matrix_v2",
        "cases": snapshots,
    });

    intellisense_testkit::assert_snapshot("m8_completion_matrix_v2.json", &snapshot);
}
