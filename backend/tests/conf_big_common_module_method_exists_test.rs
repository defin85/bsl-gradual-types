//! Интеграционный тест: проверяем, что экспортный метод из CommonModule conf_big индексируется.
//!
//! Регрессия: файлы conf_big часто в UTF-8 с BOM (и CRLF). Индексация должна видеть экспорт.

use bsl_backend::data::loaders::config_metadata_parser::parser::UniversalMetadataParser;
use bsl_backend::data::loaders::index_configuration_bsl_modules;
use std::path::PathBuf;

#[test]
fn test_conf_big_common_module_indexes_proverit_obekt_obrabotan() {
    let candidates = [
        PathBuf::from("examples/conf_big"),
        PathBuf::from("../examples/conf_big"),
    ];

    let Some(config_root) = candidates
        .into_iter()
        .find(|p| p.join("Configuration.xml").exists())
    else {
        return;
    };

    let canonical = config_root.canonicalize().expect("canonicalize conf_big");

    let module_name = "ОбновлениеИнформационнойБазы";
    let module_xml = canonical
        .join("CommonModules")
        .join(format!("{module_name}.xml"));

    assert!(
        module_xml.exists(),
        "expected CommonModule XML to exist: {}",
        module_xml.display()
    );

    let metadata = UniversalMetadataParser::parse_any_object(&module_xml)
        .expect("parse CommonModule metadata");

    assert_eq!(metadata.name, module_name);
    assert!(
        metadata.common_module_properties.is_some(),
        "expected CommonModuleProperties to be parsed for {module_name}"
    );

    let indexed =
        index_configuration_bsl_modules(&canonical, &[metadata]).expect("index module methods");

    let owner = format!("ОбщиеМодули.{module_name}");
    assert!(
        indexed
            .config_methods
            .iter()
            .any(|(t, s)| t == &owner && s.name == "ПроверитьОбъектОбработан"),
        "expected to index method ПроверитьОбъектОбработан for {owner}"
    );
}
