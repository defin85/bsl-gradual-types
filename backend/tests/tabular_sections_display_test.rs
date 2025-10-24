// Тест для проверки отображения табличных частей

use bsl_shared::api::dtos::{TypeDto, TabularSectionDto, TabularSectionAttributeDto, MethodDto};

#[test]
fn test_create_type_with_tabular_sections() {
    // Создаём тестовый документ с табличными частями
    let test_document = TypeDto {
        id: "doc-agreement".to_string(),
        name: "ДокументОбъект.Договор".to_string(),
        category: "Configuration".to_string(),
        certainty: 100,
        certainty_text: "Known (100%)".to_string(),
        facets: vec!["Object".to_string()],
        methods_count: Some(3),
        methods: vec![
            MethodDto {
                name: "Записать".to_string(),
                english_name: Some("Write".to_string()),
                return_type: None,
                params: vec![],
                description: None,
                is_deprecated: false,
                is_constructor: false,
            },
            MethodDto {
                name: "Провести".to_string(),
                english_name: Some("Post".to_string()),
                return_type: None,
                params: vec![],
                description: None,
                is_deprecated: false,
                is_constructor: false,
            },
            MethodDto {
                name: "ОтменитьПроведение".to_string(),
                english_name: Some("UndoPosting".to_string()),
                return_type: None,
                params: vec![],
                description: None,
                is_deprecated: false,
                is_constructor: false,
            },
        ],
        attributes_count: Some(3),
        properties: vec![
            "Номер".to_string(),
            "Дата".to_string(),
            "Контрагент".to_string(),
        ],
        enum_values: None,
        tabular_sections: vec![
            TabularSectionDto {
                name: "Работы".to_string(),
                attributes: vec![
                    TabularSectionAttributeDto {
                        name: "ВидРаботы".to_string(),
                        attr_type: Some("ПланВидовХарактеристикСсылка.ВидыРабот".to_string()),
                    },
                    TabularSectionAttributeDto {
                        name: "Количество".to_string(),
                        attr_type: Some("xs:decimal".to_string()),
                    },
                    TabularSectionAttributeDto {
                        name: "Стоимость".to_string(),
                        attr_type: Some("xs:decimal".to_string()),
                    },
                ],
            },
            TabularSectionDto {
                name: "Стороны".to_string(),
                attributes: vec![
                    TabularSectionAttributeDto {
                        name: "Сторона".to_string(),
                        attr_type: Some("СправочникСсылка.Контрагенты".to_string()),
                    },
                    TabularSectionAttributeDto {
                        name: "Роль".to_string(),
                        attr_type: Some("ПеречислениеСсылка.РолиСторонДоговора".to_string()),
                    },
                ],
            },
            TabularSectionDto {
                name: "Материалы".to_string(),
                attributes: vec![
                    TabularSectionAttributeDto {
                        name: "Номенклатура".to_string(),
                        attr_type: Some("СправочникСсылка.Номенклатура".to_string()),
                    },
                    TabularSectionAttributeDto {
                        name: "Количество".to_string(),
                        attr_type: Some("xs:decimal".to_string()),
                    },
                    TabularSectionAttributeDto {
                        name: "Цена".to_string(),
                        attr_type: Some("xs:decimal".to_string()),
                    },
                    TabularSectionAttributeDto {
                        name: "Сумма".to_string(),
                        attr_type: Some("xs:decimal".to_string()),
                    },
                ],
            },
        ],
        source: "Configuration".to_string(),
        flow_sensitive: false,
        description: "Документ договора с табличными частями".to_string(),
        union_types: None,
        flow_analysis: None,
        connections: None,
        warning: None,
        recommendation: None,
    };

    // Проверяем, что табличные части созданы правильно
    assert_eq!(test_document.tabular_sections.len(), 3);
    assert_eq!(test_document.tabular_sections[0].name, "Работы");
    assert_eq!(test_document.tabular_sections[0].attributes.len(), 3);
    assert_eq!(test_document.tabular_sections[1].name, "Стороны");
    assert_eq!(test_document.tabular_sections[1].attributes.len(), 2);
    assert_eq!(test_document.tabular_sections[2].name, "Материалы");
    assert_eq!(test_document.tabular_sections[2].attributes.len(), 4);

    println!("✅ Тестовый документ с табличными частями создан успешно:");
    println!("   - Документ: {}", test_document.name);
    println!("   - Табличных частей: {}", test_document.tabular_sections.len());
    for ts in &test_document.tabular_sections {
        println!("     • {} ({} атрибутов)", ts.name, ts.attributes.len());
    }
}

#[test]
fn test_catalog_with_tabular_sections() {
    // Создаём тестовый справочник с табличной частью
    let test_catalog = TypeDto {
        id: "cat-counterparty".to_string(),
        name: "СправочникОбъект.Контрагенты".to_string(),
        category: "Configuration".to_string(),
        certainty: 100,
        certainty_text: "Known (100%)".to_string(),
        facets: vec!["Object".to_string()],
        methods_count: Some(1),
        methods: vec![
            MethodDto {
                name: "Записать".to_string(),
                english_name: Some("Write".to_string()),
                return_type: None,
                params: vec![],
                description: None,
                is_deprecated: false,
                is_constructor: false,
            },
        ],
        attributes_count: Some(5),
        properties: vec![
            "Наименование".to_string(),
            "ИНН".to_string(),
            "КПП".to_string(),
        ],
        enum_values: None,
        tabular_sections: vec![
            TabularSectionDto {
                name: "КонтактнаяИнформация".to_string(),
                attributes: vec![
                    TabularSectionAttributeDto {
                        name: "Тип".to_string(),
                        attr_type: Some("ПеречислениеСсылка.ТипыКонтактнойИнформации".to_string()),
                    },
                    TabularSectionAttributeDto {
                        name: "Представление".to_string(),
                        attr_type: Some("xs:string".to_string()),
                    },
                    TabularSectionAttributeDto {
                        name: "Значение".to_string(),
                        attr_type: Some("xs:string".to_string()),
                    },
                ],
            },
            TabularSectionDto {
                name: "БанковскиеСчета".to_string(),
                attributes: vec![
                    TabularSectionAttributeDto {
                        name: "Банк".to_string(),
                        attr_type: Some("СправочникСсылка.Банки".to_string()),
                    },
                    TabularSectionAttributeDto {
                        name: "НомерСчета".to_string(),
                        attr_type: Some("xs:string".to_string()),
                    },
                ],
            },
        ],
        source: "Configuration".to_string(),
        flow_sensitive: false,
        description: "Справочник контрагентов с контактной информацией и банковскими счетами".to_string(),
        union_types: None,
        flow_analysis: None,
        connections: None,
        warning: None,
        recommendation: None,
    };

    // Проверяем
    assert_eq!(test_catalog.tabular_sections.len(), 2);
    assert_eq!(test_catalog.tabular_sections[0].name, "КонтактнаяИнформация");
    assert_eq!(test_catalog.tabular_sections[1].name, "БанковскиеСчета");

    println!("✅ Тестовый справочник с табличными частями создан успешно:");
    println!("   - Справочник: {}", test_catalog.name);
    println!("   - Табличных частей: {}", test_catalog.tabular_sections.len());
}
