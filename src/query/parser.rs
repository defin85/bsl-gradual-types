use nom::{
    branch::alt,
    bytes::complete::{tag, tag_no_case, take_till, take_while, take_while1},
    character::complete::{char, digit1, multispace0},
    combinator::{map, opt, recognize, value},
    multi::{many0, separated_list0, separated_list1},
    sequence::{delimited, pair, preceded, separated_pair, tuple},
    IResult,
};

use super::ast::*;

/// Предобработка запроса для удаления символов | в начале строк
/// и удаления однострочных комментариев //
pub fn preprocess_query(input: &str) -> String {
    input
        .lines()
        .map(|line| {
            // Удаляем | в начале строки (может быть с пробелами)
            let line = line.trim_start();
            let line = if line.starts_with('|') {
                &line[1..]
            } else {
                line
            };
            
            // Удаляем однострочные комментарии
            if let Some(comment_pos) = line.find("//") {
                &line[..comment_pos]
            } else {
                line
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Парсит несколько запросов, разделённых точкой с запятой
pub fn parse_queries(input: &str) -> IResult<&str, Vec<Query>> {
    separated_list1(
        ws(char(';')),
        parse_query
    )(input)
}

/// Парсит запрос в формате 1С (с символами | и комментариями)
/// Возвращает Result вместо IResult, так как предобработка изменяет входную строку
pub fn parse_1c_query(input: &str) -> Result<Query, String> {
    let preprocessed = preprocess_query(input);
    parse_query(&preprocessed)
        .map(|(_, query)| query)
        .map_err(|e| format!("Ошибка парсинга: {:?}", e))
}

/// Парсит несколько запросов в формате 1С
/// Возвращает Result вместо IResult, так как предобработка изменяет входную строку
pub fn parse_1c_queries(input: &str) -> Result<Vec<Query>, String> {
    let preprocessed = preprocess_query(input);
    parse_queries(&preprocessed)
        .map(|(_, queries)| queries)
        .map_err(|e| format!("Ошибка парсинга: {:?}", e))
}

/// Парсит пакетный запрос в формате 1С и анализирует зависимости
pub fn parse_1c_batch_query(input: &str) -> Result<super::batch::BatchQuery, String> {
    let queries = parse_1c_queries(input)?;
    Ok(super::batch::BatchQuery::from_queries(queries))
}

pub fn parse_query(input: &str) -> IResult<&str, Query> {
    let (input, select_clause) = parse_select_clause(input)?;
    #[cfg(debug_assertions)]
    eprintln!("[DEBUG] After SELECT: remaining = '{}'", input);
    
    let (input, from_clause) = parse_from_clause(input)?;
    #[cfg(debug_assertions)]
    eprintln!("[DEBUG] After FROM: remaining = '{}'", input);
    let (input, where_clause) = opt(parse_where_clause)(input)?;
    let (input, group_by_clause) = opt(parse_group_by_clause)(input)?;
    let (input, having_clause) = opt(parse_having_clause)(input)?;
    let (input, order_by_clause) = opt(parse_order_by_clause)(input)?;
    let (input, totals_clause) = opt(parse_totals_clause)(input)?;
    let (input, union_clause) = opt(parse_union_clause)(input)?;

    Ok((
        input,
        Query {
            select_clause,
            from_clause,
            where_clause,
            group_by_clause,
            having_clause,
            order_by_clause,
            totals_clause,
            union_clause,
        },
    ))
}

fn parse_select_clause(input: &str) -> IResult<&str, SelectClause> {
    let (input, _) = ws(tag_no_case("ВЫБРАТЬ"))(input)?;
    let (input, distinct) = map(opt(ws(tag_no_case("РАЗЛИЧНЫЕ"))), |o| o.is_some())(input)?;
    let (input, allowed) = map(opt(ws(tag_no_case("РАЗРЕШЕННЫЕ"))), |o| o.is_some())(input)?;
    let (input, top) = opt(preceded(
        ws(tag_no_case("ПЕРВЫЕ")),
        map(ws(digit1), |s: &str| s.parse::<usize>().unwrap_or(0)),
    ))(input)?;
    let (input, fields) = separated_list1(ws(char(',')), parse_select_field)(input)?;
    
    // Проверяем наличие ПОМЕСТИТЬ для временной таблицы
    let (input, into_temp_table) = opt(preceded(
        ws(tag_no_case("ПОМЕСТИТЬ")),
        parse_identifier
    ))(input)?;

    Ok((
        input,
        SelectClause {
            distinct,
            top,
            allowed,
            fields,
            into_temp_table: into_temp_table.map(|s| s.to_string()),
        },
    ))
}

fn parse_select_field(input: &str) -> IResult<&str, SelectField> {
    let (input, expression) = parse_expression(input)?;
    let (input, alias) = opt(preceded(
        ws(tag_no_case("КАК")),
        parse_identifier,
    ))(input)?;

    Ok((
        input,
        SelectField {
            expression,
            alias: alias.map(|s| s.to_string()),
        },
    ))
}

fn parse_from_clause(input: &str) -> IResult<&str, FromClause> {
    let (input, _) = ws(tag_no_case("ИЗ"))(input)?;
    #[cfg(debug_assertions)]
    eprintln!("[DEBUG] parse_from_clause after 'ИЗ': input = '{}'", input);
    
    let (input, sources) = separated_list1(ws(char(',')), parse_table_source)(input)?;
    #[cfg(debug_assertions)]
    eprintln!("[DEBUG] parse_from_clause after sources: input = '{}'", input);

    Ok((input, FromClause { sources }))
}

fn parse_table_source(input: &str) -> IResult<&str, TableSource> {
    let (input, table) = parse_table_reference(input)?;
    let (input, alias) = opt(preceded(
        ws(tag_no_case("КАК")),
        parse_identifier,
    ))(input)?;
    let (input, joins) = many0(parse_join)(input)?;

    Ok((
        input,
        TableSource {
            table,
            alias: alias.map(|s| s.to_string()),
            joins,
        },
    ))
}

fn parse_table_reference(input: &str) -> IResult<&str, TableReference> {
    alt((
        parse_subquery_reference,
        parse_virtual_table,  // Виртуальные таблицы должны быть перед простыми регистрами
        parse_catalog_reference,
        parse_document_reference,
        parse_register_reference,
        parse_simple_table,
    ))(input)
}

fn parse_catalog_reference(input: &str) -> IResult<&str, TableReference> {
    map(
        separated_pair(
            ws(tag_no_case("Справочник")),
            ws(char('.')),
            parse_identifier,
        ),
        |(_, name)| TableReference::Catalog("Справочник".to_string(), name.to_string()),
    )(input)
}

fn parse_document_reference(input: &str) -> IResult<&str, TableReference> {
    map(
        separated_pair(
            ws(tag_no_case("Документ")),
            ws(char('.')),
            parse_identifier,
        ),
        |(_, name)| TableReference::Document("Документ".to_string(), name.to_string()),
    )(input)
}

fn parse_register_reference(input: &str) -> IResult<&str, TableReference> {
    map(
        tuple((
            alt((
                tag_no_case("РегистрСведений"),
                tag_no_case("РегистрНакопления"),
                tag_no_case("РегистрБухгалтерии"),
                tag_no_case("РегистрРасчета"),
            )),
            ws(char('.')),
            parse_identifier,
        )),
        |(reg_type, _, name)| TableReference::Register(reg_type.to_string(), name.to_string()),
    )(input)
}

fn parse_virtual_table(input: &str) -> IResult<&str, TableReference> {
    // Парсим виртуальную таблицу регистра: РегистрНакопления.ТоварыНаСкладах.Остатки(...)
    map(
        tuple((
            // Парсим тип регистра + имя напрямую
            tuple((
                alt((
                    tag_no_case("РегистрСведений"),
                    tag_no_case("РегистрНакопления"),
                    tag_no_case("РегистрБухгалтерии"),
                    tag_no_case("РегистрРасчета"),
                )),
                ws(char('.')),
                parse_identifier,
            )),
            ws(char('.')),
            parse_identifier,
            delimited(
                ws(char('(')),
                separated_list0(ws(char(',')), parse_virtual_parameter),
                ws(char(')')),
            ),
        )),
        |((reg_type, _, reg_name), _, vt_name, params)| {
            TableReference::VirtualTable(
                format!("{}.{}", reg_type, reg_name),
                vt_name.to_string(),
                params,
            )
        },
    )(input)
}

fn parse_virtual_parameter(input: &str) -> IResult<&str, VirtualTableParameter> {
    map(
        separated_pair(parse_identifier, ws(char('=')), parse_expression),
        |(name, value)| VirtualTableParameter {
            name: name.to_string(),
            value,
        },
    )(input)
}

fn parse_subquery_reference(input: &str) -> IResult<&str, TableReference> {
    map(
        delimited(
            ws(char('(')),
            parse_query,
            ws(char(')')),
        ),
        |q| TableReference::Subquery(Box::new(q)),
    )(input)
}

fn parse_simple_table(input: &str) -> IResult<&str, TableReference> {
    map(parse_identifier, |name| {
        TableReference::Table(name.to_string())
    })(input)
}

fn parse_join(input: &str) -> IResult<&str, Join> {
    let (input, join_type) = parse_join_type(input)?;
    let (input, _) = ws(tag_no_case("СОЕДИНЕНИЕ"))(input)?;
    let (input, table) = parse_table_source(input)?;
    let (input, condition) = opt(preceded(
        ws(tag_no_case("ПО")),
        parse_expression,
    ))(input)?;

    Ok((
        input,
        Join {
            join_type,
            table,
            condition,
        },
    ))
}

fn parse_join_type(input: &str) -> IResult<&str, JoinType> {
    alt((
        value(JoinType::Left, ws(tag_no_case("ЛЕВОЕ"))),
        value(JoinType::Right, ws(tag_no_case("ПРАВОЕ"))),
        value(JoinType::Full, ws(tag_no_case("ПОЛНОЕ"))),
        value(JoinType::Inner, ws(tag_no_case("ВНУТРЕННЕЕ"))),
    ))(input)
}

fn parse_where_clause(input: &str) -> IResult<&str, WhereClause> {
    map(
        preceded(ws(tag_no_case("ГДЕ")), parse_expression),
        |condition| WhereClause { condition },
    )(input)
}

fn parse_group_by_clause(input: &str) -> IResult<&str, GroupByClause> {
    map(
        preceded(
            ws(tag_no_case("СГРУППИРОВАТЬ ПО")),
            separated_list1(ws(char(',')), parse_expression),
        ),
        |fields| GroupByClause { fields },
    )(input)
}

fn parse_having_clause(input: &str) -> IResult<&str, HavingClause> {
    map(
        preceded(ws(tag_no_case("ИМЕЮЩИЕ")), parse_expression),
        |condition| HavingClause { condition },
    )(input)
}

fn parse_order_by_clause(input: &str) -> IResult<&str, OrderByClause> {
    let (input, _) = ws(tag_no_case("УПОРЯДОЧИТЬ ПО"))(input)?;
    let (input, items) = separated_list1(ws(char(',')), parse_order_by_item)(input)?;
    let (input, auto_order) = map(
        opt(ws(tag_no_case("АВТОУПОРЯДОЧИВАНИЕ"))),
        |o| o.is_some(),
    )(input)?;

    Ok((
        input,
        OrderByClause { items, auto_order },
    ))
}

fn parse_order_by_item(input: &str) -> IResult<&str, OrderByItem> {
    let (input, expression) = parse_expression(input)?;
    let (input, direction) = opt(alt((
        value(OrderDirection::Asc, ws(tag_no_case("ВОЗР"))),
        value(OrderDirection::Desc, ws(tag_no_case("УБЫВ"))),
    )))(input)?;

    Ok((
        input,
        OrderByItem {
            expression,
            direction: direction.unwrap_or(OrderDirection::Asc),
        },
    ))
}

fn parse_totals_clause(input: &str) -> IResult<&str, TotalsClause> {
    let (input, _) = ws(tag_no_case("ИТОГИ"))(input)?;
    let (input, _fields) = separated_list1(ws(char(',')), parse_expression)(input)?;
    let (input, _) = ws(tag_no_case("ПО"))(input)?;
    let (input, overall) = map(opt(ws(tag_no_case("ОБЩИЕ"))), |o| o.is_some())(input)?;
    let (input, by_fields) = separated_list0(
        ws(char(',')),
        parse_identifier,
    )(input)?;

    Ok((
        input,
        TotalsClause {
            overall,
            by_fields: by_fields.iter().map(|s| s.to_string()).collect(),
        },
    ))
}

fn parse_union_clause(input: &str) -> IResult<&str, Vec<Query>> {
    preceded(
        ws(alt((
            tag_no_case("ОБЪЕДИНИТЬ ВСЕ"),
            tag_no_case("ОБЪЕДИНИТЬ"),
        ))),
        separated_list1(
            ws(alt((
                tag_no_case("ОБЪЕДИНИТЬ ВСЕ"),
                tag_no_case("ОБЪЕДИНИТЬ"),
            ))),
            parse_query,
        ),
    )(input)
}

fn parse_expression(input: &str) -> IResult<&str, Expression> {
    parse_or_expression(input)
}

fn parse_or_expression(input: &str) -> IResult<&str, Expression> {
    let (input, left) = parse_and_expression(input)?;
    
    // Проверяем, не является ли следующее слово ключевым словом запроса
    if is_query_keyword_ahead(input) {
        return Ok((input, left));
    }
    
    let (input, exprs) = many0(preceded(ws(tag_no_case("ИЛИ")), parse_and_expression))(input)?;
    Ok((input, exprs.into_iter().fold(left, |acc, expr| {
        Expression::BinaryOp(Box::new(acc), BinaryOperator::Or, Box::new(expr))
    })))
}

fn parse_and_expression(input: &str) -> IResult<&str, Expression> {
    let (input, left) = parse_not_expression(input)?;
    
    // Проверяем, не является ли следующее слово ключевым словом запроса
    if is_query_keyword_ahead(input) {
        return Ok((input, left));
    }
    
    let (input, exprs) = many0(preceded(
        ws(parse_keyword_and), 
        parse_not_expression
    ))(input)?;
    
    Ok((input, exprs.into_iter().fold(left, |acc, expr| {
        Expression::BinaryOp(Box::new(acc), BinaryOperator::And, Box::new(expr))
    })))
}

fn parse_not_expression(input: &str) -> IResult<&str, Expression> {
    alt((
        map(
            preceded(ws(tag_no_case("НЕ")), parse_not_expression),
            |expr| Expression::UnaryOp(UnaryOperator::Not, Box::new(expr)),
        ),
        parse_comparison_expression,
    ))(input)
}

fn parse_comparison_expression(input: &str) -> IResult<&str, Expression> {
    let (input, left) = parse_additive_expression(input)?;
    
    // Try BETWEEN
    if let Ok((rest, _)) = ws(tag_no_case("МЕЖДУ"))(input) {
        let (rest, lower) = parse_additive_expression(rest)?;
        let (rest, _) = ws(tag_no_case("И"))(rest)?;
        let (rest, upper) = parse_additive_expression(rest)?;
        return Ok((rest, Expression::Between(Box::new(left), Box::new(lower), Box::new(upper))));
    }
    
    // Try IN - проверяем, что "В" - это отдельное слово
    if let Ok((rest, _)) = parse_keyword_in(input) {
        let (rest, list) = delimited(
            ws(char('(')),
            separated_list1(ws(char(',')), parse_expression),
            ws(char(')')),
        )(rest)?;
        return Ok((rest, Expression::In(Box::new(left), list)));
    }
    
    // Try comparison operators
    if let Ok((rest, op)) = alt((
        value(BinaryOperator::Equal, ws(char('='))),
        value(BinaryOperator::NotEqual, ws(tag("<>"))),
        value(BinaryOperator::LessOrEqual, ws(tag("<="))),
        value(BinaryOperator::GreaterOrEqual, ws(tag(">="))),
        value(BinaryOperator::Less, ws(char('<'))),
        value(BinaryOperator::Greater, ws(char('>'))),
        value(BinaryOperator::Like, ws(tag_no_case("ПОДОБНО"))),
    ))(input) {
        let (rest, right) = parse_additive_expression(rest)?;
        return Ok((rest, Expression::BinaryOp(Box::new(left), op, Box::new(right))));
    }
    
    Ok((input, left))
}

fn parse_additive_expression(input: &str) -> IResult<&str, Expression> {
    let (input, left) = parse_multiplicative_expression(input)?;
    let (input, ops) = many0(tuple((
        alt((
            value(BinaryOperator::Add, ws(char('+'))),
            value(BinaryOperator::Subtract, ws(char('-'))),
        )),
        parse_multiplicative_expression,
    )))(input)?;
    
    Ok((input, ops.into_iter().fold(left, |acc, (op, right)| {
        Expression::BinaryOp(Box::new(acc), op, Box::new(right))
    })))
}

fn parse_multiplicative_expression(input: &str) -> IResult<&str, Expression> {
    let (input, left) = parse_unary_expression(input)?;
    let (input, ops) = many0(tuple((
        alt((
            value(BinaryOperator::Multiply, ws(char('*'))),
            value(BinaryOperator::Divide, ws(char('/'))),
        )),
        parse_unary_expression,
    )))(input)?;
    
    Ok((input, ops.into_iter().fold(left, |acc, (op, right)| {
        Expression::BinaryOp(Box::new(acc), op, Box::new(right))
    })))
}

fn parse_unary_expression(input: &str) -> IResult<&str, Expression> {
    alt((
        map(
            preceded(ws(char('-')), parse_unary_expression),
            |expr| Expression::UnaryOp(UnaryOperator::Minus, Box::new(expr)),
        ),
        parse_postfix_expression,
    ))(input)
}

fn parse_postfix_expression(input: &str) -> IResult<&str, Expression> {
    let (input, expr) = parse_primary_expression(input)?;
    
    let (input, fields) = many0(preceded(ws(char('.')), parse_identifier))(input)?;
    
    let expr = fields.into_iter().fold(expr, |acc, field| {
        if let Expression::Field(table) = acc {
            Expression::QualifiedField(table, field.to_string())
        } else {
            acc
        }
    });

    Ok((input, expr))
}

fn parse_primary_expression(input: &str) -> IResult<&str, Expression> {
    alt((
        parse_case_expression,
        parse_cast_expression,
        parse_function_call,
        parse_parameter,
        parse_literal,
        parse_subquery_expression,
        delimited(ws(char('(')), parse_expression, ws(char(')'))),
        map(parse_identifier, |id| Expression::Field(id.to_string())),
    ))(input)
}

fn parse_case_expression(input: &str) -> IResult<&str, Expression> {
    map(
        tuple((
            ws(tag_no_case("ВЫБОР")),
            many0(parse_when_clause),
            opt(preceded(ws(tag_no_case("ИНАЧЕ")), parse_expression)),
            ws(tag_no_case("КОНЕЦ")),
        )),
        |(_, when_clauses, else_clause, _)| {
            Expression::Case(CaseExpression {
                when_clauses,
                else_clause: else_clause.map(Box::new),
            })
        },
    )(input)
}

fn parse_when_clause(input: &str) -> IResult<&str, WhenClause> {
    map(
        tuple((
            ws(tag_no_case("КОГДА")),
            parse_expression,
            ws(tag_no_case("ТОГДА")),
            parse_expression,
        )),
        |(_, condition, _, result)| WhenClause { condition, result },
    )(input)
}

fn parse_cast_expression(input: &str) -> IResult<&str, Expression> {
    map(
        tuple((
            ws(tag_no_case("ВЫРАЗИТЬ")),
            ws(char('(')),
            parse_expression,
            ws(tag_no_case("КАК")),
            parse_data_type,
            ws(char(')')),
        )),
        |(_, _, expr, _, dtype, _)| Expression::Cast(Box::new(expr), dtype),
    )(input)
}

fn parse_data_type(input: &str) -> IResult<&str, DataType> {
    alt((
        map(
            tuple((
                ws(tag_no_case("ЧИСЛО")),
                opt(delimited(
                    ws(char('(')),
                    separated_pair(
                        map(digit1, |s: &str| s.parse().ok()),
                        ws(char(',')),
                        map(digit1, |s: &str| s.parse().ok()),
                    ),
                    ws(char(')')),
                )),
            )),
            |(_, params)| {
                if let Some((p, s)) = params {
                    DataType::Number(p, s)
                } else {
                    DataType::Number(None, None)
                }
            },
        ),
        map(
            tuple((
                ws(tag_no_case("СТРОКА")),
                opt(delimited(
                    ws(char('(')),
                    map(digit1, |s: &str| s.parse().ok()),
                    ws(char(')')),
                )),
            )),
            |(_, len)| DataType::String(len.flatten()),
        ),
        value(DataType::Date, ws(tag_no_case("ДАТА"))),
        value(DataType::Boolean, ws(tag_no_case("БУЛЕВО"))),
    ))(input)
}

fn parse_function_call(input: &str) -> IResult<&str, Expression> {
    map(
        tuple((
            parse_identifier,
            ws(char('(')),
            opt(ws(tag_no_case("РАЗЛИЧНЫЕ"))),
            separated_list0(ws(char(',')), parse_expression),
            ws(char(')')),
        )),
        |(name, _, distinct, args, _)| {
            Expression::Function(FunctionCall {
                name: name.to_string(),
                args,
                distinct: distinct.is_some(),
            })
        },
    )(input)
}

fn parse_parameter(input: &str) -> IResult<&str, Expression> {
    map(
        preceded(ws(char('&')), parse_identifier),
        |name| Expression::Parameter(name.to_string()),
    )(input)
}

fn parse_subquery_expression(input: &str) -> IResult<&str, Expression> {
    map(
        delimited(ws(char('(')), parse_query, ws(char(')'))),
        |q| Expression::Subquery(Box::new(q)),
    )(input)
}

fn parse_literal(input: &str) -> IResult<&str, Expression> {
    alt((
        map(parse_string_literal, |s| Expression::Literal(Literal::String(s))),
        map(parse_number_literal, |n| Expression::Literal(Literal::Number(n))),
        map(parse_date_literal, |d| Expression::Literal(Literal::Date(d))),
        value(Expression::Literal(Literal::Boolean(true)), ws(tag_no_case("ИСТИНА"))),
        value(Expression::Literal(Literal::Boolean(false)), ws(tag_no_case("ЛОЖЬ"))),
        value(Expression::Literal(Literal::Null), ws(tag_no_case("NULL"))),
        value(Expression::Literal(Literal::Undefined), ws(tag_no_case("НЕОПРЕДЕЛЕНО"))),
    ))(input)
}

fn parse_string_literal(input: &str) -> IResult<&str, String> {
    delimited(
        char('"'),
        map(
            take_till(|c| c == '"'),
            |s: &str| s.to_string(),
        ),
        char('"'),
    )(input)
}

fn parse_number_literal(input: &str) -> IResult<&str, f64> {
    map(
        recognize(tuple((
            opt(char('-')),
            digit1,
            opt(tuple((char('.'), digit1))),
        ))),
        |s: &str| s.parse().unwrap_or(0.0),
    )(input)
}

fn parse_date_literal(input: &str) -> IResult<&str, String> {
    delimited(
        ws(tag_no_case("ДАТАВРЕМЯ")),
        delimited(
            ws(char('(')),
            map(digit1, |s: &str| s.to_string()),
            ws(char(')')),
        ),
        multispace0,
    )(input)
}

fn parse_identifier(input: &str) -> IResult<&str, &str> {
    recognize(pair(
        take_while1(|c: char| c.is_alphabetic() || c == '_'),
        take_while(|c: char| c.is_alphanumeric() || c == '_'),
    ))(input)
}

fn ws<'a, F, O>(inner: F) -> impl FnMut(&'a str) -> IResult<&'a str, O>
where
    F: FnMut(&'a str) -> IResult<&'a str, O>,
{
    delimited(multispace0, inner, multispace0)
}

// Парсер для ключевого слова "И" как отдельного слова
fn parse_keyword_and(input: &str) -> IResult<&str, &str> {
    let (input, _) = multispace0(input)?;
    let (input, keyword) = tag_no_case("И")(input)?;
    
    // Проверяем, что после "И" идёт пробел или конец строки
    // (т.е. это не часть другого слова как "ИЗ")
    if let Some(next_char) = input.chars().next() {
        if next_char.is_alphabetic() || next_char == '_' {
            return Err(nom::Err::Error(nom::error::Error::new(
                input,
                nom::error::ErrorKind::Tag,
            )));
        }
    }
    
    Ok((input, keyword))
}

// Парсер для ключевого слова "В" (оператор IN) как отдельного слова
fn parse_keyword_in(input: &str) -> IResult<&str, &str> {
    let (input, _) = multispace0(input)?;
    let (input, keyword) = tag_no_case("В")(input)?;
    
    // Проверяем, что после "В" идёт пробел или '('
    // (т.е. это не часть другого слова)
    if let Some(next_char) = input.chars().next() {
        if next_char.is_alphabetic() || next_char == '_' {
            return Err(nom::Err::Error(nom::error::Error::new(
                input,
                nom::error::ErrorKind::Tag,
            )));
        }
    }
    
    Ok((input, keyword))
}

// Проверяет, является ли следующее слово ключевым словом запроса
fn is_query_keyword_ahead(input: &str) -> bool {
    let trimmed = input.trim_start();
    
    // Список ключевых слов, которые должны прерывать парсинг выражений
    let keywords = [
        "ИЗ", "ГДЕ", "СГРУППИРОВАТЬ", "ИМЕЮЩИЕ", "УПОРЯДОЧИТЬ", 
        "ИТОГИ", "ОБЪЕДИНИТЬ", "КАК", "ЛЕВОЕ", "ПРАВОЕ", "ПОЛНОЕ", 
        "ВНУТРЕННЕЕ", "СОЕДИНЕНИЕ", "ПО"
    ];
    
    for keyword in &keywords {
        if let Some(after_keyword) = trimmed.strip_prefix(keyword) {
            // Проверяем, что это отдельное слово
            if after_keyword.is_empty() || 
               !after_keyword.chars().next().unwrap_or(' ').is_alphabetic() {
                return true;
            }
        }
    }
    
    false
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_select() {
        let query = "ВЫБРАТЬ Номер, Дата ИЗ Документ.ПоступлениеТоваровУслуг";
        let result = parse_query(query);
        if let Err(e) = &result {
            eprintln!("Parse error: {:?}", e);
        }
        assert!(result.is_ok());
    }

    #[test]
    fn test_select_with_where() {
        let query = "ВЫБРАТЬ Номер ИЗ Документ.ПоступлениеТоваровУслуг ГДЕ Дата > &НачалоПериода";
        let result = parse_query(query);
        assert!(result.is_ok());
    }

    #[test]
    fn test_select_with_join() {
        let query = "ВЫБРАТЬ 
            Док.Номер,
            Контр.Наименование
        ИЗ 
            Документ.ПоступлениеТоваровУслуг КАК Док
            ЛЕВОЕ СОЕДИНЕНИЕ Справочник.Контрагенты КАК Контр
            ПО Док.Контрагент = Контр.Ссылка";
        let result = parse_query(query);
        assert!(result.is_ok());
    }
}