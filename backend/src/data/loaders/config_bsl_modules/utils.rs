use bsl_shared::domain::types::{FacetKind, MetadataKind};

pub(crate) fn normalize_union_parts(mut parts: Vec<String>) -> Vec<String> {
    parts.sort_by_key(|p| p.to_lowercase());
    parts.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
    parts
}

pub(crate) fn split_union_string(s: &str) -> Vec<String> {
    s.split('|')
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        .map(|p| p.to_string())
        .collect()
}

pub(crate) fn resolve_manager_owner_type_from_receiver(receiver: &[String]) -> Option<String> {
    // receiver: ["Справочники", "Контрагенты"] или ["Catalogs", "Контрагенты"]
    if receiver.len() != 2 {
        return None;
    }
    let kind = collection_name_to_metadata_kind(&receiver[0])?;
    let prefix = kind.faceted_type_prefix(&FacetKind::Manager);
    Some(format!("{}.{}", prefix, receiver[1]))
}

fn collection_name_to_metadata_kind(name: &str) -> Option<MetadataKind> {
    // Локальная таблица (чтобы не тянуть зависимость data layer -> application).
    match name {
        "Справочники" | "Catalogs" => Some(MetadataKind::Catalog),
        "Документы" | "Documents" => Some(MetadataKind::Document),
        "Перечисления" | "Enums" => Some(MetadataKind::Enum),
        "РегистрыСведений" | "InformationRegisters" => {
            Some(MetadataKind::InformationRegister)
        }
        "РегистрыНакопления" | "AccumulationRegisters" => {
            Some(MetadataKind::AccumulationRegister)
        }
        "РегистрыБухгалтерии" | "AccountingRegisters" => {
            Some(MetadataKind::AccountingRegister)
        }
        "РегистрыРасчета" | "CalculationRegisters" => {
            Some(MetadataKind::CalculationRegister)
        }
        "Отчеты" | "Reports" => Some(MetadataKind::Report),
        "Обработки" | "DataProcessors" => Some(MetadataKind::DataProcessor),
        "ПланыСчетов" | "ChartsOfAccounts" => Some(MetadataKind::ChartOfAccounts),
        "ПланыВидовХарактеристик" | "ChartsOfCharacteristicTypes" => {
            Some(MetadataKind::ChartOfCharacteristicTypes)
        }
        "ПланыВидовРасчета" | "ChartsOfCalculationTypes" => {
            Some(MetadataKind::ChartOfCalculationTypes)
        }
        "БизнесПроцессы" | "BusinessProcesses" => Some(MetadataKind::BusinessProcess),
        "Задачи" | "Tasks" => Some(MetadataKind::Task),
        _ => None,
    }
}
