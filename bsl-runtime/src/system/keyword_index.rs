//! KeywordIndex: источники и сборка элементов

use std::collections::BTreeSet;

use crate::system::intellisense_index::{IndexItem, IndexItemKind, IndexKind};

pub const DEFAULT_KEYWORDS: &[&str] = &[
    "Если",
    "If",
    "Тогда",
    "Then",
    "Иначе",
    "Else",
    "ИначеЕсли",
    "ElsIf",
    "КонецЕсли",
    "EndIf",
    "Пока",
    "While",
    "Для",
    "For",
    "Каждого",
    "Each",
    "Из",
    "In",
    "По",
    "To",
    "Цикл",
    "Do",
    "КонецЦикла",
    "EndDo",
    "Процедура",
    "Procedure",
    "Функция",
    "Function",
    "КонецПроцедуры",
    "EndProcedure",
    "КонецФункции",
    "EndFunction",
    "Возврат",
    "Return",
    "Прервать",
    "Break",
    "Продолжить",
    "Continue",
    "Попытка",
    "Try",
    "Исключение",
    "Except",
    "КонецПопытки",
    "EndTry",
    "ВызватьИсключение",
    "Raise",
    "Новый",
    "New",
    "Выполнить",
    "Execute",
    "Перем",
    "Var",
    "Экспорт",
    "Export",
    "#Если",
    "#Иначе",
    "#КонецЕсли",
    "#Область",
    "#КонецОбласти",
    "&НаСервере",
    "&AtServer",
    "&НаКлиенте",
    "&AtClient",
    "&НаСервереБезКонтекста",
    "&AtServerNoContext",
    "&НаКлиентеНаСервереБезКонтекста",
    "&AtClientAtServerNoContext",
];

pub fn keyword_items_from_strings(keywords: &[String]) -> Vec<IndexItem> {
    build_keyword_items(keywords.iter().map(|value| value.as_str()))
}

pub fn keyword_items_from_syntax_or_default(keywords: &[String]) -> Vec<IndexItem> {
    if keywords.is_empty() {
        default_keyword_items()
    } else {
        keyword_items_from_strings(keywords)
    }
}

pub fn default_keyword_items() -> Vec<IndexItem> {
    build_keyword_items(DEFAULT_KEYWORDS.iter().copied())
}

fn build_keyword_items<I, S>(keywords: I) -> Vec<IndexItem>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut unique = BTreeSet::new();
    for keyword in keywords {
        let trimmed = keyword.as_ref().trim();
        if !trimmed.is_empty() {
            unique.insert(trimmed.to_string());
        }
    }

    unique
        .into_iter()
        .map(|keyword| IndexItem::new(keyword, IndexItemKind::Keyword, IndexKind::Keyword))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_keywords_include_basics() {
        let items = default_keyword_items();
        let names: std::collections::HashSet<String> =
            items.into_iter().map(|item| item.name).collect();

        assert!(names.contains("Если"));
        assert!(names.contains("КонецЕсли"));
        assert!(names.contains("Процедура"));
        assert!(names.contains("Функция"));
        assert!(names.contains("Перем"));
    }

    #[test]
    fn fallback_to_default_when_empty() {
        let items = keyword_items_from_syntax_or_default(&[]);
        let names: std::collections::HashSet<String> =
            items.into_iter().map(|item| item.name).collect();

        assert!(names.contains("Если"));
        assert!(names.contains("Процедура"));
    }
}
