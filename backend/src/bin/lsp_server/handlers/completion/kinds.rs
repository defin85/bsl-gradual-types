use super::*;

pub(super) fn completion_kind_tag(item: &bsl_shared::domain::CompletionItem) -> &'static str {
    use bsl_shared::domain::CompletionKind::*;

    match item.kind {
        Method => "method",
        Property => "property",
        Function => "function",
        Keyword => "keyword",
        Type | Class | Struct => "type",
        _ => metadata_completion_kind_tag(item).unwrap_or("other"),
    }
}

fn metadata_completion_kind_tag(item: &bsl_shared::domain::CompletionItem) -> Option<&'static str> {
    use bsl_shared::domain::CompletionKind::*;

    let is_metadata_item = match item.kind {
        Catalog
        | Document
        | MetadataUnknown
        | Report
        | DataProcessor
        | Register
        | InformationRegister
        | AccumulationRegister
        | AccountingRegister
        | CalculationRegister
        | ChartOfAccounts
        | ChartOfCharacteristicTypes
        | ChartOfCalculationTypes
        | BusinessProcess
        | Task
        | ExchangePlan
        | CommonModule
        | Role
        | Subsystem
        | Language => true,
        Enum | Constant => item.detail.is_some(),
        _ => false,
    };

    if !is_metadata_item {
        return None;
    }

    Some(match item.kind {
        Catalog => "metadata.catalog",
        Document => "metadata.document",
        MetadataUnknown => "metadata.unknown",
        Report => "metadata.report",
        DataProcessor => "metadata.data_processor",
        Register => "metadata.register",
        InformationRegister => "metadata.information_register",
        AccumulationRegister => "metadata.accumulation_register",
        AccountingRegister => "metadata.accounting_register",
        CalculationRegister => "metadata.calculation_register",
        ChartOfAccounts => "metadata.chart_of_accounts",
        ChartOfCharacteristicTypes => "metadata.chart_of_characteristic_types",
        ChartOfCalculationTypes => "metadata.chart_of_calculation_types",
        BusinessProcess => "metadata.business_process",
        Task => "metadata.task",
        ExchangePlan => "metadata.exchange_plan",
        Constant => "metadata.constant",
        CommonModule => "metadata.common_module",
        Role => "metadata.role",
        Subsystem => "metadata.subsystem",
        Language => "metadata.language",
        Enum => "metadata.enum",
        _ => return None,
    })
}

pub(super) fn map_completion_kind(
    kind: bsl_shared::domain::CompletionKind,
) -> Option<CompletionItemKind> {
    use bsl_shared::domain::CompletionKind::*;
    Some(match kind {
        Method => CompletionItemKind::METHOD,
        Function => CompletionItemKind::FUNCTION,
        Constructor => CompletionItemKind::CONSTRUCTOR,
        Field => CompletionItemKind::FIELD,
        Variable => CompletionItemKind::VARIABLE,
        Class | Type => CompletionItemKind::CLASS,
        Interface => CompletionItemKind::INTERFACE,
        Module | Global => CompletionItemKind::MODULE,
        Property => CompletionItemKind::PROPERTY,
        Unit => CompletionItemKind::UNIT,
        Value => CompletionItemKind::VALUE,
        Enum => CompletionItemKind::ENUM,
        EnumMember => CompletionItemKind::ENUM_MEMBER,
        Keyword => CompletionItemKind::KEYWORD,
        Snippet => CompletionItemKind::SNIPPET,
        Color => CompletionItemKind::COLOR,
        File => CompletionItemKind::FILE,
        Reference => CompletionItemKind::REFERENCE,
        Folder => CompletionItemKind::FOLDER,
        Constant => CompletionItemKind::CONSTANT,
        Struct => CompletionItemKind::STRUCT,
        Event => CompletionItemKind::EVENT,
        Operator => CompletionItemKind::OPERATOR,
        TypeParameter => CompletionItemKind::TYPE_PARAMETER,
        Text => CompletionItemKind::TEXT,
        Catalog => CompletionItemKind::CLASS,
        Document => CompletionItemKind::FILE,
        MetadataUnknown => CompletionItemKind::TEXT,
        Report => CompletionItemKind::SNIPPET,
        DataProcessor => CompletionItemKind::CONSTRUCTOR,
        Register => CompletionItemKind::STRUCT,
        InformationRegister => CompletionItemKind::EVENT,
        AccumulationRegister => CompletionItemKind::UNIT,
        AccountingRegister => CompletionItemKind::VALUE,
        CalculationRegister => CompletionItemKind::OPERATOR,
        ChartOfAccounts => CompletionItemKind::ENUM_MEMBER,
        ChartOfCharacteristicTypes => CompletionItemKind::TYPE_PARAMETER,
        ChartOfCalculationTypes => CompletionItemKind::INTERFACE,
        BusinessProcess => CompletionItemKind::FIELD,
        Task => CompletionItemKind::PROPERTY,
        ExchangePlan => CompletionItemKind::REFERENCE,
        CommonModule => CompletionItemKind::MODULE,
        Role => CompletionItemKind::COLOR,
        Subsystem => CompletionItemKind::FOLDER,
        Language => CompletionItemKind::KEYWORD,
    })
}
