//! Регресс-тесты на конкретные проблемы IntelliSense в `examples/conf_big`:
//! - `Элементы.<Имя>` резолвится по Form.xml (в т.ч. UsualGroup)
//! - реквизиты формы (`<Attribute ...>`) доступны как идентификаторы с типом

mod support;

use std::path::PathBuf;
use std::sync::Arc;

use bsl_analysis_v2::{
    AnalysisHostV2, Change as ChangeV2, DepsSnapshotId, FileId as V2FileId, SettingsId,
};
use bsl_backend::data::loaders::config_metadata_parser::{FormParser, UniversalMetadataObject};
use bsl_shared::domain::repository::{InMemoryTypeRepository, TypeRepository};
use bsl_shared::domain::resolver::TypeResolver;
use bsl_shared::domain::types::{FacetKind, RawDataSource, RawPropertyData, RawTypeData};
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
        .find(|p| p.join("Configuration.xml").exists())
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
fn conf_big_form_module_attributes_and_elements_are_typed() {
    let Some(root) = conf_big_root() else {
        // Repo может поставляться без большой конфигурации — тогда тест пропускаем.
        return;
    };

    let form_xml = root
        .join("Documents")
        .join("РеализацияТоваровУслуг")
        .join("Forms")
        .join("ФормаДокументаОбщая")
        .join("Ext")
        .join("Form.xml");
    assert!(
        form_xml.exists(),
        "expected Form.xml to exist: {}",
        form_xml.display()
    );

    let form = FormParser::parse_form_xml(
        &form_xml,
        "Document.РеализацияТоваровУслуг",
        "ФормаДокументаОбщая",
    )
    .expect("parse Form.xml");

    // Синтетические типы формы/элементов создаются конвертером метаданных.
    let mut doc = UniversalMetadataObject::new(
        "Document".to_string(),
        "РеализацияТоваровУслуг".to_string(),
        "00000000-0000-0000-0000-000000000000".to_string(),
    );
    doc.forms.push(form);

    let mut raw_types = doc.to_raw_type_data_with_forms(None);
    raw_types.extend([
        RawTypeData {
            name: "ДокументОбъект".to_string(),
            source: RawDataSource::Platform,
            facets: vec![FacetKind::Object],
            properties: vec![RawPropertyData {
                name: "Ссылка".to_string(),
                prop_type: "ДокументСсылка".to_string(),
                is_readonly: true,
                collection_item_type: None,
            }],
            ..Default::default()
        },
        RawTypeData {
            name: "ДокументСсылка".to_string(),
            source: RawDataSource::Platform,
            facets: vec![FacetKind::Reference],
            ..Default::default()
        },
    ]);
    let repo: Arc<dyn TypeRepository> = Arc::new(InMemoryTypeRepository::new());
    repo.load_types(raw_types).expect("load synthetic types");
    let form_type_name = "Формы.Документы.РеализацияТоваровУслуг.ФормаДокументаОбщая";
    let form_type = repo
        .find_type(form_type_name)
        .expect("expected synthetic Формы.* type to be present");
    let sf_prop = form_type
        .properties
        .iter()
        .find(|p| p.name == "СчетФактура")
        .expect("expected form attribute СчетФактура to be present");
    assert!(
        sf_prop.prop_type.contains("cfg:DocumentRef."),
        "expected СчетФактура prop type to be cfg:DocumentRef.*, got {:?}",
        sf_prop.prop_type
    );
    let elements_type_name = "ЭлементыФормы.Документы.РеализацияТоваровУслуг.ФормаДокументаОбщая";
    let elements_type = repo
        .find_type(elements_type_name)
        .expect("expected synthetic ЭлементыФормы.* type to be present");
    assert!(
        elements_type
            .properties
            .iter()
            .any(|p| p.name == "СчетФактураПросмотр"),
        "expected element `СчетФактураПросмотр` to be present in {} properties",
        elements_type_name
    );
    let object_prop = form_type
        .properties
        .iter()
        .find(|p| p.name == "Объект")
        .expect("expected form property Объект to be present");
    assert_eq!(object_prop.prop_type, "ДанныеФормыСтруктура");

    // Минимальный код: используем реквизит формы и элемент формы.
    let file_path =
        "Documents/РеализацияТоваровУслуг/Forms/ФормаДокументаОбщая/Ext/Form/Module.bsl";
    let code = concat!(
        "Процедура Тест()\n",
        "    x = Элементы.СчетФактураПросмотр;\n",
        "    y = СчетФактура;\n",
        "    z = Объект.Ссылка;\n",
        "КонецПроцедуры\n",
    );

    // Прогоняем v2 pipeline (IR + semantic diagnostics) на нашем репозитории типов.
    let signature_index = repo.get_signature_index_clone();
    let resolver = Arc::new(TypeResolver::new(repo.clone()));
    let deps = Arc::new(bsl_analysis_v2::SemanticDeps {
        repository: repo.clone(),
        signature_index,
        resolver: Some(resolver),
        platform_signatures_loaded: false,
        global_context_index: Default::default(),
    });

    let mut host = AnalysisHostV2::default();
    host.apply_change(ChangeV2::SetDepsSnapshot {
        deps_id: DepsSnapshotId::from_hash("test"),
        deps,
    });
    host.apply_change(ChangeV2::SetSettingsSnapshot {
        settings_id: SettingsId::from_hash("test"),
        diagnostics_detail_level: DetailLevel::Full,
    });
    host.apply_change(ChangeV2::SetFile {
        file_id: V2FileId(1),
        text: Arc::from(code),
        version: 0,
        path: Arc::from(file_path.to_string()),
    });

    let analysis = host.analysis();

    let diags = analysis
        .semantic_diagnostics(V2FileId(1))
        .ok()
        .flatten()
        .as_deref()
        .cloned()
        .unwrap_or_default();

    assert!(
        diags
            .iter()
            .all(|d| !d.message.contains("СчетФактураПросмотр")),
        "unexpected diagnostics mentioning form element 'СчетФактураПросмотр': {:?}",
        diags.iter().map(|d| d.message.clone()).collect::<Vec<_>>()
    );
    assert!(
        diags
            .iter()
            .all(|d| !d.message.contains("Свойство 'Ссылка'")),
        "unexpected diagnostics for Объект.Ссылка: {:?}",
        diags.iter().map(|d| d.message.clone()).collect::<Vec<_>>()
    );
    assert!(
        diags
            .iter()
            .all(|d| !d.message.contains("ДанныеФормыОбъект")),
        "legacy alias leaked to diagnostics: {:?}",
        diags.iter().map(|d| d.message.clone()).collect::<Vec<_>>()
    );
    let x_offset = analysis
        .utf16_position_to_byte_offset(V2FileId(1), 1, 17)
        .expect("utf16_position_to_byte_offset query for x")
        .expect("x offset present") as u32;
    let got_x = analysis
        .type_at_byte_offset(V2FileId(1), x_offset)
        .expect("type_at_byte_offset query for x")
        .map(|ty| bsl_shared::formatting::user_facing_resolution_type_name(&ty));

    let x_receiver_offset = analysis
        .utf16_position_to_byte_offset(V2FileId(1), 1, 8)
        .expect("utf16_position_to_byte_offset query for x receiver")
        .expect("x receiver offset present") as u32;
    let got_x_receiver = analysis
        .type_at_byte_offset(V2FileId(1), x_receiver_offset)
        .expect("type_at_byte_offset query for x receiver")
        .map(|ty| bsl_shared::formatting::user_facing_resolution_type_name(&ty));

    let y_offset = analysis
        .utf16_position_to_byte_offset(V2FileId(1), 2, 8)
        .expect("utf16_position_to_byte_offset query for y")
        .expect("y offset present") as u32;
    let got_y = analysis
        .type_at_byte_offset(V2FileId(1), y_offset)
        .expect("type_at_byte_offset query for y")
        .map(|ty| bsl_shared::formatting::user_facing_resolution_type_name(&ty));
    let z_receiver_offset = analysis
        .utf16_position_to_byte_offset(V2FileId(1), 3, 8)
        .expect("utf16_position_to_byte_offset query for z receiver")
        .expect("z receiver offset present") as u32;
    let got_z_receiver = analysis
        .type_at_byte_offset(V2FileId(1), z_receiver_offset)
        .expect("type_at_byte_offset query for z receiver")
        .map(|ty| bsl_shared::formatting::user_facing_resolution_type_name(&ty));
    let z_offset = analysis
        .utf16_position_to_byte_offset(V2FileId(1), 3, 15)
        .expect("utf16_position_to_byte_offset query for z member")
        .expect("z member offset present") as u32;
    let got_z = analysis
        .type_at_byte_offset(V2FileId(1), z_offset)
        .expect("type_at_byte_offset query for z member")
        .map(|ty| bsl_shared::formatting::user_facing_resolution_type_name(&ty));

    assert_eq!(
        got_x_receiver.as_deref(),
        Some(elements_type_name),
        "Expected `Элементы` receiver to be seeded as `{}`",
        elements_type_name
    );

    assert_eq!(
        got_x.as_deref(),
        Some("ГруппаФормы"),
        "Expected `Элементы.СчетФактураПросмотр` to resolve as `ГруппаФормы`"
    );
    assert!(
        got_y
            .as_deref()
            .unwrap_or_default()
            .contains("cfg:DocumentRef."),
        "Expected `СчетФактура` to resolve as cfg:DocumentRef.*, got {:?}",
        got_y
    );
    assert_eq!(
        got_z_receiver.as_deref(),
        Some("ДанныеФормыСтруктура"),
        "Expected `Объект` receiver to resolve as strict form-data type"
    );
    assert_eq!(
        got_z.as_deref(),
        Some("ДокументСсылка.РеализацияТоваровУслуг"),
        "Expected `Объект.Ссылка` to resolve as typed document reference"
    );
}

#[test]
fn conf_big_form_module_common_module_receivers_are_resolved() {
    let Some(root) = conf_big_root() else {
        return;
    };

    let deps_bundle = support::deps_bundle_v2_for_paths(None, Some(root.as_path()), Some("8.3.25"));
    let file_path =
        "Documents/РеализацияТоваровУслуг/Forms/ФормаДокументаОбщая/Ext/Form/Module.bsl";
    let original = std::fs::read_to_string(root.join(file_path)).expect("read form module file");
    let probe = concat!(
        "\nПроцедура __Probe_CommonModuleReceiver()\n",
        "    Значение = РеализацияТоваровУслугФормы.СтрокаСсылкиПоказатьНаКарте();\n",
        "КонецПроцедуры\n",
    );
    let content = format!("{original}{probe}");

    let diagnostics =
        support::semantic_diagnostics_for_code(deps_bundle.as_ref(), file_path, &content);
    let diagnostic_messages = diagnostics
        .iter()
        .map(|diag| diag.message.clone())
        .collect::<Vec<_>>();

    assert!(
        diagnostics.iter().all(|diag| {
            !(diag.message.contains("Необъявленная переменная")
                && diag.message.contains("РеализацияТоваровУслугФормы"))
        }),
        "unexpected undeclared diagnostics for common module receiver: {:?}",
        diagnostic_messages
    );

    let receiver_offset = content
        .rfind("РеализацияТоваровУслугФормы")
        .expect("probe common module receiver");
    let (line, column) = byte_offset_to_utf16_position(&content, receiver_offset);
    let hover = support::hover_for_code(deps_bundle.as_ref(), file_path, &content, line, column)
        .expect("hover for common module receiver");

    assert!(
        hover.contains("ОбщиеМодули.РеализацияТоваровУслугФормы"),
        "expected common module receiver hover to expose its canonical type, got:\n{}",
        hover
    );
}
