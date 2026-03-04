use bsl_types::types::MetadataKind;

// Completion items are part of the repository as it's the source of truth for them.
// --- Completion Item Structures ---

/// Элемент автодополнения (совместимый с LSP)
#[derive(Debug, Clone, serde::Serialize)]
pub struct CompletionItem {
    pub label: String,
    pub kind: CompletionKind,
    pub detail: Option<String>,
    pub documentation: Option<String>,
    pub insert_text: Option<String>,
    pub filter_text: Option<String>,
    pub sort_text: Option<String>,
}

impl CompletionItem {
    pub fn new(label: String, kind: CompletionKind) -> Self {
        Self {
            insert_text: Some(label.clone()),
            filter_text: Some(label.clone()),
            sort_text: Some(label.clone()),
            label,
            kind,
            detail: None,
            documentation: None,
        }
    }

    pub fn with_details(
        label: String,
        kind: CompletionKind,
        detail: Option<String>,
        documentation: Option<String>,
    ) -> Self {
        Self {
            insert_text: Some(label.clone()),
            filter_text: Some(label.clone()),
            sort_text: Some(label.clone()),
            label,
            kind,
            detail,
            documentation,
        }
    }
}

/// Тип элемента автодополнения
#[derive(Debug, Clone, Copy, serde::Serialize)]
pub enum CompletionKind {
    Text,
    Method,
    Function,
    Constructor,
    Field,
    Variable,
    Class,
    Interface,
    Module,
    Property,
    Unit,
    Value,
    Enum,
    Keyword,
    Snippet,
    Color,
    File,
    Reference,
    Folder,
    EnumMember,
    Constant,
    Struct,
    Type,
    Event,
    Operator,
    TypeParameter,
    Global,
    Catalog,
    Document,
    MetadataUnknown,
    Report,
    DataProcessor,
    Register,
    InformationRegister,
    AccumulationRegister,
    AccountingRegister,
    CalculationRegister,
    ChartOfAccounts,
    ChartOfCharacteristicTypes,
    ChartOfCalculationTypes,
    BusinessProcess,
    Task,
    ExchangePlan,
    CommonModule,
    Role,
    Subsystem,
    Language,
}

impl CompletionKind {
    pub fn from_metadata_kind(kind: MetadataKind) -> Self {
        match kind {
            MetadataKind::Unknown => CompletionKind::MetadataUnknown,
            MetadataKind::Catalog => CompletionKind::Catalog,
            MetadataKind::Document => CompletionKind::Document,
            MetadataKind::Register => CompletionKind::Register,
            MetadataKind::Report => CompletionKind::Report,
            MetadataKind::DataProcessor => CompletionKind::DataProcessor,
            MetadataKind::Enum => CompletionKind::Enum,
            MetadataKind::ChartOfAccounts => CompletionKind::ChartOfAccounts,
            MetadataKind::ChartOfCharacteristicTypes => CompletionKind::ChartOfCharacteristicTypes,
            MetadataKind::ChartOfCalculationTypes => CompletionKind::ChartOfCalculationTypes,
            MetadataKind::InformationRegister => CompletionKind::InformationRegister,
            MetadataKind::AccumulationRegister => CompletionKind::AccumulationRegister,
            MetadataKind::AccountingRegister => CompletionKind::AccountingRegister,
            MetadataKind::CalculationRegister => CompletionKind::CalculationRegister,
            MetadataKind::BusinessProcess => CompletionKind::BusinessProcess,
            MetadataKind::Task => CompletionKind::Task,
            MetadataKind::ExchangePlan => CompletionKind::ExchangePlan,
            MetadataKind::Constant => CompletionKind::Constant,
            MetadataKind::CommonModule => CompletionKind::CommonModule,
            MetadataKind::Role => CompletionKind::Role,
            MetadataKind::Subsystem => CompletionKind::Subsystem,
            MetadataKind::Language => CompletionKind::Language,
        }
    }

    pub fn metadata_kind(self) -> Option<MetadataKind> {
        Some(match self {
            CompletionKind::Catalog => MetadataKind::Catalog,
            CompletionKind::Document => MetadataKind::Document,
            CompletionKind::MetadataUnknown => MetadataKind::Unknown,
            CompletionKind::Report => MetadataKind::Report,
            CompletionKind::DataProcessor => MetadataKind::DataProcessor,
            CompletionKind::Register => MetadataKind::Register,
            CompletionKind::InformationRegister => MetadataKind::InformationRegister,
            CompletionKind::AccumulationRegister => MetadataKind::AccumulationRegister,
            CompletionKind::AccountingRegister => MetadataKind::AccountingRegister,
            CompletionKind::CalculationRegister => MetadataKind::CalculationRegister,
            CompletionKind::ChartOfAccounts => MetadataKind::ChartOfAccounts,
            CompletionKind::ChartOfCharacteristicTypes => MetadataKind::ChartOfCharacteristicTypes,
            CompletionKind::ChartOfCalculationTypes => MetadataKind::ChartOfCalculationTypes,
            CompletionKind::BusinessProcess => MetadataKind::BusinessProcess,
            CompletionKind::Task => MetadataKind::Task,
            CompletionKind::ExchangePlan => MetadataKind::ExchangePlan,
            CompletionKind::Role => MetadataKind::Role,
            CompletionKind::Subsystem => MetadataKind::Subsystem,
            CompletionKind::Language => MetadataKind::Language,
            CompletionKind::CommonModule => MetadataKind::CommonModule,
            CompletionKind::Enum => MetadataKind::Enum,
            CompletionKind::Constant => MetadataKind::Constant,
            _ => return None,
        })
    }
}
