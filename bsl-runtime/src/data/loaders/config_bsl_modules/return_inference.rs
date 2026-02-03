use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};

use bsl_shared::domain::code_location::ModuleType;

use super::types::{CallTarget, ParsedModule, ReturnAtom, ReturnFacts};
use super::utils::resolve_manager_owner_type_from_receiver;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct ReturnDomain {
    known: BTreeSet<String>,
    has_dynamic: bool,
}

impl ReturnDomain {
    fn add_known(&mut self, t: String) {
        if !t.trim().is_empty() {
            self.known.insert(t);
        }
    }

    fn join(&mut self, other: &ReturnDomain) {
        self.known.extend(other.known.iter().cloned());
        self.has_dynamic |= other.has_dynamic;
    }

    fn to_return_type_string(&self) -> Option<String> {
        if self.known.is_empty() {
            return None;
        }
        Some(self.known.iter().cloned().collect::<Vec<_>>().join(" | "))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct FnKey {
    owner_type: String,
    name_lower: String,
}

#[derive(Debug)]
struct FnNode<'a> {
    key: FnKey,
    facts: &'a ReturnFacts,
}

pub(crate) fn infer_return_types_across_modules(
    modules: &[ParsedModule],
) -> HashMap<(String, String), Option<String>> {
    let common_module_owner_types = common_module_name_to_owner_types(modules);

    // 1) Собираем все функции (включая локальные helper функции внутри модуля).
    let mut nodes: Vec<FnNode<'_>> = Vec::new();
    let mut index_by_key: HashMap<FnKey, usize> = HashMap::new();
    for module in modules {
        for decl in &module.decls {
            let Some(facts) = decl.return_facts.as_ref() else {
                continue; // процедуры и/или отсутствующие факты
            };
            let key = FnKey {
                owner_type: module.owner_type_name.clone(),
                name_lower: decl.name.to_lowercase(),
            };
            let idx = nodes.len();
            nodes.push(FnNode { key: key.clone(), facts });
            index_by_key.insert(key, idx);
        }
    }

    // 2) Строим граф зависимостей по вызовам (callee -> callers).
    let mut callers_of: Vec<Vec<usize>> = vec![Vec::new(); nodes.len()];
    let mut callees_of: Vec<Vec<usize>> = vec![Vec::new(); nodes.len()];

    for (idx, node) in nodes.iter().enumerate() {
        let mut callees: BTreeSet<usize> = BTreeSet::new();

        let mut visit_atom = |atom: &ReturnAtom| {
            if let ReturnAtom::Call(target) = atom {
                if let Some(callee_idx) = resolve_call_to_fn(
                    &node.key.owner_type,
                    target,
                    &common_module_owner_types,
                    &index_by_key,
                ) {
                    callees.insert(callee_idx);
                }
            }
        };

        for atom in &node.facts.returns {
            visit_atom(atom);
            if let ReturnAtom::Var(name) = atom {
                if let Some(values) = node.facts.vars.get(name) {
                    for v in values {
                        visit_atom(v);
                    }
                }
            }
        }

        for &callee in &callees {
            callees_of[idx].push(callee);
            callers_of[callee].push(idx);
        }
    }

    // 3) Worklist фикс‑пойнт: домен монотонно растёт (known ∪, has_dynamic |=).
    let mut domains: Vec<ReturnDomain> = vec![ReturnDomain::default(); nodes.len()];
    let mut queue: VecDeque<usize> = (0..nodes.len()).collect();
    let mut in_queue: Vec<bool> = vec![true; nodes.len()];

    while let Some(idx) = queue.pop_front() {
        in_queue[idx] = false;

        let node = &nodes[idx];
        let new_domain = eval_return_domain(
            node.facts,
            &node.key.owner_type,
            &domains,
            &index_by_key,
            &common_module_owner_types,
        );
        if new_domain != domains[idx] {
            domains[idx] = new_domain;
            for &caller in &callers_of[idx] {
                if !in_queue[caller] {
                    in_queue[caller] = true;
                    queue.push_back(caller);
                }
            }
        }
    }

    // 4) Финализируем в (owner_type, method_name) -> Option<return_type>
    let mut out: HashMap<(String, String), Option<String>> = HashMap::new();
    for (idx, node) in nodes.iter().enumerate() {
        out.insert(
            (node.key.owner_type.clone(), node.key.name_lower.clone()),
            domains[idx].to_return_type_string(),
        );
    }
    out
}

fn common_module_name_to_owner_types(modules: &[ParsedModule]) -> HashMap<String, String> {
    let mut map: HashMap<String, String> = HashMap::new();
    for m in modules {
        if let ModuleType::CommonModule { name, .. } = &m.module_type {
            map.insert(name.to_lowercase(), m.owner_type_name.clone());
        }
    }
    map
}

fn resolve_call_to_fn(
    current_owner_type: &str,
    target: &CallTarget,
    common_modules: &HashMap<String, String>,
    index_by_key: &HashMap<FnKey, usize>,
) -> Option<usize> {
    match target {
        CallTarget::LocalFunction { name } => {
            let key = FnKey {
                owner_type: current_owner_type.to_string(),
                name_lower: name.to_lowercase(),
            };
            index_by_key.get(&key).copied()
        }
        CallTarget::QualifiedMethod { receiver, name } => {
            // Common module call: "ИмяМодуля.Метод()"
            if receiver.len() == 1 {
                if let Some(owner_type) = common_modules.get(&receiver[0].to_lowercase()) {
                    let key = FnKey {
                        owner_type: owner_type.clone(),
                        name_lower: name.to_lowercase(),
                    };
                    return index_by_key.get(&key).copied();
                }
            }

            // Manager call: "Справочники.<X>.Метод()"
            if receiver.len() == 2 {
                if let Some(owner_type) = resolve_manager_owner_type_from_receiver(receiver) {
                    let key = FnKey {
                        owner_type,
                        name_lower: name.to_lowercase(),
                    };
                    return index_by_key.get(&key).copied();
                }
            }

            None
        }
    }
}

fn eval_return_domain(
    facts: &ReturnFacts,
    current_owner_type: &str,
    domains: &[ReturnDomain],
    index_by_key: &HashMap<FnKey, usize>,
    common_modules: &HashMap<String, String>,
) -> ReturnDomain {
    fn resolve_atom(
        atom: &ReturnAtom,
        out: &mut ReturnDomain,
        visited_vars: &mut HashSet<String>,
        facts: &ReturnFacts,
        current_owner_type: &str,
        domains: &[ReturnDomain],
        index_by_key: &HashMap<FnKey, usize>,
        common_modules: &HashMap<String, String>,
    ) {
        match atom {
            ReturnAtom::Known(t) => out.add_known(t.clone()),
            ReturnAtom::Unknown => out.has_dynamic = true,
            ReturnAtom::Call(target) => {
                if let Some(callee_idx) =
                    resolve_call_to_fn(current_owner_type, target, common_modules, index_by_key)
                {
                    out.join(&domains[callee_idx]);
                } else {
                    out.has_dynamic = true;
                }
            }
            ReturnAtom::Var(name) => {
                let key = name.to_lowercase();
                if !visited_vars.insert(key.clone()) {
                    out.has_dynamic = true;
                    return;
                }
                if let Some(values) = facts.vars.get(name) {
                    for v in values {
                        resolve_atom(
                            v,
                            out,
                            visited_vars,
                            facts,
                            current_owner_type,
                            domains,
                            index_by_key,
                            common_modules,
                        );
                    }
                } else {
                    out.has_dynamic = true;
                }
                visited_vars.remove(&key);
            }
        }
    }

    let mut out = ReturnDomain::default();
    out.has_dynamic |= facts.has_dynamic;

    if facts.has_return_without_value {
        out.add_known("Неопределено".to_string());
    }
    if facts.returns.is_empty() && !facts.has_return_without_value {
        // В BSL функция без return возвращает Неопределено.
        out.add_known("Неопределено".to_string());
    }

    for atom in &facts.returns {
        let mut visited_vars = HashSet::new();
        resolve_atom(
            atom,
            &mut out,
            &mut visited_vars,
            facts,
            current_owner_type,
            domains,
            index_by_key,
            common_modules,
        );
    }

    out
}
