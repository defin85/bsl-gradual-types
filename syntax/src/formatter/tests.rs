use super::*;

#[test]
fn format_reindents_and_trims() {
    let src = "Процедура Тест()\n  Если Истина Тогда  \nСообщить(\"ok\");\n   Иначе\nСообщить(\"no\");   \nКонецЕсли;\nКонецПроцедуры";
    let out = format_document(src, &FormatOptions::default()).expect("format");
    let expected = "Процедура Тест()\n    Если Истина Тогда\n        Сообщить(\"ok\");\n    Иначе\n        Сообщить(\"no\");\n    КонецЕсли;\nКонецПроцедуры\n";
    assert_eq!(out, expected);
}

#[test]
fn format_handles_preprocessor_blocks() {
    let src = "#Если Истина Тогда\nСообщить(1);\n#Иначе\nСообщить(2);\n#КонецЕсли";
    let out = format_document(src, &FormatOptions::default()).expect("format");
    let expected = "#Если Истина Тогда\n    Сообщить(1);\n#Иначе\n    Сообщить(2);\n#КонецЕсли\n";
    assert_eq!(out, expected);
}
