//! Регрессия P0: переменные должны добавляться в SymbolTable при присваивании.
//!
//! Ранее логика регистрации переменной была завязана на type inference.
//! После refactor-type-inference-v2-only SymbolTable хранит только состояние объявления/инициализации.

use bsl_analysis_v2::AstToIrConverter;
use bsl_backend::parsing::bsl::ast::{Expression, Program, Span as AstSpan, Statement};
use bsl_shared::domain::repository::{InMemoryTypeRepository, TypeRepository};
use bsl_shared::domain::signature_index::SignatureIndex;
use bsl_shared::ir::SemanticNodeKind;
use std::sync::Arc;

fn create_test_repository() -> Arc<dyn TypeRepository> {
    Arc::new(InMemoryTypeRepository::new())
}

fn create_test_signature_index() -> SignatureIndex {
    SignatureIndex::new()
}

#[test]
fn test_assignment_declares_and_initializes_variable_in_symbol_table() {
    let ast = Program {
        statements: vec![Statement::ProcedureDecl {
            name: "Тест".to_string(),
            params: vec![],
            body: vec![Statement::Assignment {
                target: Expression::Identifier {
                    name: "МассивСтрок".to_string(),
                    span: AstSpan::stub(),
                },
                value: Expression::New {
                    type_name: "Массив".to_string(),
                    args: vec![],
                    span: AstSpan::stub(),
                },
                span: AstSpan::stub(),
            }],
            compiler_directive: None,
            is_export: false,
            span: AstSpan::stub(),
        }],
    };

    let ir = AstToIrConverter::convert(
        ast,
        "Процедура Тест()\n  МассивСтрок = Новый Массив();\nКонецПроцедуры".to_string(),
        "test.bsl".to_string(),
        create_test_repository(),
        create_test_signature_index(),
    )
    .unwrap();

    let proc_scope = ir
        .nodes
        .iter()
        .find_map(|node| match &node.kind {
            SemanticNodeKind::ProcedureDeclaration { body_scope, .. } => Some(*body_scope),
            _ => None,
        })
        .expect("Procedure scope not found");

    assert!(
        ir.symbols.has_variable(proc_scope, "МассивСтрок"),
        "Переменная МассивСтрок должна быть зарегистрирована в SymbolTable"
    );

    let state = ir
        .symbols
        .lookup_variable(proc_scope, "МассивСтрок")
        .expect("VariableState должен быть доступен через lookup_variable()");

    assert!(
        state.initialized,
        "При присваивании переменная должна считаться инициализированной"
    );
}

