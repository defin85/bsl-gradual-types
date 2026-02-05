//! Hover handler for LSP
//!
//! Handles textDocument/hover requests.

use std::sync::Arc;
use tower_lsp::lsp_types::*;
use tracing::debug;

use bsl_backend::application::get_hover_info_with_semantic_program;
use bsl_backend::helpers::hover_formatter::HoverFormatter;
use bsl_backend::helpers::hover_formatter::{HoverFormatConfig, HoverOutputFormat};
use bsl_shared::domain::resolver::TypeResolver;
use bsl_shared::domain::TypeMetadataLookup;
use bsl_shared::formatting::DetailLevel;
use bsl_shared::ir::SemanticProgram;

use crate::config::HoverSettings;

#[allow(clippy::too_many_arguments)]
pub fn handle_hover_v2(
    analysis: &bsl_analysis_v2::AnalysisV2,
    file_id: bsl_analysis_v2::FileId,
    file_content: Arc<str>,
    file_path: Arc<str>,
    ir_program: Arc<SemanticProgram>,
    deps: Arc<bsl_analysis_v2::SemanticDeps>,
    position: Position,
    uri: &Url,
    settings: &HoverSettings,
    include_flow_sensitive: bool,
) -> Option<Hover> {
    // Get syntax_helper path from environment or standard locations
    let syntax_helper_path = bsl_runtime::system::global_runtime_config()
        .get_pathbuf(bsl_runtime::system::RuntimeKey::SyntaxHelperPath)
        .or_else(|| {
            let candidates = vec![
                std::path::PathBuf::from("examples/syntax_helper"),
                std::path::PathBuf::from("../examples/syntax_helper"),
                std::path::PathBuf::from("C:/examples/syntax_helper"),
            ];
            candidates.into_iter().find(|p| p.exists())
        });

    let detail_level = DetailLevel::parse(&settings.detail_level);

    debug!(
        "Hover v2 requested: uri={}, file_path={}, detailLevel={:?}, maxMethods={}, maxProperties={}, showCertainty={}, syntax_helper={:?}",
        uri,
        file_path,
        detail_level,
        settings.max_methods,
        settings.max_properties,
        settings.show_certainty,
        syntax_helper_path.as_ref().map(|p| p.display().to_string())
    );

    let hover_config = HoverFormatConfig {
        max_methods: settings.max_methods,
        max_properties: settings.max_properties,
        detail_level,
        show_certainty: settings.show_certainty,
        syntax_helper_path,
        output_format: HoverOutputFormat::Markdown,
        ..Default::default()
    };

    let resolver = deps
        .resolver
        .clone()
        .unwrap_or_else(|| Arc::new(TypeResolver::new(deps.repository.clone())));
    let metadata_lookup = TypeMetadataLookup::new(deps.repository.clone());
    let hover_formatter = HoverFormatter::new(hover_config, metadata_lookup.clone());

    let hover_info = get_hover_info_with_semantic_program(
        analysis,
        file_id,
        file_content.as_ref(),
        position.line,
        position.character,
        include_flow_sensitive,
        &metadata_lookup,
        &hover_formatter,
        None,
        resolver.as_ref(),
        ir_program,
    );

    hover_info.map(|info| Hover {
        contents: HoverContents::Scalar(MarkedString::String(info)),
        range: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::Arc;

    use bsl_shared::domain::repository::InMemoryTypeRepository;
    use bsl_shared::domain::signature_index::{
        ConstructorSignature, MethodSignature, SignatureIndex, SignatureSource,
    };
    use bsl_shared::domain::types::{ParameterInfo, RawDataSource, RawTypeData};
    use bsl_shared::TypeRepository;
    use bsl_shared::TypeResolver;

    fn fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
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

    struct TestEnv {
        deps: Arc<bsl_analysis_v2::SemanticDeps>,
    }

    fn create_test_env() -> TestEnv {
        let repository_impl = Arc::new(InMemoryTypeRepository::new());
        repository_impl
            .load_types(vec![RawTypeData {
                name: "Массив".to_string(),
                source: RawDataSource::Platform,
                ..Default::default()
            }])
            .expect("load types");

        let mut index = SignatureIndex::new();
        let method = MethodSignature::new(
            "Добавить".to_string(),
            Some("Массив".to_string()),
            vec![ParameterInfo {
                name: "Элемент".to_string(),
                type_name: Some("Число".to_string()),
                is_optional: false,
                default_value: None,
                description: None,
            }],
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

        let repository =
            repository_impl.clone() as Arc<dyn bsl_shared::domain::repository::TypeRepository>;
        let resolver = Arc::new(TypeResolver::new(repository.clone()));

        let deps = Arc::new(bsl_analysis_v2::SemanticDeps {
            signature_index: repository.get_signature_index_clone(),
            resolver: Some(resolver),
            repository,
            platform_signatures_loaded: false,
        });

        TestEnv { deps }
    }

    fn build_v2_ir(
        content: &str,
        uri: &Url,
        deps: Arc<bsl_analysis_v2::SemanticDeps>,
    ) -> (
        bsl_analysis_v2::AnalysisV2,
        bsl_analysis_v2::FileId,
        Arc<str>,
        Arc<str>,
        Arc<SemanticProgram>,
    ) {
        let mut host = bsl_analysis_v2::AnalysisHostV2::default();
        host.apply_change(bsl_analysis_v2::Change::SetDepsSnapshot {
            deps_id: bsl_analysis_v2::DepsSnapshotId::from_hash("test"),
            deps: deps.clone(),
        });

        let path = uri
            .to_file_path()
            .ok()
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_else(|| uri.to_string());
        let file_id = bsl_analysis_v2::FileId(1);
        host.apply_change(bsl_analysis_v2::Change::SetFile {
            file_id,
            text: Arc::from(content.to_string()),
            version: 0,
            path: Arc::from(path),
        });

        let analysis = host.analysis();
        let file_content = analysis
            .file_text(file_id)
            .ok()
            .flatten()
            .expect("file_text");
        let file_path = analysis
            .file_path(file_id)
            .ok()
            .flatten()
            .expect("file_path");
        let ir_program = analysis.ir(file_id).ok().flatten().expect("ir");

        (analysis, file_id, file_content, file_path, ir_program)
    }

    fn hover_text(hover: Hover) -> String {
        match hover.contents {
            HoverContents::Scalar(MarkedString::String(value)) => value,
            HoverContents::Scalar(MarkedString::LanguageString(language)) => language.value,
            HoverContents::Markup(value) => value.value,
            HoverContents::Array(values) => values
                .into_iter()
                .map(|value| match value {
                    MarkedString::String(text) => text,
                    MarkedString::LanguageString(language) => language.value,
                })
                .collect::<Vec<_>>()
                .join("\n"),
        }
    }

    #[tokio::test]
    async fn m5_hover_v2_is_deterministic() {
        let content = read_fixture("m5_snippets_resolve.bsl");
        let mut position = find_position(&content, "    Массив");
        position.character = position.character.saturating_sub(1);
        let uri = Url::parse("file:///m5_snippets_resolve.bsl").expect("url");
        let env = create_test_env();

        let settings = HoverSettings {
            detail_level: "compact".to_string(),
            max_methods: 10,
            max_properties: 5,
            show_certainty: false,
        };

        let (analysis, file_id, file_content, file_path, ir_program) =
            build_v2_ir(&content, &uri, env.deps.clone());
        let v2 = handle_hover_v2(
            &analysis,
            file_id,
            file_content.clone(),
            file_path.clone(),
            ir_program.clone(),
            env.deps.clone(),
            position,
            &uri,
            &settings,
            false,
        )
        .expect("hover v2");
        let v2_text = hover_text(v2);
        assert!(
            v2_text.contains("Переменная"),
            "expected hover to contain variable header"
        );
        assert!(
            v2_text.contains("Массив"),
            "expected hover to contain type name"
        );

        // Determinism smoke: same input -> same output twice.
        let v2_second = handle_hover_v2(
            &analysis,
            file_id,
            file_content,
            file_path,
            ir_program,
            env.deps,
            position,
            &uri,
            &settings,
            false,
        )
        .expect("hover v2 (second)");
        let v2_second_text = hover_text(v2_second);
        assert_eq!(v2_text, v2_second_text);
    }
}
