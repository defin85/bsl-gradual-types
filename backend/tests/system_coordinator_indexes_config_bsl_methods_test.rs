//! Интеграционный тест: SystemCoordinator при старте с `project_path`
//! индексирует экспортные методы из `*.bsl` модулей конфигурации.

use bsl_backend::system::SystemCoordinator;
use std::path::Path;
use tempfile::TempDir;

#[test]
fn startup_indexes_common_module_methods() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    // Минимальный Configuration.xml, достаточный для discovery:
    // - <Properties><Name>...</Name>
    // - <ChildObjects><CommonModule>...</CommonModule>
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
        "\u{FEFF}Процедура ПроверитьОбъектОбработан() Экспорт\r\nКонецПроцедуры\r\n",
    )
    .unwrap();

    let coordinator = SystemCoordinator::new();
    coordinator
        .start_with_paths_blocking(None, Some(Path::new(root)), Some("8.3.25"), None)
        .expect("startup");

    let engine = coordinator.analysis_engine().expect("analysis_engine");
    let repo = engine.get_repository();

    let sig = repo.find_method_signature(Some("ОбщиеМодули.МойМодуль"), "ПроверитьОбъектОбработан");
    assert!(
        sig.is_some(),
        "expected method signature to be indexed at startup"
    );
}
