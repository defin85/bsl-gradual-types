//! Go to Definition regression test: configuration CommonModules.

use bsl_analysis_v2::SemanticDeps;
use bsl_backend::application::type_system;
use bsl_backend::system::build_deps_bundle_v2;
use bsl_backend::system::SystemCoordinator;
use bsl_semantic::AstToIrConverter;
use bsl_shared::domain::resolver::TypeResolver;
use bsl_syntax::{parse, ParseOptions};
use std::path::Path;
use std::sync::Arc;
use tempfile::TempDir;

fn utf16_col(line: &str, byte_idx: usize) -> u32 {
    line[..byte_idx].chars().map(|c| c.len_utf16() as u32).sum()
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

    let engine = coordinator.analysis_engine().expect("analysis_engine");
    let repo = engine.get_repository();

    let signature_index = repo.get_signature_index_clone();
    let resolver = Arc::new(TypeResolver::new(repo.clone()));
    let deps = Arc::new(SemanticDeps {
        repository: repo.clone(),
        signature_index: signature_index.clone(),
        resolver: Some(resolver.clone()),
        platform_signatures_loaded: false,
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

    let target_method =
        type_system::goto_definition_v2_with_source("inline.bsl", source, ir, deps, 1, method_col)
            .expect("method definition target");
    assert!(
        target_method.span.is_some(),
        "method definition should include span"
    );
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

    let target_method =
        type_system::goto_definition_v2_with_source("inline.bsl", source, ir, deps, 1, method_col)
            .expect("method definition target");
    assert!(target_method.span.is_some());
}
