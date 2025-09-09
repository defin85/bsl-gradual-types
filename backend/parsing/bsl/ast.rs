//! Abstract Syntax Tree для BSL

use serde::{Deserialize, Serialize};

/// Корневой узел программы
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Program {
    pub statements: Vec<Statement>,
}

/// Операторы языка
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Statement {
    /// Объявление переменной: Перем ИмяПеременной
    VarDeclaration {
        name: String,
        export: bool,
        value: Option<Expression>,
    },

    /// Объявление процедуры
    ProcedureDecl {
        name: String,
        params: Vec<Parameter>,
        body: Vec<Statement>,
        export: bool,
    },

    /// Объявление функции
    FunctionDecl {
        name: String,
        params: Vec<Parameter>,
        body: Vec<Statement>,
        return_value: Option<Expression>,
        export: bool,
    },

    /// Присваивание: Переменная = Выражение
    Assignment {
        target: Expression,
        value: Expression,
    },

    /// Вызов процедуры
    ProcedureCall { name: String, args: Vec<Expression> },

    /// Условный оператор: Если ... Тогда ... ИначеЕсли ... Иначе ... КонецЕсли
    If {
        condition: Expression,
        then_branch: Vec<Statement>,
        else_if_branches: Vec<(Expression, Vec<Statement>)>,
        else_branch: Option<Vec<Statement>>,
    },

    /// Цикл Для
    For {
        variable: String,
        from: Expression,
        to: Expression,
        step: Option<Expression>,
        body: Vec<Statement>,
    },

    /// Цикл Для Каждого
    ForEach {
        variable: String,
        collection: Expression,
        body: Vec<Statement>,
    },

    /// Цикл Пока
    While {
        condition: Expression,
        body: Vec<Statement>,
    },

    /// Возврат из функции
    Return(Option<Expression>),

    /// Прерывание цикла
    Break,

    /// Продолжение цикла
    Continue,

    /// Попытка-Исключение
    Try {
        try_block: Vec<Statement>,
        catch_block: Option<Vec<Statement>>,
    },

    /// Вызвать исключение
    Raise(String),
}

/// Выражения
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Expression {
    /// Литералы
    Number(f64),
    String(String),
    Boolean(bool),
    Date(String),
    Undefined,
    Null,

    /// Идентификатор (переменная)
    Identifier(String),

    /// Доступ к члену: Объект.Свойство
    MemberAccess {
        object: Box<Expression>,
        member: String,
    },

    /// Индексация: Массив[0]
    Index {
        object: Box<Expression>,
        index: Box<Expression>,
    },

    /// Вызов функции/метода
    Call {
        function: Box<Expression>,
        args: Vec<Expression>,
    },

    /// Новый объект: Новый ИмяТипа(параметры)
    New {
        type_name: String,
        args: Vec<Expression>,
    },

    /// Бинарные операции
    Binary {
        left: Box<Expression>,
        op: BinaryOp,
        right: Box<Expression>,
    },

    /// Унарные операции
    Unary {
        op: UnaryOp,
        operand: Box<Expression>,
    },

    /// Тернарный оператор: ?(условие, значение_если_истина, значение_если_ложь)
    Ternary {
        condition: Box<Expression>,
        then_expr: Box<Expression>,
        else_expr: Box<Expression>,
    },

    /// Массив: Массив(элементы) или через литерал
    Array(Vec<Expression>),

    /// Структура: Новый Структура("ключ1,значение1,ключ2,значение2")
    Structure(Vec<(String, Expression)>),
}

/// Параметр процедуры/функции
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter {
    pub name: String,
    pub by_value: bool, // Знач
    pub default_value: Option<Expression>,
}

/// Бинарные операторы
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BinaryOp {
    // Арифметические
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,

    // Сравнения
    Equal,
    NotEqual,
    Less,
    LessOrEqual,
    Greater,
    GreaterOrEqual,

    // Логические
    And,
    Or,
}

/// Унарные операторы
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UnaryOp {
    Not,
    Minus,
}

/// Информация о позиции в исходном коде
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceLocation {
    pub line: usize,
    pub column: usize,
    pub file: Option<String>,
}

/// Узел AST с информацией о позиции
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AstNode<T> {
    pub value: T,
    pub location: SourceLocation,
}

impl<T> AstNode<T> {
    pub fn new(value: T, location: SourceLocation) -> Self {
        Self { value, location }
    }
}
