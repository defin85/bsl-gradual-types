use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Query {
    pub select_clause: SelectClause,
    pub from_clause: FromClause,
    pub where_clause: Option<WhereClause>,
    pub group_by_clause: Option<GroupByClause>,
    pub having_clause: Option<HavingClause>,
    pub order_by_clause: Option<OrderByClause>,
    pub totals_clause: Option<TotalsClause>,
    pub union_clause: Option<Vec<Query>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SelectClause {
    pub distinct: bool,
    pub top: Option<usize>,
    pub allowed: bool,
    pub fields: Vec<SelectField>,
    pub into_temp_table: Option<String>, // Для ПОМЕСТИТЬ ИмяВременнойТаблицы
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SelectField {
    pub expression: Expression,
    pub alias: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FromClause {
    pub sources: Vec<TableSource>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TableSource {
    pub table: TableReference,
    pub alias: Option<String>,
    pub joins: Vec<Join>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TableReference {
    Table(String),
    Catalog(String, String),  // Справочник.Контрагенты
    Document(String, String), // Документ.ПоступлениеТоваровУслуг
    Register(String, String), // РегистрСведений.КурсыВалют
    VirtualTable(String, String, Vec<VirtualTableParameter>),
    Subquery(Box<Query>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VirtualTableParameter {
    pub name: String,
    pub value: Expression,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Join {
    pub join_type: JoinType,
    pub table: TableSource,
    pub condition: Option<Expression>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum JoinType {
    Inner,
    Left,
    Right,
    Full,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WhereClause {
    pub condition: Expression,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GroupByClause {
    pub fields: Vec<Expression>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HavingClause {
    pub condition: Expression,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrderByClause {
    pub items: Vec<OrderByItem>,
    pub auto_order: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrderByItem {
    pub expression: Expression,
    pub direction: OrderDirection,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrderDirection {
    Asc,
    Desc,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TotalsClause {
    pub overall: bool,
    pub by_fields: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Expression {
    Field(String),
    QualifiedField(String, String), // table.field
    Literal(Literal),
    Function(FunctionCall),
    BinaryOp(Box<Expression>, BinaryOperator, Box<Expression>),
    UnaryOp(UnaryOperator, Box<Expression>),
    Between(Box<Expression>, Box<Expression>, Box<Expression>),
    In(Box<Expression>, Vec<Expression>),
    Case(CaseExpression),
    Cast(Box<Expression>, DataType),
    Parameter(String),
    Subquery(Box<Query>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Literal {
    Number(f64),
    String(String),
    Boolean(bool),
    Date(String),
    Null,
    Undefined,
    EmptyReference,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub args: Vec<Expression>,
    pub distinct: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Equal,
    NotEqual,
    Less,
    LessOrEqual,
    Greater,
    GreaterOrEqual,
    And,
    Or,
    Like,
    Is,
    IsNot,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UnaryOperator {
    Not,
    Minus,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CaseExpression {
    pub when_clauses: Vec<WhenClause>,
    pub else_clause: Option<Box<Expression>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WhenClause {
    pub condition: Expression,
    pub result: Expression,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DataType {
    Number(Option<usize>, Option<usize>),
    String(Option<usize>),
    Date,
    Boolean,
    Reference(String),
}
