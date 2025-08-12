//! Лексический анализатор BSL

use nom::{
    IResult,
    branch::alt,
    bytes::complete::{tag, take_while, take_while1},
    character::complete::{char, multispace0, digit1},
    combinator::{opt, recognize, value},
    multi::many0,
    sequence::{preceded, tuple},
};

/// Токены BSL
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Ключевые слова
    If,
    Then,
    ElseIf,
    Else,
    EndIf,
    For,
    To,
    Do,
    EndDo,
    While,
    ForEach,
    In,
    Procedure,
    EndProcedure,
    Function,
    EndFunction,
    Return,
    Var,
    Export,
    Val,
    New,
    Try,
    Except,
    EndTry,
    Raise,
    Break,
    Continue,
    And,
    Or,
    Not,
    True,
    False,
    Undefined,
    Null,
    
    // Идентификаторы и литералы
    Identifier(String),
    Number(f64),
    String(String),
    Date(String),
    
    // Операторы
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Equal,
    NotEqual,
    Less,
    LessOrEqual,
    Greater,
    GreaterOrEqual,
    Assign,
    
    // Разделители
    LeftParen,
    RightParen,
    LeftBracket,
    RightBracket,
    Comma,
    Semicolon,
    Dot,
    Question,
    
    // Специальные
    Newline,
    Eof,
}

/// Проверка, является ли символ началом идентификатора
fn is_identifier_start(c: char) -> bool {
    c.is_alphabetic() || c == '_' || (c >= 'а' && c <= 'я') || (c >= 'А' && c <= 'Я')
}

/// Проверка, является ли символ частью идентификатора
fn is_identifier_continue(c: char) -> bool {
    is_identifier_start(c) || c.is_numeric()
}

/// Парсинг идентификатора или ключевого слова
pub fn identifier(input: &str) -> IResult<&str, Token> {
    let (input, ident) = recognize(
        tuple((
            take_while1(is_identifier_start),
            take_while(is_identifier_continue),
        ))
    )(input)?;
    
    // Проверяем, является ли это ключевым словом
    let token = match ident.to_lowercase().as_str() {
        "если" | "if" => Token::If,
        "тогда" | "then" => Token::Then,
        "иначеесли" | "elseif" => Token::ElseIf,
        "иначе" | "else" => Token::Else,
        "конецесли" | "endif" => Token::EndIf,
        "для" | "for" => Token::For,
        "по" | "to" => Token::To,
        "цикл" | "do" => Token::Do,
        "конеццикла" | "enddo" => Token::EndDo,
        "пока" | "while" => Token::While,
        "каждого" | "each" => Token::ForEach,
        "из" | "in" => Token::In,
        "процедура" | "procedure" => Token::Procedure,
        "конецпроцедуры" | "endprocedure" => Token::EndProcedure,
        "функция" | "function" => Token::Function,
        "конецфункции" | "endfunction" => Token::EndFunction,
        "возврат" | "return" => Token::Return,
        "перем" | "var" => Token::Var,
        "экспорт" | "export" => Token::Export,
        "знач" | "val" => Token::Val,
        "новый" | "new" => Token::New,
        "попытка" | "try" => Token::Try,
        "исключение" | "except" => Token::Except,
        "конецпопытки" | "endtry" => Token::EndTry,
        "вызватьисключение" | "raise" => Token::Raise,
        "прервать" | "break" => Token::Break,
        "продолжить" | "continue" => Token::Continue,
        "и" | "and" => Token::And,
        "или" | "or" => Token::Or,
        "не" | "not" => Token::Not,
        "истина" | "true" => Token::True,
        "ложь" | "false" => Token::False,
        "неопределено" | "undefined" => Token::Undefined,
        "null" => Token::Null,
        _ => Token::Identifier(ident.to_string()),
    };
    
    Ok((input, token))
}

/// Парсинг числа
pub fn number(input: &str) -> IResult<&str, Token> {
    let (input, num_str) = recognize(
        tuple((
            opt(char('-')),
            digit1,
            opt(tuple((char('.'), digit1))),
        ))
    )(input)?;
    
    let num = num_str.parse::<f64>().unwrap_or(0.0);
    Ok((input, Token::Number(num)))
}

/// Парсинг строки в кавычках
pub fn string_literal(input: &str) -> IResult<&str, Token> {
    let (input, _) = char('"')(input)?;
    let (input, content) = take_while(|c| c != '"')(input)?;
    let (input, _) = char('"')(input)?;
    
    // Обработка экранированных кавычек
    let content = content.replace("\"\"", "\"");
    
    Ok((input, Token::String(content)))
}

/// Парсинг даты
pub fn date_literal(input: &str) -> IResult<&str, Token> {
    let (input, _) = char('\'')(input)?;
    let (input, content) = take_while(|c| c != '\'')(input)?;
    let (input, _) = char('\'')(input)?;
    
    Ok((input, Token::Date(content.to_string())))
}

/// Парсинг оператора
pub fn operator(input: &str) -> IResult<&str, Token> {
    alt((
        value(Token::LessOrEqual, tag("<=")),
        value(Token::GreaterOrEqual, tag(">=")),
        value(Token::NotEqual, tag("<>")),
        value(Token::Assign, char('=')),
        value(Token::Less, char('<')),
        value(Token::Greater, char('>')),
        value(Token::Plus, char('+')),
        value(Token::Minus, char('-')),
        value(Token::Star, char('*')),
        value(Token::Slash, char('/')),
        value(Token::Percent, char('%')),
    ))(input)
}

/// Парсинг разделителя
pub fn delimiter(input: &str) -> IResult<&str, Token> {
    alt((
        value(Token::LeftParen, char('(')),
        value(Token::RightParen, char(')')),
        value(Token::LeftBracket, char('[')),
        value(Token::RightBracket, char(']')),
        value(Token::Comma, char(',')),
        value(Token::Semicolon, char(';')),
        value(Token::Dot, char('.')),
        value(Token::Question, char('?')),
    ))(input)
}

/// Пропуск комментариев
pub fn comment(input: &str) -> IResult<&str, ()> {
    value(
        (),
        preceded(
            tag("//"),
            take_while(|c| c != '\n'),
        )
    )(input)
}

/// Парсинг одного токена
pub fn token(input: &str) -> IResult<&str, Token> {
    // Пропускаем пробелы и комментарии
    let (input, _) = multispace0(input)?;
    let (input, _) = opt(comment)(input)?;
    let (input, _) = multispace0(input)?;
    
    alt((
        date_literal,
        string_literal,
        number,
        identifier,
        operator,
        delimiter,
    ))(input)
}

/// Токенизация всей строки
pub fn tokenize(input: &str) -> IResult<&str, Vec<Token>> {
    many0(token)(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_identifier() {
        assert_eq!(identifier("переменная"), Ok(("", Token::Identifier("переменная".to_string()))));
        assert_eq!(identifier("Если"), Ok(("", Token::If)));
        assert_eq!(identifier("КонецЕсли"), Ok(("", Token::EndIf)));
    }
    
    #[test]
    fn test_number() {
        assert_eq!(number("123"), Ok(("", Token::Number(123.0))));
        assert_eq!(number("123.45"), Ok(("", Token::Number(123.45))));
        assert_eq!(number("-42"), Ok(("", Token::Number(-42.0))));
    }
    
    #[test]
    fn test_string() {
        assert_eq!(string_literal("\"Hello\""), Ok(("", Token::String("Hello".to_string()))));
        assert_eq!(string_literal("\"Привет\""), Ok(("", Token::String("Привет".to_string()))));
    }
    
    #[test]
    fn test_tokenize() {
        let code = "Перем А = 10;";
        let (_, tokens) = tokenize(code).unwrap();
        assert_eq!(tokens, vec![
            Token::Var,
            Token::Identifier("А".to_string()),
            Token::Assign,
            Token::Number(10.0),
            Token::Semicolon,
        ]);
    }
}