use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::Path;

use bsl_shared::domain::{CodeLocation, ModuleType, ReturnDomain};
use bsl_syntax::ast::{Expression, Program, Statement};

use crate::ast_to_ir::lookup_global_collection;
use crate::{parse_result, SettingsSnapshot, SourceFile};
use bsl_shared::domain::signature_index::SignatureIndex;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct FnKey {
    pub(crate) owner_type_name_lower: String,
    pub(crate) function_name_lower: String,
}

impl FnKey {
    fn new(owner_type_name: &str, function_name: &str) -> Self {
        Self {
            owner_type_name_lower: owner_type_name.to_lowercase(),
            function_name_lower: function_name.to_lowercase(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReturnSummary {
    pub(crate) domain: ReturnDomain,
    pub(crate) is_export: bool,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct OpenFilesReturnOverlay {
    by_fn: HashMap<FnKey, ReturnSummary>,
    owner_type_names_lower: HashSet<String>,
}

impl OpenFilesReturnOverlay {
    pub(crate) fn get(&self, owner_type_name: &str, function_name: &str) -> Option<&ReturnSummary> {
        self.by_fn.get(&FnKey::new(owner_type_name, function_name))
    }

    pub(crate) fn has_owner_type_name(&self, owner_type_name: &str) -> bool {
        self.owner_type_names_lower
            .contains(&owner_type_name.to_lowercase())
    }
}

#[derive(Debug, Clone, Default)]
struct FunctionFacts {
    is_function: bool,
    is_export: bool,
    returns: Vec<Atom>,
    vars: HashMap<String, Vec<Atom>>,
    has_return_without_value: bool,
    has_dynamic: bool,
}

#[derive(Debug, Clone)]
enum Atom {
    Known(String),
    Var(String),
    Call(FnKey),
    Unknown,
}

pub(crate) fn build_return_overlay_for_open_files(
    db: &dyn salsa::Database,
    files: &[SourceFile],
    signature_index: &SignatureIndex,
    settings: SettingsSnapshot,
) -> OpenFilesReturnOverlay {
    // 1) Определяем "владельца" модуля для каждого open file.
    let mut file_owner: Vec<(SourceFile, String)> = Vec::new();
    let mut common_module_names_lower: HashSet<String> = HashSet::new();
    for &file in files {
        let Some(owner_type_name) = module_owner_key_from_file_path(file.path(db).as_ref()) else {
            continue;
        };
        if let Some(name) = owner_type_name
            .strip_prefix("ОбщиеМодули.")
            .or_else(|| owner_type_name.strip_prefix("общиемодули."))
        {
            common_module_names_lower.insert(name.to_lowercase());
        }
        file_owner.push((file, owner_type_name));
    }

    // 2) Сбор "фактов" по всем функциям/процедурам из открытых файлов.
    let mut facts_by_fn: HashMap<FnKey, FunctionFacts> = HashMap::new();
    for (file, owner_type_name) in &file_owner {
        let parsed = parse_result(db, *file, settings).0;
        collect_facts_from_program(
            &parsed.program,
            owner_type_name,
            &common_module_names_lower,
            &mut facts_by_fn,
        );
    }

    // 3) Фикс-пойнт: распространяем known-типы по графу вызовов (внутри open files).
    let mut domains: HashMap<FnKey, ReturnDomain> = HashMap::new();

    // Детерминированный порядок итерации.
    let mut keys: Vec<FnKey> = facts_by_fn.keys().cloned().collect();
    keys.sort_by(|a, b| {
        (
            a.owner_type_name_lower.as_str(),
            a.function_name_lower.as_str(),
        )
            .cmp(&(
                b.owner_type_name_lower.as_str(),
                b.function_name_lower.as_str(),
            ))
    });

    let mut changed = true;
    let mut iterations = 0_u32;
    while changed && iterations < 64 {
        iterations += 1;
        changed = false;

        for key in &keys {
            let Some(facts) = facts_by_fn.get(key) else {
                continue;
            };
            let new_domain = evaluate_return_domain(facts, &domains, signature_index);
            match domains.get(key) {
                Some(old) if *old == new_domain => {}
                _ => {
                    domains.insert(key.clone(), new_domain);
                    changed = true;
                }
            }
        }
    }

    let owner_type_names_lower: HashSet<String> = domains
        .keys()
        .map(|k| k.owner_type_name_lower.clone())
        .collect();

    let by_fn: HashMap<FnKey, ReturnSummary> = domains
        .into_iter()
        .map(|(key, domain)| {
            let is_export = facts_by_fn.get(&key).is_some_and(|f| f.is_export);
            (key, ReturnSummary { domain, is_export })
        })
        .collect();

    OpenFilesReturnOverlay {
        by_fn,
        owner_type_names_lower,
    }
}

fn collect_facts_from_program(
    program: &Program,
    owner_type_name: &str,
    common_module_names_lower: &HashSet<String>,
    out: &mut HashMap<FnKey, FunctionFacts>,
) {
    for stmt in &program.statements {
        match stmt {
            Statement::FunctionDecl { name, body, .. } => {
                let key = FnKey::new(owner_type_name, name);
                let mut facts = FunctionFacts {
                    is_function: true,
                    is_export: stmt_is_export(stmt),
                    ..Default::default()
                };
                collect_facts_from_statements(
                    body,
                    owner_type_name,
                    common_module_names_lower,
                    &mut facts,
                );
                out.insert(key, facts);
            }
            Statement::ProcedureDecl { name, body, .. } => {
                let key = FnKey::new(owner_type_name, name);
                let mut facts = FunctionFacts {
                    is_function: false,
                    is_export: stmt_is_export(stmt),
                    ..Default::default()
                };
                collect_facts_from_statements(
                    body,
                    owner_type_name,
                    common_module_names_lower,
                    &mut facts,
                );
                out.insert(key, facts);
            }
            _ => {}
        }
    }
}

fn collect_facts_from_statements(
    statements: &[Statement],
    owner_type_name: &str,
    common_module_names_lower: &HashSet<String>,
    facts: &mut FunctionFacts,
) {
    for stmt in statements {
        match stmt {
            Statement::Assignment { target, value, .. } => {
                if let Expression::Identifier { name, .. } = target {
                    let var_key = name.to_lowercase();
                    let atom =
                        infer_expr_atom(value, owner_type_name, common_module_names_lower, facts);
                    facts.vars.entry(var_key).or_default().push(atom);
                } else {
                    // Если target сложный (индекс, свойство), то точность падает.
                    facts.has_dynamic = true;
                }
            }
            Statement::Return {
                value: Some(value), ..
            } => {
                let atom =
                    infer_expr_atom(value, owner_type_name, common_module_names_lower, facts);
                facts.returns.push(atom);
            }
            Statement::Return { value: None, .. } => {
                facts.has_return_without_value = true;
            }
            Statement::If {
                then_body,
                else_body,
                ..
            } => {
                collect_facts_from_statements(
                    then_body,
                    owner_type_name,
                    common_module_names_lower,
                    facts,
                );
                if let Some(else_body) = else_body {
                    collect_facts_from_statements(
                        else_body,
                        owner_type_name,
                        common_module_names_lower,
                        facts,
                    );
                }
            }
            Statement::While { body, .. }
            | Statement::For { body, .. }
            | Statement::ForEach { body, .. } => {
                collect_facts_from_statements(
                    body,
                    owner_type_name,
                    common_module_names_lower,
                    facts,
                );
            }
            Statement::Try {
                try_body,
                except_body,
                ..
            } => {
                collect_facts_from_statements(
                    try_body,
                    owner_type_name,
                    common_module_names_lower,
                    facts,
                );
                collect_facts_from_statements(
                    except_body,
                    owner_type_name,
                    common_module_names_lower,
                    facts,
                );
            }
            // Локальные объявления внутри тела считаем отдельными рутинами,
            // но в рамках текущего упрощённого overlay их не обрабатываем.
            Statement::FunctionDecl { .. } | Statement::ProcedureDecl { .. } => {}
            _ => {}
        }
    }
}

fn stmt_is_export(stmt: &Statement) -> bool {
    match stmt {
        Statement::FunctionDecl { is_export, .. } | Statement::ProcedureDecl { is_export, .. } => {
            *is_export
        }
        _ => false,
    }
}

fn infer_expr_atom(
    expr: &Expression,
    owner_type_name: &str,
    common_module_names_lower: &HashSet<String>,
    facts: &mut FunctionFacts,
) -> Atom {
    match expr {
        Expression::Number { .. } => Atom::Known("Число".to_string()),
        Expression::String { .. } => Atom::Known("Строка".to_string()),
        Expression::Boolean { .. } => Atom::Known("Булево".to_string()),
        Expression::Date { .. } => Atom::Known("Дата".to_string()),
        Expression::Identifier { name, .. } => {
            let name_lower = name.to_lowercase();
            if name_lower == "неопределено" || name_lower == "undefined" {
                return Atom::Known("Неопределено".to_string());
            }
            if name_lower == "null" {
                return Atom::Known("Null".to_string());
            }
            if matches!(name_lower.as_str(), "истина" | "ложь" | "true" | "false") {
                return Atom::Known("Булево".to_string());
            }
            Atom::Var(name_lower)
        }
        Expression::Call { function, .. } => {
            infer_call_atom(function, owner_type_name, common_module_names_lower, facts)
                .unwrap_or_else(|| {
                    facts.has_dynamic = true;
                    Atom::Unknown
                })
        }
        _ => {
            facts.has_dynamic = true;
            Atom::Unknown
        }
    }
}

fn infer_call_atom(
    function_expr: &Expression,
    owner_type_name: &str,
    common_module_names_lower: &HashSet<String>,
    _facts: &mut FunctionFacts,
) -> Option<Atom> {
    match function_expr {
        Expression::Identifier { name, .. } => Some(Atom::Call(FnKey::new(owner_type_name, name))),
        Expression::PropertyAccess { .. } => {
            let chain = flatten_property_chain(function_expr)?;
            if chain.len() < 2 {
                return None;
            }
            let (receiver, method) = chain.split_at(chain.len() - 1);
            let method = method.first().expect("len>=2");

            // 1) ОбщиеМодули.<Имя>.<Метод>()
            if receiver.len() == 2 && receiver[0].eq_ignore_ascii_case("ОбщиеМодули") {
                let owner = format!("ОбщиеМодули.{}", receiver[1]);
                return Some(Atom::Call(FnKey::new(&owner, method)));
            }

            // 2) <ИмяОбщегоМодуля>.<Метод>() — резолвим только если такой common module открыт.
            if receiver.len() == 1 {
                let module_name_lower = receiver[0].to_lowercase();
                if common_module_names_lower.contains(&module_name_lower) {
                    let owner = format!("ОбщиеМодули.{}", receiver[0]);
                    return Some(Atom::Call(FnKey::new(&owner, method)));
                }
            }

            // 3) Справочники.Контрагенты.<Метод>() / Catalogs.Контрагенты.<Method>()
            if receiver.len() == 2 {
                if let Some(info) = lookup_global_collection(receiver[0].as_str()) {
                    let owner = format!("{}.{}", info.item_manager_type, receiver[1]);
                    return Some(Atom::Call(FnKey::new(&owner, method)));
                }
            }

            None
        }
        _ => None,
    }
}

fn flatten_property_chain(expr: &Expression) -> Option<Vec<String>> {
    fn walk(expr: &Expression, out: &mut Vec<String>) -> bool {
        match expr {
            Expression::Identifier { name, .. } => {
                out.push(name.clone());
                true
            }
            Expression::PropertyAccess {
                object, property, ..
            } => {
                if !walk(object, out) {
                    return false;
                }
                out.push(property.clone());
                true
            }
            _ => false,
        }
    }

    let mut out = Vec::new();
    if walk(expr, &mut out) {
        Some(out)
    } else {
        None
    }
}

fn evaluate_return_domain(
    facts: &FunctionFacts,
    known_domains: &HashMap<FnKey, ReturnDomain>,
    signature_index: &SignatureIndex,
) -> ReturnDomain {
    let mut domain = ReturnDomain {
        known: BTreeSet::new(),
        has_dynamic: facts.has_dynamic,
    };

    // Процедуры/отсутствующий возврат считаем как "Неопределено".
    if !facts.is_function {
        domain.known.insert("Неопределено".to_string());
        return domain;
    }

    if facts.returns.is_empty() {
        domain.known.insert("Неопределено".to_string());
    }
    if facts.has_return_without_value {
        domain.known.insert("Неопределено".to_string());
    }

    for atom in &facts.returns {
        eval_atom(atom, facts, known_domains, signature_index, &mut domain);
    }

    domain
}

fn eval_atom(
    atom: &Atom,
    facts: &FunctionFacts,
    known_domains: &HashMap<FnKey, ReturnDomain>,
    signature_index: &SignatureIndex,
    out: &mut ReturnDomain,
) {
    match atom {
        Atom::Known(t) => {
            out.known.insert(t.clone());
        }
        Atom::Var(name_lower) => {
            let Some(atoms) = facts.vars.get(name_lower) else {
                out.has_dynamic = true;
                return;
            };
            for atom in atoms {
                match atom {
                    Atom::Known(t) => {
                        out.known.insert(t.clone());
                    }
                    Atom::Call(key) => {
                        if let Some(callee) = known_domains.get(key) {
                            out.has_dynamic |= callee.has_dynamic;
                            out.known.extend(callee.known.iter().cloned());
                        } else {
                            out.has_dynamic = true;
                        }
                    }
                    Atom::Var(_) | Atom::Unknown => {
                        out.has_dynamic = true;
                    }
                }
            }
        }
        Atom::Call(key) => {
            if let Some(callee) = known_domains.get(key) {
                out.has_dynamic |= callee.has_dynamic;
                out.known.extend(callee.known.iter().cloned());
                return;
            }

            // Вызов в неоткрытый модуль: пробуем взять return info из SignatureIndex (без I/O).
            if let Some(sig) =
                signature_index.find_method(&key.owner_type_name_lower, &key.function_name_lower)
            {
                let mut added_known = false;
                if let Some(return_type) = sig
                    .return_type
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                {
                    for part in return_type.split('|') {
                        let part = part.trim();
                        if !part.is_empty() {
                            out.known.insert(part.to_string());
                            added_known = true;
                        }
                    }
                }
                // Если return_type отсутствует, считаем это неопределённостью.
                if !added_known {
                    out.has_dynamic = true;
                }
                out.has_dynamic |= sig.return_is_weak;
            } else {
                out.has_dynamic = true;
            }
        }
        Atom::Unknown => {
            out.has_dynamic = true;
        }
    }
}

pub(crate) fn module_owner_key_from_file_path(file_path: &str) -> Option<String> {
    let normalized = normalize_file_path_for_location(file_path);
    let location = CodeLocation::determine_from_path(Path::new(normalized.as_ref())).ok()?;

    match location.module_type {
        ModuleType::CommonModule { name, .. } => Some(format!("ОбщиеМодули.{}", name)),
        ModuleType::ObjectModule { owner_type } => {
            owner_type_to_faceted_type(&owner_type, bsl_shared::domain::types::FacetKind::Object)
        }
        ModuleType::ManagerModule { owner_type } => {
            owner_type_to_faceted_type(&owner_type, bsl_shared::domain::types::FacetKind::Manager)
        }
        ModuleType::RecordSetModule { owner_type } => {
            owner_type_to_faceted_type(&owner_type, bsl_shared::domain::types::FacetKind::Object)
        }
        _ => None,
    }
}

fn owner_type_to_faceted_type(
    owner_type: &str,
    facet: bsl_shared::domain::types::FacetKind,
) -> Option<String> {
    let (xml_collection, object_name) = owner_type.split_once('.')?;
    let kind = bsl_shared::domain::types::MetadataKind::from_xml_tag(xml_collection)?;
    let prefix = kind.faceted_type_prefix(&facet);
    Some(format!("{}.{}", prefix, object_name))
}

fn normalize_file_path_for_location(file_path: &str) -> std::borrow::Cow<'_, str> {
    if let Some(rest) = file_path.strip_prefix("file://") {
        // file:///path -> /path
        // file://host/path -> host/path (в таком виде CodeLocation всё равно может не понять).
        return std::borrow::Cow::Borrowed(rest);
    }
    std::borrow::Cow::Borrowed(file_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_owner_key_for_object_module_uses_object_facet_prefix() {
        let path = "Catalogs/Контрагенты/Ext/ObjectModule.bsl";
        assert_eq!(
            module_owner_key_from_file_path(path),
            Some("СправочникОбъект.Контрагенты".to_string())
        );
    }

    #[test]
    fn module_owner_key_for_record_set_module_uses_record_set_prefix() {
        let path = "AccumulationRegisters/РегистрНакопления/Ext/RecordSetModule.bsl";
        assert_eq!(
            module_owner_key_from_file_path(path),
            Some("РегистрНакопленияНаборЗаписей.РегистрНакопления".to_string())
        );
    }
}
