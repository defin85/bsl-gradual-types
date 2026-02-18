//! Интеграционный контракт hover для `FormModule.Объект`/`ЭтотОбъект`
//! на реальной конфигурации `examples/conf_big` и платформенных типах.

mod support;

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

#[test]
fn conf_big_form_module_hover_contract_for_object_and_this_object() {
    let (Some(conf_big), Some(syntax_helper)) = (conf_big_root(), syntax_helper_root()) else {
        // Repo может поставляться без conf_big/syntax_helper — тогда тест пропускаем.
        return;
    };

    let deps_bundle = support::deps_bundle_v2_for_paths(
        Some(syntax_helper.as_path()),
        Some(conf_big.as_path()),
        Some("8.3.25"),
    );
    let repo_stats = deps_bundle.semantic_deps.repository.get_stats();
    assert!(
        repo_stats.configuration_types > 0,
        "real configuration types must be loaded"
    );
    assert!(
        repo_stats.platform_types > 0,
        "platform types must be loaded from syntax_helper"
    );

    let module_rel = "Documents/РеализацияТоваровУслуг/Forms/ФормаДокументаОбщая/Ext/Form/Module.bsl";
    let module_path = conf_big.join(module_rel);
    assert!(
        module_path.exists(),
        "expected module to exist: {}",
        module_path.display()
    );

    let original = std::fs::read_to_string(&module_path).expect("read conf_big form module");
    let probe = concat!(
        "\nПроцедура __HoverProbe_FormObjectContract()\n",
        "    ProbeObject = Объект;\n",
        "    ProbeThisObject = ЭтотОбъект;\n",
        "КонецПроцедуры\n",
    );
    let content = format!("{original}{probe}");

    let object_marker = "ProbeObject = Объект";
    let object_offset = content
        .find(object_marker)
        .and_then(|line_start| {
            object_marker
                .find("Объект")
                .map(|local| line_start + local)
        })
        .expect("object probe marker");
    let (object_line, object_column) = byte_offset_to_utf16_position(&content, object_offset);

    let this_object_marker = "ProbeThisObject = ЭтотОбъект";
    let this_object_offset = content
        .find(this_object_marker)
        .and_then(|line_start| {
            this_object_marker
                .find("ЭтотОбъект")
                .map(|local| line_start + local)
        })
        .expect("this-object probe marker");
    let (this_line, this_column) = byte_offset_to_utf16_position(&content, this_object_offset);

    let hover_config = HoverFormatConfig {
        detail_level: DetailLevel::Detailed,
        output_format: HoverOutputFormat::Markdown,
        ..Default::default()
    };

    let object_hover = support::hover_for_code_with_config(
        deps_bundle.as_ref(),
        module_rel,
        &content,
        object_line,
        object_column,
        Some(hover_config.clone()),
    )
    .expect("hover for Объект");

    assert!(
        object_hover.contains("ДанныеФормыСтруктура"),
        "Объект hover must use strict form-data label, got:\n{}",
        object_hover
    );
    assert!(
        !object_hover.contains("ДокументОбъект.РеализацияТоваровУслуг"),
        "Объект hover must not leak owner object facet label, got:\n{}",
        object_hover
    );
    assert!(
        !object_hover.contains("**Фасет:**"),
        "Объект hover must not show active facet block, got:\n{}",
        object_hover
    );
    assert!(
        !object_hover.contains("**Доступные фасеты:**"),
        "Объект hover must not show available facets block, got:\n{}",
        object_hover
    );
    assert!(
        !object_hover.contains("ДанныеФормыОбъект"),
        "legacy form-object alias leaked to hover, got:\n{}",
        object_hover
    );

    let this_object_hover = support::hover_for_code_with_config(
        deps_bundle.as_ref(),
        module_rel,
        &content,
        this_line,
        this_column,
        Some(hover_config),
    )
    .expect("hover for ЭтотОбъект");

    assert!(
        this_object_hover.contains("Формы.Документы.РеализацияТоваровУслуг.ФормаДокументаОбщая"),
        "ЭтотОбъект hover must resolve to form context type, got:\n{}",
        this_object_hover
    );
    assert!(
        this_object_hover.contains("**Объект**: ДанныеФормыСтруктура"),
        "ЭтотОбъект hover must expose `Объект: ДанныеФормыСтруктура`, got:\n{}",
        this_object_hover
    );
    assert!(
        !this_object_hover.contains("**Объект**: ДокументОбъект.РеализацияТоваровУслуг"),
        "ЭтотОбъект hover must not expose legacy owner-facet `Объект` property, got:\n{}",
        this_object_hover
    );
}
