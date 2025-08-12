use bsl_gradual_types::query::{parse_query, parse_1c_query, parse_1c_batch_query, preprocess_query};

#[test]
fn test_simple_query() {
    let query = "ВЫБРАТЬ Номер, Дата ИЗ Документ.ПоступлениеТоваровУслуг";
    let result = parse_query(query);
    assert!(result.is_ok());
    let (remaining, ast) = result.unwrap();
    assert_eq!(remaining, "");
    assert_eq!(ast.select_clause.fields.len(), 2);
}

#[test]
fn test_query_with_pipe_symbol() {
    let query = r#"ВЫБРАТЬ
        |   Номер,
        |   Дата
        |ИЗ
        |   Документ.ПоступлениеТоваровУслуг"#;
    
    let result = parse_1c_query(query);
    assert!(result.is_ok());
    let ast = result.unwrap();
    assert_eq!(ast.select_clause.fields.len(), 2);
}

#[test]
fn test_query_with_comments() {
    let query = r#"ВЫБРАТЬ
        |   Номер,    // Комментарий
        |   Дата     // Ещё комментарий
        |ИЗ
        |   Документ.ПоступлениеТоваровУслуг"#;
    
    let preprocessed = preprocess_query(query);
    assert!(!preprocessed.contains("//"));
    
    let result = parse_1c_query(query);
    assert!(result.is_ok());
}

#[test]
fn test_query_with_temp_table() {
    let query = r#"ВЫБРАТЬ
        |   Номер,
        |   Дата
        |ПОМЕСТИТЬ ВременнаяТаблица
        |ИЗ
        |   Документ.ПоступлениеТоваровУслуг"#;
    
    let result = parse_1c_query(query);
    assert!(result.is_ok());
    let ast = result.unwrap();
    assert_eq!(ast.select_clause.into_temp_table, Some("ВременнаяТаблица".to_string()));
}

#[test]
fn test_batch_query() {
    let queries = r#"ВЫБРАТЬ Код ИЗ Справочник.Номенклатура;
                      ВЫБРАТЬ Код ИЗ Справочник.Контрагенты"#;
    
    let result = parse_1c_batch_query(queries);
    assert!(result.is_ok());
    let batch = result.unwrap();
    assert_eq!(batch.queries.len(), 2);
    assert!(!batch.is_connected);
    assert!(batch.can_parallelize());
}

#[test]
fn test_connected_batch_query() {
    let queries = r#"ВЫБРАТЬ
        |   Ссылка
        |ПОМЕСТИТЬ ВТ_Товары
        |ИЗ
        |   Справочник.Номенклатура;
        |
        |ВЫБРАТЬ
        |   *
        |ИЗ
        |   ВТ_Товары"#;
    
    let result = parse_1c_batch_query(queries);
    assert!(result.is_ok());
    let batch = result.unwrap();
    assert_eq!(batch.queries.len(), 2);
    assert!(batch.is_connected);
    assert!(!batch.can_parallelize());
    assert_eq!(batch.temp_tables.get("ВТ_Товары"), Some(&0));
}

#[test]
fn test_query_with_join() {
    let query = r#"ВЫБРАТЬ
        Док.Номер,
        Контр.Наименование
    ИЗ
        Документ.ПоступлениеТоваровУслуг КАК Док
        ЛЕВОЕ СОЕДИНЕНИЕ Справочник.Контрагенты КАК Контр
        ПО Док.Контрагент = Контр.Ссылка"#;
    
    let result = parse_query(query);
    assert!(result.is_ok());
    let (_, ast) = result.unwrap();
    assert!(!ast.from_clause.sources[0].joins.is_empty());
}

#[test]
fn test_query_with_where() {
    let query = r#"ВЫБРАТЬ Номер 
                   ИЗ Документ.ПоступлениеТоваровУслуг 
                   ГДЕ Проведен = ИСТИНА"#;
    
    let result = parse_query(query);
    assert!(result.is_ok());
    let (_, ast) = result.unwrap();
    assert!(ast.where_clause.is_some());
}

#[test]
fn test_query_with_group_by() {
    let query = r#"ВЫБРАТЬ 
        Контрагент,
        СУММА(СуммаДокумента)
    ИЗ 
        Документ.ПоступлениеТоваровУслуг
    СГРУППИРОВАТЬ ПО
        Контрагент"#;
    
    let result = parse_query(query);
    assert!(result.is_ok());
    let (_, ast) = result.unwrap();
    assert!(ast.group_by_clause.is_some());
}