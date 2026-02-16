//! Интеграционная матрица implicit `Объект`/`ЭтотОбъект` по типам модулей.

mod support;

use std::sync::Arc;

use bsl_analysis_v2::{AnalysisHostV2, Change as ChangeV2, FileId as V2FileId, SettingsId};
use bsl_backend::system::DepsBundleV2;
use bsl_shared::formatting::DetailLevel;

fn type_name_at_assignment_rhs(
    deps_bundle: &Arc<DepsBundleV2>,
    file_path: &str,
    code: &str,
    assignment_prefix: &str,
) -> String {
    let mut host = AnalysisHostV2::default();
    host.apply_change(ChangeV2::SetDepsSnapshot {
        deps_id: deps_bundle.deps_id.clone(),
        deps: deps_bundle.semantic_deps.clone(),
    });
    host.apply_change(ChangeV2::SetSettingsSnapshot {
        settings_id: SettingsId::from_hash("contextual-implicit-object-matrix"),
        diagnostics_detail_level: DetailLevel::Full,
    });
    host.apply_change(ChangeV2::SetFile {
        file_id: V2FileId(1),
        text: Arc::from(code.to_string()),
        version: 0,
        path: Arc::from(file_path.to_string()),
    });

    let offset = code
        .find(assignment_prefix)
        .map(|start| start + assignment_prefix.len())
        .expect("assignment prefix not found") as u32;

    let analysis = host.analysis();
    analysis
        .type_at_byte_offset(V2FileId(1), offset)
        .expect("type_at_byte_offset query")
        .map(|ty| bsl_shared::formatting::user_facing_resolution_type_name(&ty))
        .expect("type at assignment rhs")
}

#[test]
fn implicit_object_bindings_are_contextual_across_module_types() {
    let deps_bundle = support::deps_bundle_v2_with_syntax_helper();

    let form_code = concat!(
        "Процедура Тест()\n",
        "    form_object = Объект;\n",
        "    form_this = ЭтотОбъект;\n",
        "КонецПроцедуры\n",
    );
    assert_eq!(
        type_name_at_assignment_rhs(
            &deps_bundle,
            "Documents/Док1/Forms/Форма1/Ext/Form/Module.bsl",
            form_code,
            "form_object = ",
        ),
        "ДанныеФормыСтруктура"
    );
    assert_eq!(
        type_name_at_assignment_rhs(
            &deps_bundle,
            "Documents/Док1/Forms/Форма1/Ext/Form/Module.bsl",
            form_code,
            "form_this = ",
        ),
        "Формы.Документы.Док1.Форма1"
    );

    let manager_code = concat!(
        "Процедура Тест()\n",
        "    manager_object = Объект;\n",
        "    manager_this = ЭтотОбъект;\n",
        "КонецПроцедуры\n",
    );
    assert_eq!(
        type_name_at_assignment_rhs(
            &deps_bundle,
            "Documents/Док1/Ext/ManagerModule.bsl",
            manager_code,
            "manager_object = ",
        ),
        "ДокументМенеджер.Док1"
    );
    assert_eq!(
        type_name_at_assignment_rhs(
            &deps_bundle,
            "Documents/Док1/Ext/ManagerModule.bsl",
            manager_code,
            "manager_this = ",
        ),
        "ДокументМенеджер.Док1"
    );

    let object_code = concat!(
        "Процедура Тест()\n",
        "    object_object = Объект;\n",
        "    object_this = ЭтотОбъект;\n",
        "КонецПроцедуры\n",
    );
    assert_eq!(
        type_name_at_assignment_rhs(
            &deps_bundle,
            "Documents/Док1/Ext/ObjectModule.bsl",
            object_code,
            "object_object = ",
        ),
        "ДокументОбъект.Док1"
    );
    assert_eq!(
        type_name_at_assignment_rhs(
            &deps_bundle,
            "Documents/Док1/Ext/ObjectModule.bsl",
            object_code,
            "object_this = ",
        ),
        "ДокументОбъект.Док1"
    );

    let recordset_code = concat!(
        "Процедура Тест()\n",
        "    rs_object = Объект;\n",
        "    rs_this = ЭтотОбъект;\n",
        "КонецПроцедуры\n",
    );
    assert_eq!(
        type_name_at_assignment_rhs(
            &deps_bundle,
            "InformationRegisters/Регистр1/Ext/RecordSetModule.bsl",
            recordset_code,
            "rs_object = ",
        ),
        "РегистрСведенийНаборЗаписей.Регистр1"
    );
    assert_eq!(
        type_name_at_assignment_rhs(
            &deps_bundle,
            "InformationRegisters/Регистр1/Ext/RecordSetModule.bsl",
            recordset_code,
            "rs_this = ",
        ),
        "РегистрСведенийНаборЗаписей.Регистр1"
    );
}
