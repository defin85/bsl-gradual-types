//! Парсер BSL на основе nom

use super::lexer::{Token, tokenize};
use super::ast::*;
use super::common::Parser;
use anyhow::{Result, anyhow};

/// Основной парсер BSL
pub struct BslParser {
    tokens: Option<Vec<Token>>,
    _position: usize,
}

impl BslParser {
    /// Создание нового парсера (для совместимости)
    pub fn new(input: &str) -> Result<Self, String> {
        match tokenize(input) {
            Ok((_, tokens)) => Ok(Self {
                tokens: Some(tokens),
                _position: 0,
            }),
            Err(e) => Err(format!("Tokenization error: {:?}", e)),
        }
    }
    
    /// Парсинг программы (для совместимости со старым API)
    pub fn parse(&mut self) -> Result<Program, String> {
        if let Some(tokens) = self.tokens.take() {
            self.parse_with_tokens(tokens)
        } else {
            Err("Parser already consumed".to_string())
        }
    }
    
    /// Парсинг программы с токенами
    fn parse_with_tokens(&mut self, tokens: Vec<Token>) -> Result<Program, String> {
        let mut inner = InnerParser {
            tokens,
            position: 0,
        };
        let statements = inner.parse_statements()?;
        Ok(Program { statements })
    }
    
}

impl Parser for BslParser {
    fn parse(&mut self, source: &str) -> Result<Program> {
        // Создаём новый парсер для каждого вызова
        match tokenize(source) {
            Ok((_, tokens)) => self.parse_with_tokens(tokens)
                .map_err(|e| anyhow!(e)),
            Err(e) => Err(anyhow!("Tokenization error: {:?}", e)),
        }
    }
    
    fn name(&self) -> &str {
        "nom"
    }
}

/// Внутренний парсер с токенами
struct InnerParser {
    tokens: Vec<Token>,
    position: usize,
}

impl InnerParser {
    /// Парсинг списка операторов
    fn parse_statements(&mut self) -> Result<Vec<Statement>, String> {
        let mut statements = Vec::new();
        
        while self.position < self.tokens.len() {
            if self.check_end_keyword() {
                break;
            }
            
            statements.push(self.parse_statement()?);
            self.skip_semicolons();
        }
        
        Ok(statements)
    }
    
    /// Парсинг одного оператора
    fn parse_statement(&mut self) -> Result<Statement, String> {
        match &self.current_token() {
            Some(Token::Var) => self.parse_var_declaration(),
            Some(Token::Procedure) => self.parse_procedure(),
            Some(Token::Function) => self.parse_function(),
            Some(Token::If) => self.parse_if(),
            Some(Token::For) => self.parse_for(),
            Some(Token::While) => self.parse_while(),
            Some(Token::Return) => self.parse_return(),
            Some(Token::Break) => {
                self.advance();
                Ok(Statement::Break)
            }
            Some(Token::Continue) => {
                self.advance();
                Ok(Statement::Continue)
            }
            Some(Token::Try) => self.parse_try(),
            Some(Token::Raise) => self.parse_raise(),
            Some(Token::Identifier(_)) => self.parse_assignment_or_call(),
            _ => Err(format!("Unexpected token: {:?}", self.current_token())),
        }
    }
    
    /// Парсинг объявления переменной
    fn parse_var_declaration(&mut self) -> Result<Statement, String> {
        self.expect(Token::Var)?;
        
        let name = self.expect_identifier()?;
        
        let export = if self.check(Token::Export) {
            self.advance();
            true
        } else {
            false
        };
        
        let value = if self.check(Token::Assign) {
            self.advance();
            Some(self.parse_expression()?)
        } else {
            None
        };
        
        Ok(Statement::VarDeclaration { name, export, value })
    }
    
    /// Парсинг процедуры
    fn parse_procedure(&mut self) -> Result<Statement, String> {
        self.expect(Token::Procedure)?;
        let name = self.expect_identifier()?;
        
        self.expect(Token::LeftParen)?;
        let params = self.parse_parameters()?;
        self.expect(Token::RightParen)?;
        
        let export = if self.check(Token::Export) {
            self.advance();
            true
        } else {
            false
        };
        
        let body = self.parse_statements()?;
        self.expect(Token::EndProcedure)?;
        
        Ok(Statement::ProcedureDecl {
            name,
            params,
            body,
            export,
        })
    }
    
    /// Парсинг функции
    fn parse_function(&mut self) -> Result<Statement, String> {
        self.expect(Token::Function)?;
        let name = self.expect_identifier()?;
        
        self.expect(Token::LeftParen)?;
        let params = self.parse_parameters()?;
        self.expect(Token::RightParen)?;
        
        let export = if self.check(Token::Export) {
            self.advance();
            true
        } else {
            false
        };
        
        let body = self.parse_statements()?;
        
        // Находим последний оператор возврата в теле функции
        let mut return_value = None;
        for stmt in &body {
            if let Statement::Return(val) = stmt {
                return_value = val.clone();
            }
        }
        
        self.expect(Token::EndFunction)?;
        
        Ok(Statement::FunctionDecl {
            name,
            params,
            body,
            return_value,
            export,
        })
    }
    
    /// Парсинг условного оператора
    fn parse_if(&mut self) -> Result<Statement, String> {
        self.expect(Token::If)?;
        let condition = self.parse_expression()?;
        self.expect(Token::Then)?;
        
        let then_branch = self.parse_statements()?;
        
        let mut else_if_branches = Vec::new();
        while self.check(Token::ElseIf) {
            self.advance();
            let cond = self.parse_expression()?;
            self.expect(Token::Then)?;
            let body = self.parse_statements()?;
            else_if_branches.push((cond, body));
        }
        
        let else_branch = if self.check(Token::Else) {
            self.advance();
            Some(self.parse_statements()?)
        } else {
            None
        };
        
        self.expect(Token::EndIf)?;
        
        Ok(Statement::If {
            condition,
            then_branch,
            else_if_branches,
            else_branch,
        })
    }
    
    /// Парсинг цикла Для
    fn parse_for(&mut self) -> Result<Statement, String> {
        self.expect(Token::For)?;
        
        // Проверяем, это For Each или обычный For
        if self.check_sequence(&[Token::ForEach]) {
            return self.parse_for_each();
        }
        
        let variable = self.expect_identifier()?;
        self.expect(Token::Assign)?;
        let from = self.parse_expression()?;
        self.expect(Token::To)?;
        let to = self.parse_expression()?;
        self.expect(Token::Do)?;
        
        let body = self.parse_statements()?;
        self.expect(Token::EndDo)?;
        
        Ok(Statement::For {
            variable,
            from,
            to,
            step: None, // TODO: Поддержка шага в nom-парсере
            body,
        })
    }
    
    /// Парсинг цикла Для Каждого
    fn parse_for_each(&mut self) -> Result<Statement, String> {
        self.expect(Token::ForEach)?;
        let variable = self.expect_identifier()?;
        self.expect(Token::In)?;
        let collection = self.parse_expression()?;
        self.expect(Token::Do)?;
        
        let body = self.parse_statements()?;
        self.expect(Token::EndDo)?;
        
        Ok(Statement::ForEach {
            variable,
            collection,
            body,
        })
    }
    
    /// Парсинг цикла Пока
    fn parse_while(&mut self) -> Result<Statement, String> {
        self.expect(Token::While)?;
        let condition = self.parse_expression()?;
        self.expect(Token::Do)?;
        
        let body = self.parse_statements()?;
        self.expect(Token::EndDo)?;
        
        Ok(Statement::While { condition, body })
    }
    
    /// Парсинг возврата
    fn parse_return(&mut self) -> Result<Statement, String> {
        self.expect(Token::Return)?;
        
        let value = if !self.check_statement_end() {
            Some(self.parse_expression()?)
        } else {
            None
        };
        
        Ok(Statement::Return(value))
    }
    
    /// Парсинг блока Попытка-Исключение
    fn parse_try(&mut self) -> Result<Statement, String> {
        self.expect(Token::Try)?;
        let try_block = self.parse_statements()?;
        
        let catch_block = if self.check(Token::Except) {
            self.advance();
            Some(self.parse_statements()?)
        } else {
            None
        };
        
        self.expect(Token::EndTry)?;
        
        Ok(Statement::Try {
            try_block,
            catch_block,
        })
    }
    
    /// Парсинг вызова исключения
    fn parse_raise(&mut self) -> Result<Statement, String> {
        self.expect(Token::Raise)?;
        
        let message = if let Some(Token::String(s)) = self.current_token() {
            let msg = s.clone();
            self.advance();
            msg
        } else {
            String::new()
        };
        
        Ok(Statement::Raise(message))
    }
    
    /// Парсинг присваивания или вызова процедуры
    fn parse_assignment_or_call(&mut self) -> Result<Statement, String> {
        let expr = self.parse_expression()?;
        
        if self.check(Token::Assign) {
            self.advance();
            let value = self.parse_expression()?;
            Ok(Statement::Assignment {
                target: expr,
                value,
            })
        } else if let Expression::Call { function, args } = expr {
            if let Expression::Identifier(name) = *function {
                Ok(Statement::ProcedureCall { name, args })
            } else {
                Err("Invalid procedure call".to_string())
            }
        } else {
            Err("Expected assignment or procedure call".to_string())
        }
    }
    
    /// Парсинг выражения
    fn parse_expression(&mut self) -> Result<Expression, String> {
        self.parse_or()
    }
    
    /// Парсинг логического ИЛИ
    fn parse_or(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_and()?;
        
        while self.check(Token::Or) {
            self.advance();
            let right = self.parse_and()?;
            left = Expression::Binary {
                left: Box::new(left),
                op: BinaryOp::Or,
                right: Box::new(right),
            };
        }
        
        Ok(left)
    }
    
    /// Парсинг логического И
    fn parse_and(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_equality()?;
        
        while self.check(Token::And) {
            self.advance();
            let right = self.parse_equality()?;
            left = Expression::Binary {
                left: Box::new(left),
                op: BinaryOp::And,
                right: Box::new(right),
            };
        }
        
        Ok(left)
    }
    
    /// Парсинг операций сравнения
    fn parse_equality(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_comparison()?;
        
        while let Some(op) = self.match_equality_op() {
            let right = self.parse_comparison()?;
            left = Expression::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }
        
        Ok(left)
    }
    
    /// Парсинг операций сравнения (<, >, <=, >=)
    fn parse_comparison(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_addition()?;
        
        while let Some(op) = self.match_comparison_op() {
            let right = self.parse_addition()?;
            left = Expression::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }
        
        Ok(left)
    }
    
    /// Парсинг сложения и вычитания
    fn parse_addition(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_multiplication()?;
        
        while let Some(op) = self.match_addition_op() {
            let right = self.parse_multiplication()?;
            left = Expression::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }
        
        Ok(left)
    }
    
    /// Парсинг умножения, деления и остатка
    fn parse_multiplication(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_unary()?;
        
        while let Some(op) = self.match_multiplication_op() {
            let right = self.parse_unary()?;
            left = Expression::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }
        
        Ok(left)
    }
    
    /// Парсинг унарных операций
    fn parse_unary(&mut self) -> Result<Expression, String> {
        if self.check(Token::Not) {
            self.advance();
            let operand = self.parse_unary()?;
            return Ok(Expression::Unary {
                op: UnaryOp::Not,
                operand: Box::new(operand),
            });
        }
        
        if self.check(Token::Minus) {
            self.advance();
            let operand = self.parse_unary()?;
            return Ok(Expression::Unary {
                op: UnaryOp::Minus,
                operand: Box::new(operand),
            });
        }
        
        self.parse_postfix()
    }
    
    /// Парсинг постфиксных операций (доступ к членам, индексация, вызовы)
    fn parse_postfix(&mut self) -> Result<Expression, String> {
        let mut expr = self.parse_primary()?;
        
        loop {
            if self.check(Token::Dot) {
                self.advance();
                let member = self.expect_identifier()?;
                expr = Expression::MemberAccess {
                    object: Box::new(expr),
                    member,
                };
            } else if self.check(Token::LeftBracket) {
                self.advance();
                let index = self.parse_expression()?;
                self.expect(Token::RightBracket)?;
                expr = Expression::Index {
                    object: Box::new(expr),
                    index: Box::new(index),
                };
            } else if self.check(Token::LeftParen) {
                self.advance();
                let args = self.parse_arguments()?;
                self.expect(Token::RightParen)?;
                expr = Expression::Call {
                    function: Box::new(expr),
                    args,
                };
            } else {
                break;
            }
        }
        
        Ok(expr)
    }
    
    /// Парсинг первичных выражений
    fn parse_primary(&mut self) -> Result<Expression, String> {
        match self.current_token() {
            Some(Token::Number(n)) => {
                let num = *n;
                self.advance();
                Ok(Expression::Number(num))
            }
            Some(Token::String(s)) => {
                let str = s.clone();
                self.advance();
                Ok(Expression::String(str))
            }
            Some(Token::True) => {
                self.advance();
                Ok(Expression::Boolean(true))
            }
            Some(Token::False) => {
                self.advance();
                Ok(Expression::Boolean(false))
            }
            Some(Token::Undefined) => {
                self.advance();
                Ok(Expression::Undefined)
            }
            Some(Token::Null) => {
                self.advance();
                Ok(Expression::Null)
            }
            Some(Token::Date(d)) => {
                let date = d.clone();
                self.advance();
                Ok(Expression::Date(date))
            }
            Some(Token::New) => self.parse_new(),
            Some(Token::Identifier(name)) => {
                let id = name.clone();
                self.advance();
                Ok(Expression::Identifier(id))
            }
            // Ключевые слова, которые могут использоваться как идентификаторы в выражениях
            Some(Token::Function) => {
                self.advance();
                Ok(Expression::Identifier("Функция".to_string()))
            }
            Some(Token::Procedure) => {
                self.advance();
                Ok(Expression::Identifier("Процедура".to_string()))
            }
            Some(Token::LeftParen) => {
                self.advance();
                let expr = self.parse_expression()?;
                self.expect(Token::RightParen)?;
                Ok(expr)
            }
            Some(Token::Question) => self.parse_ternary(),
            _ => Err(format!("Unexpected token in expression: {:?}", self.current_token())),
        }
    }
    
    /// Парсинг создания нового объекта
    fn parse_new(&mut self) -> Result<Expression, String> {
        self.expect(Token::New)?;
        let type_name = self.expect_identifier()?;
        
        let args = if self.check(Token::LeftParen) {
            self.advance();
            let args = self.parse_arguments()?;
            self.expect(Token::RightParen)?;
            args
        } else {
            Vec::new()
        };
        
        Ok(Expression::New { type_name, args })
    }
    
    /// Парсинг тернарного оператора
    fn parse_ternary(&mut self) -> Result<Expression, String> {
        self.expect(Token::Question)?;
        self.expect(Token::LeftParen)?;
        let condition = self.parse_expression()?;
        self.expect(Token::Comma)?;
        let then_expr = self.parse_expression()?;
        self.expect(Token::Comma)?;
        let else_expr = self.parse_expression()?;
        self.expect(Token::RightParen)?;
        
        Ok(Expression::Ternary {
            condition: Box::new(condition),
            then_expr: Box::new(then_expr),
            else_expr: Box::new(else_expr),
        })
    }
    
    /// Парсинг параметров функции/процедуры
    fn parse_parameters(&mut self) -> Result<Vec<Parameter>, String> {
        let mut params = Vec::new();
        
        if !self.check(Token::RightParen) {
            loop {
                let by_value = if self.check(Token::Val) {
                    self.advance();
                    true
                } else {
                    false
                };
                
                let name = self.expect_identifier()?;
                
                let default_value = if self.check(Token::Assign) {
                    self.advance();
                    Some(self.parse_expression()?)
                } else {
                    None
                };
                
                params.push(Parameter {
                    name,
                    by_value,
                    default_value,
                });
                
                if !self.check(Token::Comma) {
                    break;
                }
                self.advance();
            }
        }
        
        Ok(params)
    }
    
    /// Парсинг аргументов вызова функции
    fn parse_arguments(&mut self) -> Result<Vec<Expression>, String> {
        let mut args = Vec::new();
        
        if !self.check(Token::RightParen) {
            loop {
                args.push(self.parse_expression()?);
                
                if !self.check(Token::Comma) {
                    break;
                }
                self.advance();
            }
        }
        
        Ok(args)
    }
    
    // === Вспомогательные методы ===
    
    fn current_token(&self) -> Option<&Token> {
        self.tokens.get(self.position)
    }
    
    fn advance(&mut self) {
        if self.position < self.tokens.len() {
            self.position += 1;
        }
    }
    
    fn check(&self, token: Token) -> bool {
        self.current_token() == Some(&token)
    }
    
    fn check_sequence(&self, tokens: &[Token]) -> bool {
        for (i, token) in tokens.iter().enumerate() {
            if self.tokens.get(self.position + i) != Some(token) {
                return false;
            }
        }
        true
    }
    
    fn expect(&mut self, token: Token) -> Result<(), String> {
        if self.check(token.clone()) {
            self.advance();
            Ok(())
        } else {
            Err(format!("Expected {:?}, got {:?}", token, self.current_token()))
        }
    }
    
    fn expect_identifier(&mut self) -> Result<String, String> {
        match self.current_token() {
            Some(Token::Identifier(name)) => {
                let id = name.clone();
                self.advance();
                Ok(id)
            }
            _ => Err(format!("Expected identifier, got {:?}", self.current_token())),
        }
    }
    
    fn check_end_keyword(&self) -> bool {
        matches!(
            self.current_token(),
            Some(Token::EndIf) | Some(Token::EndDo) | Some(Token::EndProcedure) | 
            Some(Token::EndFunction) | Some(Token::EndTry) | Some(Token::Else) |
            Some(Token::ElseIf) | Some(Token::Except)
        )
    }
    
    fn check_statement_end(&self) -> bool {
        matches!(
            self.current_token(),
            Some(Token::Semicolon) | None
        ) || self.check_end_keyword()
    }
    
    fn skip_semicolons(&mut self) {
        while self.check(Token::Semicolon) {
            self.advance();
        }
    }
    
    fn match_equality_op(&mut self) -> Option<BinaryOp> {
        match self.current_token() {
            Some(Token::Equal) => {
                self.advance();
                Some(BinaryOp::Equal)
            }
            Some(Token::NotEqual) => {
                self.advance();
                Some(BinaryOp::NotEqual)
            }
            _ => None,
        }
    }
    
    fn match_comparison_op(&mut self) -> Option<BinaryOp> {
        match self.current_token() {
            Some(Token::Less) => {
                self.advance();
                Some(BinaryOp::Less)
            }
            Some(Token::LessOrEqual) => {
                self.advance();
                Some(BinaryOp::LessOrEqual)
            }
            Some(Token::Greater) => {
                self.advance();
                Some(BinaryOp::Greater)
            }
            Some(Token::GreaterOrEqual) => {
                self.advance();
                Some(BinaryOp::GreaterOrEqual)
            }
            _ => None,
        }
    }
    
    fn match_addition_op(&mut self) -> Option<BinaryOp> {
        match self.current_token() {
            Some(Token::Plus) => {
                self.advance();
                Some(BinaryOp::Add)
            }
            Some(Token::Minus) => {
                self.advance();
                Some(BinaryOp::Subtract)
            }
            _ => None,
        }
    }
    
    fn match_multiplication_op(&mut self) -> Option<BinaryOp> {
        match self.current_token() {
            Some(Token::Star) => {
                self.advance();
                Some(BinaryOp::Multiply)
            }
            Some(Token::Slash) => {
                self.advance();
                Some(BinaryOp::Divide)
            }
            Some(Token::Percent) => {
                self.advance();
                Some(BinaryOp::Modulo)
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_variable() {
        let code = "Перем А = 10;";
        let mut parser = BslParser::new(code).unwrap();
        let program = parser.parse().unwrap();
        
        assert_eq!(program.statements.len(), 1);
        match &program.statements[0] {
            Statement::VarDeclaration { name, value, .. } => {
                assert_eq!(name, "А");
                assert!(matches!(value, Some(Expression::Number(10.0))));
            }
            _ => panic!("Expected variable declaration"),
        }
    }
    
    #[test]
    fn test_parse_if() {
        let code = "Если А > 10 Тогда Б = 20; КонецЕсли;";
        let mut parser = BslParser::new(code).unwrap();
        let program = parser.parse().unwrap();
        
        assert_eq!(program.statements.len(), 1);
        assert!(matches!(&program.statements[0], Statement::If { .. }));
    }
    
    #[test]
    fn test_parse_function() {
        let code = "Функция Сумма(А, Б) Возврат А + Б; КонецФункции;";
        let mut parser = BslParser::new(code).unwrap();
        let program = parser.parse().unwrap();
        
        assert_eq!(program.statements.len(), 1);
        match &program.statements[0] {
            Statement::FunctionDecl { name, params, .. } => {
                assert_eq!(name, "Сумма");
                assert_eq!(params.len(), 2);
            }
            _ => panic!("Expected function declaration"),
        }
    }
}