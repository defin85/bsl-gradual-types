//! Go to Definition regression test: configuration CommonModules.

use bsl_analysis_v2::AstToIrConverter;
use bsl_analysis_v2::SemanticDeps;
use bsl_analysis_v2::{
    AnalysisHostV2, Change, DepsSnapshotId, FileId, SemanticDiagnosticsMaterializationPath,
    SettingsId,
};
use bsl_backend::application::type_system;
use bsl_backend::system::build_deps_bundle_v2;
use bsl_backend::system::SystemCoordinator;
use bsl_shared::domain::repository::{InMemoryTypeRepository, TypeRepository};
use bsl_shared::domain::signature_index::SignatureIndex;
use bsl_shared::formatting::DetailLevel;
use bsl_shared::TypeResolver;
use bsl_syntax::{parse, ParseOptions};
use std::path::Path;
use std::sync::Arc;
use tempfile::TempDir;

fn utf16_col(line: &str, byte_idx: usize) -> u32 {
    line[..byte_idx].chars().map(|c| c.len_utf16() as u32).sum()
}

fn empty_semantic_deps() -> Arc<SemanticDeps> {
    let repository: Arc<dyn TypeRepository> = Arc::new(InMemoryTypeRepository::new());
    let resolver = Arc::new(TypeResolver::new(repository.clone()));
    Arc::new(SemanticDeps {
        repository,
        signature_index: SignatureIndex::new(),
        resolver: Some(resolver),
        platform_signatures_loaded: false,
        common_module_factory_registry: Default::default(),
        global_context_index: Default::default(),
    })
}

fn build_analysis_ir_for_source(
    source: &str,
    path: &str,
    deps: Arc<SemanticDeps>,
    tag: &str,
) -> (
    bsl_analysis_v2::AnalysisV2,
    FileId,
    Arc<bsl_shared::ir::SemanticProgram>,
) {
    let mut host = AnalysisHostV2::default();
    let file_id = FileId(1);
    host.apply_change(Change::SetDepsSnapshot {
        deps_id: DepsSnapshotId::from_hash(tag),
        deps,
    });
    host.apply_change(Change::SetSettingsSnapshot {
        settings_id: SettingsId::from_hash(tag),
        diagnostics_detail_level: DetailLevel::Full,
    });
    host.apply_change(Change::SetFile {
        file_id,
        text: Arc::from(source.to_string()),
        version: 0,
        path: Arc::from(path.to_string()),
    });

    let analysis = host.analysis();
    analysis
        .precompute_type_index_for_file(file_id, Some(0), 0)
        .expect("precompute exact type index");
    let ir = analysis.ir(file_id).ok().flatten().expect("ir");

    (analysis, file_id, ir)
}

fn goto_definition_with_analysis(
    source: &str,
    analysis: &bsl_analysis_v2::AnalysisV2,
    file_id: FileId,
    ir_program: Arc<bsl_shared::ir::SemanticProgram>,
    deps: Arc<SemanticDeps>,
    line: u32,
    character: u32,
) -> Option<type_system::DefinitionTarget> {
    type_system::goto_definition_v2_with_source_and_analysis(type_system::DefinitionRequest {
        current_file_text: Some(source),
        analysis: Some(analysis),
        file_id: Some(file_id),
        ir_program,
        deps,
        line,
        character,
        coordinator: None,
    })
}

#[test]
fn goto_definition_resolves_common_module_namespace_and_method() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    std::fs::write(
        root.join("Configuration.xml"),
        r#"<?xml version="1.0" encoding="UTF-8"?>
<MetaDataObject xmlns="http://v8.1c.ru/8.3/MDClasses" xmlns:v8="http://v8.1c.ru/8.1/data/core">
  <Configuration uuid="00000000-0000-0000-0000-000000000000">
    <Properties>
      <Name>TestConfig</Name>
      <CompatibilityMode>Version8_3_25</CompatibilityMode>
    </Properties>
    <ChildObjects>
      <CommonModule>МойМодуль</CommonModule>
    </ChildObjects>
  </Configuration>
</MetaDataObject>
"#,
    )
    .unwrap();

    std::fs::create_dir_all(root.join("CommonModules")).unwrap();
    std::fs::write(
        root.join("CommonModules").join("МойМодуль.xml"),
        r#"<?xml version="1.0" encoding="UTF-8"?>
<MetaDataObject xmlns="http://v8.1c.ru/8.3/MDClasses" xmlns:v8="http://v8.1c.ru/8.1/data/core">
  <CommonModule uuid="00000000-0000-0000-0000-000000000001">
    <Properties>
      <Name>МойМодуль</Name>
      <Global>false</Global>
      <ClientManagedApplication>false</ClientManagedApplication>
      <ClientOrdinaryApplication>false</ClientOrdinaryApplication>
      <Server>true</Server>
      <ExternalConnection>false</ExternalConnection>
      <ServerCall>false</ServerCall>
      <Privileged>false</Privileged>
      <ReturnValuesReuse>DontUse</ReturnValuesReuse>
    </Properties>
  </CommonModule>
</MetaDataObject>
"#,
    )
    .unwrap();

    std::fs::create_dir_all(root.join("CommonModules").join("МойМодуль").join("Ext")).unwrap();
    std::fs::write(
        root.join("CommonModules")
            .join("МойМодуль")
            .join("Ext")
            .join("Module.bsl"),
        "Процедура ПриСозданииНаСервере() Экспорт\nКонецПроцедуры\n",
    )
    .unwrap();

    let coordinator = SystemCoordinator::new();
    coordinator
        .start_with_paths_blocking(None, Some(Path::new(root)), Some("8.3.25"), None)
        .expect("startup");

    let domain_bundle = coordinator.domain_bundle().expect("domain_bundle");
    let repo = domain_bundle.repository.clone();
    let signature_index = repo.get_signature_index_clone();
    let resolver = domain_bundle.resolver.clone();
    let deps = Arc::new(SemanticDeps {
        repository: repo.clone(),
        signature_index: signature_index.clone(),
        resolver: Some(resolver.clone()),
        platform_signatures_loaded: repo.platform_docs_loaded(),
        common_module_factory_registry: Default::default(),
        global_context_index: Default::default(),
    });

    let source = "Процедура Тест()\n    МойМодуль.ПриСозданииНаСервере();\nКонецПроцедуры\n";
    let parsed = parse(source, &ParseOptions::default()).expect("parse");
    let ir = AstToIrConverter::convert_with_resolver(
        parsed.program,
        source.to_string(),
        "inline.bsl".to_string(),
        repo.clone(),
        signature_index,
        Some(resolver),
    )
    .expect("ir");
    let ir = Arc::new(ir);

    let call_line = source.lines().nth(1).expect("call line");
    let module_byte = call_line.find("МойМодуль").expect("module name");
    let method_byte = call_line.find("ПриСозданииНаСервере").expect("method name");

    let module_col = utf16_col(call_line, module_byte);
    let method_col = utf16_col(call_line, method_byte);

    let target_module = type_system::goto_definition_v2_with_source(
        "inline.bsl",
        source,
        ir.clone(),
        deps.clone(),
        1,
        module_col,
    )
    .expect("module definition target");

    assert!(
        target_module.span.is_none(),
        "module definition should point to file, not span"
    );

    let expected_module = root
        .join("CommonModules")
        .join("МойМодуль")
        .join("Ext")
        .join("Module.bsl")
        .canonicalize()
        .expect("canonicalize expected module");
    let actual_module = target_module
        .file_path
        .canonicalize()
        .expect("canonicalize actual module");
    assert_eq!(actual_module, expected_module);

    let (analysis, file_id, analysis_ir) = build_analysis_ir_for_source(
        source,
        "inline.bsl",
        deps,
        "goto-definition-common-module-legacy-test",
    );
    let target_method = goto_definition_with_analysis(
        source,
        &analysis,
        file_id,
        analysis_ir,
        empty_semantic_deps(),
        1,
        method_col,
    )
    .expect("method definition target");
    assert!(
        target_method.span.is_some(),
        "method definition should include span"
    );
}

#[test]
fn goto_definition_resolves_common_module_method_from_semantic_facts_with_empty_consumer_repo() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    std::fs::write(
        root.join("Configuration.xml"),
        r#"<?xml version="1.0" encoding="UTF-8"?>
<MetaDataObject xmlns="http://v8.1c.ru/8.3/MDClasses" xmlns:v8="http://v8.1c.ru/8.1/data/core">
  <Configuration uuid="00000000-0000-0000-0000-000000000000">
    <Properties>
      <Name>TestConfig</Name>
      <CompatibilityMode>Version8_3_25</CompatibilityMode>
    </Properties>
    <ChildObjects>
      <CommonModule>МойМодуль</CommonModule>
    </ChildObjects>
  </Configuration>
</MetaDataObject>
"#,
    )
    .unwrap();

    std::fs::create_dir_all(root.join("CommonModules")).unwrap();
    std::fs::write(
        root.join("CommonModules").join("МойМодуль.xml"),
        r#"<?xml version="1.0" encoding="UTF-8"?>
<MetaDataObject xmlns="http://v8.1c.ru/8.3/MDClasses" xmlns:v8="http://v8.1c.ru/8.1/data/core">
  <CommonModule uuid="00000000-0000-0000-0000-000000000001">
    <Properties>
      <Name>МойМодуль</Name>
      <Global>false</Global>
      <ClientManagedApplication>false</ClientManagedApplication>
      <ClientOrdinaryApplication>false</ClientOrdinaryApplication>
      <Server>true</Server>
      <ExternalConnection>false</ExternalConnection>
      <ServerCall>false</ServerCall>
      <Privileged>false</Privileged>
      <ReturnValuesReuse>DontUse</ReturnValuesReuse>
    </Properties>
  </CommonModule>
</MetaDataObject>
"#,
    )
    .unwrap();

    std::fs::create_dir_all(root.join("CommonModules").join("МойМодуль").join("Ext")).unwrap();
    std::fs::write(
        root.join("CommonModules")
            .join("МойМодуль")
            .join("Ext")
            .join("Module.bsl"),
        "Процедура ПриСозданииНаСервере() Экспорт\nКонецПроцедуры\n",
    )
    .unwrap();

    let coordinator = SystemCoordinator::new();
    coordinator
        .start_with_paths_blocking(None, Some(Path::new(root)), Some("8.3.25"), None)
        .expect("startup");

    let deps_bundle = build_deps_bundle_v2(&coordinator, None, Some(root)).expect("deps bundle");
    let producer_deps = deps_bundle.semantic_deps.clone();

    let source = "Процедура Тест()\n    МойМодуль.ПриСозданииНаСервере();\nКонецПроцедуры\n";
    let mut host = AnalysisHostV2::default();
    let file_id = FileId(1);
    host.apply_change(Change::SetDepsSnapshot {
        deps_id: DepsSnapshotId::from_hash("goto-definition-common-module-semantic-facts"),
        deps: producer_deps,
    });
    host.apply_change(Change::SetSettingsSnapshot {
        settings_id: SettingsId::from_hash("goto-definition-common-module-semantic-facts"),
        diagnostics_detail_level: DetailLevel::Full,
    });
    host.apply_change(Change::SetFile {
        file_id,
        text: Arc::from(source.to_string()),
        version: 0,
        path: Arc::from("inline.bsl"),
    });

    let analysis = host.analysis();
    analysis
        .precompute_type_index_for_file(file_id, Some(0), 0)
        .expect("precompute exact type index");
    let ir = analysis.ir(file_id).ok().flatten().expect("ir");
    let method_fact = ir
        .semantic_facts
        .call_method_targets_by_span
        .values()
        .find(|target| {
            target
                .owner_type
                .as_deref()
                .is_some_and(|owner| owner.eq_ignore_ascii_case("ОбщиеМодули.МойМодуль"))
                && target
                    .method_name
                    .eq_ignore_ascii_case("ПриСозданииНаСервере")
        });
    assert!(
        method_fact.is_some(),
        "missing common-module semantic method fact"
    );
    assert!(
        method_fact
            .and_then(|target| target.definition_location.as_ref())
            .is_some(),
        "common-module semantic method fact is missing definition location"
    );

    let call_line = source.lines().nth(1).expect("call line");
    let method_byte = call_line.find("ПриСозданииНаСервере").expect("method name");
    let method_col = utf16_col(call_line, method_byte);
    let method_offset = source
        .find("ПриСозданииНаСервере")
        .expect("method name offset") as u32;
    let method_fact_at_offset = ir
        .semantic_facts
        .member_method_targets_by_span
        .iter()
        .find(|(span, target)| {
            span.contains(method_offset)
                && target.definition_location.as_ref().is_some_and(|_| {
                    target
                        .method_name
                        .eq_ignore_ascii_case("ПриСозданииНаСервере")
                })
        })
        .or_else(|| {
            ir.semantic_facts
                .call_method_targets_by_span
                .iter()
                .find(|(span, target)| {
                    span.contains(method_offset)
                        && target.definition_location.is_some()
                        && target
                            .method_name
                            .eq_ignore_ascii_case("ПриСозданииНаСервере")
                })
        });
    assert!(
        method_fact_at_offset.is_some(),
        "missing semantic method definition fact at offset {method_offset}; member_spans={:?}; call_spans={:?}",
        ir.semantic_facts
            .member_method_targets_by_span
            .keys()
            .collect::<Vec<_>>(),
        ir.semantic_facts
            .call_method_targets_by_span
            .keys()
            .collect::<Vec<_>>()
    );

    let target_method = goto_definition_with_analysis(
        source,
        &analysis,
        file_id,
        ir,
        empty_semantic_deps(),
        1,
        method_col,
    )
    .expect("method definition target from semantic facts");

    assert!(
        target_method.span.is_some(),
        "method definition should include declaration span"
    );
    let expected_module = root
        .join("CommonModules")
        .join("МойМодуль")
        .join("Ext")
        .join("Module.bsl")
        .canonicalize()
        .expect("canonicalize expected module");
    let actual_module = target_method
        .file_path
        .canonicalize()
        .expect("canonicalize actual module");
    assert_eq!(actual_module, expected_module);
}

#[test]
fn goto_definition_resolves_common_module_receiver_from_semantic_facts_with_empty_consumer_repo() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    std::fs::write(
        root.join("Configuration.xml"),
        r#"<?xml version="1.0" encoding="UTF-8"?>
<MetaDataObject xmlns="http://v8.1c.ru/8.3/MDClasses" xmlns:v8="http://v8.1c.ru/8.1/data/core">
  <Configuration uuid="00000000-0000-0000-0000-000000000000">
    <Properties>
      <Name>TestConfig</Name>
      <CompatibilityMode>Version8_3_25</CompatibilityMode>
    </Properties>
    <ChildObjects>
      <CommonModule>МойМодуль</CommonModule>
    </ChildObjects>
  </Configuration>
</MetaDataObject>
"#,
    )
    .unwrap();

    std::fs::create_dir_all(root.join("CommonModules")).unwrap();
    std::fs::write(
        root.join("CommonModules").join("МойМодуль.xml"),
        r#"<?xml version="1.0" encoding="UTF-8"?>
<MetaDataObject xmlns="http://v8.1c.ru/8.3/MDClasses" xmlns:v8="http://v8.1c.ru/8.1/data/core">
  <CommonModule uuid="00000000-0000-0000-0000-000000000001">
    <Properties>
      <Name>МойМодуль</Name>
      <Global>false</Global>
      <ClientManagedApplication>false</ClientManagedApplication>
      <ClientOrdinaryApplication>false</ClientOrdinaryApplication>
      <Server>true</Server>
      <ExternalConnection>false</ExternalConnection>
      <ServerCall>false</ServerCall>
      <Privileged>false</Privileged>
      <ReturnValuesReuse>DontUse</ReturnValuesReuse>
    </Properties>
  </CommonModule>
</MetaDataObject>
"#,
    )
    .unwrap();

    std::fs::create_dir_all(root.join("CommonModules").join("МойМодуль").join("Ext")).unwrap();
    std::fs::write(
        root.join("CommonModules")
            .join("МойМодуль")
            .join("Ext")
            .join("Module.bsl"),
        "Процедура ПриСозданииНаСервере() Экспорт\nКонецПроцедуры\n",
    )
    .unwrap();

    let coordinator = SystemCoordinator::new();
    coordinator
        .start_with_paths_blocking(None, Some(Path::new(root)), Some("8.3.25"), None)
        .expect("startup");

    let deps_bundle = build_deps_bundle_v2(&coordinator, None, Some(root)).expect("deps bundle");
    let producer_deps = deps_bundle.semantic_deps.clone();

    let source = "Процедура Тест()\n    МойМодуль.ПриСозданииНаСервере();\nКонецПроцедуры\n";
    let (analysis, file_id, ir) = build_analysis_ir_for_source(
        source,
        "inline.bsl",
        producer_deps,
        "goto-definition-common-module-receiver-semantic-facts",
    );

    let call_line = source.lines().nth(1).expect("call line");
    let module_byte = call_line.find("МойМодуль").expect("module name");
    let module_col = utf16_col(call_line, module_byte);

    let target = goto_definition_with_analysis(
        source,
        &analysis,
        file_id,
        ir,
        empty_semantic_deps(),
        1,
        module_col,
    )
    .expect("receiver definition target from semantic facts");

    assert!(
        target.span.is_none(),
        "receiver definition should point to module file, not span"
    );
    let expected_module = root
        .join("CommonModules")
        .join("МойМодуль")
        .join("Ext")
        .join("Module.bsl")
        .canonicalize()
        .expect("canonicalize expected module");
    let actual_module = target
        .file_path
        .canonicalize()
        .expect("canonicalize actual module");
    assert_eq!(actual_module, expected_module);
}

#[test]
fn goto_definition_resolves_object_module_method_without_request_time_hints() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    std::fs::write(
        root.join("Configuration.xml"),
        r#"<?xml version="1.0" encoding="UTF-8"?>
<MetaDataObject xmlns="http://v8.1c.ru/8.3/MDClasses">
  <Configuration uuid="00000000-0000-0000-0000-000000000000">
    <Properties>
      <Name>TestConfig</Name>
      <CompatibilityMode>Version8_3_25</CompatibilityMode>
    </Properties>
    <ChildObjects>
      <Document>Док1</Document>
    </ChildObjects>
  </Configuration>
</MetaDataObject>
"#,
    )
    .unwrap();

    std::fs::create_dir_all(root.join("Documents")).unwrap();
    std::fs::write(
        root.join("Documents").join("Док1.xml"),
        r#"<?xml version="1.0" encoding="UTF-8"?>
<MetaDataObject xmlns="http://v8.1c.ru/8.3/MDClasses">
  <Document uuid="00000000-0000-0000-0000-000000000001">
    <Properties>
      <Name>Док1</Name>
    </Properties>
  </Document>
</MetaDataObject>
"#,
    )
    .unwrap();

    std::fs::create_dir_all(root.join("Documents").join("Док1").join("Ext")).unwrap();
    std::fs::write(
        root.join("Documents")
            .join("Док1")
            .join("Ext")
            .join("ObjectModule.bsl"),
        concat!(
            "Процедура МойМетод() Экспорт\n",
            "КонецПроцедуры\n",
            "\n",
            "Процедура Тест()\n",
            "    ЭтотОбъект.МойМетод();\n",
            "КонецПроцедуры\n"
        ),
    )
    .unwrap();

    let coordinator = SystemCoordinator::new();
    coordinator
        .start_with_paths_blocking(None, Some(Path::new(root)), Some("8.3.25"), None)
        .expect("startup");

    let deps_bundle = build_deps_bundle_v2(&coordinator, None, Some(root)).expect("deps bundle");
    let repo = deps_bundle.semantic_deps.repository.clone();
    let signature_index = deps_bundle.semantic_deps.signature_index.clone();
    let resolver = deps_bundle
        .semantic_deps
        .resolver
        .clone()
        .expect("resolver");

    let deps = Arc::new(SemanticDeps {
        repository: repo.clone(),
        signature_index: signature_index.clone(),
        resolver: Some(resolver.clone()),
        platform_signatures_loaded: false,
        common_module_factory_registry: Default::default(),
        global_context_index: Default::default(),
    });

    let source = concat!(
        "Процедура Тест()\n",
        "    ЭтотОбъект.МойМетод();\n",
        "КонецПроцедуры\n"
    );
    let parsed = parse(source, &ParseOptions::default()).expect("parse");
    let _ir = AstToIrConverter::convert_with_resolver(
        parsed.program,
        source.to_string(),
        "Documents/Док1/Ext/ObjectModule.bsl".to_string(),
        repo,
        signature_index,
        Some(resolver),
    )
    .expect("ir");

    let call_line = source.lines().nth(1).expect("call line");
    let method_byte = call_line.find("МойМетод").expect("method name");
    let method_col = utf16_col(call_line, method_byte);

    let (analysis, file_id, analysis_ir) = build_analysis_ir_for_source(
        source,
        "Documents/Док1/Ext/ObjectModule.bsl",
        deps,
        "goto-definition-object-module-legacy-test",
    );
    let target_method = goto_definition_with_analysis(
        source,
        &analysis,
        file_id,
        analysis_ir,
        empty_semantic_deps(),
        1,
        method_col,
    )
    .expect("method definition target");

    assert!(
        target_method.span.is_some(),
        "method definition should include declaration span"
    );
    let expected_module = root
        .join("Documents")
        .join("Док1")
        .join("Ext")
        .join("ObjectModule.bsl")
        .canonicalize()
        .expect("canonicalize expected module");
    let actual_module = target_method
        .file_path
        .canonicalize()
        .expect("canonicalize actual module");
    assert_eq!(actual_module, expected_module);
}

#[test]
fn goto_definition_resolves_configuration_symbol_metadata_xml_from_exact_semantic_index_with_empty_consumer_repo(
) {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    std::fs::write(
        root.join("Configuration.xml"),
        r#"<?xml version="1.0" encoding="UTF-8"?>
<MetaDataObject xmlns="http://v8.1c.ru/8.3/MDClasses">
  <Configuration uuid="00000000-0000-0000-0000-000000000000">
    <Properties>
      <Name>TestConfig</Name>
      <CompatibilityMode>Version8_3_25</CompatibilityMode>
    </Properties>
    <ChildObjects>
      <Document>Док1</Document>
    </ChildObjects>
  </Configuration>
</MetaDataObject>
"#,
    )
    .unwrap();

    std::fs::create_dir_all(root.join("Documents")).unwrap();
    std::fs::write(
        root.join("Documents").join("Док1.xml"),
        r#"<?xml version="1.0" encoding="UTF-8"?>
<MetaDataObject xmlns="http://v8.1c.ru/8.3/MDClasses">
  <Document uuid="00000000-0000-0000-0000-000000000001">
    <Properties>
      <Name>Док1</Name>
    </Properties>
  </Document>
</MetaDataObject>
"#,
    )
    .unwrap();

    let coordinator = SystemCoordinator::new();
    coordinator
        .start_with_paths_blocking(None, Some(Path::new(root)), Some("8.3.25"), None)
        .expect("startup");

    let deps_bundle = build_deps_bundle_v2(&coordinator, None, Some(root)).expect("deps bundle");
    let producer_deps = deps_bundle.semantic_deps.clone();
    let raw_document = producer_deps
        .repository
        .find_type("Документы.Док1")
        .expect("producer repo must contain document raw type");
    assert_eq!(
        raw_document
            .metadata_path
            .as_deref()
            .expect("document raw type must preserve metadata xml path"),
        root.join("Documents").join("Док1.xml").as_path()
    );

    let source = "Процедура Тест()\n    Результат = Документы.Док1;\nКонецПроцедуры\n";
    let (analysis, file_id, ir) = build_analysis_ir_for_source(
        source,
        "inline.bsl",
        producer_deps,
        "goto-definition-config-symbol-semantic-facts",
    );

    let line = source.lines().nth(1).expect("line");
    let doc_byte = line.rfind("Док1").expect("document name");
    let doc_col = utf16_col(line, doc_byte);
    let doc_offset = source
        .find("Документы.Док1")
        .expect("document access offset") as u32
        + "Документы.".len() as u32;
    let type_at_offset = analysis
        .type_at_byte_offset_serve_only(file_id, doc_offset)
        .ok()
        .flatten()
        .unwrap_or_else(|| {
            panic!(
                "type at document access is missing in exact semantic index; probe={doc_offset}; type_spans={:?}",
                ir.semantic_facts
                    .type_entries
                    .iter()
                    .map(|entry| (entry.span, entry.resolution.type_name()))
                    .collect::<Vec<_>>()
            )
        });
    let semantic_target = analysis
        .definition_location_at_byte_offset_serve_only(file_id, doc_offset)
        .ok()
        .flatten()
        .unwrap_or_else(|| {
            panic!(
                "exact semantic index must expose config definition location at document access; type_at_offset={:?}; spans={:?}",
                type_at_offset,
                ir.semantic_facts
                    .definition_locations_by_span
                    .keys()
                    .collect::<Vec<_>>()
            )
        });
    assert_eq!(
        semantic_target
            .primary_path()
            .expect("config definition path"),
        &root.join("Documents").join("Док1.xml")
    );

    let target = goto_definition_with_analysis(
        source,
        &analysis,
        file_id,
        ir,
        empty_semantic_deps(),
        1,
        doc_col,
    )
    .expect("configuration definition target from exact semantic index");

    assert!(
        target.span.is_none(),
        "configuration definition should point to metadata xml file"
    );
    let expected_xml = root
        .join("Documents")
        .join("Док1.xml")
        .canonicalize()
        .expect("canonicalize expected xml");
    let actual_xml = target
        .file_path
        .canonicalize()
        .expect("canonicalize actual xml target");
    assert_eq!(actual_xml, expected_xml);
}

#[test]
fn goto_definition_resolves_object_module_method_from_semantic_facts_with_empty_consumer_repo() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    std::fs::write(
        root.join("Configuration.xml"),
        r#"<?xml version="1.0" encoding="UTF-8"?>
<MetaDataObject xmlns="http://v8.1c.ru/8.3/MDClasses">
  <Configuration uuid="00000000-0000-0000-0000-000000000000">
    <Properties>
      <Name>TestConfig</Name>
      <CompatibilityMode>Version8_3_25</CompatibilityMode>
    </Properties>
    <ChildObjects>
      <Document>Док1</Document>
    </ChildObjects>
  </Configuration>
</MetaDataObject>
"#,
    )
    .unwrap();

    std::fs::create_dir_all(root.join("Documents")).unwrap();
    std::fs::write(
        root.join("Documents").join("Док1.xml"),
        r#"<?xml version="1.0" encoding="UTF-8"?>
<MetaDataObject xmlns="http://v8.1c.ru/8.3/MDClasses">
  <Document uuid="00000000-0000-0000-0000-000000000001">
    <Properties>
      <Name>Док1</Name>
    </Properties>
  </Document>
</MetaDataObject>
"#,
    )
    .unwrap();

    std::fs::create_dir_all(root.join("Documents").join("Док1").join("Ext")).unwrap();
    std::fs::write(
        root.join("Documents")
            .join("Док1")
            .join("Ext")
            .join("ObjectModule.bsl"),
        concat!(
            "Процедура МойМетод() Экспорт\n",
            "КонецПроцедуры\n",
            "\n",
            "Процедура Тест()\n",
            "    ЭтотОбъект.МойМетод();\n",
            "КонецПроцедуры\n"
        ),
    )
    .unwrap();

    let coordinator = SystemCoordinator::new();
    coordinator
        .start_with_paths_blocking(None, Some(Path::new(root)), Some("8.3.25"), None)
        .expect("startup");

    let deps_bundle = build_deps_bundle_v2(&coordinator, None, Some(root)).expect("deps bundle");
    let producer_deps = deps_bundle.semantic_deps.clone();

    let source = concat!(
        "Процедура Тест()\n",
        "    ЭтотОбъект.МойМетод();\n",
        "КонецПроцедуры\n"
    );
    let mut host = AnalysisHostV2::default();
    let file_id = FileId(1);
    host.apply_change(Change::SetDepsSnapshot {
        deps_id: DepsSnapshotId::from_hash("goto-definition-object-module-semantic-facts"),
        deps: producer_deps,
    });
    host.apply_change(Change::SetSettingsSnapshot {
        settings_id: SettingsId::from_hash("goto-definition-object-module-semantic-facts"),
        diagnostics_detail_level: DetailLevel::Full,
    });
    host.apply_change(Change::SetFile {
        file_id,
        text: Arc::from(source.to_string()),
        version: 0,
        path: Arc::from("Documents/Док1/Ext/ObjectModule.bsl"),
    });

    let analysis = host.analysis();
    analysis
        .precompute_type_index_for_file(file_id, Some(0), 0)
        .expect("precompute exact type index");
    let ir = analysis.ir(file_id).ok().flatten().expect("ir");

    let call_line = source.lines().nth(1).expect("call line");
    let method_byte = call_line.find("МойМетод").expect("method byte");
    let method_col = utf16_col(call_line, method_byte);
    let method_offset = source.find("МойМетод").expect("method offset") as u32;
    let method_fact = ir
        .semantic_facts
        .member_method_targets_by_span
        .iter()
        .find(|(span, target)| {
            span.contains(method_offset)
                && target.method_name.eq_ignore_ascii_case("МойМетод")
                && target.definition_location.is_some()
        })
        .or_else(|| {
            ir.semantic_facts
                .call_method_targets_by_span
                .iter()
                .find(|(span, target)| {
                    span.contains(method_offset)
                        && target.method_name.eq_ignore_ascii_case("МойМетод")
                        && target.definition_location.is_some()
                })
        });
    assert!(
        method_fact.is_some(),
        "missing object-module semantic method fact at offset {method_offset}; member_spans={:?}; call_spans={:?}",
        ir.semantic_facts
            .member_method_targets_by_span
            .keys()
            .collect::<Vec<_>>(),
        ir.semantic_facts
            .call_method_targets_by_span
            .keys()
            .collect::<Vec<_>>()
    );

    let target_method = goto_definition_with_analysis(
        source,
        &analysis,
        file_id,
        ir,
        empty_semantic_deps(),
        1,
        method_col,
    )
    .expect("method definition target from semantic facts");

    assert!(
        target_method.span.is_some(),
        "method definition should include declaration span"
    );
    let expected_module = root
        .join("Documents")
        .join("Док1")
        .join("Ext")
        .join("ObjectModule.bsl")
        .canonicalize()
        .expect("canonicalize expected module");
    let actual_module = target_method
        .file_path
        .canonicalize()
        .expect("canonicalize actual module");
    assert_eq!(actual_module, expected_module);
}

#[test]
fn goto_definition_uses_exact_semantic_index_when_runtime_ir_facts_are_missing() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    std::fs::write(
        root.join("Configuration.xml"),
        r#"<?xml version="1.0" encoding="UTF-8"?>
<MetaDataObject xmlns="http://v8.1c.ru/8.3/MDClasses">
  <Configuration uuid="00000000-0000-0000-0000-000000000000">
    <Properties>
      <Name>TestConfig</Name>
      <CompatibilityMode>Version8_3_25</CompatibilityMode>
    </Properties>
    <ChildObjects>
      <Document>Док1</Document>
    </ChildObjects>
  </Configuration>
</MetaDataObject>
"#,
    )
    .unwrap();

    std::fs::create_dir_all(root.join("Documents")).unwrap();
    std::fs::write(
        root.join("Documents").join("Док1.xml"),
        r#"<?xml version="1.0" encoding="UTF-8"?>
<MetaDataObject xmlns="http://v8.1c.ru/8.3/MDClasses">
  <Document uuid="00000000-0000-0000-0000-000000000001">
    <Properties>
      <Name>Док1</Name>
    </Properties>
  </Document>
</MetaDataObject>
"#,
    )
    .unwrap();

    std::fs::create_dir_all(root.join("Documents").join("Док1").join("Ext")).unwrap();
    std::fs::write(
        root.join("Documents")
            .join("Док1")
            .join("Ext")
            .join("ObjectModule.bsl"),
        concat!(
            "Процедура МойМетод() Экспорт\n",
            "КонецПроцедуры\n",
            "\n",
            "Процедура Тест()\n",
            "    ЭтотОбъект.МойМетод();\n",
            "КонецПроцедуры\n"
        ),
    )
    .unwrap();

    let coordinator = SystemCoordinator::new();
    coordinator
        .start_with_paths_blocking(None, Some(Path::new(root)), Some("8.3.25"), None)
        .expect("startup");

    let deps_bundle = build_deps_bundle_v2(&coordinator, None, Some(root)).expect("deps bundle");
    let producer_deps = deps_bundle.semantic_deps.clone();

    let source = concat!(
        "Процедура Тест()\n",
        "    ЭтотОбъект.МойМетод();\n",
        "КонецПроцедуры\n"
    );
    let mut host = AnalysisHostV2::default();
    let file_id = FileId(1);
    host.apply_change(Change::SetDepsSnapshot {
        deps_id: DepsSnapshotId::from_hash("goto-definition-exact-index-over-poisoned-ir"),
        deps: producer_deps,
    });
    host.apply_change(Change::SetSettingsSnapshot {
        settings_id: SettingsId::from_hash("goto-definition-exact-index-over-poisoned-ir"),
        diagnostics_detail_level: DetailLevel::Full,
    });
    host.apply_change(Change::SetFile {
        file_id,
        text: Arc::from(source.to_string()),
        version: 0,
        path: Arc::from("Documents/Док1/Ext/ObjectModule.bsl"),
    });

    let analysis = host.analysis();
    analysis
        .precompute_type_index_for_file(file_id, Some(0), 0)
        .expect("precompute exact type index");
    let ir = analysis.ir(file_id).ok().flatten().expect("ir");
    let mut poisoned_program = ir.as_ref().clone();
    poisoned_program.semantic_facts = Default::default();
    let poisoned_ir = Arc::new(poisoned_program);

    let call_line = source.lines().nth(1).expect("call line");
    let method_byte = call_line.find("МойМетод").expect("method byte");
    let method_col = utf16_col(call_line, method_byte);

    let target_method = goto_definition_with_analysis(
        source,
        &analysis,
        file_id,
        poisoned_ir,
        empty_semantic_deps(),
        1,
        method_col,
    )
    .expect("method definition target from exact semantic index");

    assert!(
        target_method.span.is_some(),
        "method definition should include declaration span"
    );
    let expected_module = root
        .join("Documents")
        .join("Док1")
        .join("Ext")
        .join("ObjectModule.bsl")
        .canonicalize()
        .expect("canonicalize expected module");
    let actual_module = target_method
        .file_path
        .canonicalize()
        .expect("canonicalize actual module");
    assert_eq!(actual_module, expected_module);
}

#[test]
fn goto_definition_uses_exact_semantic_index_after_diagnostics_only_query() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    std::fs::write(
        root.join("Configuration.xml"),
        r#"<?xml version="1.0" encoding="UTF-8"?>
<MetaDataObject xmlns="http://v8.1c.ru/8.3/MDClasses">
  <Configuration uuid="00000000-0000-0000-0000-000000000000">
    <Properties>
      <Name>TestConfig</Name>
      <CompatibilityMode>Version8_3_25</CompatibilityMode>
    </Properties>
    <ChildObjects>
      <Document>Док1</Document>
    </ChildObjects>
  </Configuration>
</MetaDataObject>
"#,
    )
    .unwrap();

    std::fs::create_dir_all(root.join("Documents")).unwrap();
    std::fs::write(
        root.join("Documents").join("Док1.xml"),
        r#"<?xml version="1.0" encoding="UTF-8"?>
<MetaDataObject xmlns="http://v8.1c.ru/8.3/MDClasses">
  <Document uuid="00000000-0000-0000-0000-000000000001">
    <Properties>
      <Name>Док1</Name>
    </Properties>
  </Document>
</MetaDataObject>
"#,
    )
    .unwrap();

    std::fs::create_dir_all(root.join("Documents").join("Док1").join("Ext")).unwrap();
    std::fs::write(
        root.join("Documents")
            .join("Док1")
            .join("Ext")
            .join("ObjectModule.bsl"),
        concat!(
            "Процедура МойМетод() Экспорт\n",
            "КонецПроцедуры\n",
            "\n",
            "Процедура Тест()\n",
            "    ЭтотОбъект.МойМетод();\n",
            "КонецПроцедуры\n"
        ),
    )
    .unwrap();

    let coordinator = SystemCoordinator::new();
    coordinator
        .start_with_paths_blocking(None, Some(Path::new(root)), Some("8.3.25"), None)
        .expect("startup");

    let deps_bundle = build_deps_bundle_v2(&coordinator, None, Some(root)).expect("deps bundle");
    let producer_deps = deps_bundle.semantic_deps.clone();

    let source = concat!(
        "Процедура Тест()\n",
        "    ЭтотОбъект.МойМетод();\n",
        "КонецПроцедуры\n"
    );
    let mut host = AnalysisHostV2::default();
    let file_id = FileId(1);
    host.apply_change(Change::SetDepsSnapshot {
        deps_id: DepsSnapshotId::from_hash("goto-definition-exact-index-after-diagnostics-only"),
        deps: producer_deps,
    });
    host.apply_change(Change::SetSettingsSnapshot {
        settings_id: SettingsId::from_hash("goto-definition-exact-index-after-diagnostics-only"),
        diagnostics_detail_level: DetailLevel::Full,
    });
    host.apply_change(Change::SetFile {
        file_id,
        text: Arc::from(source.to_string()),
        version: 0,
        path: Arc::from("Documents/Док1/Ext/ObjectModule.bsl"),
    });

    let analysis = host.analysis();
    let profiled = analysis
        .semantic_diagnostics_profiled(file_id)
        .expect("semantic diagnostics profile")
        .expect("semantic diagnostics result");
    assert_eq!(
        profiled.profile.materialization_path,
        Some(SemanticDiagnosticsMaterializationPath::DiagnosticsOnly)
    );

    analysis
        .precompute_type_index_for_file(file_id, Some(0), 0)
        .expect("precompute exact type index");
    let ir = analysis.ir(file_id).ok().flatten().expect("ir");
    let mut poisoned_program = ir.as_ref().clone();
    poisoned_program.semantic_facts = Default::default();
    let poisoned_ir = Arc::new(poisoned_program);

    let call_line = source.lines().nth(1).expect("call line");
    let method_byte = call_line.find("МойМетод").expect("method byte");
    let method_col = utf16_col(call_line, method_byte);

    let target_method = goto_definition_with_analysis(
        source,
        &analysis,
        file_id,
        poisoned_ir,
        empty_semantic_deps(),
        1,
        method_col,
    )
    .expect("method definition target from exact semantic index after diagnostics-only query");

    assert!(
        target_method.span.is_some(),
        "method definition should include declaration span"
    );
    let expected_module = root
        .join("Documents")
        .join("Док1")
        .join("Ext")
        .join("ObjectModule.bsl")
        .canonicalize()
        .expect("canonicalize expected module");
    let actual_module = target_method
        .file_path
        .canonicalize()
        .expect("canonicalize actual module");
    assert_eq!(actual_module, expected_module);
}

#[test]
fn goto_definition_resolves_local_function_from_semantic_facts_with_empty_consumer_repo() {
    let source = concat!(
        "Функция Локальная(Аргумент, Доп = Неопределено)\n",
        "    Возврат Аргумент;\n",
        "КонецФункции\n",
        "\n",
        "Процедура Тест()\n",
        "    Локальная(1, );\n",
        "КонецПроцедуры\n"
    );

    let producer_deps = empty_semantic_deps();
    let mut host = AnalysisHostV2::default();
    let file_id = FileId(1);
    host.apply_change(Change::SetDepsSnapshot {
        deps_id: DepsSnapshotId::from_hash("goto-definition-local-function-semantic-facts"),
        deps: producer_deps,
    });
    host.apply_change(Change::SetSettingsSnapshot {
        settings_id: SettingsId::from_hash("goto-definition-local-function-semantic-facts"),
        diagnostics_detail_level: DetailLevel::Full,
    });
    host.apply_change(Change::SetFile {
        file_id,
        text: Arc::from(source.to_string()),
        version: 0,
        path: Arc::from("inline.bsl"),
    });

    let analysis = host.analysis();
    analysis
        .precompute_type_index_for_file(file_id, Some(0), 0)
        .expect("precompute exact type index");
    let ir = analysis.ir(file_id).ok().flatten().expect("ir");

    let call_line = source.lines().nth(5).expect("call line");
    let method_byte = call_line.find("Локальная").expect("call name");
    let method_col = utf16_col(call_line, method_byte);
    let call_offset = source.rfind("Локальная(1, )").expect("call offset") as u32;
    let local_fact = ir
        .semantic_facts
        .call_method_targets_by_span
        .iter()
        .find(|(span, target)| {
            span.contains(call_offset)
                && target.method_name.eq_ignore_ascii_case("Локальная")
                && target.signature.is_some()
                && target.definition_location.is_some()
        });
    assert!(
        local_fact.is_some(),
        "missing local callable semantic fact at offset {call_offset}; spans={:?}",
        ir.semantic_facts
            .call_method_targets_by_span
            .keys()
            .collect::<Vec<_>>()
    );

    let target = goto_definition_with_analysis(
        source,
        &analysis,
        file_id,
        ir,
        empty_semantic_deps(),
        5,
        method_col,
    )
    .expect("local definition target from semantic facts");

    assert_eq!(target.file_path, std::path::PathBuf::from("inline.bsl"));
    assert!(target.span.is_some(), "local definition must include span");
}

#[test]
fn goto_definition_resolves_common_module_method_with_deps_bundle_v2_snapshot() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    std::fs::write(
        root.join("Configuration.xml"),
        r#"<?xml version="1.0" encoding="UTF-8"?>
<MetaDataObject xmlns="http://v8.1c.ru/8.3/MDClasses" xmlns:v8="http://v8.1c.ru/8.1/data/core">
  <Configuration uuid="00000000-0000-0000-0000-000000000000">
    <Properties>
      <Name>TestConfig</Name>
      <CompatibilityMode>Version8_3_25</CompatibilityMode>
    </Properties>
    <ChildObjects>
      <CommonModule>МойМодуль</CommonModule>
    </ChildObjects>
  </Configuration>
</MetaDataObject>
"#,
    )
    .unwrap();

    std::fs::create_dir_all(root.join("CommonModules")).unwrap();
    std::fs::write(
        root.join("CommonModules").join("МойМодуль.xml"),
        r#"<?xml version="1.0" encoding="UTF-8"?>
<MetaDataObject xmlns="http://v8.1c.ru/8.3/MDClasses" xmlns:v8="http://v8.1c.ru/8.1/data/core">
  <CommonModule uuid="00000000-0000-0000-0000-000000000001">
    <Properties>
      <Name>МойМодуль</Name>
      <Global>false</Global>
      <ClientManagedApplication>false</ClientManagedApplication>
      <ClientOrdinaryApplication>false</ClientOrdinaryApplication>
      <Server>true</Server>
      <ExternalConnection>false</ExternalConnection>
      <ServerCall>false</ServerCall>
      <Privileged>false</Privileged>
      <ReturnValuesReuse>DontUse</ReturnValuesReuse>
    </Properties>
  </CommonModule>
</MetaDataObject>
"#,
    )
    .unwrap();

    std::fs::create_dir_all(root.join("CommonModules").join("МойМодуль").join("Ext")).unwrap();
    std::fs::write(
        root.join("CommonModules")
            .join("МойМодуль")
            .join("Ext")
            .join("Module.bsl"),
        "Процедура ПриСозданииНаСервере() Экспорт\nКонецПроцедуры\n",
    )
    .unwrap();

    let coordinator = SystemCoordinator::new();
    coordinator
        .start_with_paths_blocking(None, Some(Path::new(root)), Some("8.3.25"), None)
        .expect("startup");

    let deps_bundle = build_deps_bundle_v2(&coordinator, None, Some(root)).expect("deps bundle");
    let repo = deps_bundle.semantic_deps.repository.clone();
    let signature_index = deps_bundle.semantic_deps.signature_index.clone();
    let resolver = deps_bundle
        .semantic_deps
        .resolver
        .clone()
        .expect("resolver");

    let deps = Arc::new(SemanticDeps {
        repository: repo.clone(),
        signature_index: signature_index.clone(),
        resolver: Some(resolver.clone()),
        platform_signatures_loaded: false,
        common_module_factory_registry: Default::default(),
        global_context_index: Default::default(),
    });

    let source = "Процедура Тест()\n    МойМодуль.ПриСозданииНаСервере();\nКонецПроцедуры\n";
    let parsed = parse(source, &ParseOptions::default()).expect("parse");
    let ir = AstToIrConverter::convert_with_resolver(
        parsed.program,
        source.to_string(),
        "inline.bsl".to_string(),
        repo,
        signature_index,
        Some(resolver),
    )
    .expect("ir");
    let ir = Arc::new(ir);

    let call_line = source.lines().nth(1).expect("call line");
    let module_byte = call_line.find("МойМодуль").expect("module name");
    let method_byte = call_line.find("ПриСозданииНаСервере").expect("method name");

    let module_col = utf16_col(call_line, module_byte);
    let method_col = utf16_col(call_line, method_byte);

    let target_module = type_system::goto_definition_v2_with_source(
        "inline.bsl",
        source,
        ir.clone(),
        deps.clone(),
        1,
        module_col,
    )
    .expect("module definition target");
    assert!(target_module.span.is_none());

    let (analysis, file_id, analysis_ir) = build_analysis_ir_for_source(
        source,
        "inline.bsl",
        deps,
        "goto-definition-common-module-snapshot-legacy-test",
    );
    let target_method = goto_definition_with_analysis(
        source,
        &analysis,
        file_id,
        analysis_ir,
        empty_semantic_deps(),
        1,
        method_col,
    )
    .expect("method definition target");
    assert!(target_method.span.is_some());
}
