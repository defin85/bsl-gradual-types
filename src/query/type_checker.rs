use std::collections::HashMap;
use std::path::Path;
use crate::core::{
    types::{ResolutionResult, TypeResolution, ConcreteType, PrimitiveType, SpecialType, ConfigurationType, MetadataKind},
    context::ContextResolver,
};
use crate::adapters::config_parser_xml::ConfigParserXml;
use super::ast::*;

pub struct QueryTypeChecker {
    _context: ContextResolver,
    config_parser: Option<ConfigParserXml>,
    table_schemas: HashMap<String, TableSchema>,
    parameters: HashMap<String, TypeResolution>,
}

#[derive(Debug, Clone)]
pub struct TableSchema {
    pub fields: HashMap<String, FieldSchema>,
}

#[derive(Debug, Clone)]
pub struct FieldSchema {
    pub name: String,
    pub type_resolution: TypeResolution,
    pub nullable: bool,
}

#[derive(Debug, Clone)]
pub struct QueryResult {
    pub fields: Vec<ResultField>,
    pub errors: Vec<QueryError>,
}

#[derive(Debug, Clone)]
pub struct ResultField {
    pub name: String,
    pub type_resolution: TypeResolution,
    pub source_table: Option<String>,
}

#[derive(Debug, Clone)]
pub struct QueryError {
    pub message: String,
    pub location: Option<String>,
}

impl QueryTypeChecker {
    pub fn new(context: ContextResolver) -> Self {
        Self {
            _context: context,
            config_parser: None,
            table_schemas: HashMap::new(),
            parameters: HashMap::new(),
        }
    }
    
    /// Create type checker with configuration metadata
    pub fn with_config(config_path: &Path) -> anyhow::Result<Self> {
        let mut config_parser = ConfigParserXml::new(config_path);
        // Load all metadata into cache
        config_parser.load_all_types()?;
        
        Ok(Self {
            _context: ContextResolver,
            config_parser: Some(config_parser),
            table_schemas: HashMap::new(),
            parameters: HashMap::new(),
        })
    }

    pub fn check_query(&mut self, query: &Query) -> QueryResult {
        let mut errors = Vec::new();
        let mut result_fields = Vec::new();

        // Анализируем FROM clause для загрузки схем таблиц
        self.analyze_from_clause(&query.from_clause, &mut errors);

        // Проверяем SELECT clause
        for field in &query.select_clause.fields {
            let field_type = self.check_expression(&field.expression, &mut errors);
            let field_name = field.alias.clone().unwrap_or_else(|| {
                self.expression_to_string(&field.expression)
            });

            result_fields.push(ResultField {
                name: field_name,
                type_resolution: field_type,
                source_table: self.get_expression_source_table(&field.expression),
            });
        }

        // Проверяем WHERE clause
        if let Some(where_clause) = &query.where_clause {
            let where_type = self.check_expression(&where_clause.condition, &mut errors);
            if !self.is_boolean_type(&where_type) {
                errors.push(QueryError {
                    message: "Условие WHERE должно возвращать булево значение".to_string(),
                    location: Some("WHERE".to_string()),
                });
            }
        }

        // Проверяем GROUP BY clause
        if let Some(group_by) = &query.group_by_clause {
            for expr in &group_by.fields {
                self.check_expression(expr, &mut errors);
            }
            
            // Проверяем, что не-агрегатные поля в SELECT есть в GROUP BY
            self.validate_group_by(&query.select_clause, group_by, &mut errors);
        }

        // Проверяем HAVING clause
        if let Some(having) = &query.having_clause {
            let having_type = self.check_expression(&having.condition, &mut errors);
            if !self.is_boolean_type(&having_type) {
                errors.push(QueryError {
                    message: "Условие HAVING должно возвращать булево значение".to_string(),
                    location: Some("HAVING".to_string()),
                });
            }
        }

        // Проверяем ORDER BY clause
        if let Some(order_by) = &query.order_by_clause {
            for item in &order_by.items {
                self.check_expression(&item.expression, &mut errors);
            }
        }

        QueryResult {
            fields: result_fields,
            errors,
        }
    }

    fn analyze_from_clause(&mut self, from_clause: &FromClause, errors: &mut Vec<QueryError>) {
        for source in &from_clause.sources {
            self.load_table_schema(&source.table, source.alias.as_deref(), errors);
            
            // Анализируем JOIN-ы
            for join in &source.joins {
                self.load_table_schema(&join.table.table, join.table.alias.as_deref(), errors);
                
                if let Some(condition) = &join.condition {
                    let condition_type = self.check_expression(condition, errors);
                    if !self.is_boolean_type(&condition_type) {
                        errors.push(QueryError {
                            message: "Условие JOIN должно возвращать булево значение".to_string(),
                            location: Some("JOIN".to_string()),
                        });
                    }
                }
            }
        }
    }

    fn load_table_schema(&mut self, table_ref: &TableReference, alias: Option<&str>, errors: &mut Vec<QueryError>) {
        let schema = match table_ref {
            TableReference::Catalog(_, name) => {
                self.load_catalog_schema(name)
            }
            TableReference::Document(_, name) => {
                self.load_document_schema(name)
            }
            TableReference::Register(reg_type, name) => {
                self.load_register_schema(reg_type, name)
            }
            TableReference::VirtualTable(base, vt_name, params) => {
                self.load_virtual_table_schema(base, vt_name, params)
            }
            TableReference::Table(name) => {
                self.load_generic_table_schema(name)
            }
            TableReference::Subquery(query) => {
                let mut result = self.check_query(query);
                let schema = self.create_schema_from_query_result(&result);
                errors.append(&mut result.errors);
                schema
            }
        };

        let table_name = alias.map(|s| s.to_string()).unwrap_or_else(|| {
            self.table_reference_to_string(table_ref)
        });
        
        self.table_schemas.insert(table_name, schema);
    }

    fn load_catalog_schema(&self, name: &str) -> TableSchema {
        let mut fields = HashMap::new();
        
        // Стандартные поля справочника
        fields.insert("Ссылка".to_string(), FieldSchema {
            name: "Ссылка".to_string(),
            type_resolution: TypeResolution::known(
                ConcreteType::Configuration(ConfigurationType {
                    kind: MetadataKind::Catalog,
                    name: name.to_string(),
                    attributes: vec![],
                    tabular_sections: vec![],
                })
            ),
            nullable: false,
        });
        
        fields.insert("Код".to_string(), FieldSchema {
            name: "Код".to_string(),
            type_resolution: TypeResolution::known(
                ConcreteType::Primitive(PrimitiveType::String)
            ),
            nullable: false,
        });
        
        fields.insert("Наименование".to_string(), FieldSchema {
            name: "Наименование".to_string(),
            type_resolution: TypeResolution::known(
                ConcreteType::Primitive(PrimitiveType::String)
            ),
            nullable: false,
        });
        
        fields.insert("ПометкаУдаления".to_string(), FieldSchema {
            name: "ПометкаУдаления".to_string(),
            type_resolution: TypeResolution::known(
                ConcreteType::Primitive(PrimitiveType::Boolean)
            ),
            nullable: false,
        });

        // Загружаем реальные реквизиты из конфигурации если доступно
        if let Some(ref config_parser) = self.config_parser {
            if let Some(catalog) = config_parser.get_catalog(name) {
                // Добавляем реквизиты из метаданных
                for attribute in &catalog.attributes {
                    fields.insert(attribute.name.clone(), FieldSchema {
                        name: attribute.name.clone(),
                        type_resolution: TypeResolution::known(
                            ConcreteType::Primitive(PrimitiveType::String) // TODO: parse attribute.type_
                        ),
                        nullable: false, // TODO: определять из метаданных
                    });
                }
                
                // Добавляем табличные части как поля-коллекции
                for tab_section in &catalog.tabular_sections {
                    fields.insert(tab_section.name.clone(), FieldSchema {
                        name: tab_section.name.clone(),
                        type_resolution: TypeResolution::known(
                            ConcreteType::Primitive(PrimitiveType::String) // TODO: правильный тип для табличной части
                        ),
                        nullable: false,
                    });
                }
            }
        }
        
        TableSchema { fields }
    }

    fn load_document_schema(&self, name: &str) -> TableSchema {
        let mut fields = HashMap::new();
        
        // Стандартные поля документа
        fields.insert("Ссылка".to_string(), FieldSchema {
            name: "Ссылка".to_string(),
            type_resolution: TypeResolution::known(
                ConcreteType::Configuration(ConfigurationType {
                    kind: MetadataKind::Document,
                    name: name.to_string(),
                    attributes: vec![],
                    tabular_sections: vec![],
                })
            ),
            nullable: false,
        });
        
        fields.insert("Номер".to_string(), FieldSchema {
            name: "Номер".to_string(),
            type_resolution: TypeResolution::known(
                ConcreteType::Primitive(PrimitiveType::String)
            ),
            nullable: false,
        });
        
        fields.insert("Дата".to_string(), FieldSchema {
            name: "Дата".to_string(),
            type_resolution: TypeResolution::known(
                ConcreteType::Primitive(PrimitiveType::Date)
            ),
            nullable: false,
        });
        
        fields.insert("Проведен".to_string(), FieldSchema {
            name: "Проведен".to_string(),
            type_resolution: TypeResolution::known(
                ConcreteType::Primitive(PrimitiveType::Boolean)
            ),
            nullable: false,
        });

        // Загружаем реальные реквизиты из конфигурации если доступно
        if let Some(ref config_parser) = self.config_parser {
            if let Some(document) = config_parser.get_document(name) {
                // Добавляем реквизиты из метаданных
                for attribute in &document.attributes {
                    fields.insert(attribute.name.clone(), FieldSchema {
                        name: attribute.name.clone(),
                        type_resolution: TypeResolution::known(
                            ConcreteType::Primitive(PrimitiveType::String) // TODO: parse attribute.type_
                        ),
                        nullable: false,
                    });
                }
                
                // Добавляем табличные части
                for tab_section in &document.tabular_sections {
                    fields.insert(tab_section.name.clone(), FieldSchema {
                        name: tab_section.name.clone(),
                        type_resolution: TypeResolution::known(
                            ConcreteType::Primitive(PrimitiveType::String) // TODO: правильный тип
                        ),
                        nullable: false,
                    });
                }
            }
        }
        
        TableSchema { fields }
    }

    fn load_register_schema(&self, _reg_type: &str, _name: &str) -> TableSchema {
        let mut fields = HashMap::new();
        
        // Базовые поля регистра
        fields.insert("Период".to_string(), FieldSchema {
            name: "Период".to_string(),
            type_resolution: TypeResolution::known(
                ConcreteType::Primitive(PrimitiveType::Date)
            ),
            nullable: false,
        });
        
        fields.insert("Регистратор".to_string(), FieldSchema {
            name: "Регистратор".to_string(),
            type_resolution: TypeResolution::known(
                ConcreteType::Configuration(ConfigurationType {
                    kind: MetadataKind::Document,
                    name: "Unknown".to_string(),
                    attributes: vec![],
                    tabular_sections: vec![],
                })
            ),
            nullable: true,
        });
        
        fields.insert("Активность".to_string(), FieldSchema {
            name: "Активность".to_string(),
            type_resolution: TypeResolution::known(
                ConcreteType::Primitive(PrimitiveType::Boolean)
            ),
            nullable: false,
        });

        // TODO: Загрузить измерения и ресурсы из конфигурации
        
        TableSchema { fields }
    }

    fn load_virtual_table_schema(&self, _base: &str, vt_name: &str, _params: &[VirtualTableParameter]) -> TableSchema {
        // Виртуальные таблицы имеют специфические поля в зависимости от типа
        let mut fields = HashMap::new();
        
        match vt_name {
            "СрезПоследних" => {
                // Возвращает последние записи регистра сведений
                fields.insert("Период".to_string(), FieldSchema {
                    name: "Период".to_string(),
                    type_resolution: TypeResolution::known(
                        ConcreteType::Primitive(PrimitiveType::Date)
                    ),
                    nullable: false,
                });
            }
            "Остатки" => {
                // Для регистров накопления - остатки
                fields.insert("КоличествоОстаток".to_string(), FieldSchema {
                    name: "КоличествоОстаток".to_string(),
                    type_resolution: TypeResolution::known(
                        ConcreteType::Primitive(PrimitiveType::Number)
                    ),
                    nullable: false,
                });
            }
            "Обороты" => {
                // Для регистров накопления - обороты
                fields.insert("КоличествоПриход".to_string(), FieldSchema {
                    name: "КоличествоПриход".to_string(),
                    type_resolution: TypeResolution::known(
                        ConcreteType::Primitive(PrimitiveType::Number)
                    ),
                    nullable: false,
                });
                fields.insert("КоличествоРасход".to_string(), FieldSchema {
                    name: "КоличествоРасход".to_string(),
                    type_resolution: TypeResolution::known(
                        ConcreteType::Primitive(PrimitiveType::Number)
                    ),
                    nullable: false,
                });
            }
            _ => {}
        }
        
        TableSchema { fields }
    }

    fn load_generic_table_schema(&self, _name: &str) -> TableSchema {
        // Для неизвестных таблиц возвращаем пустую схему
        TableSchema {
            fields: HashMap::new(),
        }
    }

    fn create_schema_from_query_result(&self, result: &QueryResult) -> TableSchema {
        let mut fields = HashMap::new();
        
        for field in &result.fields {
            fields.insert(field.name.clone(), FieldSchema {
                name: field.name.clone(),
                type_resolution: field.type_resolution.clone(),
                nullable: true,
            });
        }
        
        TableSchema { fields }
    }

    fn check_expression(&mut self, expr: &Expression, errors: &mut Vec<QueryError>) -> TypeResolution {
        match expr {
            Expression::Field(name) => {
                self.resolve_field_type(None, name, errors)
            }
            Expression::QualifiedField(table, field) => {
                self.resolve_field_type(Some(table), field, errors)
            }
            Expression::Literal(lit) => {
                self.literal_type(lit)
            }
            Expression::Function(func) => {
                self.check_function_call(func, errors)
            }
            Expression::BinaryOp(left, op, right) => {
                self.check_binary_op(left, op, right, errors)
            }
            Expression::UnaryOp(op, expr) => {
                self.check_unary_op(op, expr, errors)
            }
            Expression::Between(expr, lower, upper) => {
                let _expr_type = self.check_expression(expr, errors);
                let _lower_type = self.check_expression(lower, errors);
                let _upper_type = self.check_expression(upper, errors);
                
                // TODO: Проверить совместимость типов
                
                TypeResolution::known(ConcreteType::Primitive(PrimitiveType::Boolean))
            }
            Expression::In(expr, list) => {
                let _expr_type = self.check_expression(expr, errors);
                for item in list {
                    let _item_type = self.check_expression(item, errors);
                    // TODO: Проверить совместимость типов
                }
                
                TypeResolution::known(ConcreteType::Primitive(PrimitiveType::Boolean))
            }
            Expression::Case(case_expr) => {
                self.check_case_expression(case_expr, errors)
            }
            Expression::Cast(expr, dtype) => {
                let _ = self.check_expression(expr, errors);
                self.data_type_to_resolution(dtype)
            }
            Expression::Parameter(name) => {
                self.parameters.get(name).cloned().unwrap_or_else(|| {
                    TypeResolution::unknown()
                })
            }
            Expression::Subquery(query) => {
                let result = self.check_query(query);
                errors.extend(result.errors);
                
                if result.fields.len() == 1 {
                    result.fields[0].type_resolution.clone()
                } else {
                    TypeResolution::unknown()
                }
            }
        }
    }

    fn resolve_field_type(&self, table: Option<&str>, field: &str, errors: &mut Vec<QueryError>) -> TypeResolution {
        if let Some(table_name) = table {
            if let Some(schema) = self.table_schemas.get(table_name) {
                if let Some(field_schema) = schema.fields.get(field) {
                    return field_schema.type_resolution.clone();
                } else {
                    errors.push(QueryError {
                        message: format!("Поле '{}' не найдено в таблице '{}'", field, table_name),
                        location: Some(format!("{}.{}", table_name, field)),
                    });
                }
            } else {
                errors.push(QueryError {
                    message: format!("Таблица '{}' не найдена", table_name),
                    location: Some(table_name.to_string()),
                });
            }
        } else {
            // Ищем поле во всех таблицах
            let mut found_types = Vec::new();
            for schema in self.table_schemas.values() {
                if let Some(field_schema) = schema.fields.get(field) {
                    found_types.push(field_schema.type_resolution.clone());
                }
            }
            
            if found_types.is_empty() {
                errors.push(QueryError {
                    message: format!("Поле '{}' не найдено", field),
                    location: Some(field.to_string()),
                });
                return TypeResolution::unknown();
            } else if found_types.len() > 1 {
                errors.push(QueryError {
                    message: format!("Поле '{}' неоднозначно, укажите таблицу", field),
                    location: Some(field.to_string()),
                });
            }
            
            return found_types.into_iter().next().unwrap_or_else(TypeResolution::unknown);
        }
        
        TypeResolution::unknown()
    }

    fn literal_type(&self, lit: &Literal) -> TypeResolution {
        match lit {
            Literal::Number(_) => TypeResolution::known(ConcreteType::Primitive(PrimitiveType::Number)),
            Literal::String(_) => TypeResolution::known(ConcreteType::Primitive(PrimitiveType::String)),
            Literal::Boolean(_) => TypeResolution::known(ConcreteType::Primitive(PrimitiveType::Boolean)),
            Literal::Date(_) => TypeResolution::known(ConcreteType::Primitive(PrimitiveType::Date)),
            Literal::Null | Literal::Undefined => TypeResolution::known(ConcreteType::Special(SpecialType::Undefined)),
            Literal::EmptyReference => TypeResolution::known(ConcreteType::Special(SpecialType::Null)),
        }
    }

    fn check_function_call(&mut self, func: &FunctionCall, errors: &mut Vec<QueryError>) -> TypeResolution {
        // Проверяем аргументы функции
        for arg in &func.args {
            self.check_expression(arg, errors);
        }
        
        // Определяем тип результата функции
        match func.name.to_uppercase().as_str() {
            "СУММА" | "СРЕДНЕЕ" | "МИНИМУМ" | "МАКСИМУМ" => {
                TypeResolution::known(ConcreteType::Primitive(PrimitiveType::Number))
            }
            "КОЛИЧЕСТВО" => {
                TypeResolution::known(ConcreteType::Primitive(PrimitiveType::Number))
            }
            "СТРОКА" => {
                TypeResolution::known(ConcreteType::Primitive(PrimitiveType::String))
            }
            "ДАТАВРЕМЯ" => {
                TypeResolution::known(ConcreteType::Primitive(PrimitiveType::Date))
            }
            "ГОД" | "МЕСЯЦ" | "ДЕНЬ" | "ЧАС" | "МИНУТА" | "СЕКУНДА" => {
                TypeResolution::known(ConcreteType::Primitive(PrimitiveType::Number))
            }
            "НАЧАЛОПЕРИОДА" | "КОНЕЦПЕРИОДА" => {
                TypeResolution::known(ConcreteType::Primitive(PrimitiveType::Date))
            }
            "ЕСТЬNULL" => {
                if func.args.len() >= 2 {
                    self.check_expression(&func.args[1], errors)
                } else {
                    TypeResolution::unknown()
                }
            }
            _ => TypeResolution::unknown()
        }
    }

    fn check_binary_op(&mut self, left: &Expression, op: &BinaryOperator, right: &Expression, errors: &mut Vec<QueryError>) -> TypeResolution {
        let _left_type = self.check_expression(left, errors);
        let _right_type = self.check_expression(right, errors);
        
        match op {
            BinaryOperator::Add | BinaryOperator::Subtract | BinaryOperator::Multiply | BinaryOperator::Divide => {
                // Арифметические операции возвращают число
                TypeResolution::known(ConcreteType::Primitive(PrimitiveType::Number))
            }
            BinaryOperator::Equal | BinaryOperator::NotEqual | BinaryOperator::Less | 
            BinaryOperator::LessOrEqual | BinaryOperator::Greater | BinaryOperator::GreaterOrEqual |
            BinaryOperator::Like | BinaryOperator::Is | BinaryOperator::IsNot => {
                // Операции сравнения возвращают булево
                TypeResolution::known(ConcreteType::Primitive(PrimitiveType::Boolean))
            }
            BinaryOperator::And | BinaryOperator::Or => {
                // Логические операции возвращают булево
                TypeResolution::known(ConcreteType::Primitive(PrimitiveType::Boolean))
            }
        }
    }

    fn check_unary_op(&mut self, op: &UnaryOperator, expr: &Expression, errors: &mut Vec<QueryError>) -> TypeResolution {
        let _expr_type = self.check_expression(expr, errors);
        
        match op {
            UnaryOperator::Not => {
                TypeResolution::known(ConcreteType::Primitive(PrimitiveType::Boolean))
            }
            UnaryOperator::Minus => {
                TypeResolution::known(ConcreteType::Primitive(PrimitiveType::Number))
            }
        }
    }

    fn check_case_expression(&mut self, case: &CaseExpression, errors: &mut Vec<QueryError>) -> TypeResolution {
        let mut result_types = Vec::new();
        
        for when in &case.when_clauses {
            let condition_type = self.check_expression(&when.condition, errors);
            if !self.is_boolean_type(&condition_type) {
                errors.push(QueryError {
                    message: "Условие WHEN должно возвращать булево значение".to_string(),
                    location: Some("CASE WHEN".to_string()),
                });
            }
            
            let result_type = self.check_expression(&when.result, errors);
            result_types.push(result_type);
        }
        
        if let Some(else_expr) = &case.else_clause {
            let else_type = self.check_expression(else_expr, errors);
            result_types.push(else_type);
        }
        
        // TODO: Определить общий тип из всех веток
        result_types.into_iter().next().unwrap_or_else(TypeResolution::unknown)
    }

    fn data_type_to_resolution(&self, dtype: &DataType) -> TypeResolution {
        match dtype {
            DataType::Number(_, _) => TypeResolution::known(ConcreteType::Primitive(PrimitiveType::Number)),
            DataType::String(_) => TypeResolution::known(ConcreteType::Primitive(PrimitiveType::String)),
            DataType::Date => TypeResolution::known(ConcreteType::Primitive(PrimitiveType::Date)),
            DataType::Boolean => TypeResolution::known(ConcreteType::Primitive(PrimitiveType::Boolean)),
            DataType::Reference(name) => TypeResolution::known(
                ConcreteType::Configuration(ConfigurationType {
                    kind: MetadataKind::Catalog,
                    name: name.to_string(),
                    attributes: vec![],
                    tabular_sections: vec![],
                })
            ),
        }
    }

    fn is_boolean_type(&self, type_res: &TypeResolution) -> bool {
        matches!(type_res.result, ResolutionResult::Concrete(ConcreteType::Primitive(PrimitiveType::Boolean)))
    }

    fn validate_group_by(&self, select: &SelectClause, group_by: &GroupByClause, errors: &mut Vec<QueryError>) {
        for field in &select.fields {
            if !self.is_aggregate_expression(&field.expression) && !self.is_in_group_by(&field.expression, group_by) {
                errors.push(QueryError {
                    message: format!(
                        "Поле '{}' должно быть в GROUP BY или быть агрегатной функцией",
                        self.expression_to_string(&field.expression)
                    ),
                    location: Some("SELECT".to_string()),
                });
            }
        }
    }

    fn is_aggregate_expression(&self, expr: &Expression) -> bool {
        match expr {
            Expression::Function(func) => {
                matches!(
                    func.name.to_uppercase().as_str(),
                    "СУММА" | "СРЕДНЕЕ" | "МИНИМУМ" | "МАКСИМУМ" | "КОЛИЧЕСТВО"
                )
            }
            _ => false
        }
    }

    fn is_in_group_by(&self, expr: &Expression, group_by: &GroupByClause) -> bool {
        // Упрощенная проверка - сравниваем строковые представления
        let expr_str = self.expression_to_string(expr);
        group_by.fields.iter().any(|gb_expr| {
            self.expression_to_string(gb_expr) == expr_str
        })
    }

    fn expression_to_string(&self, expr: &Expression) -> String {
        match expr {
            Expression::Field(name) => name.clone(),
            Expression::QualifiedField(table, field) => format!("{}.{}", table, field),
            Expression::Literal(lit) => format!("{:?}", lit),
            Expression::Function(func) => func.name.clone(),
            Expression::Parameter(name) => format!("&{}", name),
            _ => "Expression".to_string()
        }
    }

    fn get_expression_source_table(&self, expr: &Expression) -> Option<String> {
        match expr {
            Expression::QualifiedField(table, _) => Some(table.clone()),
            _ => None
        }
    }

    fn table_reference_to_string(&self, table_ref: &TableReference) -> String {
        match table_ref {
            TableReference::Table(name) => name.clone(),
            TableReference::Catalog(_, name) => format!("Справочник.{}", name),
            TableReference::Document(_, name) => format!("Документ.{}", name),
            TableReference::Register(reg_type, name) => format!("{}.{}", reg_type, name),
            TableReference::VirtualTable(base, vt_name, _) => format!("{}.{}", base, vt_name),
            TableReference::Subquery(_) => "Subquery".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_query_type_check() {
        let mut checker = QueryTypeChecker::new(ContextResolver);
        
        let query = Query {
            select_clause: SelectClause {
                distinct: false,
                top: None,
                allowed: false,
                fields: vec![
                    SelectField {
                        expression: Expression::Field("Номер".to_string()),
                        alias: None,
                    },
                    SelectField {
                        expression: Expression::Field("Дата".to_string()),
                        alias: None,
                    },
                ],
                into_temp_table: None,
            },
            from_clause: FromClause {
                sources: vec![
                    TableSource {
                        table: TableReference::Document("Документ".to_string(), "ПоступлениеТоваровУслуг".to_string()),
                        alias: None,
                        joins: vec![],
                    },
                ],
            },
            where_clause: None,
            group_by_clause: None,
            having_clause: None,
            order_by_clause: None,
            totals_clause: None,
            union_clause: None,
        };

        let result = checker.check_query(&query);
        
        assert_eq!(result.fields.len(), 2);
        assert!(result.errors.is_empty());
    }
}