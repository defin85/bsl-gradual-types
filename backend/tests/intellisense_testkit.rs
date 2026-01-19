//! IntelliSense testkit: фикстуры, snapshot helpers, shared сервис.

#![allow(dead_code)]

use std::fs;
use std::path::PathBuf;

use bsl_shared::domain::{
    CompletionItem as DomainCompletionItem, CompletionKind as DomainCompletionKind,
};
use serde_json::Value;
use tower_lsp::lsp_types::{CompletionItem, CompletionItemKind, InsertTextFormat};

pub const UPDATE_GOLDEN_ENV: &str = "UPDATE_GOLDEN";

pub fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

pub fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("golden")
}

pub fn syntax_helper_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("examples")
        .join("syntax_helper")
}

pub fn config_fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("examples")
        .join("conf")
        .join("conf_test")
}

pub fn fixture_path(name: &str) -> PathBuf {
    fixtures_dir().join(name)
}

pub fn golden_path(name: &str) -> PathBuf {
    golden_dir().join(name)
}

pub fn read_fixture(name: &str) -> String {
    let path = fixture_path(name);
    fs::read_to_string(&path).unwrap_or_else(|err| {
        panic!("Failed to read fixture {}: {}", path.display(), err);
    })
}

pub fn assert_snapshot(name: &str, value: &Value) {
    let path = golden_path(name);
    let json = serde_json::to_string_pretty(value).expect("snapshot json");
    if std::env::var(UPDATE_GOLDEN_ENV).ok().as_deref() == Some("1") {
        fs::create_dir_all(path.parent().expect("golden dir")).expect("create golden dir");
        fs::write(&path, json).expect("write golden");
        return;
    }
    let expected = fs::read_to_string(&path).expect("read golden");
    assert_eq!(expected, json);
}

pub fn completion_snapshot(items: &[CompletionItem], is_incomplete: bool) -> Value {
    let items_json: Vec<Value> = items
        .iter()
        .map(|item| {
            serde_json::json!({
                "label": item.label.as_str(),
                "kind": completion_kind(item.kind),
                "insertText": item.insert_text.as_deref(),
                "insertTextFormat": insert_text_format(item.insert_text_format),
            })
        })
        .collect();

    serde_json::json!({
        "isIncomplete": is_incomplete,
        "items": items_json,
    })
}

pub fn completion_snapshot_domain(items: &[DomainCompletionItem], is_incomplete: bool) -> Value {
    let items_json: Vec<Value> = items
        .iter()
        .map(|item| {
            serde_json::json!({
                "label": item.label.as_str(),
                "kind": domain_completion_kind(item.kind),
                "insertText": item.insert_text.as_deref(),
                "insertTextFormat": Option::<&'static str>::None,
            })
        })
        .collect();

    serde_json::json!({
        "isIncomplete": is_incomplete,
        "items": items_json,
    })
}

pub fn completion_snapshot_domain_top_n(
    items: &[DomainCompletionItem],
    is_incomplete: bool,
    top_n: usize,
) -> Value {
    let items_json: Vec<Value> = items
        .iter()
        .take(top_n)
        .map(|item| {
            serde_json::json!({
                "label": item.label.as_str(),
                "kind": domain_completion_kind(item.kind),
                "insertText": item.insert_text.as_deref(),
                "insertTextFormat": Option::<&'static str>::None,
            })
        })
        .collect();

    serde_json::json!({
        "isIncomplete": is_incomplete,
        "topN": top_n,
        "items": items_json,
    })
}

pub fn find_marker_position(content: &str, marker: &str) -> (u32, u32) {
    let byte_index = content
        .find(marker)
        .unwrap_or_else(|| panic!("Marker not found: {}", marker));
    let before = &content[..byte_index + marker.len()];
    let line = before.lines().count().saturating_sub(1) as u32;
    let last_line = before.lines().last().unwrap_or("");
    let column = last_line.chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32;
    (line, column)
}

fn completion_kind(kind: Option<CompletionItemKind>) -> Option<&'static str> {
    match kind {
        Some(CompletionItemKind::TEXT) => Some("TEXT"),
        Some(CompletionItemKind::METHOD) => Some("METHOD"),
        Some(CompletionItemKind::FUNCTION) => Some("FUNCTION"),
        Some(CompletionItemKind::CONSTRUCTOR) => Some("CONSTRUCTOR"),
        Some(CompletionItemKind::FIELD) => Some("FIELD"),
        Some(CompletionItemKind::VARIABLE) => Some("VARIABLE"),
        Some(CompletionItemKind::CLASS) => Some("CLASS"),
        Some(CompletionItemKind::INTERFACE) => Some("INTERFACE"),
        Some(CompletionItemKind::MODULE) => Some("MODULE"),
        Some(CompletionItemKind::PROPERTY) => Some("PROPERTY"),
        Some(CompletionItemKind::UNIT) => Some("UNIT"),
        Some(CompletionItemKind::VALUE) => Some("VALUE"),
        Some(CompletionItemKind::ENUM) => Some("ENUM"),
        Some(CompletionItemKind::KEYWORD) => Some("KEYWORD"),
        Some(CompletionItemKind::SNIPPET) => Some("SNIPPET"),
        Some(CompletionItemKind::COLOR) => Some("COLOR"),
        Some(CompletionItemKind::FILE) => Some("FILE"),
        Some(CompletionItemKind::REFERENCE) => Some("REFERENCE"),
        Some(CompletionItemKind::FOLDER) => Some("FOLDER"),
        Some(CompletionItemKind::ENUM_MEMBER) => Some("ENUM_MEMBER"),
        Some(CompletionItemKind::CONSTANT) => Some("CONSTANT"),
        Some(CompletionItemKind::STRUCT) => Some("STRUCT"),
        Some(CompletionItemKind::EVENT) => Some("EVENT"),
        Some(CompletionItemKind::OPERATOR) => Some("OPERATOR"),
        Some(CompletionItemKind::TYPE_PARAMETER) => Some("TYPE_PARAMETER"),
        Some(_) => Some("UNKNOWN"),
        None => None,
    }
}

fn insert_text_format(format: Option<InsertTextFormat>) -> Option<&'static str> {
    match format {
        Some(InsertTextFormat::PLAIN_TEXT) => Some("PLAIN_TEXT"),
        Some(InsertTextFormat::SNIPPET) => Some("SNIPPET"),
        Some(_) => Some("UNKNOWN"),
        None => None,
    }
}

fn domain_completion_kind(kind: DomainCompletionKind) -> &'static str {
    match kind {
        DomainCompletionKind::Text => "TEXT",
        DomainCompletionKind::Method => "METHOD",
        DomainCompletionKind::Function => "FUNCTION",
        DomainCompletionKind::Constructor => "CONSTRUCTOR",
        DomainCompletionKind::Field => "FIELD",
        DomainCompletionKind::Variable => "VARIABLE",
        DomainCompletionKind::Class => "CLASS",
        DomainCompletionKind::Interface => "INTERFACE",
        DomainCompletionKind::Module => "MODULE",
        DomainCompletionKind::Property => "PROPERTY",
        DomainCompletionKind::Unit => "UNIT",
        DomainCompletionKind::Value => "VALUE",
        DomainCompletionKind::Enum => "ENUM",
        DomainCompletionKind::Keyword => "KEYWORD",
        DomainCompletionKind::Snippet => "SNIPPET",
        DomainCompletionKind::Color => "COLOR",
        DomainCompletionKind::File => "FILE",
        DomainCompletionKind::Reference => "REFERENCE",
        DomainCompletionKind::Folder => "FOLDER",
        DomainCompletionKind::EnumMember => "ENUM_MEMBER",
        DomainCompletionKind::Constant => "CONSTANT",
        DomainCompletionKind::Struct => "STRUCT",
        DomainCompletionKind::Type => "TYPE",
        DomainCompletionKind::Event => "EVENT",
        DomainCompletionKind::Operator => "OPERATOR",
        DomainCompletionKind::TypeParameter => "TYPE_PARAMETER",
        DomainCompletionKind::Global => "GLOBAL",
        DomainCompletionKind::Catalog => "CATALOG",
        DomainCompletionKind::Document => "DOCUMENT",
        DomainCompletionKind::MetadataUnknown => "METADATA_UNKNOWN",
        DomainCompletionKind::Report => "REPORT",
        DomainCompletionKind::DataProcessor => "DATA_PROCESSOR",
        DomainCompletionKind::Register => "REGISTER",
        DomainCompletionKind::InformationRegister => "INFORMATION_REGISTER",
        DomainCompletionKind::AccumulationRegister => "ACCUMULATION_REGISTER",
        DomainCompletionKind::AccountingRegister => "ACCOUNTING_REGISTER",
        DomainCompletionKind::CalculationRegister => "CALCULATION_REGISTER",
        DomainCompletionKind::ChartOfAccounts => "CHART_OF_ACCOUNTS",
        DomainCompletionKind::ChartOfCharacteristicTypes => "CHART_OF_CHARACTERISTIC_TYPES",
        DomainCompletionKind::ChartOfCalculationTypes => "CHART_OF_CALCULATION_TYPES",
        DomainCompletionKind::BusinessProcess => "BUSINESS_PROCESS",
        DomainCompletionKind::Task => "TASK",
        DomainCompletionKind::ExchangePlan => "EXCHANGE_PLAN",
        DomainCompletionKind::CommonModule => "COMMON_MODULE",
        DomainCompletionKind::Role => "ROLE",
        DomainCompletionKind::Subsystem => "SUBSYSTEM",
        DomainCompletionKind::Language => "LANGUAGE",
    }
}
