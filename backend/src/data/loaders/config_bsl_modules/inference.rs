use std::collections::HashMap;

use bsl_shared::domain::code_location::ModuleType;

use super::types::{CallTarget, ParsedModule};
use super::utils::{
    normalize_union_parts, resolve_manager_owner_type_from_receiver, split_union_string,
};

pub(crate) fn infer_export_param_types_across_modules(
    modules: &[ParsedModule],
) -> HashMap<(String, String), Vec<Option<String>>> {
    let mut export_param_slots: HashMap<(String, String), usize> = HashMap::new();

    // 1) Собираем все экспортные сигнатуры (owner_type + method_name -> param_count)
    for m in modules {
        for d in &m.decls {
            if !d.is_export {
                continue;
            }
            export_param_slots.insert((m.owner_type_name.clone(), d.name.clone()), d.params.len());
        }
    }

    // 2) Собираем наблюдения по вызовам.
    // Ключ: (owner_type_name, method_name, param_index) -> vec типов аргумента.
    let mut observations: HashMap<(String, String, usize), Vec<String>> = HashMap::new();

    // Карта CommonModuleName -> owner_type_name ("ОбщиеМодули.<Name>")
    let mut common_module_owner_types: HashMap<String, String> = HashMap::new();
    for m in modules {
        if let ModuleType::CommonModule { name, .. } = &m.module_type {
            common_module_owner_types.insert(name.clone(), m.owner_type_name.clone());
        }
    }

    for module in modules {
        // Для локальных вызовов резолвим только внутри текущего модуля (иначе не отличить от глобального namespace).
        let local_exports: HashMap<String, usize> = module
            .decls
            .iter()
            .filter(|d| d.is_export)
            .map(|d| (d.name.clone(), d.params.len()))
            .collect();

        for call in &module.call_sites {
            let arg_types = &call.arg_types;

            match &call.target {
                CallTarget::LocalFunction { name } => {
                    let Some(_param_count) = local_exports.get(name) else {
                        continue;
                    };
                    for (idx, t) in arg_types.iter().enumerate() {
                        let Some(t) = t else { continue };
                        for part in split_union_string(t) {
                            observations
                                .entry((module.owner_type_name.clone(), name.clone(), idx))
                                .or_default()
                                .push(part);
                        }
                    }
                }
                CallTarget::QualifiedMethod { receiver, name } => {
                    // Common module call: "ИмяМодуля.Метод()"
                    if receiver.len() == 1 {
                        if let Some(owner) = common_module_owner_types.get(&receiver[0]) {
                            if export_param_slots.contains_key(&(owner.clone(), name.clone())) {
                                for (idx, t) in arg_types.iter().enumerate() {
                                    let Some(t) = t else { continue };
                                    for part in split_union_string(t) {
                                        observations
                                            .entry((owner.clone(), name.clone(), idx))
                                            .or_default()
                                            .push(part);
                                    }
                                }
                                continue;
                            }
                        }
                    }

                    // Manager call: "Справочники.<X>.Метод()"/"Documents.<X>.Method()"
                    if receiver.len() == 2 {
                        if let Some(owner) = resolve_manager_owner_type_from_receiver(receiver) {
                            if export_param_slots.contains_key(&(owner.clone(), name.clone())) {
                                for (idx, t) in arg_types.iter().enumerate() {
                                    let Some(t) = t else { continue };
                                    for part in split_union_string(t) {
                                        observations
                                            .entry((owner.clone(), name.clone(), idx))
                                            .or_default()
                                            .push(part);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // 3) Финализируем в (owner_type, method_name) -> Vec<Option<union>>
    let mut out: HashMap<(String, String), Vec<Option<String>>> = HashMap::new();
    for ((owner, name), param_count) in export_param_slots {
        out.insert((owner, name), vec![None; param_count]);
    }

    for ((owner, name, idx), types) in observations {
        let union = normalize_union_parts(types);
        let union = (!union.is_empty()).then(|| union.join(" | "));
        if let Some(v) = out.get_mut(&(owner, name)) {
            if idx < v.len() {
                v[idx] = union;
            }
        }
    }

    out
}
