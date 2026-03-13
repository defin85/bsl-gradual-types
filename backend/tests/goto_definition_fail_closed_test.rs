use std::sync::Arc;

use bsl_analysis_v2::{AnalysisHostV2, Change, DepsSnapshotId, FileId, SettingsId};
use bsl_backend::application::type_system;
use bsl_shared::domain::repository::{InMemoryTypeRepository, TypeRepository};
use bsl_shared::domain::resolver::TypeResolver;
use bsl_shared::domain::signature_index::SignatureIndex;
use bsl_shared::domain::types::{FacetKind, MetadataKind, RawDataSource, RawTypeData};
use bsl_shared::formatting::DetailLevel;

fn utf16_col(line: &str, byte_idx: usize) -> u32 {
    line[..byte_idx].chars().map(|c| c.len_utf16() as u32).sum()
}

#[test]
fn goto_definition_fails_closed_without_exact_type_index_artifact() {
    let repository_impl = Arc::new(InMemoryTypeRepository::new());
    repository_impl
        .load_types(vec![RawTypeData {
            name: "Документы.Док1".to_string(),
            source: RawDataSource::Configuration,
            facets: vec![FacetKind::Manager, FacetKind::Object, FacetKind::Reference],
            kind: Some(MetadataKind::Document),
            ..Default::default()
        }])
        .expect("load types");
    let repository = repository_impl.clone() as Arc<dyn TypeRepository>;
    let resolver = Arc::new(TypeResolver::new(repository.clone()));
    let deps = Arc::new(bsl_analysis_v2::SemanticDeps {
        repository,
        signature_index: SignatureIndex::new(),
        resolver: Some(resolver),
        platform_signatures_loaded: true,
    });

    let content = concat!(
        "Процедура МойМетод() Экспорт\n",
        "КонецПроцедуры\n",
        "\n",
        "Процедура Тест()\n",
        "    ЭтотОбъект.МойМетод();\n",
        "КонецПроцедуры\n"
    );
    let mut host = AnalysisHostV2::default();
    let file_id = FileId(1);
    host.apply_change(Change::SetDepsSnapshot {
        deps_id: DepsSnapshotId::from_hash("goto-definition-fail-closed"),
        deps,
    });
    host.apply_change(Change::SetSettingsSnapshot {
        settings_id: SettingsId::from_hash("goto-definition-fail-closed"),
        diagnostics_detail_level: DetailLevel::Full,
    });
    host.apply_change(Change::SetFile {
        file_id,
        text: Arc::from(content.to_string()),
        version: 1,
        path: Arc::from("Documents/Док1/Ext/ObjectModule.bsl"),
    });

    let analysis = host.snapshot();
    let ir_program = analysis.ir(file_id).ok().flatten().expect("ir");
    let call_line = content.lines().nth(4).expect("call line");
    let method_byte = call_line.find("МойМетод").expect("method byte");
    let method_col = utf16_col(call_line, method_byte);

    let target =
        type_system::goto_definition_v2_with_source_and_analysis(type_system::DefinitionRequest {
            current_file_text: Some(content),
            analysis: Some(&analysis),
            file_id: Some(file_id),
            ir_program,
            deps: analysis.deps_data().expect("deps"),
            line: 4,
            character: method_col,
            coordinator: None,
        });

    assert!(
        target.is_none(),
        "definition must fail closed without exact type_index artifact"
    );
}
