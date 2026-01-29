//! Регресс-тесты на конкретные проблемы IntelliSense в `examples/conf_big`:
//! - `Элементы.<Имя>` резолвится по Form.xml (в т.ч. UsualGroup)
//! - реквизиты формы (`<Attribute ...>`) доступны как идентификаторы с типом

use std::path::PathBuf;
use std::sync::Arc;

use bsl_analysis_v2::{
    AnalysisHostV2, Change as ChangeV2, DepsSnapshotId, FileId as V2FileId, SettingsId,
};
use bsl_backend::data::loaders::config_metadata_parser::{FormParser, UniversalMetadataObject};
use bsl_shared::domain::repository::{InMemoryTypeRepository, TypeRepository};
use bsl_shared::domain::resolver::TypeResolver;
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

    let raw_types = doc.to_raw_type_data_with_forms(None);
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
    let elements_type_name =
        "ЭлементыФормы.Документы.РеализацияТоваровУслуг.ФормаДокументаОбщая";
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

    // Минимальный код: используем реквизит формы и элемент формы.
    let file_path =
        "Documents/РеализацияТоваровУслуг/Forms/ФормаДокументаОбщая/Ext/Form/Module.bsl";
    let code = concat!(
        "Процедура Тест()\n",
        "    x = Элементы.СчетФактураПросмотр;\n",
        "    y = СчетФактура;\n",
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

    let parsed = analysis
        .parse_result(V2FileId(1))
        .expect("parse_result query")
        .expect("parse_result present");
    let (x_property, y_ident) = parsed
        .program
        .statements
        .iter()
        .find_map(|stmt| match stmt {
            bsl_syntax::ast::Statement::ProcedureDecl { body, .. } => {
                let mut x_property: Option<&str> = None;
                let mut y_ident: Option<&str> = None;
                for s in body {
                    if let bsl_syntax::ast::Statement::Assignment { value, .. } = s {
                        match value {
                            bsl_syntax::ast::Expression::PropertyAccess { property, .. } => {
                                x_property = Some(property.as_str());
                            }
                            bsl_syntax::ast::Expression::Identifier { name, .. } => {
                                y_ident = Some(name.as_str());
                            }
                            _ => {}
                        }
                    }
                }
                Some((x_property?, y_ident?))
            }
            _ => None,
        })
        .expect("Expected x/y assignments to be parsed");
    assert_eq!(x_property, "СчетФактураПросмотр");
    assert_eq!(y_ident, "СчетФактура");

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
    let got_x = analysis
        .type_at_position(V2FileId(1), 1, 17)
        .expect("type_at_position query for x")
        .map(|ty| ty.type_name());
    let got_x_receiver = analysis
        .type_at_position(V2FileId(1), 1, 8)
        .expect("type_at_position query for x receiver")
        .map(|ty| ty.type_name());
    let got_y = analysis
        .type_at_position(V2FileId(1), 2, 8)
        .expect("type_at_position query for y")
        .map(|ty| ty.type_name());

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
        got_y.as_deref().unwrap_or_default().contains("cfg:DocumentRef."),
        "Expected `СчетФактура` to resolve as cfg:DocumentRef.*, got {:?}",
        got_y
    );
}
