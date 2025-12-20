use std::path::PathBuf;

use bsl_shared::domain::code_location::ModuleType;
use bsl_shared::domain::signature_index::ContextRequirements;
use bsl_shared::domain::type_definition_location::TypeDefinitionLocation;

#[derive(Debug, Clone)]
pub struct ModuleIndexProgress {
    pub current: usize,
    pub total: usize,
    pub module_path: PathBuf,
}

#[derive(Debug, Default)]
pub struct IndexedConfigSignatures {
    pub config_methods: Vec<(String, bsl_shared::domain::signature_index::MethodSignature)>,
    pub global_functions: Vec<(String, bsl_shared::domain::signature_index::MethodSignature)>,
    pub definition_locations: Vec<(String, String, TypeDefinitionLocation)>,
    pub global_definition_locations: Vec<(String, TypeDefinitionLocation)>,
}

#[derive(Debug)]
pub struct ModuleParseStats {
    pub decls: usize,
    pub export_decls: usize,
    pub call_sites: usize,
}

#[derive(Debug, Clone, Copy)]
pub enum SinglePassMode {
    Lite,
    Full,
}

#[derive(Debug)]
pub struct ModuleParseComparison {
    pub module_path: PathBuf,
    pub single_pass: ModuleParseStats,
    pub ast: ModuleParseStats,
    pub missing_decls: Vec<String>,
    pub extra_decls: Vec<String>,
    pub callsite_mismatches: Vec<String>,
}

#[derive(Debug)]
pub(crate) struct ParsedDecl {
    pub(crate) name: String,
    pub(crate) params: Vec<String>,
    pub(crate) is_export: bool,
    pub(crate) directive_ctx: Option<ContextRequirements>,
    pub(crate) return_type: Option<String>,
    pub(crate) span: crate::parsing::bsl::ast::Span,
}

#[derive(Debug)]
pub(crate) struct ParsedModuleData {
    pub(crate) decls: Vec<ParsedDecl>,
    pub(crate) call_sites: Vec<CallSite>,
}

#[derive(Debug)]
pub(crate) struct ParsedModule {
    pub(crate) owner_type_name: String,
    pub(crate) module_type: ModuleType,
    pub(crate) is_global_common_module: bool,
    pub(crate) module_path: PathBuf,
    pub(crate) decls: Vec<ParsedDecl>,
    pub(crate) call_sites: Vec<CallSite>,
}

#[derive(Debug)]
pub(crate) enum CallTarget {
    /// Невозможно различить между "локальной" функцией и глобальной (например, из Global common module),
    /// поэтому этот таргет резолвим только в рамках текущего модуля.
    LocalFunction { name: String },
    /// Вызов вида `ModuleName.Method(...)` или `Справочники.Номенклатура.Метод(...)`
    QualifiedMethod { receiver: Vec<String>, name: String },
}

#[derive(Debug)]
pub(crate) struct CallSite {
    pub(crate) target: CallTarget,
    pub(crate) arg_types: Vec<Option<String>>,
}
