use super::*;
use crate::system::{IndexItem, IndexSnapshot, SymbolKind, SymbolScope, TypeKind};
use bsl_analysis_v2::{
    AnalysisHostV2, Change as ChangeV2, DepsSnapshotId, FileId as V2FileId, SettingsId,
};
use bsl_shared::domain::repository::{InMemoryTypeRepository, TypeRepository};
use bsl_shared::domain::resolver::TypeResolver;
use bsl_shared::domain::signature_index::{
    ContextRequirements, MethodSignature, SignatureIndex, SignatureSource,
};
use bsl_shared::domain::type_id::TypeId;
use bsl_shared::domain::types::{
    Certainty, ConcreteType, ConfigurationType, FacetKind, MetadataKind, RawDataSource,
    RawMethodData, RawPropertyData, RawTypeData, ResolutionMetadata, ResolutionResult,
    ResolutionSource, TypeResolution, FORM_DATA_FORM_TYPE_NOTE_PREFIX, FORM_DATA_SEMANTICS_NOTE,
};
use bsl_shared::formatting::DetailLevel;
use std::sync::Arc;

#[test]
fn trim_to_window_keeps_tail() {
    let input = "0123456789";
    let trimmed = trim_to_window(input, 4);
    assert_eq!(trimmed, "6789");
}

#[test]
fn with_sort_text_uses_original_label_as_case_tie_break() {
    let item = CompletionItem::new("Apple".to_string(), CompletionKind::Property);
    let with_sort = with_sort_text(item, 0.5, 1, "apple");
    let sort_text = with_sort.sort_text.expect("sort_text should be set");

    assert_eq!(sort_text, "apple-Apple-01-0500");
}

#[test]
fn extract_member_base_simple() {
    let base = extract_member_base("Объект.").unwrap();
    assert_eq!(base, "Объект");
}

fn utf16_column(content: &str, marker: &str) -> (u32, u32) {
    let byte_index = content
        .find(marker)
        .unwrap_or_else(|| panic!("Marker not found: {}", marker));
    let before = &content[..byte_index];
    let line = before.lines().count().saturating_sub(1) as u32;
    let last_line = before.lines().last().unwrap_or("");
    let column = last_line.chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32;
    (line, column)
}

fn byte_offset_of(content: &str, marker: &str) -> u32 {
    content
        .find(marker)
        .map(|index| index as u32)
        .unwrap_or_else(|| panic!("Marker not found: {}", marker))
}

#[test]
fn completion_context_detects_member_access_and_trigger_char() {
    let content = "Объект.";
    let (line, column) = utf16_column(content, ".");
    let ctx = analyze_completion_context(content, line, column + 1);

    assert!(ctx.member_access);
    assert_eq!(ctx.member_base.as_deref(), Some("Объект"));
    assert_eq!(ctx.trigger_char, Some('.'));
}

#[test]
fn completion_context_detects_member_access_when_cursor_is_on_dot() {
    let content = "Объект.";
    let (line, column) = utf16_column(content, ".");
    let ctx = analyze_completion_context(content, line, column);

    assert!(ctx.member_access);
    assert_eq!(ctx.member_base.as_deref(), Some("Объект"));
    assert_eq!(ctx.trigger_char, Some('.'));
    assert!(
        ctx.current_word.is_empty(),
        "cursor-on-dot should not keep previous identifier as prefix"
    );
}

#[test]
fn completion_context_detects_trigger_char_for_call() {
    let content = "Функция(";
    let (line, column) = utf16_column(content, "(");
    let ctx = analyze_completion_context(content, line, column + 1);

    assert!(!ctx.member_access);
    assert_eq!(ctx.trigger_char, Some('('));
}

#[test]
fn completion_context_uses_trigger_hint_for_member_access() {
    let content = "Объект";
    let line = 0;
    let column = content.chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32;
    let ctx = analyze_completion_context_with_trigger_hint(content, line, column, Some('.'));

    assert!(ctx.member_access);
    assert_eq!(ctx.trigger_char, Some('.'));
    assert!(
        ctx.current_word.is_empty(),
        "trigger hint for '.' should clear prefix in member-access context"
    );
}

#[test]
fn completion_context_reads_current_word_with_utf16_column() {
    let content = "Перем a😀b";
    let (line, column) = utf16_column(content, "b");
    let ctx = analyze_completion_context(content, line, column + 1);

    assert_eq!(ctx.current_word, "b");
}

#[test]
fn completion_head_receiver_resolves_implicit_form_object_descriptor_from_module_path() {
    let repository = Arc::new(InMemoryTypeRepository::new());
    repository
        .load_types(vec![
            RawTypeData {
                name: "Документы.Док1".to_string(),
                source: RawDataSource::Configuration,
                facets: vec![FacetKind::Manager, FacetKind::Object, FacetKind::Reference],
                kind: Some(MetadataKind::Document),
                ..Default::default()
            },
            RawTypeData {
                name: "Формы.Документы.Док1.Форма1".to_string(),
                source: RawDataSource::Configuration,
                ..Default::default()
            },
            RawTypeData {
                name: "ЭлементыФормы.Документы.Док1.Форма1".to_string(),
                source: RawDataSource::Platform,
                ..Default::default()
            },
        ])
        .expect("load types");

    let repo: Arc<dyn TypeRepository> = repository.clone();
    let resolver = Arc::new(TypeResolver::new(repo.clone()));
    let content = concat!("Процедура Тест()\n", "    Объект.\n", "КонецПроцедуры\n");
    let (line, dot_column) = utf16_column(content, ".");
    let column = dot_column + 1;

    let hints = completion_member_access_owner_type_hints_from_head_receiver(
        content,
        line,
        column,
        "Documents/Док1/Forms/Форма1/Ext/Form/Module.bsl",
        resolver.as_ref(),
        repo.as_ref(),
    );

    assert_eq!(
        hints.len(),
        1,
        "expected one implicit form-data hint: {hints:?}"
    );
    let hint = &hints[0];
    assert_eq!(hint.type_name(), "Документы.Док1");
    assert_eq!(hint.active_facet, None);
    assert!(
        hint.metadata
            .notes
            .contains(&FORM_DATA_SEMANTICS_NOTE.to_string()),
        "implicit form-data head hint must preserve form-data semantics notes: {:?}",
        hint.metadata.notes
    );
    assert!(
        hint.metadata.notes.iter().any(|note| {
            note == &format!(
                "{}{}",
                FORM_DATA_FORM_TYPE_NOTE_PREFIX, "Формы.Документы.Док1.Форма1"
            )
        }),
        "implicit form-data head hint must preserve synthetic form type note: {:?}",
        hint.metadata.notes
    );
    assert!(
        hint.metadata.notes.iter().any(|note| {
            note == &format!(
                "{}{}",
                bsl_shared::domain::types::FORM_DATA_ELEMENTS_TYPE_NOTE_PREFIX,
                "ЭлементыФормы.Документы.Док1.Форма1"
            )
        }),
        "implicit form-data head hint must preserve synthetic form-elements type note: {:?}",
        hint.metadata.notes
    );
}

#[test]
fn completion_head_receiver_returns_empty_outside_supported_module_context() {
    let repository = Arc::new(InMemoryTypeRepository::new());
    let repo: Arc<dyn TypeRepository> = repository.clone();
    let resolver = Arc::new(TypeResolver::new(repo.clone()));
    let content = concat!("Процедура Тест()\n", "    Объект.\n", "КонецПроцедуры\n");
    let (line, dot_column) = utf16_column(content, ".");
    let column = dot_column + 1;

    let hints = completion_member_access_owner_type_hints_from_head_receiver(
        content,
        line,
        column,
        "CommonModules/ОбщийМодуль/Ext/Module.bsl",
        resolver.as_ref(),
        repo.as_ref(),
    );

    assert!(
        hints.is_empty(),
        "unsupported module context must not invent head hints: {hints:?}"
    );
}

#[test]
fn completion_context_can_add_statements_flags() {
    let content = "Если Истина Тогда";
    let ctx = analyze_completion_context(content, 0, content.len() as u32);
    assert!(ctx.can_add_statements);

    let content = "Перем Значение";
    let ctx = analyze_completion_context(content, 0, content.len() as u32);
    assert!(!ctx.can_add_statements);
}

#[test]
fn add_properties_from_resolution_preserves_form_data_provider_order_priorities() {
    let repository = Arc::new(InMemoryTypeRepository::new());
    repository
        .load_types(vec![
            RawTypeData {
                name: "Документы.Док1".to_string(),
                source: RawDataSource::Configuration,
                facets: vec![FacetKind::Manager, FacetKind::Object, FacetKind::Reference],
                kind: Some(MetadataKind::Document),
                properties: vec![RawPropertyData {
                    name: "СвойствоМетаданных".to_string(),
                    prop_type: "Число".to_string(),
                    is_readonly: false,
                }],
                ..Default::default()
            },
            RawTypeData {
                name: "Формы.Документы.Док1.Форма1".to_string(),
                source: RawDataSource::Configuration,
                properties: vec![RawPropertyData {
                    name: "РеквизитФормы".to_string(),
                    prop_type: "Строка".to_string(),
                    is_readonly: false,
                }],
                ..Default::default()
            },
            RawTypeData {
                name: "ДокументОбъект".to_string(),
                source: RawDataSource::Platform,
                facets: vec![FacetKind::Object],
                properties: vec![RawPropertyData {
                    name: "ФацетСвойство".to_string(),
                    prop_type: "Число".to_string(),
                    is_readonly: false,
                }],
                ..Default::default()
            },
        ])
        .expect("load types");

    let repo: Arc<dyn TypeRepository> = repository.clone();
    let metadata_lookup = TypeMetadataLookup::new(repo);
    let resolution = TypeResolution {
        certainty: Certainty::Known,
        result: ResolutionResult::Concrete(ConcreteType::Configuration(ConfigurationType {
            kind: MetadataKind::Document,
            name: "Док1".to_string(),
            facet: Some(FacetKind::Object),
            attributes: vec![],
            tabular_sections: vec![],
        })),
        source: ResolutionSource::Static,
        metadata: ResolutionMetadata {
            notes: vec![
                FORM_DATA_SEMANTICS_NOTE.to_string(),
                format!(
                    "{}{}",
                    FORM_DATA_FORM_TYPE_NOTE_PREFIX, "Формы.Документы.Док1.Форма1"
                ),
            ],
            ..Default::default()
        },
        active_facet: Some(FacetKind::Object),
        available_facets: vec![FacetKind::Manager, FacetKind::Object, FacetKind::Reference],
    };

    let mut target = Vec::new();
    add_properties_from_resolution(&metadata_lookup, &resolution, &mut target, 1);

    let link = target
        .iter()
        .find(|candidate| candidate.item.label == "Ссылка")
        .expect("missing intrinsic Ссылка");
    assert_eq!(link.source_priority, 2);

    let deletion_mark = target
        .iter()
        .find(|candidate| candidate.item.label == "ПометкаУдаления")
        .expect("missing intrinsic ПометкаУдаления");
    assert_eq!(deletion_mark.source_priority, 2);

    let metadata_prop = target
        .iter()
        .find(|candidate| candidate.item.label == "СвойствоМетаданных")
        .expect("missing metadata property");
    assert_eq!(metadata_prop.source_priority, 3);

    assert!(
        target
            .iter()
            .all(|candidate| candidate.item.label != "РеквизитФормы"),
        "form-data property completion must not include form-shape properties"
    );
    assert!(
        target
            .iter()
            .all(|candidate| candidate.item.label != "ФацетСвойство"),
        "form-data property completion must not include object-facet fallback properties"
    );
}

#[test]
fn form_data_member_completion_includes_intrinsic_and_excludes_shape_and_facet_members() {
    let repository = Arc::new(InMemoryTypeRepository::new());
    repository
        .load_types(vec![
            RawTypeData {
                name: "Документы.Док1".to_string(),
                source: RawDataSource::Configuration,
                facets: vec![FacetKind::Manager, FacetKind::Object, FacetKind::Reference],
                kind: Some(MetadataKind::Document),
                ..Default::default()
            },
            RawTypeData {
                name: "Формы.Документы.Док1.Форма1".to_string(),
                source: RawDataSource::Configuration,
                properties: vec![RawPropertyData {
                    name: "РеквизитФормы".to_string(),
                    prop_type: "Строка".to_string(),
                    is_readonly: false,
                }],
                ..Default::default()
            },
            RawTypeData {
                name: "ДокументОбъект".to_string(),
                source: RawDataSource::Platform,
                facets: vec![FacetKind::Object],
                methods: vec![RawMethodData {
                    name: "Записать".to_string(),
                    return_type: "Булево".to_string(),
                    ..Default::default()
                }],
                properties: vec![RawPropertyData {
                    name: "ФацетСвойство".to_string(),
                    prop_type: "Число".to_string(),
                    is_readonly: false,
                }],
                ..Default::default()
            },
        ])
        .expect("load types");

    let repo: Arc<dyn TypeRepository> = repository.clone();
    let metadata_lookup = TypeMetadataLookup::new(repo);
    let resolution = TypeResolution {
        certainty: Certainty::Known,
        result: ResolutionResult::Concrete(ConcreteType::Configuration(ConfigurationType {
            kind: MetadataKind::Document,
            name: "Док1".to_string(),
            facet: Some(FacetKind::Object),
            attributes: vec![],
            tabular_sections: vec![],
        })),
        source: ResolutionSource::Static,
        metadata: ResolutionMetadata {
            notes: vec![
                FORM_DATA_SEMANTICS_NOTE.to_string(),
                format!(
                    "{}{}",
                    FORM_DATA_FORM_TYPE_NOTE_PREFIX, "Формы.Документы.Док1.Форма1"
                ),
            ],
            ..Default::default()
        },
        active_facet: Some(FacetKind::Object),
        available_facets: vec![FacetKind::Manager, FacetKind::Object, FacetKind::Reference],
    };

    let mut target = Vec::new();
    add_methods_from_resolution(&metadata_lookup, &resolution, &mut target, 0);
    add_properties_from_resolution(&metadata_lookup, &resolution, &mut target, 1);

    assert!(target.iter().any(|candidate| {
        matches!(candidate.item.kind, CompletionKind::Property) && candidate.item.label == "Ссылка"
    }));
    assert!(target.iter().any(|candidate| {
        matches!(candidate.item.kind, CompletionKind::Property)
            && candidate.item.label == "ПометкаУдаления"
    }));
    assert!(target.iter().all(|candidate| {
        !(matches!(candidate.item.kind, CompletionKind::Property)
            && candidate.item.label == "РеквизитФормы")
    }));
    assert!(target.iter().all(|candidate| {
        !(matches!(candidate.item.kind, CompletionKind::Property)
            && candidate.item.label == "ФацетСвойство")
    }));
    assert!(target.iter().all(|candidate| {
        !(matches!(candidate.item.kind, CompletionKind::Method)
            && candidate.item.label == "Записать")
    }));
}

#[test]
fn completion_context_expects_type_flags() {
    let content = "Перем Значение: ";
    let ctx = analyze_completion_context(content, 0, content.len() as u32);
    assert!(ctx.expects_type);

    let content = "Тип(";
    let ctx = analyze_completion_context(content, 0, content.len() as u32);
    assert!(ctx.expects_type);
}

#[test]
fn completion_context_can_add_functions_flags() {
    let content = "Процедура Тест()";
    let ctx = analyze_completion_context(content, 0, content.len() as u32);
    assert!(!ctx.can_add_functions);

    let content = "Функция Тест()";
    let ctx = analyze_completion_context(content, 0, content.len() as u32);
    assert!(!ctx.can_add_functions);

    let content = "Перем Значение";
    let ctx = analyze_completion_context(content, 0, content.len() as u32);
    assert!(ctx.can_add_functions);
}

#[tokio::test]
async fn completion_filters_by_prefix() {
    let index = IntellisenseIndexStore::new("cfg", "platform");
    index.set_keywords(vec![
        IndexItem::new(
            "Процедура",
            IndexItemKind::Keyword,
            crate::system::IndexKind::Keyword,
        ),
        IndexItem::new(
            "Функция",
            IndexItemKind::Keyword,
            crate::system::IndexKind::Keyword,
        ),
    ]);
    index.upsert_type(IndexItem::new(
        "Массив",
        IndexItemKind::Type(TypeKind::Platform),
        crate::system::IndexKind::Type,
    ));

    let repository = Arc::new(InMemoryTypeRepository::new());
    let metadata_lookup = TypeMetadataLookup::new(repository);

    let result = get_completion("Про", 0, 3, None, &index, &metadata_lookup)
        .await
        .expect("completion ok");
    let labels: Vec<String> = result.items.into_iter().map(|c| c.item.label).collect();

    assert_eq!(labels, vec!["Процедура".to_string()]);
}

#[test]
fn build_call_snippet_includes_optional_placeholders() {
    let params = vec![("Путь".to_string(), false), ("Режим".to_string(), true)];
    let snippet = build_call_snippet("Открыть", &params).expect("snippet");
    assert_eq!(snippet, "Открыть(${1:Путь}, ${2:Режим})$0");
}

#[test]
fn build_call_snippet_normalizes_angle_brackets_in_labels() {
    let params = vec![
        ("<Имя>".to_string(), false),
        ("&lt;Тип&gt;".to_string(), false),
        ("<Заголовок>".to_string(), false),
        ("<Ширина>".to_string(), false),
    ];
    let snippet = build_call_snippet("Добавить", &params).expect("snippet");
    assert_eq!(
        snippet,
        "Добавить(${1:Имя}, ${2:Тип}, ${3:Заголовок}, ${4:Ширина})$0"
    );
}

#[test]
fn build_call_snippet_escapes_special_chars() {
    let params = vec![("Имя}".to_string(), false)];
    let snippet = build_call_snippet("Функция$", &params).expect("snippet");
    assert_eq!(snippet, "Функция\\$(${1:Имя\\}})$0");
}

#[tokio::test]
async fn completion_limits_output() {
    let index = IntellisenseIndexStore::new("cfg", "platform");
    let keywords = (0..300)
        .map(|i| {
            IndexItem::new(
                format!("Ключ{}", i),
                IndexItemKind::Keyword,
                crate::system::IndexKind::Keyword,
            )
        })
        .collect();
    index.set_keywords(keywords);

    let repository = Arc::new(InMemoryTypeRepository::new());
    let metadata_lookup = TypeMetadataLookup::new(repository);

    let result = get_completion("", 0, 0, None, &index, &metadata_lookup)
        .await
        .expect("completion ok");

    assert!(result.is_incomplete);
    assert_eq!(result.items.len(), COMPLETION_MAX_ITEMS);
}

#[tokio::test]
async fn completion_non_member_test_helper_without_ir_uses_snapshot_modules_but_not_file_locals() {
    let index = IntellisenseIndexStore::new("cfg", "platform");
    let uri = "file:///completion_non_member_no_ir_sources_test.bsl";

    let mut local_from_index = IndexItem::new(
        "ИндексЛокал",
        IndexItemKind::Symbol(SymbolKind::Variable),
        crate::system::IndexKind::Symbol,
    );
    local_from_index.scope = Some(SymbolScope::Local);

    let mut module_from_index = IndexItem::new(
        "ИндексМодуль",
        IndexItemKind::Symbol(SymbolKind::Function),
        crate::system::IndexKind::Symbol,
    );
    module_from_index.scope = Some(SymbolScope::Module);

    index.replace_symbols_for_uri(uri, vec![local_from_index, module_from_index]);
    index.set_keywords(vec![IndexItem::new(
        "ИндексКлюч".to_string(),
        IndexItemKind::Keyword,
        crate::system::IndexKind::Keyword,
    )]);

    let repository = Arc::new(InMemoryTypeRepository::new());
    let metadata_lookup = TypeMetadataLookup::new(repository);
    let content = "    Инд";
    let column = content.chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32;

    let result = get_completion(content, 0, column, Some(uri), &index, &metadata_lookup)
        .await
        .expect("completion ok");
    let labels: Vec<String> = result
        .items
        .into_iter()
        .map(|candidate| candidate.item.label)
        .collect();

    assert!(
        labels.iter().any(|label| label == "ИндексМодуль"),
        "labels: {:?}",
        labels
    );
    assert!(
        labels.iter().any(|label| label == "ИндексКлюч"),
        "labels: {:?}",
        labels
    );
    assert!(
        !labels.iter().any(|label| label == "ИндексЛокал"),
        "labels: {:?}",
        labels
    );
}

#[tokio::test]
async fn completion_non_member_without_ir_includes_form_implicit_context_symbols() {
    let index = IntellisenseIndexStore::new("cfg", "platform");
    let index_snapshot = index.snapshot();
    let repository = Arc::new(InMemoryTypeRepository::new());
    let repo: Arc<dyn TypeRepository> = repository.clone();
    let resolver = Arc::new(TypeResolver::new(repo.clone()));
    let metadata_lookup = TypeMetadataLookup::new(repo);
    let content = "Процедура Тест()\n    Этот\nКонецПроцедуры\n";
    let line = 1;
    let column = "    Этот".chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32;

    let result = get_completion_with_trigger_hint_and_owner_hints_without_ir(
        content,
        line,
        column,
        Some("file:///Documents/Док1/Forms/Форма1/Ext/Form/Module.bsl"),
        &index_snapshot,
        &metadata_lookup,
        "Documents/Док1/Forms/Форма1/Ext/Form/Module.bsl",
        resolver.as_ref(),
        Vec::new(),
        false,
        None,
    )
    .await
    .expect("completion ok");
    let labels: Vec<String> = result
        .items
        .into_iter()
        .map(|candidate| candidate.item.label)
        .collect();

    assert!(
        labels.iter().any(|label| label == "ЭтотОбъект"),
        "labels: {:?}",
        labels
    );
}

fn build_non_member_completion_fixture(
    content: &str,
    file_path: &str,
) -> (
    IntellisenseIndexStore,
    TypeMetadataLookup,
    Arc<TypeResolver>,
    Arc<bsl_shared::ir::SemanticProgram>,
) {
    let repository = Arc::new(InMemoryTypeRepository::new());
    let repo: Arc<dyn TypeRepository> = repository.clone();
    let resolver = Arc::new(TypeResolver::new(repo.clone()));
    let metadata_lookup = TypeMetadataLookup::new(repo.clone());

    let deps = Arc::new(bsl_analysis_v2::SemanticDeps {
        signature_index: repo.get_signature_index_clone(),
        resolver: Some(resolver.clone()),
        repository: repo,
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
        text: Arc::from(content.to_string()),
        version: 0,
        path: Arc::from(file_path.to_string()),
    });

    let analysis = host.analysis();
    let ir_program = analysis.ir(V2FileId(1)).ok().flatten().expect("ir");
    let index = IntellisenseIndexStore::new("cfg", "platform");

    (index, metadata_lookup, resolver, ir_program)
}

#[allow(clippy::too_many_arguments)]
async fn completion_labels_non_member(
    content: &str,
    line: u32,
    column: u32,
    file_uri: Option<&str>,
    file_path: &str,
    index: &IntellisenseIndexStore,
    metadata_lookup: &TypeMetadataLookup,
    resolver: &TypeResolver,
    ir_program: Arc<bsl_shared::ir::SemanticProgram>,
) -> Vec<String> {
    let ctx = CompletionAnalysisContext {
        ir_program: Some(ir_program),
        resolver,
        file_path,
        member_access_owner_type_hints: Vec::new(),
        include_flow_sensitive: false,
        deps_id: None,
        settings_id: None,
    };

    let result = get_completion_with_analysis(
        content,
        line,
        column,
        file_uri,
        index,
        metadata_lookup,
        &ctx,
        None,
    )
    .await
    .expect("completion ok");

    result
        .items
        .into_iter()
        .map(|candidate| candidate.item.label)
        .collect()
}

#[allow(clippy::too_many_arguments)]
async fn completion_labels_non_member_with_snapshot_ids(
    content: &str,
    line: u32,
    column: u32,
    file_uri: Option<&str>,
    file_path: &str,
    index: &IntellisenseIndexStore,
    metadata_lookup: &TypeMetadataLookup,
    resolver: &TypeResolver,
    ir_program: Arc<bsl_shared::ir::SemanticProgram>,
    deps_id: &DepsSnapshotId,
    settings_id: &SettingsId,
) -> Vec<String> {
    let snapshot = index.snapshot();
    let result =
        get_completion_with_semantic_program_snapshot_with_trigger_hint_and_owner_hints_with_snapshot_ids(
            content,
            line,
            column,
            file_uri,
            &snapshot,
            metadata_lookup,
            file_path,
            resolver,
            ir_program,
            Vec::new(),
            false,
            Some(deps_id),
            Some(settings_id),
            None,
        )
        .await
        .expect("completion ok");

    result
        .items
        .into_iter()
        .map(|candidate| candidate.item.label)
        .collect()
}

#[allow(clippy::too_many_arguments)]
async fn completion_labels_non_member_without_ir_with_snapshot_ids(
    content: &str,
    line: u32,
    column: u32,
    file_uri: Option<&str>,
    file_path: &str,
    index_snapshot: &IndexSnapshot,
    metadata_lookup: &TypeMetadataLookup,
    resolver: &TypeResolver,
    deps_id: &DepsSnapshotId,
    settings_id: &SettingsId,
) -> Vec<String> {
    let result = get_completion_with_trigger_hint_and_owner_hints_without_ir_with_snapshot_ids(
        content,
        line,
        column,
        file_uri,
        index_snapshot,
        metadata_lookup,
        file_path,
        resolver,
        Vec::new(),
        false,
        Some(deps_id),
        Some(settings_id),
        None,
    )
    .await
    .expect("completion ok");

    result
        .items
        .into_iter()
        .map(|candidate| candidate.item.label)
        .collect()
}

fn owner_hint(type_name: &str) -> Vec<TypeResolution> {
    vec![TypeResolution::explicit(type_name)]
}

fn build_prefiltered_non_member_repository() -> Arc<InMemoryTypeRepository> {
    let repository = Arc::new(InMemoryTypeRepository::new());
    repository
        .load_types(vec![
            RawTypeData {
                name: "Справочники.ЭтотСправочник".to_string(),
                source: RawDataSource::Configuration,
                kind: Some(MetadataKind::Catalog),
                ..Default::default()
            },
            RawTypeData {
                name: "ЭтотТип".to_string(),
                source: RawDataSource::Platform,
                ..Default::default()
            },
        ])
        .expect("load prefitered non-member types");

    let method = MethodSignature::new(
        "ЭтотГлобал".to_string(),
        None,
        vec![],
        None,
        None,
        None,
        SignatureSource::Configuration,
        None,
        ContextRequirements::default(),
    );
    repository.add_global_function_signature("ЭтотГлобал", method);

    repository
}

#[tokio::test]
async fn completion_non_member_hides_block_locals_outside_if() {
    let content = concat!(
        "Процедура Тест()\n",
        "    Если Истина Тогда\n",
        "        ВнутриБлока = 1;\n",
        "    КонецЕсли;\n",
        "    Вн\n",
        "КонецПроцедуры\n"
    );
    let file_path = "completion_lexical_block_scope_test.bsl";
    let (index, metadata_lookup, resolver, ir_program) =
        build_non_member_completion_fixture(content, file_path);

    let line = 4;
    let column = "    Вн".chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32;
    let labels = completion_labels_non_member(
        content,
        line,
        column,
        Some("file:///completion_lexical_block_scope_test.bsl"),
        file_path,
        &index,
        &metadata_lookup,
        resolver.as_ref(),
        ir_program,
    )
    .await;

    assert!(
        !labels.iter().any(|label| label == "ВнутриБлока"),
        "labels: {:?}",
        labels
    );
}

#[tokio::test]
async fn completion_non_member_handles_else_boundary_without_then_leak() {
    let content = concat!(
        "Процедура Тест()\n",
        "    Если Истина Тогда\n",
        "        ТогдаЛокал = 1;\n",
        "    Иначе\n",
        "        Ло\n",
        "        ЛокалИначе = 2;\n",
        "    КонецЕсли;\n",
        "КонецПроцедуры\n"
    );
    let file_path = "completion_lexical_else_boundary_scope_test.bsl";
    let (index, metadata_lookup, resolver, ir_program) =
        build_non_member_completion_fixture(content, file_path);

    let query_column = "        Ло".chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32;
    let labels = completion_labels_non_member(
        content,
        4,
        query_column,
        Some("file:///completion_lexical_else_boundary_scope_test.bsl"),
        file_path,
        &index,
        &metadata_lookup,
        resolver.as_ref(),
        ir_program,
    )
    .await;
    assert!(
        !labels.iter().any(|label| label == "ТогдаЛокал"),
        "labels: {:?}",
        labels
    );
    assert!(
        !labels.iter().any(|label| label == "ЛокалИначе"),
        "labels: {:?}",
        labels
    );
}

#[tokio::test]
async fn completion_non_member_after_if_end_does_not_leak_branch_locals() {
    let content = concat!(
        "Процедура Тест()\n",
        "    Если Истина Тогда\n",
        "        ТогдаЛокал = 1;\n",
        "    Иначе\n",
        "        ИначеЛокал = 2;\n",
        "    КонецЕсли;\n",
        "    Ло\n",
        "КонецПроцедуры\n"
    );
    let file_path = "completion_lexical_after_if_end_scope_test.bsl";
    let (index, metadata_lookup, resolver, ir_program) =
        build_non_member_completion_fixture(content, file_path);

    let query_column = "    Ло".chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32;
    let labels = completion_labels_non_member(
        content,
        6,
        query_column,
        Some("file:///completion_lexical_after_if_end_scope_test.bsl"),
        file_path,
        &index,
        &metadata_lookup,
        resolver.as_ref(),
        ir_program,
    )
    .await;

    assert!(
        !labels.iter().any(|label| label == "ТогдаЛокал"),
        "labels: {:?}",
        labels
    );
    assert!(
        !labels.iter().any(|label| label == "ИначеЛокал"),
        "labels: {:?}",
        labels
    );
}

#[tokio::test]
async fn completion_non_member_respects_position_before_and_after_declaration() {
    let content = concat!(
        "Процедура Тест()\n",
        "    Пос\n",
        "    После = 1;\n",
        "    Пос\n",
        "КонецПроцедуры\n"
    );
    let file_path = "completion_lexical_position_scope_test.bsl";
    let (index, metadata_lookup, resolver, ir_program) =
        build_non_member_completion_fixture(content, file_path);

    let query_column = "    Пос".chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32;
    let labels_before = completion_labels_non_member(
        content,
        1,
        query_column,
        Some("file:///completion_lexical_position_scope_test.bsl"),
        file_path,
        &index,
        &metadata_lookup,
        resolver.as_ref(),
        ir_program.clone(),
    )
    .await;
    assert!(
        !labels_before.iter().any(|label| label == "После"),
        "labels_before: {:?}",
        labels_before
    );

    let labels_after = completion_labels_non_member(
        content,
        3,
        query_column,
        Some("file:///completion_lexical_position_scope_test.bsl"),
        file_path,
        &index,
        &metadata_lookup,
        resolver.as_ref(),
        ir_program,
    )
    .await;
    assert!(
        labels_after.iter().any(|label| label == "После"),
        "labels_after: {:?}",
        labels_after
    );
}

#[tokio::test]
async fn completion_non_member_prefers_nearest_scope_for_shadowed_names() {
    let content = concat!(
        "Процедура Тест()\n",
        "    Имя = 1;\n",
        "    Если Истина Тогда\n",
        "        ИМЯ = 2;\n",
        "        им\n",
        "    КонецЕсли;\n",
        "    им\n",
        "КонецПроцедуры\n"
    );
    let file_path = "completion_lexical_shadow_scope_test.bsl";
    let (index, metadata_lookup, resolver, ir_program) =
        build_non_member_completion_fixture(content, file_path);

    let inner_column = "        им".chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32;
    let labels_inner = completion_labels_non_member(
        content,
        4,
        inner_column,
        Some("file:///completion_lexical_shadow_scope_test.bsl"),
        file_path,
        &index,
        &metadata_lookup,
        resolver.as_ref(),
        ir_program.clone(),
    )
    .await;
    assert!(
        labels_inner.iter().any(|label| label == "ИМЯ"),
        "labels_inner: {:?}",
        labels_inner
    );
    assert!(
        !labels_inner.iter().any(|label| label == "Имя"),
        "labels_inner: {:?}",
        labels_inner
    );

    let outer_column = "    им".chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32;
    let labels_outer = completion_labels_non_member(
        content,
        6,
        outer_column,
        Some("file:///completion_lexical_shadow_scope_test.bsl"),
        file_path,
        &index,
        &metadata_lookup,
        resolver.as_ref(),
        ir_program,
    )
    .await;
    assert!(
        labels_outer.iter().any(|label| label == "Имя"),
        "labels_outer: {:?}",
        labels_outer
    );
    assert!(
        !labels_outer.iter().any(|label| label == "ИМЯ"),
        "labels_outer: {:?}",
        labels_outer
    );
}

#[tokio::test]
async fn completion_non_member_implicit_local_visible_from_assignment() {
    let content = concat!(
        "Процедура Тест()\n",
        "    Ло\n",
        "    Локал = Новый Массив;\n",
        "    Ло\n",
        "КонецПроцедуры\n"
    );
    let file_path = "completion_lexical_implicit_local_test.bsl";
    let (index, metadata_lookup, resolver, ir_program) =
        build_non_member_completion_fixture(content, file_path);

    let query_column = "    Ло".chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32;
    let labels_before = completion_labels_non_member(
        content,
        1,
        query_column,
        Some("file:///completion_lexical_implicit_local_test.bsl"),
        file_path,
        &index,
        &metadata_lookup,
        resolver.as_ref(),
        ir_program.clone(),
    )
    .await;
    assert!(
        !labels_before.iter().any(|label| label == "Локал"),
        "labels_before: {:?}",
        labels_before
    );

    let labels_after = completion_labels_non_member(
        content,
        3,
        query_column,
        Some("file:///completion_lexical_implicit_local_test.bsl"),
        file_path,
        &index,
        &metadata_lookup,
        resolver.as_ref(),
        ir_program,
    )
    .await;
    assert!(
        labels_after.iter().any(|label| label == "Локал"),
        "labels_after: {:?}",
        labels_after
    );
}

#[tokio::test]
async fn completion_non_member_hides_loop_locals_outside_loop() {
    let content = concat!(
        "Процедура Тест()\n",
        "    Для Счетчик = 1 По 2 Цикл\n",
        "        ВЦикле = Счетчик;\n",
        "    КонецЦикла;\n",
        "    ВЦ\n",
        "КонецПроцедуры\n"
    );
    let file_path = "completion_lexical_loop_scope_test.bsl";
    let (index, metadata_lookup, resolver, ir_program) =
        build_non_member_completion_fixture(content, file_path);

    let query_column = "    ВЦ".chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32;
    let labels = completion_labels_non_member(
        content,
        4,
        query_column,
        Some("file:///completion_lexical_loop_scope_test.bsl"),
        file_path,
        &index,
        &metadata_lookup,
        resolver.as_ref(),
        ir_program,
    )
    .await;
    assert!(
        !labels.iter().any(|label| label == "ВЦикле"),
        "labels: {:?}",
        labels
    );
    assert!(
        !labels.iter().any(|label| label == "Счетчик"),
        "labels: {:?}",
        labels
    );
}

#[tokio::test]
async fn completion_non_member_shows_loop_variable_inside_loop() {
    let content = concat!(
        "Процедура Тест()\n",
        "    Для Счетчик = 1 По 2 Цикл\n",
        "        Сч\n",
        "    КонецЦикла;\n",
        "КонецПроцедуры\n"
    );
    let file_path = "completion_lexical_loop_variable_inside_test.bsl";
    let (index, metadata_lookup, resolver, ir_program) =
        build_non_member_completion_fixture(content, file_path);

    let query_column = "        Сч".chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32;
    let labels = completion_labels_non_member(
        content,
        2,
        query_column,
        Some("file:///completion_lexical_loop_variable_inside_test.bsl"),
        file_path,
        &index,
        &metadata_lookup,
        resolver.as_ref(),
        ir_program,
    )
    .await;
    assert!(
        labels.iter().any(|label| label == "Счетчик"),
        "labels: {:?}",
        labels
    );
}

#[tokio::test]
async fn completion_non_member_ignores_non_identifier_assignment_targets() {
    let content = concat!(
        "Процедура Тест()\n",
        "    Объект.Поле = 1;\n",
        "    По\n",
        "КонецПроцедуры\n"
    );
    let file_path = "completion_lexical_assignment_target_test.bsl";
    let (index, metadata_lookup, resolver, ir_program) =
        build_non_member_completion_fixture(content, file_path);

    let column = "    По".chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32;
    let labels = completion_labels_non_member(
        content,
        2,
        column,
        Some("file:///completion_lexical_assignment_target_test.bsl"),
        file_path,
        &index,
        &metadata_lookup,
        resolver.as_ref(),
        ir_program,
    )
    .await;

    assert!(
        !labels.iter().any(|label| label == "Поле"),
        "labels: {:?}",
        labels
    );
}

#[tokio::test]
async fn completion_non_member_handles_except_boundary_without_try_leak() {
    let content = concat!(
        "Процедура Тест()\n",
        "    Попытка\n",
        "        ЛокалПопытка = 1;\n",
        "    Исключение\n",
        "        Ло\n",
        "        ЛокалИсключение = 2;\n",
        "    КонецПопытки;\n",
        "КонецПроцедуры\n"
    );
    let file_path = "completion_lexical_except_boundary_scope_test.bsl";
    let (index, metadata_lookup, resolver, ir_program) =
        build_non_member_completion_fixture(content, file_path);

    let query_column = "        Ло".chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32;
    let labels = completion_labels_non_member(
        content,
        4,
        query_column,
        Some("file:///completion_lexical_except_boundary_scope_test.bsl"),
        file_path,
        &index,
        &metadata_lookup,
        resolver.as_ref(),
        ir_program,
    )
    .await;
    assert!(
        !labels.iter().any(|label| label == "ЛокалПопытка"),
        "labels: {:?}",
        labels
    );
    assert!(
        !labels.iter().any(|label| label == "ЛокалИсключение"),
        "labels: {:?}",
        labels
    );
}

#[tokio::test]
async fn completion_non_member_after_try_end_does_not_leak_except_locals() {
    let content = concat!(
        "Процедура Тест()\n",
        "    Попытка\n",
        "        ЛокалПопытка = 1;\n",
        "    Исключение\n",
        "        ЛокалИсключение = 2;\n",
        "    КонецПопытки;\n",
        "    Ло\n",
        "КонецПроцедуры\n"
    );
    let file_path = "completion_lexical_after_try_end_scope_test.bsl";
    let (index, metadata_lookup, resolver, ir_program) =
        build_non_member_completion_fixture(content, file_path);

    let query_column = "    Ло".chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32;
    let labels = completion_labels_non_member(
        content,
        6,
        query_column,
        Some("file:///completion_lexical_after_try_end_scope_test.bsl"),
        file_path,
        &index,
        &metadata_lookup,
        resolver.as_ref(),
        ir_program,
    )
    .await;

    assert!(
        !labels.iter().any(|label| label == "ЛокалПопытка"),
        "labels: {:?}",
        labels
    );
    assert!(
        !labels.iter().any(|label| label == "ЛокалИсключение"),
        "labels: {:?}",
        labels
    );
}

#[tokio::test]
async fn completion_non_member_warm_snapshot_reuses_immutable_catalogs() {
    let content = concat!(
        "Процедура Тест()\n",
        "    ЭтотЛокал = 1;\n",
        "    Этот\n",
        "КонецПроцедуры\n"
    );
    let file_path = "completion_non_member_cached_catalog_test.bsl";
    let repository = build_prefiltered_non_member_repository();
    let repo: Arc<dyn TypeRepository> = repository.clone();
    let metadata_lookup = TypeMetadataLookup::new(repo.clone());
    let resolver = Arc::new(TypeResolver::new(repo.clone()));
    let deps = Arc::new(bsl_analysis_v2::SemanticDeps {
        signature_index: repo.get_signature_index_clone(),
        resolver: Some(resolver.clone()),
        repository: repo,
        platform_signatures_loaded: false,
    });

    let deps_id = DepsSnapshotId::from_hash("refactor-13-cache-reuse");
    let settings_id = SettingsId::from_hash("refactor-13-cache-reuse");

    let mut host = AnalysisHostV2::default();
    host.apply_change(ChangeV2::SetDepsSnapshot {
        deps_id: deps_id.clone(),
        deps,
    });
    host.apply_change(ChangeV2::SetSettingsSnapshot {
        settings_id: settings_id.clone(),
        diagnostics_detail_level: DetailLevel::Full,
    });
    host.apply_change(ChangeV2::SetFile {
        file_id: V2FileId(1),
        text: Arc::from(content.to_string()),
        version: 0,
        path: Arc::from(file_path.to_string()),
    });

    let analysis = host.analysis();
    let ir_program = analysis.ir(V2FileId(1)).ok().flatten().expect("ir");
    let index = IntellisenseIndexStore::new("cfg", "platform");
    let column = "    Этот".chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32;

    let builds_before =
        immutable_non_member_catalog_build_count_for_tests(&deps_id, Some(&settings_id));
    let first_labels = completion_labels_non_member_with_snapshot_ids(
        content,
        2,
        column,
        Some("file:///completion_non_member_cached_catalog_test.bsl"),
        file_path,
        &index,
        &metadata_lookup,
        resolver.as_ref(),
        ir_program.clone(),
        &deps_id,
        &settings_id,
    )
    .await;
    let builds_after_first =
        immutable_non_member_catalog_build_count_for_tests(&deps_id, Some(&settings_id));
    let second_labels = completion_labels_non_member_with_snapshot_ids(
        content,
        2,
        column,
        Some("file:///completion_non_member_cached_catalog_test.bsl"),
        file_path,
        &index,
        &metadata_lookup,
        resolver.as_ref(),
        ir_program,
        &deps_id,
        &settings_id,
    )
    .await;
    let builds_after_second =
        immutable_non_member_catalog_build_count_for_tests(&deps_id, Some(&settings_id));

    assert_eq!(
        builds_after_first.saturating_sub(builds_before),
        1,
        "first warm request must build immutable catalog exactly once"
    );
    assert_eq!(
        builds_after_second, builds_after_first,
        "second warm request must reuse immutable catalog without rebuilding"
    );
    assert_eq!(
        first_labels, second_labels,
        "warm snapshot completion must stay stable across immutable catalog reuse"
    );
    for expected in ["ЭтотЛокал", "ЭтотГлобал", "ЭтотСправочник", "ЭтотТип"]
    {
        assert!(
            second_labels.iter().any(|label| label == expected),
            "expected {expected} in warm non-member labels: {second_labels:?}"
        );
    }
}

#[tokio::test]
async fn completion_non_member_cache_keeps_local_and_contextual_candidates_stable() {
    let repository = build_prefiltered_non_member_repository();
    let repo: Arc<dyn TypeRepository> = repository.clone();
    let metadata_lookup = TypeMetadataLookup::new(repo.clone());
    let resolver = Arc::new(TypeResolver::new(repo.clone()));
    let deps = Arc::new(bsl_analysis_v2::SemanticDeps {
        signature_index: repo.get_signature_index_clone(),
        resolver: Some(resolver.clone()),
        repository: repo,
        platform_signatures_loaded: false,
    });
    let deps_id = DepsSnapshotId::from_hash("refactor-13-local-contextual");
    let settings_id = SettingsId::from_hash("refactor-13-local-contextual");

    let semantic_content = concat!(
        "Процедура Тест()\n",
        "    ЭтотЛокал = 1;\n",
        "    Этот\n",
        "КонецПроцедуры\n"
    );
    let semantic_file_path = "completion_non_member_local_stability_test.bsl";
    let mut host = AnalysisHostV2::default();
    host.apply_change(ChangeV2::SetDepsSnapshot {
        deps_id: deps_id.clone(),
        deps: deps.clone(),
    });
    host.apply_change(ChangeV2::SetSettingsSnapshot {
        settings_id: settings_id.clone(),
        diagnostics_detail_level: DetailLevel::Full,
    });
    host.apply_change(ChangeV2::SetFile {
        file_id: V2FileId(1),
        text: Arc::from(semantic_content.to_string()),
        version: 0,
        path: Arc::from(semantic_file_path.to_string()),
    });
    let analysis = host.analysis();
    let ir_program = analysis.ir(V2FileId(1)).ok().flatten().expect("ir");
    let index = IntellisenseIndexStore::new("cfg", "platform");
    let semantic_column = "    Этот".chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32;
    let semantic_labels_first = completion_labels_non_member_with_snapshot_ids(
        semantic_content,
        2,
        semantic_column,
        Some("file:///completion_non_member_local_stability_test.bsl"),
        semantic_file_path,
        &index,
        &metadata_lookup,
        resolver.as_ref(),
        ir_program.clone(),
        &deps_id,
        &settings_id,
    )
    .await;
    let semantic_labels_second = completion_labels_non_member_with_snapshot_ids(
        semantic_content,
        2,
        semantic_column,
        Some("file:///completion_non_member_local_stability_test.bsl"),
        semantic_file_path,
        &index,
        &metadata_lookup,
        resolver.as_ref(),
        ir_program,
        &deps_id,
        &settings_id,
    )
    .await;
    assert_eq!(
        semantic_labels_first, semantic_labels_second,
        "local non-member labels must stay unchanged across immutable catalog reuse"
    );
    assert!(
        semantic_labels_second
            .iter()
            .any(|label| label == "ЭтотЛокал"),
        "local candidate must survive cached immutable catalog path: {semantic_labels_second:?}"
    );

    let contextual_content = concat!("Процедура Тест()\n", "    Этот\n", "КонецПроцедуры\n");
    let contextual_file_path = "Documents/Док1/Forms/Форма1/Ext/Form/Module.bsl";
    let contextual_column = "    Этот".chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32;
    let index_snapshot = IndexSnapshot::empty(crate::system::IndexSnapshotId::from_hash(
        "refactor-13-contextual-stability",
    ));
    let contextual_labels_first = completion_labels_non_member_without_ir_with_snapshot_ids(
        contextual_content,
        1,
        contextual_column,
        Some("file:///Documents/Док1/Forms/Форма1/Ext/Form/Module.bsl"),
        contextual_file_path,
        &index_snapshot,
        &metadata_lookup,
        resolver.as_ref(),
        &deps_id,
        &settings_id,
    )
    .await;
    let contextual_labels_second = completion_labels_non_member_without_ir_with_snapshot_ids(
        contextual_content,
        1,
        contextual_column,
        Some("file:///Documents/Док1/Forms/Форма1/Ext/Form/Module.bsl"),
        contextual_file_path,
        &index_snapshot,
        &metadata_lookup,
        resolver.as_ref(),
        &deps_id,
        &settings_id,
    )
    .await;
    assert_eq!(
        contextual_labels_first, contextual_labels_second,
        "context-sensitive non-member labels must stay unchanged across immutable catalog reuse"
    );
    assert!(
        contextual_labels_second
            .iter()
            .any(|label| label == "ЭтотОбъект"),
        "implicit contextual candidate must survive cached immutable catalog path: {contextual_labels_second:?}"
    );
}

#[tokio::test]
async fn completion_non_member_semantic_path_ignores_polluted_index_snapshot() {
    let content = concat!("Процедура Тест()\n", "    Кан\n", "КонецПроцедуры\n");
    let file_path = "completion_lexical_sources_test.bsl";
    let repository = Arc::new(InMemoryTypeRepository::new());
    let method = MethodSignature::new(
        "КанонГлобал".to_string(),
        None,
        vec![],
        None,
        None,
        None,
        SignatureSource::Configuration,
        None,
        ContextRequirements::default(),
    );
    repository.add_global_function_signature("КанонГлобал", method);

    let repo: Arc<dyn TypeRepository> = repository.clone();
    let metadata_lookup = TypeMetadataLookup::new(repo.clone());
    let resolver = Arc::new(TypeResolver::new(repo.clone()));

    let uri = "file:///completion_lexical_sources_test.bsl";
    let index = IntellisenseIndexStore::new("cfg", "platform");
    let mut local_from_index = IndexItem::new(
        "КанонЛокалИзИндекса",
        IndexItemKind::Symbol(SymbolKind::Variable),
        crate::system::IndexKind::Symbol,
    );
    local_from_index.scope = Some(SymbolScope::Local);

    let mut module_from_index = IndexItem::new(
        "КанонМодульИзИндекса",
        IndexItemKind::Symbol(SymbolKind::Function),
        crate::system::IndexKind::Symbol,
    );
    module_from_index.scope = Some(SymbolScope::Module);

    let fake_type_from_index = IndexItem::new(
        "КанонТипИзИндекса",
        IndexItemKind::Type(TypeKind::Platform),
        crate::system::IndexKind::Type,
    );
    let fake_keyword_from_index = IndexItem::new(
        "КанонКлючевикИзИндекса".to_string(),
        IndexItemKind::Keyword,
        crate::system::IndexKind::Keyword,
    );

    index.replace_symbols_for_uri(uri, vec![local_from_index, module_from_index]);
    index.replace_modules_for_key(
        "completion_lexical_sources_test",
        vec![IndexItem::new(
            "КанонГлобалИзModuleIndex".to_string(),
            IndexItemKind::Symbol(SymbolKind::Function),
            crate::system::IndexKind::Module,
        )],
    );
    index.upsert_type(fake_type_from_index);
    index.set_keywords(vec![fake_keyword_from_index]);

    let deps = Arc::new(bsl_analysis_v2::SemanticDeps {
        signature_index: repo.get_signature_index_clone(),
        resolver: Some(resolver.clone()),
        repository: repo,
        platform_signatures_loaded: false,
    });
    let mut host = AnalysisHostV2::default();
    host.apply_change(ChangeV2::SetDepsSnapshot {
        deps_id: DepsSnapshotId::from_hash("semantic"),
        deps,
    });
    host.apply_change(ChangeV2::SetSettingsSnapshot {
        settings_id: SettingsId::from_hash("semantic"),
        diagnostics_detail_level: DetailLevel::Full,
    });
    host.apply_change(ChangeV2::SetFile {
        file_id: V2FileId(1),
        text: Arc::from(content.to_string()),
        version: 0,
        path: Arc::from(file_path.to_string()),
    });
    let analysis = host.analysis();
    let ir_program = analysis.ir(V2FileId(1)).ok().flatten().expect("ir");

    let column = "    Кан".chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32;
    let labels = completion_labels_non_member(
        content,
        1,
        column,
        Some(uri),
        file_path,
        &index,
        &metadata_lookup,
        resolver.as_ref(),
        ir_program,
    )
    .await;

    assert!(
        labels.iter().any(|label| label == "КанонГлобал"),
        "labels: {:?}",
        labels
    );
    assert!(
        !labels.iter().any(|label| label == "КанонЛокалИзИндекса"),
        "labels: {:?}",
        labels
    );
    assert!(
        !labels.iter().any(|label| label == "КанонМодульИзИндекса"),
        "labels: {:?}",
        labels
    );
    assert!(
        !labels
            .iter()
            .any(|label| label == "КанонГлобалИзModuleIndex"),
        "labels: {:?}",
        labels
    );
    assert!(
        !labels.iter().any(|label| label == "КанонТипИзИндекса"),
        "labels: {:?}",
        labels
    );
    assert!(
        !labels.iter().any(|label| label == "КанонКлючевикИзИндекса"),
        "labels: {:?}",
        labels
    );
}

#[tokio::test]
async fn completion_resolves_variable_type_for_member_access() {
    let repository = Arc::new(InMemoryTypeRepository::new());
    repository
        .load_types(vec![RawTypeData {
            name: "ТаблицаЗначений".to_string(),
            source: RawDataSource::Platform,
            methods: vec![RawMethodData {
                name: "Добавить".to_string(),
                return_type: "Булево".to_string(),
                ..Default::default()
            }],
            properties: vec![RawPropertyData {
                name: "Количество".to_string(),
                prop_type: "Число".to_string(),
                is_readonly: true,
            }],
            ..Default::default()
        }])
        .expect("load types");

    let repo: Arc<dyn bsl_shared::domain::repository::TypeRepository> = repository.clone();
    let resolver = Arc::new(TypeResolver::new(repo.clone()));
    let metadata_lookup = TypeMetadataLookup::new(repo.clone());

    let index = IntellisenseIndexStore::new("cfg", "platform");
    let content = concat!(
        "Процедура Тест()\n",
        "    ТаблЗнач = Новый ТаблицаЗначений;\n",
        "    ТаблЗнач.\n",
        "КонецПроцедуры\n"
    );
    let line = 2;
    let line_text = "    ТаблЗнач.";
    let column = line_text.chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32;

    let deps = Arc::new(bsl_analysis_v2::SemanticDeps {
        signature_index: repo.get_signature_index_clone(),
        resolver: Some(resolver.clone()),
        repository: repo.clone(),
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
        text: Arc::from(content.to_string()),
        version: 0,
        path: Arc::from("completion_test.bsl"),
    });
    let analysis = host.analysis();
    let ir_program = analysis.ir(V2FileId(1)).ok().flatten().expect("ir");

    let ctx = CompletionAnalysisContext {
        ir_program: Some(ir_program),
        resolver: resolver.as_ref(),
        file_path: "completion_test.bsl",
        member_access_owner_type_hints: owner_hint("ТаблицаЗначений"),
        include_flow_sensitive: false,
        deps_id: None,
        settings_id: None,
    };

    let resolved = resolve_member_owner_type(Some(&ctx), content, line, column, "ТаблЗнач")
        .await
        .expect("member type");
    assert_eq!(resolved.type_name(), "ТаблицаЗначений");

    let result = get_completion_with_analysis(
        content,
        line,
        column,
        Some("completion_test.bsl"),
        &index,
        &metadata_lookup,
        &ctx,
        None,
    )
    .await
    .expect("completion ok");

    let labels: Vec<String> = result.items.into_iter().map(|c| c.item.label).collect();
    assert!(
        labels.contains(&"Добавить".to_string()),
        "labels: {:?}",
        labels
    );
    assert!(
        labels.contains(&"Количество".to_string()),
        "labels: {:?}",
        labels
    );
}

#[tokio::test]
async fn completion_fails_closed_without_owner_hint_even_when_ir_has_owner_fact() {
    let repository = Arc::new(InMemoryTypeRepository::new());
    repository
        .load_types(vec![RawTypeData {
            name: "ТаблицаЗначений".to_string(),
            source: RawDataSource::Platform,
            methods: vec![RawMethodData {
                name: "Добавить".to_string(),
                return_type: "Булево".to_string(),
                ..Default::default()
            }],
            properties: vec![RawPropertyData {
                name: "Количество".to_string(),
                prop_type: "Число".to_string(),
                is_readonly: true,
            }],
            ..Default::default()
        }])
        .expect("load types");

    let repo: Arc<dyn bsl_shared::domain::repository::TypeRepository> = repository.clone();
    let resolver = Arc::new(TypeResolver::new(repo.clone()));
    let metadata_lookup = TypeMetadataLookup::new(repo.clone());

    let index = IntellisenseIndexStore::new("cfg", "platform");
    index.set_keywords(vec![IndexItem::new(
        "Процедура",
        IndexItemKind::Keyword,
        crate::system::IndexKind::Keyword,
    )]);
    let content = concat!(
        "Процедура Тест()\n",
        "    ТаблЗнач = Новый ТаблицаЗначений;\n",
        "    ТаблЗнач.\n",
        "КонецПроцедуры\n"
    );
    let line = 2;
    let line_text = "    ТаблЗнач.";
    let column = line_text.chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32;

    let deps = Arc::new(bsl_analysis_v2::SemanticDeps {
        signature_index: repo.get_signature_index_clone(),
        resolver: Some(resolver.clone()),
        repository: repo.clone(),
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
        text: Arc::from(content.to_string()),
        version: 0,
        path: Arc::from("completion_no_owner_hint_test.bsl"),
    });
    let analysis = host.analysis();
    let ir_program = analysis.ir(V2FileId(1)).ok().flatten().expect("ir");

    let ctx = CompletionAnalysisContext {
        ir_program: Some(ir_program),
        resolver: resolver.as_ref(),
        file_path: "completion_no_owner_hint_test.bsl",
        member_access_owner_type_hints: Vec::new(),
        include_flow_sensitive: false,
        deps_id: None,
        settings_id: None,
    };

    let resolved = resolve_member_owner_type(Some(&ctx), content, line, column, "ТаблЗнач").await;
    assert!(
        resolved.is_none(),
        "member owner resolution must fail closed without adapter-supplied exact owner hint"
    );

    let result = get_completion_with_analysis(
        content,
        line,
        column,
        Some("completion_no_owner_hint_test.bsl"),
        &index,
        &metadata_lookup,
        &ctx,
        None,
    )
    .await
    .expect("completion ok");

    let labels: Vec<String> = result.items.into_iter().map(|c| c.item.label).collect();
    assert!(
        !labels.contains(&"Добавить".to_string()) && !labels.contains(&"Количество".to_string()),
        "member-access completion must fail closed without shared exact owner hint, labels: {:?}",
        labels
    );
}

#[tokio::test]
async fn completion_unknown_bare_receiver_member_access_ignores_polluted_index_snapshot() {
    let repository = Arc::new(InMemoryTypeRepository::new());
    repository
        .load_types(vec![RawTypeData {
            name: "ТаблицаЗначений".to_string(),
            source: RawDataSource::Platform,
            methods: vec![RawMethodData {
                name: "Добавить".to_string(),
                return_type: "Булево".to_string(),
                ..Default::default()
            }],
            properties: vec![RawPropertyData {
                name: "Количество".to_string(),
                prop_type: "Число".to_string(),
                is_readonly: true,
            }],
            ..Default::default()
        }])
        .expect("load types");

    let repo: Arc<dyn bsl_shared::domain::repository::TypeRepository> = repository.clone();
    let resolver = Arc::new(TypeResolver::new(repo.clone()));
    let metadata_lookup = TypeMetadataLookup::new(repo.clone());

    let index = IntellisenseIndexStore::new("cfg", "platform");
    index.set_keywords(vec![IndexItem::new(
        "Процедура",
        IndexItemKind::Keyword,
        crate::system::IndexKind::Keyword,
    )]);
    index.replace_symbols_for_uri(
        "completion_unknown_receiver_member_access_test.bsl",
        vec![IndexItem::new(
            "ЛожныйСимволИзИндекса",
            IndexItemKind::Symbol(SymbolKind::Function),
            crate::system::IndexKind::Symbol,
        )],
    );
    index.replace_modules_for_key(
        "completion_unknown_receiver_member_access_test.bsl",
        vec![IndexItem::new(
            "ЛожныйМодульИзИндекса",
            IndexItemKind::Symbol(SymbolKind::Procedure),
            crate::system::IndexKind::Module,
        )],
    );
    let mut polluted_snapshot = index.snapshot();
    Arc::make_mut(&mut polluted_snapshot.type_index).insert(
        "ЛожныйТипИзИндекса".to_string(),
        Arc::new(IndexItem::new(
            "ЛожныйТипИзИндекса",
            IndexItemKind::Type(TypeKind::Generic),
            crate::system::IndexKind::Type,
        )),
    );
    index.replace_snapshot(polluted_snapshot);
    let content = concat!(
        "Процедура Тест()\n",
        "    ТаблЗнач = Новый ТаблицаЗначений;\n",
        "    ТаблЗначКолонки.\n",
        "КонецПроцедуры\n"
    );
    let line = 2;
    let line_text = "    ТаблЗначКолонки.";
    let column = line_text.chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32;

    let deps = Arc::new(bsl_analysis_v2::SemanticDeps {
        signature_index: repo.get_signature_index_clone(),
        resolver: Some(resolver.clone()),
        repository: repo.clone(),
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
        text: Arc::from(content.to_string()),
        version: 0,
        path: Arc::from("completion_unknown_receiver_member_access_test.bsl"),
    });
    let analysis = host.analysis();
    let ir_program = analysis.ir(V2FileId(1)).ok().flatten().expect("ir");

    let ctx = CompletionAnalysisContext {
        ir_program: Some(ir_program),
        resolver: resolver.as_ref(),
        file_path: "completion_unknown_receiver_member_access_test.bsl",
        member_access_owner_type_hints: Vec::new(),
        include_flow_sensitive: false,
        deps_id: None,
        settings_id: None,
    };

    let result = get_completion_with_analysis(
        content,
        line,
        column,
        Some("completion_unknown_receiver_member_access_test.bsl"),
        &index,
        &metadata_lookup,
        &ctx,
        None,
    )
    .await
    .expect("completion ok");

    let labels: Vec<String> = result.items.into_iter().map(|c| c.item.label).collect();
    assert!(
        labels.is_empty(),
        "unknown bare receiver must stay fail-closed instead of degrading to generic completion, labels: {:?}",
        labels
    );
    assert!(
        !labels.contains(&"Добавить".to_string()),
        "fail-closed result must not reconstruct semantic member candidates, labels: {:?}",
        labels
    );
    assert!(
        !labels.contains(&"Количество".to_string()),
        "fail-closed result must not reconstruct semantic member candidates, labels: {:?}",
        labels
    );
    assert!(
        !labels.contains(&"ЛожныйСимволИзИндекса".to_string()),
        "member-access semantic miss must not backfill from symbol_index, labels: {:?}",
        labels
    );
    assert!(
        !labels.contains(&"ЛожныйМодульИзИндекса".to_string()),
        "member-access semantic miss must not backfill from module_index, labels: {:?}",
        labels
    );
    assert!(
        !labels.contains(&"ЛожныйТипИзИндекса".to_string()),
        "member-access semantic miss must not backfill from type_index, labels: {:?}",
        labels
    );
    assert!(
        !labels.contains(&"Процедура".to_string()),
        "member-access semantic miss must not degrade to keyword/index completion, labels: {:?}",
        labels
    );
}

#[tokio::test]
async fn completion_member_access_does_not_reconstruct_type_name_without_canonical_owner() {
    let repository = Arc::new(InMemoryTypeRepository::new());
    repository
        .load_types(vec![RawTypeData {
            name: "TypeA".to_string(),
            source: RawDataSource::Platform,
            methods: vec![RawMethodData {
                name: "DoWork".to_string(),
                return_type: "Булево".to_string(),
                ..Default::default()
            }],
            properties: vec![RawPropertyData {
                name: "Prop".to_string(),
                prop_type: "Строка".to_string(),
                is_readonly: true,
            }],
            ..Default::default()
        }])
        .expect("load types");

    let repo: Arc<dyn bsl_shared::domain::repository::TypeRepository> = repository.clone();
    let resolver = Arc::new(TypeResolver::new(repo.clone()));
    let metadata_lookup = TypeMetadataLookup::new(repo.clone());
    let index = IntellisenseIndexStore::new("cfg", "platform");
    let content = concat!("Процедура Тест()\n", "    TypeA.\n", "КонецПроцедуры\n");
    let line = 1;
    let line_text = "    TypeA.";
    let column = line_text.chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32;

    let deps = Arc::new(bsl_analysis_v2::SemanticDeps {
        signature_index: repo.get_signature_index_clone(),
        resolver: Some(resolver.clone()),
        repository: repo.clone(),
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
        text: Arc::from(content.to_string()),
        version: 0,
        path: Arc::from("completion_type_name_receiver_without_owner_hint_test.bsl"),
    });
    let analysis = host.analysis();
    let ir_program = analysis.ir(V2FileId(1)).ok().flatten().expect("ir");

    let ctx = CompletionAnalysisContext {
        ir_program: Some(ir_program),
        resolver: resolver.as_ref(),
        file_path: "completion_type_name_receiver_without_owner_hint_test.bsl",
        member_access_owner_type_hints: Vec::new(),
        include_flow_sensitive: false,
        deps_id: None,
        settings_id: None,
    };

    let result = get_completion_with_analysis(
        content,
        line,
        column,
        Some("completion_type_name_receiver_without_owner_hint_test.bsl"),
        &index,
        &metadata_lookup,
        &ctx,
        None,
    )
    .await
    .expect("completion ok");

    let labels: Vec<String> = result.items.into_iter().map(|c| c.item.label).collect();
    assert!(
        labels.is_empty(),
        "bare type-name receiver without canonical owner binding must stay fail-closed, labels: {:?}",
        labels
    );
    assert!(
        !labels.contains(&"DoWork".to_string()) && !labels.contains(&"Prop".to_string()),
        "semantic member candidates must not be reconstructed from repository type names, labels: {:?}",
        labels
    );
}

#[tokio::test]
async fn completion_implicit_form_object_member_access_resolves_from_ir_without_shared_hint() {
    let repository = Arc::new(InMemoryTypeRepository::new());
    repository
        .load_types(vec![
            RawTypeData {
                name: "Документы.Док1".to_string(),
                source: RawDataSource::Configuration,
                facets: vec![FacetKind::Manager, FacetKind::Object, FacetKind::Reference],
                kind: Some(MetadataKind::Document),
                ..Default::default()
            },
            RawTypeData {
                name: "Формы.Документы.Док1.Форма1".to_string(),
                source: RawDataSource::Configuration,
                properties: vec![RawPropertyData {
                    name: "РеквизитФормы".to_string(),
                    prop_type: "Строка".to_string(),
                    is_readonly: false,
                }],
                ..Default::default()
            },
            RawTypeData {
                name: "ДокументОбъект".to_string(),
                source: RawDataSource::Platform,
                facets: vec![FacetKind::Object],
                methods: vec![RawMethodData {
                    name: "Записать".to_string(),
                    return_type: "Булево".to_string(),
                    ..Default::default()
                }],
                properties: vec![RawPropertyData {
                    name: "ФацетСвойство".to_string(),
                    prop_type: "Число".to_string(),
                    is_readonly: false,
                }],
                ..Default::default()
            },
        ])
        .expect("load types");

    let repo: Arc<dyn bsl_shared::domain::repository::TypeRepository> = repository.clone();
    let resolver = Arc::new(TypeResolver::new(repo.clone()));
    let metadata_lookup = TypeMetadataLookup::new(repo.clone());
    let index = IntellisenseIndexStore::new("cfg", "platform");

    let content = concat!("Процедура Тест()\n", "    Объект.\n", "КонецПроцедуры\n");
    let file_path = "Documents/Док1/Forms/Форма1/Ext/Form/Module.bsl";
    let line = 1;
    // Cursor is positioned on '.' (not after it) to emulate editor behavior.
    let column = "    Объект".chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32;

    let deps = Arc::new(bsl_analysis_v2::SemanticDeps {
        signature_index: repo.get_signature_index_clone(),
        resolver: Some(resolver.clone()),
        repository: repo.clone(),
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
        text: Arc::from(content.to_string()),
        version: 0,
        path: Arc::from(file_path),
    });
    let analysis = host.analysis();
    let ir_program = analysis.ir(V2FileId(1)).ok().flatten().expect("ir");

    let ctx = CompletionAnalysisContext {
        ir_program: Some(ir_program),
        resolver: resolver.as_ref(),
        file_path,
        member_access_owner_type_hints: Vec::new(),
        include_flow_sensitive: false,
        deps_id: None,
        settings_id: None,
    };

    let result = get_completion_with_analysis(
        content,
        line,
        column,
        Some("file:///completion_form_module_implicit_owner_test.bsl"),
        &index,
        &metadata_lookup,
        &ctx,
        None,
    )
    .await
    .expect("completion ok");

    let labels: Vec<String> = result.items.into_iter().map(|c| c.item.label).collect();
    assert!(
        !labels.contains(&"Записать".to_string()),
        "labels: {:?}",
        labels
    );
    assert!(
        labels.contains(&"Ссылка".to_string()),
        "labels: {:?}",
        labels
    );
    assert!(
        labels.contains(&"ПометкаУдаления".to_string()),
        "labels: {:?}",
        labels
    );
}

#[tokio::test]
async fn completion_resolves_implicit_form_object_member_access_with_shared_hint() {
    let repository = Arc::new(InMemoryTypeRepository::new());
    repository
        .load_types(vec![
            RawTypeData {
                name: "Документы.Док1".to_string(),
                source: RawDataSource::Configuration,
                facets: vec![FacetKind::Manager, FacetKind::Object, FacetKind::Reference],
                kind: Some(MetadataKind::Document),
                ..Default::default()
            },
            RawTypeData {
                name: "Формы.Документы.Док1.Форма1".to_string(),
                source: RawDataSource::Configuration,
                properties: vec![RawPropertyData {
                    name: "РеквизитФормы".to_string(),
                    prop_type: "Строка".to_string(),
                    is_readonly: false,
                }],
                ..Default::default()
            },
            RawTypeData {
                name: "ДокументОбъект".to_string(),
                source: RawDataSource::Platform,
                facets: vec![FacetKind::Object],
                methods: vec![RawMethodData {
                    name: "Записать".to_string(),
                    return_type: "Булево".to_string(),
                    ..Default::default()
                }],
                properties: vec![RawPropertyData {
                    name: "ФацетСвойство".to_string(),
                    prop_type: "Число".to_string(),
                    is_readonly: false,
                }],
                ..Default::default()
            },
        ])
        .expect("load types");

    let repo: Arc<dyn bsl_shared::domain::repository::TypeRepository> = repository.clone();
    let resolver = Arc::new(TypeResolver::new(repo.clone()));
    let metadata_lookup = TypeMetadataLookup::new(repo.clone());
    let index = IntellisenseIndexStore::new("cfg", "platform");

    let content = concat!(
        "Процедура Тест()\n",
        "    x = Объект;\n",
        "    Объект.\n",
        "КонецПроцедуры\n"
    );
    let file_path = "Documents/Док1/Forms/Форма1/Ext/Form/Module.bsl";
    let line = 2;
    let column = "    Объект".chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32;

    let deps = Arc::new(bsl_analysis_v2::SemanticDeps {
        signature_index: repo.get_signature_index_clone(),
        resolver: Some(resolver.clone()),
        repository: repo.clone(),
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
        text: Arc::from(content.to_string()),
        version: 0,
        path: Arc::from(file_path),
    });
    let analysis = host.analysis();
    let member_access_owner_type_hint = analysis
        .type_at_byte_offset(
            V2FileId(1),
            byte_offset_of(content, "x = Объект") + "x = ".len() as u32,
        )
        .expect("type_at_byte_offset query");
    assert!(
        member_access_owner_type_hint.is_some(),
        "expected shared owner hint for FormModule.Объект"
    );
    let ir_program = analysis.ir(V2FileId(1)).ok().flatten().expect("ir");

    let ctx = CompletionAnalysisContext {
        ir_program: Some(ir_program),
        resolver: resolver.as_ref(),
        file_path,
        member_access_owner_type_hints: member_access_owner_type_hint.into_iter().collect(),
        include_flow_sensitive: false,
        deps_id: None,
        settings_id: None,
    };

    let result = get_completion_with_analysis(
        content,
        line,
        column,
        Some("file:///completion_form_module_implicit_owner_test.bsl"),
        &index,
        &metadata_lookup,
        &ctx,
        None,
    )
    .await
    .expect("completion ok");

    let labels: Vec<String> = result.items.into_iter().map(|c| c.item.label).collect();
    assert!(
        !labels.contains(&"Записать".to_string()),
        "labels: {:?}",
        labels
    );
    assert!(
        labels.contains(&"Ссылка".to_string()),
        "labels: {:?}",
        labels
    );
    assert!(
        labels.contains(&"ПометкаУдаления".to_string()),
        "labels: {:?}",
        labels
    );
}

#[tokio::test]
async fn completion_uses_owner_hint_for_member_access_when_flow_sensitive_is_enabled() {
    let repository = Arc::new(InMemoryTypeRepository::new());
    repository
        .load_types(vec![RawTypeData {
            name: "Строка".to_string(),
            source: RawDataSource::Platform,
            methods: vec![RawMethodData {
                name: "Длина".to_string(),
                return_type: "Число".to_string(),
                ..Default::default()
            }],
            ..Default::default()
        }])
        .expect("load types");

    let repo: Arc<dyn bsl_shared::domain::repository::TypeRepository> = repository.clone();
    let resolver = Arc::new(TypeResolver::new(repo.clone()));
    let metadata_lookup = TypeMetadataLookup::new(repo.clone());
    let index = IntellisenseIndexStore::new("cfg", "platform");

    let content = concat!(
        "Процедура Тест()\n",
        "    Перем x;\n",
        "    Если ТипЗнч(x) = Тип(\"Строка\") Тогда\n",
        "        x.\n",
        "    КонецЕсли;\n",
        "КонецПроцедуры\n"
    );

    let line = 3;
    let line_text = "        x.";
    let column = line_text.chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32;

    let deps = Arc::new(bsl_analysis_v2::SemanticDeps {
        signature_index: repo.get_signature_index_clone(),
        resolver: Some(resolver.clone()),
        repository: repo.clone(),
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
        text: Arc::from(content.to_string()),
        version: 0,
        path: Arc::from("completion_narrowing_test.bsl"),
    });
    let analysis = host.analysis();
    let ir_program = analysis.ir(V2FileId(1)).ok().flatten().expect("ir");

    let ctx = CompletionAnalysisContext {
        ir_program: Some(ir_program),
        resolver: resolver.as_ref(),
        file_path: "completion_narrowing_test.bsl",
        member_access_owner_type_hints: owner_hint("Строка"),
        include_flow_sensitive: true,
        deps_id: None,
        settings_id: None,
    };

    let result = get_completion_with_analysis(
        content,
        line,
        column,
        Some("completion_narrowing_test.bsl"),
        &index,
        &metadata_lookup,
        &ctx,
        None,
    )
    .await
    .expect("completion ok");

    let labels: Vec<String> = result.items.into_iter().map(|c| c.item.label).collect();
    assert!(
        labels.contains(&"Длина".to_string()),
        "labels: {:?}",
        labels
    );
}

#[test]
fn implicit_module_context_owner_resolution_uses_shared_exact_owner_hints_for_supported_modules() {
    let repository = Arc::new(InMemoryTypeRepository::new());
    repository
        .load_types(vec![RawTypeData {
            name: "Формы.Документы.Док1.Форма1".to_string(),
            source: RawDataSource::Configuration,
            ..Default::default()
        }])
        .expect("load types");

    let repo: Arc<dyn TypeRepository> = repository.clone();
    let resolver = Arc::new(TypeResolver::new(repo.clone()));
    let deps = Arc::new(bsl_analysis_v2::SemanticDeps {
        signature_index: repo.get_signature_index_clone(),
        resolver: Some(resolver.clone()),
        repository: repo.clone(),
        platform_signatures_loaded: false,
    });

    let cases = [
        ("Documents/Док1/Forms/Форма1/Ext/Form/Module.bsl", "Объект"),
        (
            "Documents/Док1/Forms/Форма1/Ext/Form/Module.bsl",
            "ЭтотОбъект",
        ),
        ("Documents/Док1/Ext/ManagerModule.bsl", "Объект"),
        ("Documents/Док1/Ext/ObjectModule.bsl", "ЭтотОбъект"),
        (
            "InformationRegisters/Регистр1/Ext/RecordSetModule.bsl",
            "Объект",
        ),
    ];

    for (file_path, base_name) in cases {
        let content = format!(
            "Процедура Тест()\n    expected = {base_name};\n    {base_name}.\nКонецПроцедуры\n"
        );
        let assignment_marker = format!("expected = {base_name}");
        let assignment_offset =
            byte_offset_of(&content, &assignment_marker) + "expected = ".len() as u32;
        let (_, dot_column) = utf16_column(&content, ".");
        let access_column = dot_column + 1;

        let mut host = AnalysisHostV2::default();
        host.apply_change(ChangeV2::SetDepsSnapshot {
            deps_id: DepsSnapshotId::from_hash(format!("implicit-module-context-{base_name}")),
            deps: deps.clone(),
        });
        host.apply_change(ChangeV2::SetSettingsSnapshot {
            settings_id: SettingsId::from_hash("implicit-module-context-fallback"),
            diagnostics_detail_level: DetailLevel::Full,
        });
        host.apply_change(ChangeV2::SetFile {
            file_id: V2FileId(1),
            text: Arc::from(content.clone()),
            version: 0,
            path: Arc::from(file_path),
        });

        let analysis = host.analysis();
        analysis
            .precompute_type_index_for_file(V2FileId(1), Some(0), 0)
            .expect("precompute exact type index");
        let expected = analysis
            .type_at_byte_offset(V2FileId(1), assignment_offset)
            .expect("type_at_byte_offset query")
            .expect("shared resolution for implicit context symbol");
        let ir_program = analysis.ir(V2FileId(1)).ok().flatten().expect("ir");

        let shared_owner_hints = completion_member_access_owner_type_hints_from_analysis(
            &analysis,
            V2FileId(1),
            &content,
            2,
            access_column,
        );
        assert_eq!(
            shared_owner_hints,
            vec![expected.clone()],
            "supported module-context path must surface canonical exact owner hints for {file_path}:{base_name}"
        );

        let ctx_without_hint = CompletionAnalysisContext {
            ir_program: Some(ir_program),
            resolver: resolver.as_ref(),
            file_path,
            member_access_owner_type_hints: Vec::new(),
            include_flow_sensitive: false,
            deps_id: None,
            settings_id: None,
        };

        let without_hint = resolve_member_owner_type_sync(
            Some(&ctx_without_hint),
            &content,
            2,
            access_column,
            base_name,
        );
        assert!(
            without_hint.is_none(),
            "member-access owner resolution must fail closed without shared exact hint for {file_path}:{base_name}"
        );

        let ctx_with_hint = CompletionAnalysisContext {
            ir_program: Some(ctx_without_hint.ir_program.expect("ir program available")),
            resolver: resolver.as_ref(),
            file_path,
            member_access_owner_type_hints: shared_owner_hints,
            include_flow_sensitive: false,
            deps_id: None,
            settings_id: None,
        };
        let resolved = resolve_member_owner_type_sync(
            Some(&ctx_with_hint),
            &content,
            2,
            access_column,
            base_name,
        )
        .expect("shared owner hint must resolve supported implicit module symbol");

        assert_eq!(
            resolved, expected,
            "shared owner hint must match analysis resolution for {file_path}:{base_name}"
        );
    }
}

#[test]
fn implicit_module_context_owner_resolution_fails_closed_outside_supported_modules() {
    let repository = Arc::new(InMemoryTypeRepository::new());
    let repo: Arc<dyn TypeRepository> = repository.clone();
    let resolver = Arc::new(TypeResolver::new(repo.clone()));
    let deps = Arc::new(bsl_analysis_v2::SemanticDeps {
        signature_index: repo.get_signature_index_clone(),
        resolver: Some(resolver.clone()),
        repository: repo,
        platform_signatures_loaded: false,
    });

    let content = concat!("Процедура Тест()\n", "    Объект.\n", "КонецПроцедуры\n",);
    let access_column = "    Объект".chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32;

    let mut host = AnalysisHostV2::default();
    host.apply_change(ChangeV2::SetDepsSnapshot {
        deps_id: DepsSnapshotId::from_hash("implicit-module-context-unsupported"),
        deps,
    });
    host.apply_change(ChangeV2::SetSettingsSnapshot {
        settings_id: SettingsId::from_hash("implicit-module-context-fallback"),
        diagnostics_detail_level: DetailLevel::Full,
    });
    host.apply_change(ChangeV2::SetFile {
        file_id: V2FileId(1),
        text: Arc::from(content),
        version: 0,
        path: Arc::from("CommonModules/ОбщегоНазначения/Ext/Module.bsl"),
    });

    let analysis = host.analysis();
    let ir_program = analysis.ir(V2FileId(1)).ok().flatten().expect("ir");
    let ctx = CompletionAnalysisContext {
        ir_program: Some(ir_program),
        resolver: resolver.as_ref(),
        file_path: "CommonModules/ОбщегоНазначения/Ext/Module.bsl",
        member_access_owner_type_hints: Vec::new(),
        include_flow_sensitive: false,
        deps_id: None,
        settings_id: None,
    };

    let fallback = resolve_member_owner_type_sync(Some(&ctx), content, 1, access_column, "Объект");
    assert!(
        fallback.is_none(),
        "implicit module-context owner resolution must fail closed outside supported module-context paths"
    );
}

#[tokio::test]
async fn completion_resolves_nested_member_access_chain() {
    let repository = Arc::new(InMemoryTypeRepository::new());
    repository
        .load_types(vec![
            RawTypeData {
                name: "ТаблицаЗначений".to_string(),
                source: RawDataSource::Platform,
                properties: vec![RawPropertyData {
                    name: "Колонки".to_string(),
                    prop_type: "КоллекцияКолонокТаблицыЗначений".to_string(),
                    is_readonly: true,
                }],
                ..Default::default()
            },
            RawTypeData {
                name: "КоллекцияКолонокТаблицыЗначений".to_string(),
                source: RawDataSource::Platform,
                methods: vec![RawMethodData {
                    name: "Добавить".to_string(),
                    return_type: "КолонкаТаблицыЗначений".to_string(),
                    ..Default::default()
                }],
                ..Default::default()
            },
        ])
        .expect("load types");

    let repo: Arc<dyn bsl_shared::domain::repository::TypeRepository> = repository.clone();
    let resolver = Arc::new(TypeResolver::new(repo.clone()));
    let metadata_lookup = TypeMetadataLookup::new(repo.clone());

    let index = IntellisenseIndexStore::new("cfg", "platform");
    let content = concat!(
        "Процедура Тест()\n",
        "    ТаблЗнач = Новый ТаблицаЗначений;\n",
        "    ТаблЗнач.Колонки.\n",
        "КонецПроцедуры\n"
    );
    let line = 2;
    let line_text = "    ТаблЗнач.Колонки.";
    let column = line_text.chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32;

    let deps = Arc::new(bsl_analysis_v2::SemanticDeps {
        signature_index: repo.get_signature_index_clone(),
        resolver: Some(resolver.clone()),
        repository: repo.clone(),
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
        text: Arc::from(content.to_string()),
        version: 0,
        path: Arc::from("completion_nested_chain_test.bsl"),
    });
    let analysis = host.analysis();
    let ir_program = analysis.ir(V2FileId(1)).ok().flatten().expect("ir");

    let ctx = CompletionAnalysisContext {
        ir_program: Some(ir_program),
        resolver: resolver.as_ref(),
        file_path: "completion_nested_chain_test.bsl",
        member_access_owner_type_hints: owner_hint("КоллекцияКолонокТаблицыЗначений"),
        include_flow_sensitive: false,
        deps_id: None,
        settings_id: None,
    };

    let result = get_completion_with_analysis(
        content,
        line,
        column,
        Some("completion_nested_chain_test.bsl"),
        &index,
        &metadata_lookup,
        &ctx,
        None,
    )
    .await
    .expect("completion ok");

    let labels: Vec<String> = result.items.into_iter().map(|c| c.item.label).collect();
    assert!(
        labels.contains(&"Добавить".to_string()),
        "labels: {:?}",
        labels
    );
}

#[tokio::test]
async fn completion_supports_member_access_after_method_call() {
    let repository = Arc::new(InMemoryTypeRepository::new());
    repository
        .load_types(vec![
            RawTypeData {
                name: "ТаблицаЗначений".to_string(),
                source: RawDataSource::Platform,
                properties: vec![RawPropertyData {
                    name: "Колонки".to_string(),
                    prop_type: "КоллекцияКолонокТаблицыЗначений".to_string(),
                    is_readonly: true,
                }],
                ..Default::default()
            },
            RawTypeData {
                name: "КоллекцияКолонокТаблицыЗначений".to_string(),
                source: RawDataSource::Platform,
                methods: vec![RawMethodData {
                    name: "Добавить".to_string(),
                    return_type: "КолонкаТаблицыЗначений".to_string(),
                    ..Default::default()
                }],
                ..Default::default()
            },
            RawTypeData {
                name: "КолонкаТаблицыЗначений".to_string(),
                source: RawDataSource::Platform,
                properties: vec![RawPropertyData {
                    name: "Имя".to_string(),
                    prop_type: "Строка".to_string(),
                    is_readonly: true,
                }],
                ..Default::default()
            },
        ])
        .expect("load types");

    let repo: Arc<dyn bsl_shared::domain::repository::TypeRepository> = repository.clone();
    let resolver = Arc::new(TypeResolver::new(repo.clone()));
    let metadata_lookup = TypeMetadataLookup::new(repo.clone());

    let index = IntellisenseIndexStore::new("cfg", "platform");
    let content = concat!(
        "Процедура Тест()\n",
        "    ТаблЗнач = Новый ТаблицаЗначений;\n",
        "    ТаблЗнач.Колонки.Добавить().\n",
        "КонецПроцедуры\n"
    );
    let line = 2;
    let line_text = "    ТаблЗнач.Колонки.Добавить().";
    let column = line_text.chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32;

    let deps = Arc::new(bsl_analysis_v2::SemanticDeps {
        signature_index: repo.get_signature_index_clone(),
        resolver: Some(resolver.clone()),
        repository: repo.clone(),
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
        text: Arc::from(content.to_string()),
        version: 0,
        path: Arc::from("completion_call_chain_test.bsl"),
    });
    let analysis = host.analysis();
    let ir_program = analysis.ir(V2FileId(1)).ok().flatten().expect("ir");
    let ctx = CompletionAnalysisContext {
        ir_program: Some(ir_program),
        resolver: resolver.as_ref(),
        file_path: "completion_call_chain_test.bsl",
        member_access_owner_type_hints: owner_hint("КолонкаТаблицыЗначений"),
        include_flow_sensitive: false,
        deps_id: None,
        settings_id: None,
    };

    let result = get_completion_with_analysis(
        content,
        line,
        column,
        Some("completion_call_chain_test.bsl"),
        &index,
        &metadata_lookup,
        &ctx,
        None,
    )
    .await
    .expect("completion ok");

    let labels: Vec<String> = result.items.into_iter().map(|c| c.item.label).collect();
    assert!(labels.contains(&"Имя".to_string()), "labels: {:?}", labels);
}

#[tokio::test]
async fn completion_supports_member_access_after_index_access() {
    let repository = Arc::new(InMemoryTypeRepository::new());
    repository
        .load_types(vec![
            RawTypeData {
                name: "Массив".to_string(),
                source: RawDataSource::Platform,
                collection_item_type: Some("КолонкаТаблицыЗначений".to_string()),
                ..Default::default()
            },
            RawTypeData {
                name: "КолонкаТаблицыЗначений".to_string(),
                source: RawDataSource::Platform,
                properties: vec![RawPropertyData {
                    name: "Имя".to_string(),
                    prop_type: "Строка".to_string(),
                    is_readonly: true,
                }],
                ..Default::default()
            },
        ])
        .expect("load types");

    let repo: Arc<dyn bsl_shared::domain::repository::TypeRepository> = repository.clone();
    let resolver = Arc::new(TypeResolver::new(repo.clone()));
    let metadata_lookup = TypeMetadataLookup::new(repo.clone());

    let index = IntellisenseIndexStore::new("cfg", "platform");
    let content = concat!(
        "Процедура Тест()\n",
        "    Перем arr;\n",
        "    arr = Новый Массив;\n",
        "    arr[0].\n",
        "КонецПроцедуры\n"
    );
    let line = 3;
    let line_text = "    arr[0].";
    let column = line_text.chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32;

    let deps = Arc::new(bsl_analysis_v2::SemanticDeps {
        signature_index: repo.get_signature_index_clone(),
        resolver: Some(resolver.clone()),
        repository: repo.clone(),
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
        text: Arc::from(content.to_string()),
        version: 0,
        path: Arc::from("completion_index_access_test.bsl"),
    });
    let analysis = host.analysis();
    let ir_program = analysis.ir(V2FileId(1)).ok().flatten().expect("ir");
    let ctx = CompletionAnalysisContext {
        ir_program: Some(ir_program),
        resolver: resolver.as_ref(),
        file_path: "completion_index_access_test.bsl",
        member_access_owner_type_hints: owner_hint("КолонкаТаблицыЗначений"),
        include_flow_sensitive: false,
        deps_id: None,
        settings_id: None,
    };

    let result = get_completion_with_analysis(
        content,
        line,
        column,
        Some("completion_index_access_test.bsl"),
        &index,
        &metadata_lookup,
        &ctx,
        None,
    )
    .await
    .expect("completion ok");

    let labels: Vec<String> = result.items.into_iter().map(|c| c.item.label).collect();
    assert!(labels.contains(&"Имя".to_string()), "labels: {:?}", labels);
}

#[tokio::test]
async fn completion_supports_member_access_after_map_index_access() {
    let repository = Arc::new(InMemoryTypeRepository::new());
    repository
        .load_types(vec![
            RawTypeData {
                name: "Соответствие".to_string(),
                source: RawDataSource::Platform,
                collection_item_type: Some("КолонкаТаблицыЗначений".to_string()),
                ..Default::default()
            },
            RawTypeData {
                name: "КолонкаТаблицыЗначений".to_string(),
                source: RawDataSource::Platform,
                properties: vec![RawPropertyData {
                    name: "Имя".to_string(),
                    prop_type: "Строка".to_string(),
                    is_readonly: true,
                }],
                ..Default::default()
            },
        ])
        .expect("load types");

    let repo: Arc<dyn bsl_shared::domain::repository::TypeRepository> = repository.clone();
    let resolver = Arc::new(TypeResolver::new(repo.clone()));
    let metadata_lookup = TypeMetadataLookup::new(repo.clone());

    let index = IntellisenseIndexStore::new("cfg", "platform");
    let content = concat!(
        "Процедура Тест()\n",
        "    Перем map;\n",
        "    map = Новый Соответствие;\n",
        "    map[\"k\"].\n",
        "КонецПроцедуры\n"
    );
    let line = 3;
    let line_text = "    map[\"k\"].";
    let column = line_text.chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32;

    let deps = Arc::new(bsl_analysis_v2::SemanticDeps {
        signature_index: repo.get_signature_index_clone(),
        resolver: Some(resolver.clone()),
        repository: repo.clone(),
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
        text: Arc::from(content.to_string()),
        version: 0,
        path: Arc::from("completion_map_index_access_test.bsl"),
    });
    let analysis = host.analysis();
    let ir_program = analysis.ir(V2FileId(1)).ok().flatten().expect("ir");
    let ctx = CompletionAnalysisContext {
        ir_program: Some(ir_program),
        resolver: resolver.as_ref(),
        file_path: "completion_map_index_access_test.bsl",
        member_access_owner_type_hints: owner_hint("КолонкаТаблицыЗначений"),
        include_flow_sensitive: false,
        deps_id: None,
        settings_id: None,
    };

    let result = get_completion_with_analysis(
        content,
        line,
        column,
        Some("completion_map_index_access_test.bsl"),
        &index,
        &metadata_lookup,
        &ctx,
        None,
    )
    .await
    .expect("completion ok");

    let labels: Vec<String> = result.items.into_iter().map(|c| c.item.label).collect();
    assert!(labels.contains(&"Имя".to_string()), "labels: {:?}", labels);
}

#[tokio::test]
async fn completion_does_not_infer_map_index_owner_without_shared_hint() {
    let repository = Arc::new(InMemoryTypeRepository::new());
    repository
        .load_types(vec![
            RawTypeData {
                name: "Соответствие".to_string(),
                source: RawDataSource::Platform,
                collection_item_type: Some("КолонкаТаблицыЗначений".to_string()),
                ..Default::default()
            },
            RawTypeData {
                name: "КолонкаТаблицыЗначений".to_string(),
                source: RawDataSource::Platform,
                properties: vec![RawPropertyData {
                    name: "Имя".to_string(),
                    prop_type: "Строка".to_string(),
                    is_readonly: true,
                }],
                ..Default::default()
            },
        ])
        .expect("load types");

    let repo: Arc<dyn bsl_shared::domain::repository::TypeRepository> = repository.clone();
    let resolver = Arc::new(TypeResolver::new(repo.clone()));
    let metadata_lookup = TypeMetadataLookup::new(repo.clone());

    let index = IntellisenseIndexStore::new("cfg", "platform");
    let content = concat!(
        "Процедура Тест()\n",
        "    Перем map;\n",
        "    map = Новый Соответствие;\n",
        "    map[\"k\"].\n",
        "КонецПроцедуры\n"
    );
    let line = 3;
    let line_text = "    map[\"k\"].";
    let column = line_text.chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32;

    let deps = Arc::new(bsl_analysis_v2::SemanticDeps {
        signature_index: repo.get_signature_index_clone(),
        resolver: Some(resolver.clone()),
        repository: repo.clone(),
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
        text: Arc::from(content.to_string()),
        version: 0,
        path: Arc::from("completion_map_index_access_no_hint_test.bsl"),
    });
    let analysis = host.analysis();
    let ir_program = analysis.ir(V2FileId(1)).ok().flatten().expect("ir");
    let ctx = CompletionAnalysisContext {
        ir_program: Some(ir_program),
        resolver: resolver.as_ref(),
        file_path: "completion_map_index_access_no_hint_test.bsl",
        member_access_owner_type_hints: Vec::new(),
        include_flow_sensitive: false,
        deps_id: None,
        settings_id: None,
    };

    let result = get_completion_with_analysis(
        content,
        line,
        column,
        Some("completion_map_index_access_no_hint_test.bsl"),
        &index,
        &metadata_lookup,
        &ctx,
        None,
    )
    .await
    .expect("completion ok");

    let labels: Vec<String> = result.items.into_iter().map(|c| c.item.label).collect();
    assert!(
        !labels.contains(&"Имя".to_string()),
        "map index owner must come from shared owner hint, labels: {:?}",
        labels
    );
}

#[tokio::test]
async fn completion_does_not_infer_type_name_member_access_without_canonical_owner_hint() {
    let repository = Arc::new(InMemoryTypeRepository::new());
    repository
        .load_types(vec![RawTypeData {
            name: "ТаблицаЗначений".to_string(),
            source: RawDataSource::Platform,
            methods: vec![RawMethodData {
                name: "Добавить".to_string(),
                return_type: "Булево".to_string(),
                ..Default::default()
            }],
            properties: vec![RawPropertyData {
                name: "Количество".to_string(),
                prop_type: "Число".to_string(),
                is_readonly: true,
            }],
            ..Default::default()
        }])
        .expect("load types");

    let repo: Arc<dyn bsl_shared::domain::repository::TypeRepository> = repository.clone();
    let resolver = Arc::new(TypeResolver::new(repo.clone()));
    let metadata_lookup = TypeMetadataLookup::new(repo.clone());

    let index = IntellisenseIndexStore::new("cfg", "platform");
    let content = concat!(
        "Процедура Тест()\n",
        "    ТаблицаЗначений.\n",
        "КонецПроцедуры\n"
    );
    let line = 1;
    let line_text = "    ТаблицаЗначений.";
    let column = line_text.chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32;

    let deps = Arc::new(bsl_analysis_v2::SemanticDeps {
        signature_index: repo.get_signature_index_clone(),
        resolver: Some(resolver.clone()),
        repository: repo.clone(),
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
        text: Arc::from(content.to_string()),
        version: 0,
        path: Arc::from("completion_type_name_no_hint_test.bsl"),
    });
    let analysis = host.analysis();
    let ir_program = analysis.ir(V2FileId(1)).ok().flatten().expect("ir");
    let ctx = CompletionAnalysisContext {
        ir_program: Some(ir_program),
        resolver: resolver.as_ref(),
        file_path: "completion_type_name_no_hint_test.bsl",
        member_access_owner_type_hints: Vec::new(),
        include_flow_sensitive: false,
        deps_id: None,
        settings_id: None,
    };

    let result = get_completion_with_analysis(
        content,
        line,
        column,
        Some("completion_type_name_no_hint_test.bsl"),
        &index,
        &metadata_lookup,
        &ctx,
        None,
    )
    .await
    .expect("completion ok");

    let labels: Vec<String> = result.items.into_iter().map(|c| c.item.label).collect();
    assert!(
        !labels.contains(&"Добавить".to_string()) && !labels.contains(&"Количество".to_string()),
        "type-name member access without canonical owner hint must fail closed, labels: {:?}",
        labels
    );
}

#[tokio::test]
async fn completion_does_not_infer_type_name_member_chain_without_canonical_owner_hint() {
    let repository = Arc::new(InMemoryTypeRepository::new());
    repository
        .load_types(vec![
            RawTypeData {
                name: "ТаблицаЗначений".to_string(),
                source: RawDataSource::Platform,
                properties: vec![RawPropertyData {
                    name: "Колонки".to_string(),
                    prop_type: "КоллекцияКолонокТаблицыЗначений".to_string(),
                    is_readonly: true,
                }],
                ..Default::default()
            },
            RawTypeData {
                name: "КоллекцияКолонокТаблицыЗначений".to_string(),
                source: RawDataSource::Platform,
                methods: vec![RawMethodData {
                    name: "Добавить".to_string(),
                    return_type: "КолонкаТаблицыЗначений".to_string(),
                    ..Default::default()
                }],
                ..Default::default()
            },
        ])
        .expect("load types");

    let repo: Arc<dyn bsl_shared::domain::repository::TypeRepository> = repository.clone();
    let resolver = Arc::new(TypeResolver::new(repo.clone()));
    let metadata_lookup = TypeMetadataLookup::new(repo.clone());

    let index = IntellisenseIndexStore::new("cfg", "platform");
    let content = concat!(
        "Процедура Тест()\n",
        "    ТаблицаЗначений.Колонки.\n",
        "КонецПроцедуры\n"
    );
    let line = 1;
    let line_text = "    ТаблицаЗначений.Колонки.";
    let column = line_text.chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32;

    let deps = Arc::new(bsl_analysis_v2::SemanticDeps {
        signature_index: repo.get_signature_index_clone(),
        resolver: Some(resolver.clone()),
        repository: repo.clone(),
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
        text: Arc::from(content.to_string()),
        version: 0,
        path: Arc::from("completion_type_name_chain_no_hint_test.bsl"),
    });
    let analysis = host.analysis();
    let ir_program = analysis.ir(V2FileId(1)).ok().flatten().expect("ir");
    let ctx = CompletionAnalysisContext {
        ir_program: Some(ir_program),
        resolver: resolver.as_ref(),
        file_path: "completion_type_name_chain_no_hint_test.bsl",
        member_access_owner_type_hints: Vec::new(),
        include_flow_sensitive: false,
        deps_id: None,
        settings_id: None,
    };

    let result = get_completion_with_analysis(
        content,
        line,
        column,
        Some("completion_type_name_chain_no_hint_test.bsl"),
        &index,
        &metadata_lookup,
        &ctx,
        None,
    )
    .await
    .expect("completion ok");

    let labels: Vec<String> = result.items.into_iter().map(|c| c.item.label).collect();
    assert!(
        !labels.contains(&"Добавить".to_string()),
        "type-name member chain without canonical owner hint must fail closed, labels: {:?}",
        labels
    );
}

#[tokio::test]
async fn completion_supports_member_access_after_ternary_expression() {
    let repository = Arc::new(InMemoryTypeRepository::new());
    repository
        .load_types(vec![
            RawTypeData {
                name: "TypeA".to_string(),
                source: RawDataSource::Platform,
                properties: vec![RawPropertyData {
                    name: "PropA".to_string(),
                    prop_type: "Строка".to_string(),
                    is_readonly: true,
                }],
                ..Default::default()
            },
            RawTypeData {
                name: "TypeB".to_string(),
                source: RawDataSource::Platform,
                properties: vec![RawPropertyData {
                    name: "PropB".to_string(),
                    prop_type: "Строка".to_string(),
                    is_readonly: true,
                }],
                ..Default::default()
            },
        ])
        .expect("load types");

    let repo: Arc<dyn bsl_shared::domain::repository::TypeRepository> = repository.clone();
    let resolver = Arc::new(TypeResolver::new(repo.clone()));
    let metadata_lookup = TypeMetadataLookup::new(repo.clone());

    let index = IntellisenseIndexStore::new("cfg", "platform");
    let content = concat!(
        "Процедура Тест()\n",
        "    ?(Истина, Новый TypeA, Новый TypeB).\n",
        "КонецПроцедуры\n"
    );
    let line = 1;
    let line_text = "    ?(Истина, Новый TypeA, Новый TypeB).";
    let column = line_text.chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32;

    let deps = Arc::new(bsl_analysis_v2::SemanticDeps {
        signature_index: repo.get_signature_index_clone(),
        resolver: Some(resolver.clone()),
        repository: repo.clone(),
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
        text: Arc::from(content.to_string()),
        version: 0,
        path: Arc::from("completion_ternary_test.bsl"),
    });
    let analysis = host.analysis();
    analysis
        .precompute_type_index_for_file(V2FileId(1), Some(0), 0)
        .expect("precompute exact type index");
    let owner_types = completion_member_access_owner_type_hints_from_analysis(
        &analysis,
        V2FileId(1),
        content,
        line,
        column,
    );
    assert_eq!(
        owner_types
            .iter()
            .map(TypeResolution::type_name)
            .collect::<Vec<_>>(),
        vec!["TypeA".to_string(), "TypeB".to_string()],
        "ternary receiver must resolve canonical owner alternatives from the exact shared type index"
    );
    let ir_program = analysis.ir(V2FileId(1)).ok().flatten().expect("ir");
    let ctx = CompletionAnalysisContext {
        ir_program: Some(ir_program),
        resolver: resolver.as_ref(),
        file_path: "completion_ternary_test.bsl",
        member_access_owner_type_hints: owner_types,
        include_flow_sensitive: false,
        deps_id: None,
        settings_id: None,
    };

    let result = get_completion_with_analysis(
        content,
        line,
        column,
        Some("completion_ternary_test.bsl"),
        &index,
        &metadata_lookup,
        &ctx,
        None,
    )
    .await
    .expect("completion ok");

    let labels: Vec<String> = result.items.into_iter().map(|c| c.item.label).collect();
    assert!(
        labels.contains(&"PropA".to_string()),
        "labels: {:?}",
        labels
    );
    assert!(
        labels.contains(&"PropB".to_string()),
        "labels: {:?}",
        labels
    );
}

#[tokio::test]
async fn completion_supports_member_access_after_choice_expression() {
    let repository = Arc::new(InMemoryTypeRepository::new());
    repository
        .load_types(vec![
            RawTypeData {
                name: "TypeA".to_string(),
                source: RawDataSource::Platform,
                properties: vec![RawPropertyData {
                    name: "PropA".to_string(),
                    prop_type: "Строка".to_string(),
                    is_readonly: true,
                }],
                ..Default::default()
            },
            RawTypeData {
                name: "TypeB".to_string(),
                source: RawDataSource::Platform,
                properties: vec![RawPropertyData {
                    name: "PropB".to_string(),
                    prop_type: "Строка".to_string(),
                    is_readonly: true,
                }],
                ..Default::default()
            },
        ])
        .expect("load types");

    let repo: Arc<dyn bsl_shared::domain::repository::TypeRepository> = repository.clone();
    let resolver = Arc::new(TypeResolver::new(repo.clone()));
    let metadata_lookup = TypeMetadataLookup::new(repo.clone());

    let index = IntellisenseIndexStore::new("cfg", "platform");
    let content = concat!(
        "Процедура Тест()\n",
        "    Выбор\n",
        "        Когда Истина Тогда Новый TypeA\n",
        "        Иначе Новый TypeB\n",
        "    Конец.\n",
        "КонецПроцедуры\n"
    );
    let line = 4;
    let line_text = "    Конец.";
    let column = line_text.chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32;

    let deps = Arc::new(bsl_analysis_v2::SemanticDeps {
        signature_index: repo.get_signature_index_clone(),
        resolver: Some(resolver.clone()),
        repository: repo.clone(),
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
        text: Arc::from(content.to_string()),
        version: 0,
        path: Arc::from("completion_choice_test.bsl"),
    });
    let analysis = host.analysis();
    analysis
        .precompute_type_index_for_file(V2FileId(1), Some(0), 0)
        .expect("precompute exact type index");
    let owner_types = completion_member_access_owner_type_hints_from_analysis(
        &analysis,
        V2FileId(1),
        content,
        line,
        column,
    );
    assert_eq!(
        owner_types
            .iter()
            .map(TypeResolution::type_name)
            .collect::<Vec<_>>(),
        vec!["TypeA".to_string(), "TypeB".to_string()],
        "choice receiver must resolve canonical owner alternatives from the exact shared type index"
    );
    let ir_program = analysis.ir(V2FileId(1)).ok().flatten().expect("ir");
    let ctx = CompletionAnalysisContext {
        ir_program: Some(ir_program),
        resolver: resolver.as_ref(),
        file_path: "completion_choice_test.bsl",
        member_access_owner_type_hints: owner_types,
        include_flow_sensitive: false,
        deps_id: None,
        settings_id: None,
    };

    let result = get_completion_with_analysis(
        content,
        line,
        column,
        Some("completion_choice_test.bsl"),
        &index,
        &metadata_lookup,
        &ctx,
        None,
    )
    .await
    .expect("completion ok");

    let labels: Vec<String> = result.items.into_iter().map(|c| c.item.label).collect();
    assert!(
        labels.contains(&"PropA".to_string()),
        "labels: {:?}",
        labels
    );
    assert!(
        labels.contains(&"PropB".to_string()),
        "labels: {:?}",
        labels
    );
}

#[tokio::test]
async fn completion_substitutes_faceted_metadata_name_in_return_type() {
    let repository = Arc::new(InMemoryTypeRepository::new());
    repository
        .load_types(vec![RawTypeData {
            name: "Справочники.Контрагенты".to_string(),
            source: RawDataSource::Configuration,
            properties: vec![RawPropertyData {
                name: "Наименование".to_string(),
                prop_type: "Строка".to_string(),
                is_readonly: false,
            }],
            ..Default::default()
        }])
        .expect("load types");

    let mut signatures = SignatureIndex::new();
    signatures.add_platform_method(
        TypeId::new("СправочникМенеджер"),
        MethodSignature::new(
            "СоздатьЭлемент".to_string(),
            Some("СправочникМенеджер".to_string()),
            vec![],
            Some("СправочникОбъект".to_string()),
            None,
            None,
            SignatureSource::Platform,
            None,
            ContextRequirements::default(),
        ),
    );
    repository.set_signature_index(signatures);

    let repo: Arc<dyn bsl_shared::domain::repository::TypeRepository> = repository.clone();
    let resolver = Arc::new(TypeResolver::new(repo.clone()));
    let metadata_lookup = TypeMetadataLookup::new(repo.clone());

    let index = IntellisenseIndexStore::new("cfg", "platform");
    let content = concat!(
        "Процедура Тест()\n",
        "    Manager = Справочники.Контрагенты;\n",
        "    Manager.СоздатьЭлемент().\n",
        "КонецПроцедуры\n"
    );
    let line = 2;
    let line_text = "    Manager.СоздатьЭлемент().";
    let column = line_text.chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32;

    let deps = Arc::new(bsl_analysis_v2::SemanticDeps {
        signature_index: repo.get_signature_index_clone(),
        resolver: Some(resolver.clone()),
        repository: repo.clone(),
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
        text: Arc::from(content.to_string()),
        version: 0,
        path: Arc::from("completion_facet_substitution_test.bsl"),
    });
    let analysis = host.analysis();
    let ir_program = analysis.ir(V2FileId(1)).ok().flatten().expect("ir");
    let ctx = CompletionAnalysisContext {
        ir_program: Some(ir_program),
        resolver: resolver.as_ref(),
        file_path: "completion_facet_substitution_test.bsl",
        member_access_owner_type_hints: owner_hint("Справочники.Контрагенты"),
        include_flow_sensitive: false,
        deps_id: None,
        settings_id: None,
    };

    let result = get_completion_with_analysis(
        content,
        line,
        column,
        Some("completion_facet_substitution_test.bsl"),
        &index,
        &metadata_lookup,
        &ctx,
        None,
    )
    .await
    .expect("completion ok");

    let labels: Vec<String> = result.items.into_iter().map(|c| c.item.label).collect();
    assert!(
        labels.contains(&"Наименование".to_string()),
        "labels: {:?}",
        labels
    );
}
