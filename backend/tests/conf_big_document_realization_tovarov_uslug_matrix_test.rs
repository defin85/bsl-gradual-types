//! Интеграционный тест-матрица для конкретного документа
//! `examples/conf_big/Documents/РеализацияТоваровУслуг`.
//!
//! Проверяет hover-контракт для `ЭтотОбъект`/`Объект` в:
//! - FormModule (`ФормаДокументаОбщая`)
//! - ObjectModule
//! - ManagerModule

mod support;

use std::collections::BTreeSet;
use std::path::PathBuf;

use bsl_backend::helpers::hover_formatter::{HoverFormatConfig, HoverOutputFormat};
use bsl_shared::formatting::DetailLevel;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn conf_big_root() -> Option<PathBuf> {
    let candidates = [
        workspace_root().join("examples").join("conf_big"),
        PathBuf::from("examples/conf_big"),
        PathBuf::from("../examples/conf_big"),
    ];
    candidates
        .into_iter()
        .find(|path| path.join("Configuration.xml").exists())
}

fn syntax_helper_root() -> Option<PathBuf> {
    let candidates = [
        workspace_root().join("examples").join("syntax_helper"),
        PathBuf::from("examples/syntax_helper"),
        PathBuf::from("../examples/syntax_helper"),
    ];
    candidates
        .into_iter()
        .find(|path| path.join("syntax.xml").exists())
}

fn byte_offset_to_utf16_position(content: &str, offset: usize) -> (u32, u32) {
    let prefix = &content[..offset];
    let line = prefix.bytes().filter(|b| *b == b'\n').count() as u32;
    let line_start = prefix.rfind('\n').map_or(0usize, |idx| idx + 1);
    let column = content[line_start..offset]
        .chars()
        .map(|ch| ch.len_utf16())
        .sum::<usize>() as u32;
    (line, column)
}

fn hover_for_marker(
    deps_bundle: &bsl_backend::system::DepsBundleV2,
    file_path: &str,
    content: &str,
    line_fragment: &str,
    symbol: &str,
) -> String {
    let offset = content
        .find(line_fragment)
        .and_then(|line_start| {
            line_fragment
                .find(symbol)
                .map(|local| line_start + local)
        })
        .expect("symbol marker in probe code");
    let (line, column) = byte_offset_to_utf16_position(content, offset);
    let hover_config = HoverFormatConfig {
        detail_level: DetailLevel::Detailed,
        output_format: HoverOutputFormat::Markdown,
        ..Default::default()
    };
    support::hover_for_code_with_config(
        deps_bundle,
        file_path,
        content,
        line,
        column,
        Some(hover_config),
    )
    .expect("hover")
}

fn parse_debugger_property_names(path: &std::path::Path) -> BTreeSet<String> {
    let raw = std::fs::read_to_string(path).expect("read debugger dump");
    let raw = raw.trim_start_matches('\u{feff}');

    raw.lines()
        .filter_map(|line| {
            let name = line.split('\t').next()?.trim();
            if name.is_empty() || name == "Свойство" {
                return None;
            }
            Some(name.to_string())
        })
        .collect()
}

fn parse_hover_property_names(hover: &str) -> BTreeSet<String> {
    let mut in_properties = false;
    let mut out = BTreeSet::new();

    for line in hover.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("Свойства (показано ") {
            in_properties = true;
            continue;
        }

        if !in_properties {
            continue;
        }

        if trimmed.starts_with("* **") {
            if let Some(after_prefix) = trimmed.strip_prefix("* **") {
                if let Some((name, _)) = after_prefix.split_once("**:") {
                    out.insert(name.to_string());
                }
            }
            continue;
        }

        if trimmed.is_empty() || trimmed.starts_with("...") {
            continue;
        }

        // Началась следующая секция hover.
        break;
    }

    out
}

#[test]
fn conf_big_realizatsiya_tovarov_uslug_contextual_object_matrix() {
    let (Some(conf_big), Some(syntax_helper)) = (conf_big_root(), syntax_helper_root()) else {
        // Конфиг/платформенные типы могут отсутствовать локально — тогда пропускаем.
        return;
    };

    let deps_bundle = support::deps_bundle_v2_for_paths(
        Some(syntax_helper.as_path()),
        Some(conf_big.as_path()),
        Some("8.3.25"),
    );
    let stats = deps_bundle.semantic_deps.repository.get_stats();
    assert!(
        stats.configuration_types > 0,
        "configuration types must be loaded"
    );
    assert!(stats.platform_types > 0, "platform types must be loaded");

    // 1) FormModule
    let form_rel =
        "Documents/РеализацияТоваровУслуг/Forms/ФормаДокументаОбщая/Ext/Form/Module.bsl";
    let form_original =
        std::fs::read_to_string(conf_big.join(form_rel)).expect("read form module file");
    let form_probe = concat!(
        "\nПроцедура __Probe_Form_RealizationTovaryUslugi()\n",
        "    ProbeThis = ЭтотОбъект;\n",
        "    ProbeObject = Объект;\n",
        "КонецПроцедуры\n",
    );
    let form_content = format!("{form_original}{form_probe}");
    let form_object_hover = hover_for_marker(
        deps_bundle.as_ref(),
        form_rel,
        &form_content,
        "ProbeObject = Объект;",
        "Объект",
    );
    let form_this_hover = hover_for_marker(
        deps_bundle.as_ref(),
        form_rel,
        &form_content,
        "ProbeThis = ЭтотОбъект;",
        "ЭтотОбъект",
    );

    assert!(
        form_object_hover.contains("ДанныеФормыСтруктура"),
        "FormModule.Объект must be form-data, got:\n{}",
        form_object_hover
    );
    assert!(
        !form_object_hover.contains("ДокументОбъект.РеализацияТоваровУслуг"),
        "FormModule.Объект must not leak object facet label, got:\n{}",
        form_object_hover
    );
    assert!(
        !form_object_hover.contains("**Фасет:**"),
        "FormModule.Объект must not show facet block, got:\n{}",
        form_object_hover
    );
    assert!(
        form_this_hover.contains("Формы.Документы.РеализацияТоваровУслуг.ФормаДокументаОбщая"),
        "FormModule.ЭтотОбъект must resolve as form type, got:\n{}",
        form_this_hover
    );
    assert!(
        form_this_hover.contains("**Объект**: ДанныеФормыСтруктура"),
        "FormModule.ЭтотОбъект must expose Объект as form-data property, got:\n{}",
        form_this_hover
    );

    // 2) ObjectModule
    let object_rel = "Documents/РеализацияТоваровУслуг/Ext/ObjectModule.bsl";
    let object_original =
        std::fs::read_to_string(conf_big.join(object_rel)).expect("read object module file");
    let object_probe = concat!(
        "\nПроцедура __Probe_Object_RealizationTovaryUslugi()\n",
        "    ProbeThis = ЭтотОбъект;\n",
        "    ProbeObject = Объект;\n",
        "КонецПроцедуры\n",
    );
    let object_content = format!("{object_original}{object_probe}");
    let object_this_hover = hover_for_marker(
        deps_bundle.as_ref(),
        object_rel,
        &object_content,
        "ProbeThis = ЭтотОбъект;",
        "ЭтотОбъект",
    );
    let object_object_hover = hover_for_marker(
        deps_bundle.as_ref(),
        object_rel,
        &object_content,
        "ProbeObject = Объект;",
        "Объект",
    );

    assert!(
        object_this_hover.contains("ДокументОбъект.РеализацияТоваровУслуг"),
        "ObjectModule.ЭтотОбъект must resolve as document object facet, got:\n{}",
        object_this_hover
    );
    assert!(
        object_object_hover.contains("ДокументОбъект.РеализацияТоваровУслуг"),
        "ObjectModule.Объект must resolve as document object facet, got:\n{}",
        object_object_hover
    );

    // 3) ManagerModule
    let manager_rel = "Documents/РеализацияТоваровУслуг/Ext/ManagerModule.bsl";
    let manager_original =
        std::fs::read_to_string(conf_big.join(manager_rel)).expect("read manager module file");
    let manager_probe = concat!(
        "\nПроцедура __Probe_Manager_RealizationTovaryUslugi()\n",
        "    ProbeThis = ЭтотОбъект;\n",
        "    ProbeObject = Объект;\n",
        "КонецПроцедуры\n",
    );
    let manager_content = format!("{manager_original}{manager_probe}");
    let manager_this_hover = hover_for_marker(
        deps_bundle.as_ref(),
        manager_rel,
        &manager_content,
        "ProbeThis = ЭтотОбъект;",
        "ЭтотОбъект",
    );
    let manager_object_hover = hover_for_marker(
        deps_bundle.as_ref(),
        manager_rel,
        &manager_content,
        "ProbeObject = Объект;",
        "Объект",
    );

    assert!(
        manager_this_hover.contains("ДокументМенеджер.РеализацияТоваровУслуг"),
        "ManagerModule.ЭтотОбъект must resolve as document manager facet, got:\n{}",
        manager_this_hover
    );
    assert!(
        manager_object_hover.contains("ДокументМенеджер.РеализацияТоваровУслуг"),
        "ManagerModule.Объект must resolve as document manager facet, got:\n{}",
        manager_object_hover
    );
}

#[test]
fn conf_big_realizatsiya_tovarov_uslug_hover_properties_compare_with_debugger_samples() {
    let (Some(conf_big), Some(syntax_helper)) = (conf_big_root(), syntax_helper_root()) else {
        return;
    };

    let workspace = workspace_root();
    let object_dump_path = workspace.join("Объект.txt");
    let this_object_dump_path = workspace.join("ЭтотОбъект.txt");
    if !object_dump_path.exists() || !this_object_dump_path.exists() {
        // Локальные дампы отладчика могут отсутствовать в CI/у других разработчиков.
        return;
    }

    let deps_bundle = support::deps_bundle_v2_for_paths(
        Some(syntax_helper.as_path()),
        Some(conf_big.as_path()),
        Some("8.3.25"),
    );
    let stats = deps_bundle.semantic_deps.repository.get_stats();
    assert!(
        stats.configuration_types > 0 && stats.platform_types > 0,
        "expected loaded config and platform types"
    );

    let form_rel =
        "Documents/РеализацияТоваровУслуг/Forms/ФормаДокументаОбщая/Ext/Form/Module.bsl";
    let original = std::fs::read_to_string(conf_big.join(form_rel)).expect("read form module");
    let probe = concat!(
        "\nПроцедура __Probe_DebuggerParity_RealizationTovaryUslugi()\n",
        "    ProbeObject = Объект;\n",
        "    ProbeThis = ЭтотОбъект;\n",
        "КонецПроцедуры\n",
    );
    let content = format!("{original}{probe}");

    let object_hover = hover_for_marker(
        deps_bundle.as_ref(),
        form_rel,
        &content,
        "ProbeObject = Объект;",
        "Объект",
    );
    let this_object_hover = hover_for_marker(
        deps_bundle.as_ref(),
        form_rel,
        &content,
        "ProbeThis = ЭтотОбъект;",
        "ЭтотОбъект",
    );

    let object_hover_props = parse_hover_property_names(&object_hover);
    let this_object_hover_props = parse_hover_property_names(&this_object_hover);
    assert!(
        !object_hover_props.is_empty(),
        "failed to parse properties from Объект hover:\n{}",
        object_hover
    );
    assert!(
        !this_object_hover_props.is_empty(),
        "failed to parse properties from ЭтотОбъект hover:\n{}",
        this_object_hover
    );

    let object_debugger_props = parse_debugger_property_names(&object_dump_path);
    let this_object_debugger_props = parse_debugger_property_names(&this_object_dump_path);
    assert!(
        !object_debugger_props.is_empty(),
        "Объект.txt has no parsed properties"
    );
    assert!(
        !this_object_debugger_props.is_empty(),
        "ЭтотОбъект.txt has no parsed properties"
    );

    let unexpected_object: Vec<_> = object_hover_props
        .difference(&object_debugger_props)
        .cloned()
        .collect();
    assert!(
        unexpected_object.is_empty(),
        "hover(Объект) has properties absent in Объект.txt: {:?}",
        unexpected_object
    );

    let unexpected_this_object: Vec<_> = this_object_hover_props
        .difference(&this_object_debugger_props)
        .cloned()
        .collect();
    assert!(
        unexpected_this_object.is_empty(),
        "hover(ЭтотОбъект) has properties absent in ЭтотОбъект.txt: {:?}",
        unexpected_this_object
    );

    let missing_from_object_hover: Vec<_> = object_debugger_props
        .difference(&object_hover_props)
        .take(25)
        .cloned()
        .collect();
    let missing_from_this_object_hover: Vec<_> = this_object_debugger_props
        .difference(&this_object_hover_props)
        .take(25)
        .cloned()
        .collect();

    println!(
        "debugger parity report: object hover={} debugger={} missing_sample={:?}",
        object_hover_props.len(),
        object_debugger_props.len(),
        missing_from_object_hover
    );
    println!(
        "debugger parity report: this_object hover={} debugger={} missing_sample={:?}",
        this_object_hover_props.len(),
        this_object_debugger_props.len(),
        missing_from_this_object_hover
    );

    // Smoke checks для наиболее критичных свойств контекста формы.
    assert!(
        object_hover_props.contains("Ссылка"),
        "hover(Объект) must contain property 'Ссылка'"
    );
    assert!(
        this_object_hover_props.contains("Объект")
            && this_object_hover_props.contains("Элементы")
            && this_object_hover_props.contains("Параметры"),
        "hover(ЭтотОбъект) must contain baseline form context properties"
    );
}
