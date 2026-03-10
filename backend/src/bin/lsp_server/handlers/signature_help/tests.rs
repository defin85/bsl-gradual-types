use super::*;

use std::fs;
use std::path::PathBuf;

use bsl_analysis_v2::{AnalysisHostV2, Change, DepsSnapshotId, FileId as V2FileId, SettingsId};
use bsl_shared::domain::repository::{InMemoryTypeRepository, TypeRepository};
use bsl_shared::domain::resolver::TypeResolver;
use bsl_shared::domain::signature_index::{
    ConstructorSignature, MethodSignature, SignatureIndex, SignatureSource,
};
use bsl_shared::domain::types::{ParameterInfo, RawDataSource, RawTypeData};
use bsl_shared::formatting::DetailLevel;
use bsl_shared::ir::SemanticProgram;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

fn golden_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("golden")
        .join(name)
}

fn read_fixture(name: &str) -> String {
    fs::read_to_string(fixture_path(name)).expect("fixture read")
}

fn find_position(content: &str, marker: &str) -> Position {
    let byte_index = content.find(marker).expect("marker not found");
    let before = &content[..byte_index + marker.len()];
    let line = before.lines().count() - 1;
    let last_line = before.lines().last().unwrap_or("");
    let character = last_line.chars().map(|ch| ch.len_utf16()).sum::<usize>();
    Position {
        line: line as u32,
        character: character as u32,
    }
}

fn assert_snapshot(name: &str, value: &serde_json::Value) {
    let path = golden_path(name);
    let json = serde_json::to_string_pretty(value).expect("snapshot json");
    if std::env::var("UPDATE_GOLDEN").ok().as_deref() == Some("1") {
        fs::create_dir_all(path.parent().expect("golden dir")).expect("create golden dir");
        fs::write(&path, json).expect("write golden");
        return;
    }
    let expected = fs::read_to_string(&path).expect("read golden");
    assert_eq!(expected, json);
}

fn create_test_deps() -> Arc<bsl_analysis_v2::SemanticDeps> {
    let repository_impl = Arc::new(InMemoryTypeRepository::new());
    let raw_type = RawTypeData {
        name: "Массив".to_string(),
        source: RawDataSource::Platform,
        ..Default::default()
    };
    repository_impl
        .load_types(vec![raw_type])
        .expect("load types");

    let mut index = SignatureIndex::new();
    let method = MethodSignature::new(
        "Добавить".to_string(),
        Some("Массив".to_string()),
        vec![
            ParameterInfo {
                name: "Элемент".to_string(),
                type_name: Some("Число".to_string()),
                is_optional: false,
                default_value: None,
                description: None,
            },
            ParameterInfo {
                name: "Позиция".to_string(),
                type_name: Some("Число".to_string()),
                is_optional: true,
                default_value: None,
                description: None,
            },
        ],
        Some("Булево".to_string()),
        None,
        None,
        SignatureSource::Platform,
        None,
        Default::default(),
    );
    index.add_platform_method(bsl_shared::domain::type_id::TypeId::new("Массив"), method);
    index.add_constructor(
        bsl_shared::domain::type_id::TypeId::new("Массив"),
        ConstructorSignature {
            type_name: "Массив".to_string(),
            params: vec![ParameterInfo {
                name: "Размер".to_string(),
                type_name: Some("Число".to_string()),
                is_optional: true,
                default_value: None,
                description: None,
            }],
            facet: None,
            source: SignatureSource::Platform,
            is_collection: true,
            generic_params_count: 1,
        },
    );
    repository_impl.set_signature_index(index);

    let repository = repository_impl as Arc<dyn TypeRepository>;
    let resolver = Arc::new(TypeResolver::new(repository.clone()));
    Arc::new(bsl_analysis_v2::SemanticDeps {
        signature_index: repository.get_signature_index_clone(),
        resolver: Some(resolver),
        repository,
        platform_signatures_loaded: false,
    })
}

fn build_analysis_host(
    content: &str,
    deps: Arc<bsl_analysis_v2::SemanticDeps>,
) -> (bsl_analysis_v2::AnalysisV2, Arc<SemanticProgram>, V2FileId) {
    let mut host = AnalysisHostV2::default();
    host.apply_change(Change::SetDepsSnapshot {
        deps_id: DepsSnapshotId::from_hash("test"),
        deps,
    });
    host.apply_change(Change::SetSettingsSnapshot {
        settings_id: SettingsId::from_hash("test"),
        diagnostics_detail_level: DetailLevel::Full,
    });

    let file_id = V2FileId(1);
    host.apply_change(Change::SetFile {
        file_id,
        text: Arc::from(content.to_string()),
        version: 0,
        path: Arc::from("test.bsl"),
    });

    let analysis = host.snapshot();
    let ir_program = analysis.ir(file_id).ok().flatten().expect("ir");
    (analysis, ir_program, file_id)
}

#[tokio::test]
async fn m5_signature_help_snapshot() {
    let content = read_fixture("m5_signature_help.bsl");
    let deps = create_test_deps();
    let (analysis, ir_program, file_id) = build_analysis_host(&content, deps.clone());
    analysis
        .precompute_type_index_for_file(file_id, Some(0), 0)
        .expect("precompute exact type index");

    let constructor_pos = find_position(&content, "Новый Массив(1, ");
    let constructor_v2 = handle_signature_help_v2(
        &analysis,
        file_id,
        Arc::from(content.clone()),
        constructor_pos,
        ir_program.clone(),
        deps.clone(),
        None,
    )
    .expect("constructor signature help (v2)");

    let method_pos = find_position(&content, "МойМассив.Добавить(1, ");
    let method_v2 = handle_signature_help_v2(
        &analysis,
        file_id,
        Arc::from(content.clone()),
        method_pos,
        ir_program,
        deps,
        None,
    )
    .expect("method signature help (v2)");

    let snapshot = serde_json::json!({
        "constructor": {
            "label": constructor_v2.signatures.first().map(|sig| sig.label.clone()),
            "activeParameter": constructor_v2.active_parameter,
        },
        "method": {
            "label": method_v2.signatures.first().map(|sig| sig.label.clone()),
            "activeParameter": method_v2.active_parameter,
        },
    });

    assert_snapshot("m5_signature_help.json", &snapshot);
}

#[tokio::test]
async fn signature_help_fails_closed_without_exact_type_index_artifact() {
    let deps = create_test_deps();
    let content = concat!(
        "Процедура Тест()\n",
        "    МойМассив = Новый Массив;\n",
        "    МойМассив.Добавить(1, );\n",
        "КонецПроцедуры\n"
    );
    let (analysis, ir_program, file_id) = build_analysis_host(content, deps.clone());
    let position = find_position(content, "МойМассив.Добавить(1, ");

    let result = handle_signature_help_v2(
        &analysis,
        file_id,
        Arc::from(content.to_string()),
        position,
        ir_program,
        deps,
        None,
    );

    assert!(
        result.is_none(),
        "signatureHelp must fail closed without exact type_index artifact"
    );
}
