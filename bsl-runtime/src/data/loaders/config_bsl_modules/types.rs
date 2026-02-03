use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use bsl_shared::domain::code_location::ModuleType;
use bsl_shared::domain::signature_index::ContextRequirements;
use bsl_shared::domain::type_definition_location::TypeDefinitionLocation;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleIndexProgress {
    pub current: usize,
    pub total: usize,
    pub module_path: PathBuf,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IndexedConfigSignatures {
    pub config_methods: Vec<(String, bsl_shared::domain::signature_index::MethodSignature)>,
    pub global_functions: Vec<(String, bsl_shared::domain::signature_index::MethodSignature)>,
    pub definition_locations: Vec<(String, String, TypeDefinitionLocation)>,
    pub global_definition_locations: Vec<(String, TypeDefinitionLocation)>,
    pub module_signatures: Vec<ModuleSignatureSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleSignatureSnapshot {
    pub module_path: PathBuf,
    pub owner_type: Option<String>,
    pub method_names: Vec<String>,
    pub global_function_names: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModuleIndexResult {
    pub module_path: PathBuf,
    pub config_methods: Vec<(String, bsl_shared::domain::signature_index::MethodSignature)>,
    pub global_functions: Vec<(String, bsl_shared::domain::signature_index::MethodSignature)>,
    pub definition_locations: Vec<(String, String, TypeDefinitionLocation)>,
    pub global_definition_locations: Vec<(String, TypeDefinitionLocation)>,
    pub snapshot: ModuleSignatureSnapshot,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModuleParseStats {
    pub decls: usize,
    pub export_decls: usize,
    pub call_sites: usize,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SinglePassMode {
    Lite,
    Full,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModuleParseComparison {
    pub module_path: PathBuf,
    pub single_pass: ModuleParseStats,
    pub ast: ModuleParseStats,
    pub missing_decls: Vec<String>,
    pub extra_decls: Vec<String>,
    pub callsite_mismatches: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ParsedDecl {
    pub(crate) name: String,
    pub(crate) params: Vec<ParsedParam>,
    pub(crate) is_export: bool,
    pub(crate) directive_ctx: Option<ContextRequirements>,
    pub(crate) return_type: Option<String>,
    /// Данные для межпроцедурного вывода return-типа (summary-based).
    ///
    /// Заполняется парсером модуля (tree-sitter single-pass или AST fallback).
    /// Может быть None для процедур или если парсер не смог извлечь факты.
    #[serde(default)]
    pub(crate) return_facts: Option<ReturnFacts>,
    pub(crate) span: crate::parsing::bsl::ast::Span,
}

/// Атом выражения, достаточный для межпроцедурного вывода return-типа.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum ReturnAtom {
    /// Известный тип (в строковом виде, например "Строка", "Число", "СправочникСсылка.Номенклатура").
    Known(String),
    /// Ссылка на переменную (case-sensitive, как в исходнике; нормализация на этапе inference).
    Var(String),
    /// Результат вызова функции/метода (таргет для построения графа вызовов).
    Call(CallTarget),
    /// Не удалось извлечь полезную информацию (динамика/неподдерживаемая конструкция).
    Unknown,
}

/// Набор фактов о теле функции, необходимый для межпроцедурного вывода return-типа.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ReturnFacts {
    /// Атомы значений, встреченных в `Возврат ...`.
    ///
    /// `Возврат;` отражается флагом `has_return_without_value`.
    pub(crate) returns: Vec<ReturnAtom>,
    /// Снимок значений переменных, которые фигурируют в `returns` как `Var`.
    ///
    /// Хранит только подмножество переменных, чтобы не раздувать кэш.
    #[serde(default)]
    pub(crate) vars: HashMap<String, Vec<ReturnAtom>>,
    /// Были ли return'ы без значения (`Возврат;`) в функции.
    #[serde(default)]
    pub(crate) has_return_without_value: bool,
    /// Есть ли в собранных фактах явная неопределённость (`Unknown`).
    #[serde(default)]
    pub(crate) has_dynamic: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ParsedParam {
    pub(crate) name: String,
    pub(crate) is_optional: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ParsedModuleData {
    pub(crate) decls: Vec<ParsedDecl>,
    pub(crate) call_sites: Vec<CallSite>,
}

#[derive(Debug, Clone)]
pub(crate) struct ParsedModule {
    pub(crate) owner_type_name: String,
    pub(crate) module_type: ModuleType,
    pub(crate) is_global_common_module: bool,
    pub(crate) module_path: PathBuf,
    pub(crate) decls: Vec<ParsedDecl>,
    pub(crate) call_sites: Vec<CallSite>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum CallTarget {
    /// Невозможно различить между "локальной" функцией и глобальной (например, из Global common module),
    /// поэтому этот таргет резолвим только в рамках текущего модуля.
    LocalFunction { name: String },
    /// Вызов вида `ModuleName.Method(...)` или `Справочники.Номенклатура.Метод(...)`
    QualifiedMethod { receiver: Vec<String>, name: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct CallSite {
    pub(crate) target: CallTarget,
    pub(crate) arg_types: Vec<Option<String>>,
}
