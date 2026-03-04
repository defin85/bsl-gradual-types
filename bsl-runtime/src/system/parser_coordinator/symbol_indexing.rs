use super::*;

pub(super) fn path_to_uri(path: &Path) -> String {
    let canonical = std::fs::canonicalize(path).ok();
    if let Some(canonical) = canonical {
        if let Ok(url) = Url::from_file_path(canonical) {
            return url.to_string();
        }
    }
    if let Ok(url) = Url::from_file_path(path) {
        return url.to_string();
    }
    let normalized = path.to_string_lossy().replace('\\', "/");
    format!("file://{}", normalized)
}

pub(super) fn collect_symbol_items(program: &Program, uri: &str) -> Vec<IndexItem> {
    let mut items = Vec::new();
    collect_symbol_items_from_statements(&program.statements, uri, true, &mut items);
    items
}

fn collect_symbol_items_from_statements(
    statements: &[Statement],
    uri: &str,
    is_top_level: bool,
    items: &mut Vec<IndexItem>,
) {
    for statement in statements {
        match statement {
            Statement::VarDeclaration { name, span, .. } => {
                let scope = if is_top_level {
                    SymbolScope::Module
                } else {
                    SymbolScope::Local
                };
                items.push(symbol_item(
                    name,
                    SymbolKind::Variable,
                    scope,
                    Some(*span),
                    uri,
                ));
            }
            Statement::FunctionDecl {
                name,
                params,
                body,
                is_export,
                span,
                ..
            } => {
                let mut item = symbol_item(
                    name,
                    SymbolKind::Function,
                    SymbolScope::Module,
                    Some(*span),
                    uri,
                );
                if *is_export {
                    item.visibility = Some(crate::system::intellisense_index::Visibility::Public);
                }
                items.push(item);
                for param in params {
                    items.push(symbol_item(
                        param,
                        SymbolKind::Parameter,
                        SymbolScope::Local,
                        None,
                        uri,
                    ));
                }
                collect_symbol_items_from_statements(body, uri, false, items);
            }
            Statement::ProcedureDecl {
                name,
                params,
                body,
                is_export,
                span,
                ..
            } => {
                let mut item = symbol_item(
                    name,
                    SymbolKind::Procedure,
                    SymbolScope::Module,
                    Some(*span),
                    uri,
                );
                if *is_export {
                    item.visibility = Some(crate::system::intellisense_index::Visibility::Public);
                }
                items.push(item);
                for param in params {
                    items.push(symbol_item(
                        param,
                        SymbolKind::Parameter,
                        SymbolScope::Local,
                        None,
                        uri,
                    ));
                }
                collect_symbol_items_from_statements(body, uri, false, items);
            }
            Statement::For {
                variable,
                body,
                span,
                ..
            } => {
                items.push(symbol_item(
                    variable,
                    SymbolKind::Variable,
                    SymbolScope::Local,
                    Some(*span),
                    uri,
                ));
                collect_symbol_items_from_statements(body, uri, false, items);
            }
            Statement::ForEach {
                variable,
                body,
                span,
                ..
            } => {
                items.push(symbol_item(
                    variable,
                    SymbolKind::Variable,
                    SymbolScope::Local,
                    Some(*span),
                    uri,
                ));
                collect_symbol_items_from_statements(body, uri, false, items);
            }
            Statement::Assignment { target, span, .. } => {
                if let Expression::Identifier { name, .. } = target {
                    let scope = if is_top_level {
                        SymbolScope::Module
                    } else {
                        SymbolScope::Local
                    };
                    items.push(symbol_item(
                        name,
                        SymbolKind::Variable,
                        scope,
                        Some(*span),
                        uri,
                    ));
                }
            }
            Statement::If {
                then_body,
                else_body,
                ..
            } => {
                collect_symbol_items_from_statements(then_body, uri, false, items);
                if let Some(else_body) = else_body {
                    collect_symbol_items_from_statements(else_body, uri, false, items);
                }
            }
            Statement::While { body, .. } => {
                collect_symbol_items_from_statements(body, uri, false, items);
            }
            Statement::Try {
                try_body,
                except_body,
                ..
            } => {
                collect_symbol_items_from_statements(try_body, uri, false, items);
                collect_symbol_items_from_statements(except_body, uri, false, items);
            }
            Statement::Return { .. }
            | Statement::Call { .. }
            | Statement::Break { .. }
            | Statement::Continue { .. }
            | Statement::Goto { .. }
            | Statement::Label { .. }
            | Statement::Execute { .. }
            | Statement::RaiseError { .. }
            | Statement::AddHandler { .. }
            | Statement::RemoveHandler { .. }
            | Statement::Await { .. } => {}
        }
    }
}

fn symbol_item(
    name: &str,
    kind: SymbolKind,
    scope: SymbolScope,
    span: Option<bsl_shared::ir::Span>,
    uri: &str,
) -> IndexItem {
    let mut item = IndexItem::new(name, IndexItemKind::Symbol(kind), IndexKind::Symbol);
    item.uri = Some(uri.to_string());
    item.scope = Some(scope);
    item.span = span;
    item
}
