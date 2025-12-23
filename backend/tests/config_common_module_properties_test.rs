//! Тесты парсинга свойств общих модулей (CommonModule)
//!
//! Проверяет корректность извлечения контекстных свойств из Configuration.xml:
//! - Server, Client, ExternalConnection - контексты выполнения
//! - Global, Privileged, ServerCall - специальные свойства
//! - ReturnValuesReuse - режим повторного использования

use bsl_backend::data::loaders::config_metadata_parser::{
    parser::UniversalMetadataParser,
    types::{ExecutionContext, ReturnValuesReuse},
};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Создать временный XML файл с содержимым CommonModule
fn create_temp_common_module(xml_content: &str) -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("CommonModule.xml");
    fs::write(&file_path, xml_content).unwrap();
    (temp_dir, file_path)
}

/// Хелпер для создания XML CommonModule с кастомными свойствами
#[allow(clippy::too_many_arguments)]
fn create_common_module_xml(
    name: &str,
    uuid: &str,
    server: bool,
    client_managed: bool,
    client_ordinary: bool,
    external_connection: bool,
    server_call: bool,
    global: bool,
    privileged: bool,
    return_values_reuse: &str,
) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<MetaDataObject xmlns="http://v8.1c.ru/8.3/MDClasses" xmlns:app="http://v8.1c.ru/8.2/managed-application/core" xmlns:cfg="http://v8.1c.ru/8.1/data/enterprise/current-config" xmlns:cmi="http://v8.1c.ru/8.2/managed-application/cmi" xmlns:ent="http://v8.1c.ru/8.1/data/enterprise" xmlns:lf="http://v8.1c.ru/8.2/managed-application/logform" xmlns:style="http://v8.1c.ru/8.1/data/ui/style" xmlns:sys="http://v8.1c.ru/8.1/data/ui/fonts/system" xmlns:v8="http://v8.1c.ru/8.1/data/core" xmlns:v8ui="http://v8.1c.ru/8.1/data/ui" xmlns:web="http://v8.1c.ru/8.1/data/ui/colors/web" xmlns:win="http://v8.1c.ru/8.1/data/ui/colors/windows" xmlns:xen="http://v8.1c.ru/8.3/xcf/enums" xmlns:xpr="http://v8.1c.ru/8.3/xcf/predef" xmlns:xr="http://v8.1c.ru/8.3/xcf/readable" xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" version="2.19">
  <CommonModule uuid="{}">
    <Properties>
      <Name>{}</Name>
      <Synonym>
        <v8:item>
          <v8:lang>ru</v8:lang>
          <v8:content>{}</v8:content>
        </v8:item>
      </Synonym>
      <Server>{}</Server>
      <ClientManagedApplication>{}</ClientManagedApplication>
      <ClientOrdinaryApplication>{}</ClientOrdinaryApplication>
      <ExternalConnection>{}</ExternalConnection>
      <ServerCall>{}</ServerCall>
      <Global>{}</Global>
      <Privileged>{}</Privileged>
      <ReturnValuesReuse>{}</ReturnValuesReuse>
    </Properties>
  </CommonModule>
</MetaDataObject>"#,
        uuid,
        name,
        name,
        server,
        client_managed,
        client_ordinary,
        external_connection,
        server_call,
        global,
        privileged,
        return_values_reuse
    )
}

#[test]
fn test_parse_server_only_module() {
    let xml = create_common_module_xml(
        "СерверныйМодуль",
        "00000000-0000-0000-0000-000000000001",
        true,  // server
        false, // client_managed
        false, // client_ordinary
        false, // external_connection
        false, // server_call
        false, // global
        false, // privileged
        "DontUse",
    );

    let (_temp, path) = create_temp_common_module(&xml);
    let metadata = UniversalMetadataParser::parse_any_object(&path).unwrap();

    assert_eq!(metadata.name, "СерверныйМодуль");
    assert_eq!(metadata.object_type_raw, "CommonModule");

    let props = metadata.common_module_properties.as_ref().unwrap();
    assert!(props.server);
    assert!(!props.client_managed_application);
    assert!(!props.global);

    assert_eq!(metadata.execution_contexts.len(), 1);
    assert!(metadata
        .execution_contexts
        .contains(&ExecutionContext::Server));
}

#[test]
fn test_parse_client_only_module() {
    let xml = create_common_module_xml(
        "КлиентскийМодуль",
        "00000000-0000-0000-0000-000000000002",
        false, // server
        true,  // client_managed
        false, // client_ordinary
        false, // external_connection
        false, // server_call
        true,  // global
        false, // privileged
        "DontUse",
    );

    let (_temp, path) = create_temp_common_module(&xml);
    let metadata = UniversalMetadataParser::parse_any_object(&path).unwrap();

    let props = metadata.common_module_properties.as_ref().unwrap();
    assert!(!props.server);
    assert!(props.client_managed_application);
    assert!(props.global);

    assert_eq!(metadata.execution_contexts.len(), 1);
    assert!(metadata
        .execution_contexts
        .contains(&ExecutionContext::ClientManaged));
}

#[test]
fn test_parse_client_server_module() {
    let xml = create_common_module_xml(
        "КлиентСерверМодуль",
        "00000000-0000-0000-0000-000000000003",
        true,  // server
        true,  // client_managed
        true,  // client_ordinary
        false, // external_connection
        false, // server_call
        true,  // global
        false, // privileged
        "DontUse",
    );

    let (_temp, path) = create_temp_common_module(&xml);
    let metadata = UniversalMetadataParser::parse_any_object(&path).unwrap();

    let props = metadata.common_module_properties.as_ref().unwrap();
    assert!(props.server);
    assert!(props.client_managed_application);
    assert!(props.client_ordinary_application);
    assert!(props.global);

    assert_eq!(metadata.execution_contexts.len(), 3);
    assert!(metadata
        .execution_contexts
        .contains(&ExecutionContext::Server));
    assert!(metadata
        .execution_contexts
        .contains(&ExecutionContext::ClientManaged));
    assert!(metadata
        .execution_contexts
        .contains(&ExecutionContext::ClientOrdinary));
}

#[test]
fn test_parse_privileged_module() {
    let xml = create_common_module_xml(
        "ПривилегированныйМодуль",
        "00000000-0000-0000-0000-000000000004",
        true,  // server
        false, // client_managed
        false, // client_ordinary
        false, // external_connection
        false, // server_call
        false, // global
        true,  // privileged
        "DontUse",
    );

    let (_temp, path) = create_temp_common_module(&xml);
    let metadata = UniversalMetadataParser::parse_any_object(&path).unwrap();

    let props = metadata.common_module_properties.as_ref().unwrap();
    assert!(props.server);
    assert!(props.privileged);
    assert!(!props.global);
}

#[test]
fn test_parse_server_call_module() {
    let xml = create_common_module_xml(
        "СерверныйВызовМодуль",
        "00000000-0000-0000-0000-000000000005",
        true,  // server
        true,  // client_managed
        false, // client_ordinary
        false, // external_connection
        true,  // server_call
        false, // global
        false, // privileged
        "DontUse",
    );

    let (_temp, path) = create_temp_common_module(&xml);
    let metadata = UniversalMetadataParser::parse_any_object(&path).unwrap();

    let props = metadata.common_module_properties.as_ref().unwrap();
    assert!(props.server);
    assert!(props.client_managed_application);
    assert!(props.server_call);
}

#[test]
fn test_parse_external_connection_module() {
    let xml = create_common_module_xml(
        "ВнешнееСоединениеМодуль",
        "00000000-0000-0000-0000-000000000006",
        true,  // server
        false, // client_managed
        false, // client_ordinary
        true,  // external_connection
        false, // server_call
        false, // global
        false, // privileged
        "DontUse",
    );

    let (_temp, path) = create_temp_common_module(&xml);
    let metadata = UniversalMetadataParser::parse_any_object(&path).unwrap();

    let props = metadata.common_module_properties.as_ref().unwrap();
    assert!(props.server);
    assert!(props.external_connection);

    assert_eq!(metadata.execution_contexts.len(), 2);
    assert!(metadata
        .execution_contexts
        .contains(&ExecutionContext::Server));
    assert!(metadata
        .execution_contexts
        .contains(&ExecutionContext::ExternalConnection));
}

#[test]
fn test_parse_return_values_reuse_during_request() {
    let xml = create_common_module_xml(
        "ПовторноеИспользованиеЗапрос",
        "00000000-0000-0000-0000-000000000007",
        true,  // server
        false, // client_managed
        false, // client_ordinary
        false, // external_connection
        false, // server_call
        false, // global
        false, // privileged
        "DuringRequest",
    );

    let (_temp, path) = create_temp_common_module(&xml);
    let metadata = UniversalMetadataParser::parse_any_object(&path).unwrap();

    let props = metadata.common_module_properties.as_ref().unwrap();
    assert_eq!(props.return_values_reuse, ReturnValuesReuse::DuringRequest);
}

#[test]
fn test_parse_return_values_reuse_during_session() {
    let xml = create_common_module_xml(
        "ПовторноеИспользованиеСеанс",
        "00000000-0000-0000-0000-000000000008",
        true,  // server
        false, // client_managed
        false, // client_ordinary
        false, // external_connection
        false, // server_call
        false, // global
        false, // privileged
        "DuringSession",
    );

    let (_temp, path) = create_temp_common_module(&xml);
    let metadata = UniversalMetadataParser::parse_any_object(&path).unwrap();

    let props = metadata.common_module_properties.as_ref().unwrap();
    assert_eq!(props.return_values_reuse, ReturnValuesReuse::DuringSession);
}

#[test]
fn test_parse_global_client_server_module() {
    let xml = create_common_module_xml(
        "ГлобальныйКлиентСервер",
        "00000000-0000-0000-0000-000000000009",
        true,  // server
        true,  // client_managed
        true,  // client_ordinary
        false, // external_connection
        false, // server_call
        true,  // global
        false, // privileged
        "DontUse",
    );

    let (_temp, path) = create_temp_common_module(&xml);
    let metadata = UniversalMetadataParser::parse_any_object(&path).unwrap();

    let props = metadata.common_module_properties.as_ref().unwrap();
    assert!(props.server);
    assert!(props.client_managed_application);
    assert!(props.client_ordinary_application);
    assert!(props.global);
    assert!(!props.privileged);
}

#[test]
fn test_execution_contexts_mapping() {
    let xml = create_common_module_xml(
        "ВсеКонтексты",
        "00000000-0000-0000-0000-000000000010",
        true, // server
        true, // client_managed
        true, // client_ordinary
        true, // external_connection
        false,
        false,
        false,
        "DontUse",
    );

    let (_temp, path) = create_temp_common_module(&xml);
    let metadata = UniversalMetadataParser::parse_any_object(&path).unwrap();

    assert_eq!(metadata.execution_contexts.len(), 4);
    assert!(metadata
        .execution_contexts
        .contains(&ExecutionContext::Server));
    assert!(metadata
        .execution_contexts
        .contains(&ExecutionContext::ClientManaged));
    assert!(metadata
        .execution_contexts
        .contains(&ExecutionContext::ClientOrdinary));
    assert!(metadata
        .execution_contexts
        .contains(&ExecutionContext::ExternalConnection));
}

#[test]
fn test_minimal_module_all_false() {
    let xml = create_common_module_xml(
        "МинимальныйМодуль",
        "00000000-0000-0000-0000-000000000011",
        false, // server
        false, // client_managed
        false, // client_ordinary
        false, // external_connection
        false, // server_call
        false, // global
        false, // privileged
        "DontUse",
    );

    let (_temp, path) = create_temp_common_module(&xml);
    let metadata = UniversalMetadataParser::parse_any_object(&path).unwrap();

    let props = metadata.common_module_properties.as_ref().unwrap();
    assert!(!props.server);
    assert!(!props.client_managed_application);
    assert!(!props.global);
    assert!(!props.privileged);

    assert_eq!(metadata.execution_contexts.len(), 0);
}

#[test]
fn test_common_module_properties_not_set_for_catalog() {
    // Проверка graceful degradation - для Catalog свойства не должны парситься
    let catalog_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<MetaDataObject xmlns="http://v8.1c.ru/8.3/MDClasses" version="2.19">
  <Catalog uuid="11111111-1111-1111-1111-111111111111">
    <Properties>
      <Name>ТестовыйСправочник</Name>
      <Synonym>
        <v8:item xmlns:v8="http://v8.1c.ru/8.1/data/core">
          <v8:lang>ru</v8:lang>
          <v8:content>Тестовый справочник</v8:content>
        </v8:item>
      </Synonym>
    </Properties>
  </Catalog>
</MetaDataObject>"#;

    let (_temp, path) = create_temp_common_module(catalog_xml);
    let metadata = UniversalMetadataParser::parse_any_object(&path).unwrap();

    assert_eq!(metadata.object_type_raw, "Catalog");
    assert!(metadata.common_module_properties.is_none());
    assert_eq!(metadata.execution_contexts.len(), 0);
}
