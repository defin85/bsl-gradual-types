use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use bsl_shared::domain::is_configuration_type_pattern;
use bsl_shared::domain::resolver::TypeResolver;
use bsl_shared::domain::signature_index::SignatureIndex;
use bsl_shared::domain::types::MetadataKind;
use bsl_shared::domain::types::{
    Certainty, ResolutionMetadata, ResolutionResult, ResolutionSource,
};
use bsl_shared::domain::types::{ConcreteType, TypeResolution, UncertaintyReason, WeightedType};
use bsl_shared::domain::TypeMetadataLookup;
use bsl_shared::domain::{CodeLocation, ModuleType};
use bsl_syntax::ast::{Expression, Program, Statement};

use crate::ast_to_ir::{is_global_collection, lookup_global_collection};
use crate::implicit_bindings::{
    directive_disables_form_context, ImplicitBindingResolver, FORM_CONTEXT_BOUND_SYMBOL_KEYS,
};
use crate::SemanticDeps;

#[derive(Debug, Clone)]
pub(crate) struct TypeIndexEntry {
    pub span: bsl_shared::ir::Span,
    pub resolution: TypeResolution,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct TypeIndex {
    entries: Vec<TypeIndexEntry>,
}

impl TypeIndex {
    pub(crate) fn type_for_exact_span(&self, span: bsl_shared::ir::Span) -> Option<TypeResolution> {
        self.entries
            .iter()
            .find(|entry| entry.span == span)
            .map(|entry| entry.resolution.clone())
    }

    pub(crate) fn type_at_byte_offset(&self, byte_offset: u32) -> Option<TypeResolution> {
        let find = |offset: u32| {
            self.entries
                .iter()
                .filter(|entry| entry.span.contains(offset))
                .min_by_key(|entry| entry.span.len())
                .map(|entry| entry.resolution.clone())
        };

        // Аналогично IR `find_node_at_byte_offset`: если курсор на границе `end`,
        // пробуем сместиться на 1 байт влево.
        find(byte_offset).or_else(|| byte_offset.checked_sub(1).and_then(find))
    }
}

#[derive(Clone)]
struct TypeEnv {
    variables: HashMap<String, TypeResolution>,
    local_function_summaries: Arc<HashMap<String, LocalFunctionSummary>>,
}

impl Default for TypeEnv {
    fn default() -> Self {
        Self {
            variables: HashMap::new(),
            local_function_summaries: Arc::new(HashMap::new()),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct LocalFunctionSummary {
    return_type: TypeResolution,
    may_fallthrough: bool,
}

struct TypeInferencer {
    deps: Arc<SemanticDeps>,
    resolver: Arc<TypeResolver>,
    signature_index: SignatureIndex,
    metadata_lookup: TypeMetadataLookup,
}

impl TypeInferencer {
    fn new(deps: Arc<SemanticDeps>) -> Self {
        let resolver = deps
            .resolver
            .clone()
            .unwrap_or_else(|| Arc::new(TypeResolver::new(deps.repository.clone())));
        let signature_index = deps.signature_index.clone();
        let metadata_lookup = TypeMetadataLookup::new(deps.repository.clone());
        Self {
            deps,
            resolver,
            signature_index,
            metadata_lookup,
        }
    }

    fn build_index(&self, program: &Program, file_path: &str) -> TypeIndex {
        let mut env = TypeEnv::default();
        let mut index = TypeIndex::default();
        self.seed_module_context(file_path, &mut env);
        env.local_function_summaries = Arc::new(self.infer_local_function_summaries(program, &env));
        for stmt in &program.statements {
            self.visit_statement(stmt, &mut env, &mut index);
        }
        index
    }

    fn infer_local_function_summaries(
        &self,
        program: &Program,
        base_env: &TypeEnv,
    ) -> HashMap<String, LocalFunctionSummary> {
        #[derive(Clone, Copy, Debug)]
        struct Def<'a> {
            params: &'a [String],
            body: &'a [Statement],
        }

        fn collect_called_locals_in_expr(expr: &Expression, out: &mut BTreeSet<String>) {
            match expr {
                Expression::Call { function, args, .. } => {
                    if let Expression::Identifier { name, .. } = function.as_ref() {
                        out.insert(name.to_lowercase());
                    }
                    collect_called_locals_in_expr(function, out);
                    for arg in args {
                        collect_called_locals_in_expr(arg, out);
                    }
                }
                Expression::PropertyAccess { object, .. } => {
                    collect_called_locals_in_expr(object, out)
                }
                Expression::IndexAccess { object, index, .. } => {
                    collect_called_locals_in_expr(object, out);
                    collect_called_locals_in_expr(index, out);
                }
                Expression::Binary { left, right, .. } => {
                    collect_called_locals_in_expr(left, out);
                    collect_called_locals_in_expr(right, out);
                }
                Expression::Unary { operand, .. } => collect_called_locals_in_expr(operand, out),
                Expression::Ternary {
                    condition,
                    then_expr,
                    else_expr,
                    ..
                } => {
                    collect_called_locals_in_expr(condition, out);
                    collect_called_locals_in_expr(then_expr, out);
                    collect_called_locals_in_expr(else_expr, out);
                }
                Expression::New { args, .. } => {
                    for arg in args {
                        collect_called_locals_in_expr(arg, out);
                    }
                }
                Expression::Await { expression, .. } => {
                    collect_called_locals_in_expr(expression, out)
                }
                _ => {}
            }
        }

        fn collect_called_locals_in_stmt(stmt: &Statement, out: &mut BTreeSet<String>) {
            match stmt {
                Statement::Assignment { target, value, .. } => {
                    collect_called_locals_in_expr(target, out);
                    collect_called_locals_in_expr(value, out);
                }
                Statement::VarDeclaration { .. } => {}
                Statement::FunctionDecl { .. } | Statement::ProcedureDecl { .. } => {}
                Statement::If {
                    condition,
                    then_body,
                    else_body,
                    ..
                } => {
                    collect_called_locals_in_expr(condition, out);
                    for s in then_body {
                        collect_called_locals_in_stmt(s, out);
                    }
                    if let Some(else_body) = else_body {
                        for s in else_body {
                            collect_called_locals_in_stmt(s, out);
                        }
                    }
                }
                Statement::For {
                    start, end, body, ..
                } => {
                    collect_called_locals_in_expr(start, out);
                    collect_called_locals_in_expr(end, out);
                    for s in body {
                        collect_called_locals_in_stmt(s, out);
                    }
                }
                Statement::ForEach {
                    collection, body, ..
                } => {
                    collect_called_locals_in_expr(collection, out);
                    for s in body {
                        collect_called_locals_in_stmt(s, out);
                    }
                }
                Statement::While {
                    condition, body, ..
                } => {
                    collect_called_locals_in_expr(condition, out);
                    for s in body {
                        collect_called_locals_in_stmt(s, out);
                    }
                }
                Statement::Return { value, .. } => {
                    if let Some(v) = value {
                        collect_called_locals_in_expr(v, out);
                    }
                }
                Statement::Try {
                    try_body,
                    except_body,
                    ..
                } => {
                    for s in try_body {
                        collect_called_locals_in_stmt(s, out);
                    }
                    for s in except_body {
                        collect_called_locals_in_stmt(s, out);
                    }
                }
                Statement::Call { expression, .. } => {
                    collect_called_locals_in_expr(expression, out)
                }
                Statement::Execute { code, .. } => collect_called_locals_in_expr(code, out),
                Statement::RaiseError { message, .. } => {
                    if let Some(m) = message {
                        collect_called_locals_in_expr(m, out);
                    }
                }
                Statement::AddHandler { event, handler, .. }
                | Statement::RemoveHandler { event, handler, .. } => {
                    collect_called_locals_in_expr(event, out);
                    collect_called_locals_in_expr(handler, out);
                }
                Statement::Await { expression, .. } => {
                    collect_called_locals_in_expr(expression, out)
                }
                Statement::Break { .. }
                | Statement::Continue { .. }
                | Statement::Goto { .. }
                | Statement::Label { .. } => {}
            }
        }

        fn block_always_exits(body: &[Statement]) -> bool {
            for stmt in body {
                let exits = match stmt {
                    Statement::Return { .. } => true,
                    Statement::RaiseError { .. } => true,
                    Statement::If {
                        then_body,
                        else_body,
                        ..
                    } => else_body.as_ref().is_some_and(|else_body| {
                        block_always_exits(then_body) && block_always_exits(else_body)
                    }),
                    Statement::Try {
                        try_body,
                        except_body,
                        ..
                    } => block_always_exits(try_body) && block_always_exits(except_body),
                    _ => false,
                };
                if exits {
                    return true;
                }
            }
            false
        }

        #[derive(Debug, Clone)]
        struct LocalFunctionState {
            return_types: ReturnTypeSet,
        }

        impl LocalFunctionState {
            fn return_type(&self) -> TypeResolution {
                self.return_types.to_resolution()
            }
        }

        #[derive(Debug, Clone, PartialEq, Default)]
        struct ReturnTypeSet {
            /// If any return expression is unknown/dynamic, the whole return type degrades to Dynamic.
            has_dynamic: bool,
            // Key is a stable display name for deterministic ordering.
            concrete_variants: BTreeMap<String, ConcreteType>,
            // If a function returns a non-concrete type (e.g. Generic/Nullable/Intersection), we keep it structurally
            // only if it's the sole variant. Otherwise we degrade to Dynamic.
            non_concrete: Option<TypeResolution>,
        }

        impl ReturnTypeSet {
            fn insert_concrete(&mut self, concrete: ConcreteType) {
                let key = TypeResolution::known(concrete.clone()).type_name();
                if self.non_concrete.is_some() {
                    self.has_dynamic = true;
                    return;
                }
                self.concrete_variants.entry(key).or_insert(concrete);
            }

            fn insert_named(&mut self, type_name: &str) {
                self.insert_concrete(TypeResolution::string_to_concrete(type_name));
            }

            fn insert_resolution(&mut self, resolution: &TypeResolution) {
                // Dynamic/Unknown is the top type for our purposes.
                if resolution.is_unknown() || resolution.is_dynamic() {
                    self.has_dynamic = true;
                    return;
                }

                match &resolution.result {
                    ResolutionResult::Concrete(concrete) => self.insert_concrete(concrete.clone()),
                    ResolutionResult::Union(variants) => {
                        if variants.is_empty() {
                            self.has_dynamic = true;
                        } else {
                            for v in variants {
                                self.insert_concrete(v.type_.clone());
                            }
                        }
                    }
                    ResolutionResult::Dynamic => self.has_dynamic = true,
                    // Non-concrete return types are preserved only if they are the only return variant.
                    _ => {
                        if !self.concrete_variants.is_empty() {
                            self.has_dynamic = true;
                            return;
                        }
                        match self.non_concrete.as_ref() {
                            None => self.non_concrete = Some(resolution.clone()),
                            Some(existing) => {
                                if existing.type_name() != resolution.type_name() {
                                    self.has_dynamic = true;
                                }
                            }
                        }
                    }
                }
            }

            fn is_empty(&self) -> bool {
                !self.has_dynamic
                    && self.concrete_variants.is_empty()
                    && self.non_concrete.is_none()
            }

            fn to_resolution(&self) -> TypeResolution {
                if self.has_dynamic {
                    return TypeResolution::unknown();
                }
                if let Some(non_concrete) = self.non_concrete.as_ref() {
                    return non_concrete.clone();
                }
                if self.concrete_variants.is_empty() {
                    return TypeResolution::unknown();
                }
                if self.concrete_variants.len() == 1 {
                    let concrete = self
                        .concrete_variants
                        .values()
                        .next()
                        .expect("len=1")
                        .clone();
                    return TypeResolution {
                        certainty: Certainty::Inferred,
                        result: ResolutionResult::Concrete(concrete),
                        source: ResolutionSource::Inferred,
                        metadata: ResolutionMetadata::default(),
                        active_facet: None,
                        available_facets: vec![],
                    };
                }

                let variants: Vec<WeightedType> = self
                    .concrete_variants
                    .values()
                    .cloned()
                    .map(WeightedType::new)
                    .collect();
                TypeResolution {
                    certainty: Certainty::Inferred,
                    result: ResolutionResult::Union(variants),
                    source: ResolutionSource::Inferred,
                    metadata: ResolutionMetadata::default(),
                    active_facet: None,
                    available_facets: vec![],
                }
            }
        }

        fn collect_return_type_names(
            inferencer: &TypeInferencer,
            body: &[Statement],
            env: &mut TypeEnv,
            index: &mut TypeIndex,
            out: &mut ReturnTypeSet,
        ) {
            for stmt in body {
                match stmt {
                    Statement::Return { value, .. } => {
                        if let Some(expr) = value {
                            let t = inferencer.infer_expr(expr, env, index);
                            out.insert_resolution(&t);
                        } else {
                            out.insert_named("Неопределено");
                        }
                    }
                    Statement::If {
                        condition,
                        then_body,
                        else_body,
                        ..
                    } => {
                        let _ = inferencer.infer_expr(condition, env, index);
                        let mut then_env = env.clone();
                        collect_return_type_names(inferencer, then_body, &mut then_env, index, out);
                        if let Some(else_body) = else_body {
                            let mut else_env = env.clone();
                            collect_return_type_names(
                                inferencer,
                                else_body,
                                &mut else_env,
                                index,
                                out,
                            );
                        }
                    }
                    Statement::While {
                        condition, body, ..
                    } => {
                        let _ = inferencer.infer_expr(condition, env, index);
                        let mut body_env = env.clone();
                        collect_return_type_names(inferencer, body, &mut body_env, index, out);
                    }
                    Statement::For {
                        variable,
                        start,
                        end,
                        body,
                        ..
                    } => {
                        let _ = inferencer.infer_expr(start, env, index);
                        let _ = inferencer.infer_expr(end, env, index);
                        let mut body_env = env.clone();
                        body_env
                            .variables
                            .insert(variable.to_lowercase(), TypeResolution::primitive("Число"));
                        collect_return_type_names(inferencer, body, &mut body_env, index, out);
                    }
                    Statement::ForEach {
                        variable,
                        collection,
                        body,
                        ..
                    } => {
                        let _ = inferencer.infer_expr(collection, env, index);
                        let mut body_env = env.clone();
                        body_env
                            .variables
                            .insert(variable.to_lowercase(), TypeResolution::unknown());
                        collect_return_type_names(inferencer, body, &mut body_env, index, out);
                    }
                    Statement::Try {
                        try_body,
                        except_body,
                        ..
                    } => {
                        let mut try_env = env.clone();
                        collect_return_type_names(inferencer, try_body, &mut try_env, index, out);
                        let mut except_env = env.clone();
                        collect_return_type_names(
                            inferencer,
                            except_body,
                            &mut except_env,
                            index,
                            out,
                        );
                    }
                    Statement::FunctionDecl { .. } | Statement::ProcedureDecl { .. } => {}
                    _ => inferencer.visit_statement(stmt, env, index),
                }
            }
        }

        let mut function_defs: Vec<(String, Def<'_>)> = Vec::new();
        for stmt in &program.statements {
            if let Statement::FunctionDecl {
                name, params, body, ..
            } = stmt
            {
                function_defs.push((name.to_lowercase(), Def { params, body }));
            }
        }
        if function_defs.is_empty() {
            return HashMap::new();
        }

        // Stable node ordering: appearance in file.
        let n = function_defs.len();
        let mut name_to_idx: HashMap<String, usize> = HashMap::new();
        for (idx, (name_lower, _)) in function_defs.iter().enumerate() {
            name_to_idx.insert(name_lower.clone(), idx);
        }

        let mut may_fallthrough: Vec<bool> = Vec::with_capacity(n);
        for (_, def) in &function_defs {
            may_fallthrough.push(!block_always_exits(def.body));
        }

        // Call graph edges: caller -> callee (by index), only for local functions in this file.
        let mut edges: Vec<Vec<usize>> = vec![Vec::new(); n];
        for (caller_idx, (_, def)) in function_defs.iter().enumerate() {
            let mut called = BTreeSet::<String>::new();
            for stmt in def.body {
                collect_called_locals_in_stmt(stmt, &mut called);
            }
            for callee in called {
                if let Some(&callee_idx) = name_to_idx.get(&callee) {
                    edges[caller_idx].push(callee_idx);
                }
            }
            edges[caller_idx].sort_unstable();
            edges[caller_idx].dedup();
        }

        // Tarjan SCC
        fn scc_tarjan(edges: &[Vec<usize>]) -> Vec<Vec<usize>> {
            let n = edges.len();
            let mut index: usize = 0;
            let mut stack: Vec<usize> = Vec::new();
            let mut on_stack = vec![false; n];
            let mut indices: Vec<Option<usize>> = vec![None; n];
            let mut lowlink: Vec<usize> = vec![0; n];
            let mut sccs: Vec<Vec<usize>> = Vec::new();

            fn strongconnect(
                v: usize,
                index: &mut usize,
                stack: &mut Vec<usize>,
                on_stack: &mut [bool],
                indices: &mut [Option<usize>],
                lowlink: &mut [usize],
                edges: &[Vec<usize>],
                sccs: &mut Vec<Vec<usize>>,
            ) {
                indices[v] = Some(*index);
                lowlink[v] = *index;
                *index += 1;
                stack.push(v);
                on_stack[v] = true;

                for &w in &edges[v] {
                    if indices[w].is_none() {
                        strongconnect(w, index, stack, on_stack, indices, lowlink, edges, sccs);
                        lowlink[v] = lowlink[v].min(lowlink[w]);
                    } else if on_stack[w] {
                        lowlink[v] = lowlink[v].min(indices[w].expect("index set"));
                    }
                }

                if lowlink[v] == indices[v].expect("index set") {
                    let mut component = Vec::new();
                    loop {
                        let w = stack.pop().expect("stack pop");
                        on_stack[w] = false;
                        component.push(w);
                        if w == v {
                            break;
                        }
                    }
                    component.sort_unstable();
                    sccs.push(component);
                }
            }

            for v in 0..n {
                if indices[v].is_none() {
                    strongconnect(
                        v,
                        &mut index,
                        &mut stack,
                        &mut on_stack,
                        &mut indices,
                        &mut lowlink,
                        edges,
                        &mut sccs,
                    );
                }
            }

            sccs
        }

        let sccs = scc_tarjan(&edges);
        let mut node_to_scc = vec![0usize; n];
        for (scc_id, nodes) in sccs.iter().enumerate() {
            for &node in nodes {
                node_to_scc[node] = scc_id;
            }
        }

        // Condensation graph topo (reverse order: callees first).
        let scc_count = sccs.len();
        let mut scc_edges: Vec<BTreeSet<usize>> = vec![BTreeSet::new(); scc_count];
        let mut indegree: Vec<usize> = vec![0; scc_count];
        for (from, tos) in edges.iter().enumerate() {
            let from_scc = node_to_scc[from];
            for &to in tos {
                let to_scc = node_to_scc[to];
                if from_scc == to_scc {
                    continue;
                }
                if scc_edges[from_scc].insert(to_scc) {
                    indegree[to_scc] += 1;
                }
            }
        }

        let mut ready: Vec<usize> = (0..scc_count).filter(|&i| indegree[i] == 0).collect();
        // Deterministic ordering: smallest node index inside SCC first.
        ready.sort_by_key(|&scc_id| sccs[scc_id].first().copied().unwrap_or(usize::MAX));

        let mut topo: Vec<usize> = Vec::with_capacity(scc_count);
        while let Some(scc_id) = ready.first().copied() {
            ready.remove(0);
            topo.push(scc_id);
            for &to in &scc_edges[scc_id] {
                indegree[to] = indegree[to].saturating_sub(1);
                if indegree[to] == 0 {
                    ready.push(to);
                    ready
                        .sort_by_key(|&scc_id| sccs[scc_id].first().copied().unwrap_or(usize::MAX));
                }
            }
        }

        let mut states: Vec<LocalFunctionState> = (0..n)
            .map(|_| LocalFunctionState {
                return_types: ReturnTypeSet::default(),
            })
            .collect();

        // Process SCCs in reverse topo order so that callees are stabilized first.
        for &scc_id in topo.iter().rev() {
            let nodes = &sccs[scc_id];
            loop {
                let mut changed = false;

                // Snapshot: expose current return types to expression inference via env.local_function_summaries.
                let mut snapshot: HashMap<String, LocalFunctionSummary> = HashMap::new();
                for (idx, (name_lower, _def)) in function_defs.iter().enumerate() {
                    snapshot.insert(
                        name_lower.clone(),
                        LocalFunctionSummary {
                            return_type: states[idx].return_type(),
                            may_fallthrough: may_fallthrough[idx],
                        },
                    );
                }
                let snapshot = Arc::new(snapshot);

                for &node_idx in nodes {
                    let (_name_lower, def) = &function_defs[node_idx];

                    let mut fn_env = base_env.clone();
                    fn_env.local_function_summaries = snapshot.clone();
                    for p in def.params {
                        fn_env
                            .variables
                            .insert(p.to_lowercase(), TypeResolution::unknown());
                    }

                    let mut scratch_index = TypeIndex::default();
                    let mut return_types = ReturnTypeSet::default();
                    collect_return_type_names(
                        self,
                        def.body,
                        &mut fn_env,
                        &mut scratch_index,
                        &mut return_types,
                    );

                    if return_types.is_empty() || may_fallthrough[node_idx] {
                        return_types.insert_named("Неопределено");
                    }

                    if states[node_idx].return_types != return_types {
                        states[node_idx].return_types = return_types;
                        changed = true;
                    }
                }

                if !changed {
                    break;
                }
            }
        }

        let mut out: HashMap<String, LocalFunctionSummary> = HashMap::new();
        for (idx, (name_lower, _def)) in function_defs.iter().enumerate() {
            out.insert(
                name_lower.clone(),
                LocalFunctionSummary {
                    return_type: states[idx].return_type(),
                    may_fallthrough: may_fallthrough[idx],
                },
            );
        }

        out
    }

    fn seed_module_context(&self, file_path: &str, env: &mut TypeEnv) {
        let path = Path::new(file_path);
        let Ok(location) = CodeLocation::determine_from_path(path) else {
            return;
        };

        let binding_resolver = ImplicitBindingResolver::new();
        for binding in binding_resolver.bindings_for_module(&location.module_type) {
            let resolution = match binding.type_name.as_deref() {
                Some(type_name) => {
                    let resolved = self.resolver.resolve_expression_sync(type_name);
                    if resolved.is_unknown() {
                        if self.deps.repository.find_type(type_name).is_some() {
                            TypeResolution::explicit(type_name)
                        } else {
                            TypeResolution::inferred_weak(type_name)
                        }
                    } else {
                        resolved
                    }
                }
                None => TypeResolution::unknown(),
            };
            env.variables
                .insert(binding.name.to_lowercase(), resolution);
        }

        let ModuleType::FormModule {
            ref form_name,
            ref owner_type,
        } = location.module_type
        else {
            return;
        };

        let Some(type_names) = binding_resolver.form_module_type_names(owner_type, form_name)
        else {
            return;
        };
        let Some(form_type) = self.deps.repository.find_type(&type_names.form_type_name) else {
            return;
        };

        for prop in form_type.properties {
            let key = prop.name.to_lowercase();
            if env.variables.contains_key(&key) {
                continue;
            }
            if prop.prop_type.is_empty() {
                continue;
            }
            if prop.prop_type.contains("cfg:") {
                env.variables
                    .insert(key, TypeResolution::inferred(&prop.prop_type));
                continue;
            }
            if is_configuration_type_pattern(&prop.prop_type) {
                let resolved = self.resolver.resolve_expression_sync(&prop.prop_type);
                let resolved = if resolved.is_unknown() {
                    TypeResolution::inferred(&prop.prop_type)
                } else {
                    resolved
                };
                env.variables.insert(key, resolved);
                continue;
            }
            let resolved = if self.deps.repository.find_type(&prop.prop_type).is_some() {
                TypeResolution::explicit(&prop.prop_type)
            } else {
                self.try_resolve_configuration_type(&prop.prop_type)
                    .unwrap_or_else(|| self.resolver.resolve_expression_sync(&prop.prop_type))
            };
            env.variables.insert(key, resolved);
        }
    }

    fn visit_statement(&self, stmt: &Statement, env: &mut TypeEnv, index: &mut TypeIndex) {
        match stmt {
            Statement::VarDeclaration {
                name, type_hint, ..
            } => {
                let resolution = type_hint
                    .as_deref()
                    .map(TypeResolution::explicit)
                    .unwrap_or_else(TypeResolution::unknown);
                env.variables.insert(name.to_lowercase(), resolution);
            }
            Statement::Assignment { target, value, .. } => {
                let value_type = self.infer_expr(value, env, index);
                if let Expression::Identifier { name, .. } = target {
                    let key = name.to_lowercase();
                    env.variables.insert(key.clone(), value_type);
                    // Hover/type-at-position на имени переменной после присваивания
                    // должен видеть новый тип.
                    self.record(expr_span(target), env.variables[&key].clone(), index);
                }
            }
            Statement::If {
                condition,
                then_body,
                else_body,
                ..
            } => {
                let _ = self.infer_expr(condition, env, index);
                let mut then_env = env.clone();
                for stmt in then_body {
                    self.visit_statement(stmt, &mut then_env, index);
                }
                if let Some(else_body) = else_body {
                    let mut else_env = env.clone();
                    for stmt in else_body {
                        self.visit_statement(stmt, &mut else_env, index);
                    }
                }
            }
            Statement::While {
                condition, body, ..
            } => {
                let _ = self.infer_expr(condition, env, index);
                let mut body_env = env.clone();
                for stmt in body {
                    self.visit_statement(stmt, &mut body_env, index);
                }
            }
            Statement::For {
                variable,
                start,
                end,
                body,
                ..
            } => {
                let _ = self.infer_expr(start, env, index);
                let _ = self.infer_expr(end, env, index);
                let mut body_env = env.clone();
                body_env
                    .variables
                    .insert(variable.to_lowercase(), TypeResolution::primitive("Число"));
                for stmt in body {
                    self.visit_statement(stmt, &mut body_env, index);
                }
            }
            Statement::ForEach {
                variable,
                collection,
                body,
                ..
            } => {
                let _ = self.infer_expr(collection, env, index);
                let mut body_env = env.clone();
                body_env
                    .variables
                    .insert(variable.to_lowercase(), TypeResolution::unknown());
                for stmt in body {
                    self.visit_statement(stmt, &mut body_env, index);
                }
            }
            Statement::Return {
                value: Some(value), ..
            } => {
                let _ = self.infer_expr(value, env, index);
            }
            Statement::Return { value: None, .. } => {}
            Statement::Try {
                try_body,
                except_body,
                ..
            } => {
                let mut try_env = env.clone();
                for stmt in try_body {
                    self.visit_statement(stmt, &mut try_env, index);
                }
                let mut except_env = env.clone();
                for stmt in except_body {
                    self.visit_statement(stmt, &mut except_env, index);
                }
            }
            Statement::Call { expression, .. } => {
                let _ = self.infer_expr(expression, env, index);
            }
            Statement::Execute { code, .. } => {
                let _ = self.infer_expr(code, env, index);
            }
            Statement::RaiseError {
                message: Some(message),
                ..
            } => {
                let _ = self.infer_expr(message, env, index);
            }
            Statement::RaiseError { message: None, .. } => {}
            Statement::AddHandler { event, handler, .. }
            | Statement::RemoveHandler { event, handler, .. } => {
                let _ = self.infer_expr(event, env, index);
                let _ = self.infer_expr(handler, env, index);
            }
            Statement::Await { expression, .. } => {
                let _ = self.infer_expr(expression, env, index);
            }
            Statement::FunctionDecl {
                params,
                body,
                compiler_directive,
                ..
            }
            | Statement::ProcedureDecl {
                params,
                body,
                compiler_directive,
                ..
            } => {
                // TODO(v2): полноценное вычисление типов внутри функций на основе call graph.
                // Пока строим индекс внутри тела, наследуя module-level окружение
                // (например, implicit переменные модуля формы) и добавляя параметры.
                let mut fn_env = env.clone();
                if directive_disables_form_context(*compiler_directive) {
                    for key in FORM_CONTEXT_BOUND_SYMBOL_KEYS {
                        fn_env.variables.remove(key);
                    }
                }
                for param in params {
                    fn_env
                        .variables
                        .insert(param.to_lowercase(), TypeResolution::unknown());
                }
                for stmt in body {
                    self.visit_statement(stmt, &mut fn_env, index);
                }
            }
            _ => {}
        }
    }

    fn record(
        &self,
        span: bsl_shared::ir::Span,
        resolution: TypeResolution,
        index: &mut TypeIndex,
    ) {
        index.entries.push(TypeIndexEntry { span, resolution });
    }

    fn infer_expr(
        &self,
        expr: &Expression,
        env: &mut TypeEnv,
        index: &mut TypeIndex,
    ) -> TypeResolution {
        let resolution = match expr {
            Expression::Number { .. } => TypeResolution::primitive("Число"),
            Expression::String { .. } => TypeResolution::primitive("Строка"),
            Expression::Boolean { .. } => TypeResolution::primitive("Булево"),
            Expression::Date { .. } => TypeResolution::primitive("Дата"),
            Expression::Identifier { name, .. } => self.infer_identifier(name, env),
            Expression::New {
                type_name, args, ..
            } => {
                for arg in args {
                    let _ = self.infer_expr(arg, env, index);
                }
                self.infer_new_expression(type_name)
            }
            Expression::PropertyAccess {
                object, property, ..
            } => {
                let object_resolution = self.infer_expr(object, env, index);
                self.infer_property_access(&object_resolution, property)
            }
            Expression::Call { function, args, .. } => {
                for arg in args {
                    let _ = self.infer_expr(arg, env, index);
                }
                self.infer_call(function, env, index)
            }
            Expression::Binary {
                left,
                operator,
                right,
                ..
            } => {
                let left_type = self.infer_expr(left, env, index);
                let right_type = self.infer_expr(right, env, index);
                self.infer_binary(operator, &left_type, &right_type)
            }
            Expression::Unary { operand, .. } => self.infer_expr(operand, env, index),
            Expression::Ternary {
                condition,
                then_expr,
                else_expr,
                ..
            } => {
                let _ = self.infer_expr(condition, env, index);
                let then_type = self.infer_expr(then_expr, env, index);
                let else_type = self.infer_expr(else_expr, env, index);
                // TODO(v2): union типов.
                if then_type
                    .type_name()
                    .eq_ignore_ascii_case(&else_type.type_name())
                {
                    then_type
                } else {
                    TypeResolution::unknown()
                }
            }
            Expression::IndexAccess {
                object,
                index: index_expr,
                ..
            } => {
                let _ = self.infer_expr(object, env, index);
                let _ = self.infer_expr(index_expr, env, index);
                TypeResolution::unknown()
            }
            Expression::Await { expression, .. } => self.infer_expr(expression, env, index),
        };

        self.record(expr_span(expr), resolution.clone(), index);
        resolution
    }

    fn infer_identifier(&self, name: &str, env: &TypeEnv) -> TypeResolution {
        let name_lower = name.to_lowercase();
        if name_lower == "неопределено" || name_lower == "undefined" {
            return TypeResolution::primitive("Неопределено");
        }
        if name_lower == "null" {
            return TypeResolution::primitive("Null");
        }
        if matches!(name_lower.as_str(), "истина" | "ложь" | "true" | "false") {
            return TypeResolution::primitive("Булево");
        }

        if is_global_collection(name).is_some() {
            return TypeResolution::inferred(name);
        }

        if let Some(value) = env.variables.get(&name_lower) {
            return value.clone();
        }

        let common_module_type = format!("ОбщиеМодули.{}", name);
        if self
            .deps
            .repository
            .find_type(&common_module_type)
            .is_some()
            || !self
                .signature_index
                .get_type_methods(&common_module_type)
                .is_empty()
        {
            return TypeResolution::metadata_type(MetadataKind::CommonModule, name, None);
        }

        TypeResolution::undeclared_variable(name)
    }

    fn infer_new_expression(&self, type_name: &str) -> TypeResolution {
        let clean = type_name.trim().trim_end_matches("()").trim();
        match clean {
            "Массив" => TypeResolution::generic("Массив", &["?"], Certainty::InferredWeak),
            "Соответствие" => {
                TypeResolution::generic("Соответствие", &["?", "?"], Certainty::InferredWeak)
            }
            "Список" => TypeResolution::generic("Список", &["?"], Certainty::InferredWeak),
            _ => {
                if self.deps.repository.find_type(clean).is_some() {
                    TypeResolution::explicit(clean)
                } else {
                    let mut res = TypeResolution::primitive(clean);
                    res.certainty = Certainty::Unknown;
                    res.metadata.uncertainty_reason = Some(UncertaintyReason::TypeNotFound {
                        name: clean.to_string(),
                    });
                    res
                }
            }
        }
    }

    fn infer_property_access(
        &self,
        object_type: &TypeResolution,
        property: &str,
    ) -> TypeResolution {
        let base_type = object_type.type_name();
        if let Some(info) = lookup_global_collection(&base_type) {
            // Справочники.Контрагенты -> СправочникМенеджер.Контрагенты
            let manager = format!("{}.{}", info.item_manager_type, property);
            return self.resolver.resolve_expression_sync(&manager);
        }

        let property_key = property.to_lowercase();
        let properties = self.metadata_lookup.get_properties(object_type);
        let properties = if properties.is_empty() {
            self.deps
                .repository
                .find_type(&object_type.type_name())
                .map(|t| t.properties)
                .unwrap_or_default()
        } else {
            properties
        };
        if let Some(prop) = properties
            .into_iter()
            .find(|p| p.name.to_lowercase() == property_key)
        {
            if let Some(resolved) = self.try_resolve_configuration_type(&prop.prop_type) {
                return resolved;
            }
            if self.deps.repository.find_type(&prop.prop_type).is_some() {
                return self.resolver.resolve_expression_sync(&prop.prop_type);
            }
            // Типы свойств из metadata (в т.ч. синтетические UI-типы форм вроде "ГруппаФормы")
            // должны возвращаться даже если их документация не загружена в repository.
            return TypeResolution::inferred(&prop.prop_type);
        }

        TypeResolution::unknown()
    }

    fn infer_call(
        &self,
        function: &Expression,
        env: &mut TypeEnv,
        index: &mut TypeIndex,
    ) -> TypeResolution {
        match function {
            Expression::Identifier { name, .. } => self.infer_global_function_call(name, env),
            Expression::PropertyAccess {
                object, property, ..
            } => {
                let receiver = self.infer_expr(object, env, index);
                self.infer_method_call(&receiver, property)
            }
            _ => TypeResolution::unknown(),
        }
    }

    fn infer_global_function_call(&self, name: &str, env: &TypeEnv) -> TypeResolution {
        let name_lower = name.to_lowercase();
        if let Some(local) = env.local_function_summaries.get(&name_lower) {
            return local.return_type.clone();
        }

        if let Some(sig) = self.signature_index.find_global_function(name) {
            if let Some(return_type) = sig.return_type.as_deref().filter(|s| !s.is_empty()) {
                if let Some(resolved) = self.try_resolve_configuration_type(return_type) {
                    return resolved;
                }
                return self.resolver.resolve_expression_sync(return_type);
            }
        }
        TypeResolution::unknown()
    }

    fn infer_method_call(&self, receiver: &TypeResolution, method: &str) -> TypeResolution {
        let type_name = signature_lookup_type_name(receiver);
        let metadata_name = SignatureIndex::extract_metadata_name(&type_name);
        let concretize_return_type = |return_type: &str| -> String {
            let Some(metadata_name) = metadata_name else {
                return return_type.to_string();
            };

            // Подставляем имя объекта только когда return type действительно шаблонный:
            // - содержит placeholder "<...>" / "&lt;...&gt;"
            // - или является фасетным базовым типом без ".Имя"
            //
            // Это снижает риск перезаписать уже-конкретизированный return type
            // (например "СправочникСсылка.Номенклатура").
            if return_type.contains('<')
                || return_type.contains("&lt;")
                || !return_type.contains('.')
            {
                let substituted = SignatureIndex::substitute_type_name(return_type, metadata_name);
                if substituted != return_type {
                    return substituted;
                }
            }

            return_type.to_string()
        };

        if let Some(sig) = self.signature_index.find_method(&type_name, method) {
            if let Some(return_type) = sig.return_type.as_deref().filter(|s| !s.is_empty()) {
                let return_type = concretize_return_type(return_type);
                if let Some(resolved) = self.try_resolve_configuration_type(&return_type) {
                    return resolved;
                }
                return self.resolver.resolve_expression_sync(&return_type);
            }
        }

        let methods = self.metadata_lookup.get_methods(receiver);
        let method_key = method.to_lowercase();
        if let Some(m) = methods
            .into_iter()
            .find(|m| m.name.to_lowercase() == method_key)
        {
            if let Some(return_type) = (!m.return_type.is_empty()).then_some(m.return_type) {
                let return_type = concretize_return_type(&return_type);
                if let Some(resolved) = self.try_resolve_configuration_type(&return_type) {
                    return resolved;
                }
                return self.resolver.resolve_expression_sync(&return_type);
            }
        }

        TypeResolution::unknown()
    }

    fn infer_binary(
        &self,
        operator: &str,
        left_type: &TypeResolution,
        right_type: &TypeResolution,
    ) -> TypeResolution {
        match operator {
            "+" => {
                let left_is_string = left_type.type_name().eq_ignore_ascii_case("Строка");
                let right_is_string = right_type.type_name().eq_ignore_ascii_case("Строка");

                if left_is_string && right_is_string {
                    return TypeResolution::primitive("Строка");
                }

                if left_is_string || right_is_string {
                    let mut res = TypeResolution::primitive("Строка");
                    res.certainty = Certainty::Unknown;
                    res.metadata.uncertainty_reason =
                        Some(UncertaintyReason::InvalidStringConcatenation {
                            left_type: left_type.type_name().to_string(),
                            right_type: right_type.type_name().to_string(),
                        });
                    return res;
                }

                TypeResolution::primitive("Число")
            }
            "-" | "*" | "/" => TypeResolution::primitive("Число"),
            "=" | "<>" | ">" | "<" | ">=" | "<=" => TypeResolution::primitive("Булево"),
            _ => TypeResolution::unknown(),
        }
    }

    fn try_resolve_configuration_type(&self, type_name: &str) -> Option<TypeResolution> {
        if is_configuration_type_pattern(type_name) {
            return Some(self.resolver.resolve_expression_sync(type_name));
        }
        None
    }
}

fn expr_span(expr: &Expression) -> bsl_shared::ir::Span {
    match expr {
        Expression::Identifier { span, .. }
        | Expression::String { span, .. }
        | Expression::Number { span, .. }
        | Expression::Boolean { span, .. }
        | Expression::Date { span, .. }
        | Expression::Call { span, .. }
        | Expression::Binary { span, .. }
        | Expression::Unary { span, .. }
        | Expression::Ternary { span, .. }
        | Expression::New { span, .. }
        | Expression::PropertyAccess { span, .. }
        | Expression::IndexAccess { span, .. }
        | Expression::Await { span, .. } => *span,
    }
}

fn signature_lookup_type_name(resolution: &TypeResolution) -> String {
    let type_name = resolution.type_name();
    type_name
        .split('<')
        .next()
        .unwrap_or(type_name.as_str())
        .trim()
        .to_string()
}

pub(crate) fn build_type_index_with_path(
    program: &Program,
    file_path: &str,
    deps: Arc<SemanticDeps>,
) -> TypeIndex {
    TypeInferencer::new(deps).build_index(program, file_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bsl_shared::domain::repository::InMemoryTypeRepository;
    use bsl_shared::domain::signature_index::{MethodSignature, SignatureSource};
    use bsl_shared::domain::type_id::TypeId;
    use bsl_shared::domain::types::{
        FacetKind, MetadataKind, PrimitiveType, RawDataSource, RawPropertyData, RawTypeData,
    };
    use bsl_shared::TypeRepository;
    use bsl_syntax::ParseOptions;

    fn parse(code: &str) -> Program {
        let parsed = bsl_syntax::parse(code, &ParseOptions::default()).expect("parse ok");
        parsed.program
    }

    fn deps_with_array_method() -> Arc<SemanticDeps> {
        let repository_impl = Arc::new(InMemoryTypeRepository::new());
        repository_impl
            .load_types(vec![RawTypeData {
                name: "Массив".to_string(),
                source: RawDataSource::Platform,
                ..Default::default()
            }])
            .expect("load types");

        let mut sigs = SignatureIndex::new();
        sigs.add_platform_method(
            TypeId::new("Массив"),
            MethodSignature::new(
                "Количество".to_string(),
                Some("Массив".to_string()),
                vec![],
                Some("Число".to_string()),
                None,
                None,
                SignatureSource::Platform,
                None,
                Default::default(),
            ),
        );
        repository_impl.set_signature_index(sigs.clone());

        let repository =
            repository_impl.clone() as Arc<dyn bsl_shared::domain::repository::TypeRepository>;
        let resolver = Arc::new(TypeResolver::new(repository.clone()));

        Arc::new(SemanticDeps {
            repository,
            signature_index: sigs,
            resolver: Some(resolver),
            platform_signatures_loaded: true,
        })
    }

    fn deps_with_common_module_method() -> Arc<SemanticDeps> {
        let repository_impl = Arc::new(InMemoryTypeRepository::new());
        let mut sigs = SignatureIndex::new();
        sigs.add_config_method(
            TypeId::new("ОбщиеМодули.ОбщийМодуль1"),
            MethodSignature::new(
                "Ф1".to_string(),
                Some("ОбщиеМодули.ОбщийМодуль1".to_string()),
                vec![],
                Some("Число".to_string()),
                None,
                None,
                SignatureSource::Configuration,
                None,
                Default::default(),
            ),
        );
        repository_impl.set_signature_index(sigs.clone());

        let repository =
            repository_impl.clone() as Arc<dyn bsl_shared::domain::repository::TypeRepository>;
        let resolver = Arc::new(TypeResolver::new(repository.clone()));

        Arc::new(SemanticDeps {
            repository,
            signature_index: sigs,
            resolver: Some(resolver),
            platform_signatures_loaded: true,
        })
    }

    fn deps_with_document_create_document_method() -> Arc<SemanticDeps> {
        let repository_impl = Arc::new(InMemoryTypeRepository::new());
        repository_impl
            .load_types(vec![RawTypeData {
                name: "Документы.РеализацияТоваровУслуг".to_string(),
                source: RawDataSource::Configuration,
                facets: vec![FacetKind::Manager, FacetKind::Object, FacetKind::Reference],
                kind: Some(MetadataKind::Document),
                ..Default::default()
            }])
            .expect("load config document type");

        let mut sigs = SignatureIndex::new();
        sigs.add_platform_method(
            TypeId::new("ДокументМенеджер"),
            MethodSignature::new(
                "СоздатьДокумент".to_string(),
                Some("ДокументМенеджер".to_string()),
                vec![],
                Some("ДокументОбъект.<Имя документа>".to_string()),
                None,
                None,
                SignatureSource::Platform,
                None,
                Default::default(),
            ),
        );
        repository_impl.set_signature_index(sigs.clone());

        let repository =
            repository_impl.clone() as Arc<dyn bsl_shared::domain::repository::TypeRepository>;
        let resolver = Arc::new(TypeResolver::new(repository.clone()));

        Arc::new(SemanticDeps {
            repository,
            signature_index: sigs,
            resolver: Some(resolver),
            platform_signatures_loaded: true,
        })
    }

    #[test]
    fn builds_type_index_for_simple_assignment_and_method_call() {
        let source = r#"Перем М;
М = Новый Массив();
Р = М.Количество();
"#;
        let program = parse(source);
        let deps = deps_with_array_method();
        let index = build_type_index_with_path(&program, "test.bsl", deps);

        let array_ident_offset = source
            .find("\nМ =")
            .map(|idx| idx + 1)
            .expect("assignment line start") as u32;
        let array_ident = index
            .type_at_byte_offset(array_ident_offset)
            .expect("type at assignment");
        assert_eq!(array_ident.type_name(), "Массив<Неопределено>");

        let method_call_offset = source.find("Количество").expect("method name") as u32;
        let method_call = index
            .type_at_byte_offset(method_call_offset)
            .expect("type at method call");
        assert_eq!(method_call.type_name(), "Число");
    }

    #[test]
    fn resolves_common_module_method_return_type_from_signature_index() {
        let source = r#"Процедура Тест()
    x = ОбщийМодуль1.Ф1();
КонецПроцедуры
"#;
        let program = parse(source);
        let deps = deps_with_common_module_method();
        let index = build_type_index_with_path(&program, "test.bsl", deps);

        let offset = source.find("Ф1").expect("method name") as u32;
        let result = index
            .type_at_byte_offset(offset)
            .expect("type at method call");
        assert_eq!(result.type_name(), "Число");
    }

    #[test]
    fn resolves_local_function_return_type_defined_later_in_common_module_file() {
        let source = r#"Процедура Тест()
    x = ФункцияКотораяВозвращаетСтроку();
КонецПроцедуры

Функция ФункцияКотораяВозвращаетСтроку()
    Возврат "ТестоваяСтрока";
КонецФункции
"#;
        let program = parse(source);
        let deps = deps_with_array_method();
        let file_path = "CommonModules/АвансовыйОтчетФормы/Ext/Module.bsl";
        let index = build_type_index_with_path(&program, file_path, deps);

        let offset = source
            .find("ФункцияКотораяВозвращаетСтроку")
            .expect("function name") as u32;
        let result = index
            .type_at_byte_offset(offset)
            .expect("type at function call");
        assert_eq!(result.type_name(), "Строка");
    }

    #[test]
    fn substitutes_placeholder_return_type_for_document_method_call() {
        let source = r#"Процедура Тест()
    Док = Документы.РеализацияТоваровУслуг.СоздатьДокумент();
КонецПроцедуры
"#;
        let program = parse(source);
        let deps = deps_with_document_create_document_method();
        let index = build_type_index_with_path(&program, "test.bsl", deps);

        let offset = source
            .find("СоздатьДокумент()")
            .map(|idx| idx + "СоздатьДокумент".len())
            .expect("method call") as u32;
        let result = index
            .type_at_byte_offset(offset)
            .expect("type at method call");
        assert_eq!(
            result.type_name(),
            "ДокументОбъект.РеализацияТоваровУслуг",
            "Expected placeholder <Имя документа> to be substituted from receiver metadata name"
        );
        assert!(!result.is_unknown());
    }

    #[test]
    fn infers_union_return_type_for_local_function() {
        let source = r#"Функция F(Флаг)
    Если Флаг Тогда
        Возврат 1;
    Иначе
        Возврат "x";
    КонецЕсли;
КонецФункции

Процедура Тест()
    x = F(Истина);
КонецПроцедуры
"#;
        let program = parse(source);
        let deps = deps_with_array_method();
        let index = build_type_index_with_path(&program, "test.bsl", deps);

        let offset = source.find("F(Истина)").expect("call") as u32;
        let result = index.type_at_byte_offset(offset).expect("type at call");
        assert_eq!(result.type_name(), "Строка | Число");
    }

    #[test]
    fn propagates_union_return_type_through_local_function_call() {
        let source = r#"Функция B(Флаг)
    Если Флаг Тогда
        Возврат 1;
    Иначе
        Возврат "x";
    КонецЕсли;
КонецФункции

Функция A(Флаг)
    Возврат B(Флаг);
КонецФункции

Процедура Тест()
    x = A(Истина);
КонецПроцедуры
"#;
        let program = parse(source);
        let deps = deps_with_array_method();
        let index = build_type_index_with_path(&program, "test.bsl", deps);

        let offset = source.find("A(Истина)").expect("call") as u32;
        let result = index.type_at_byte_offset(offset).expect("type at call");

        match result.result {
            ResolutionResult::Union(variants) => {
                assert!(
                    variants
                        .iter()
                        .any(|v| matches!(v.type_, ConcreteType::Primitive(PrimitiveType::String))),
                    "expected String variant, got: {:?}",
                    variants
                );
                assert!(
                    variants
                        .iter()
                        .any(|v| matches!(v.type_, ConcreteType::Primitive(PrimitiveType::Number))),
                    "expected Number variant, got: {:?}",
                    variants
                );
            }
            other => panic!("expected Union, got: {:?}", other),
        }
    }

    #[test]
    fn adds_undefined_when_function_can_fallthrough() {
        let source = r#"Функция F(Флаг)
    Если Флаг Тогда
        Возврат 1;
    КонецЕсли;
КонецФункции

Процедура Тест()
    x = F(Истина);
КонецПроцедуры
"#;
        let program = parse(source);
        let deps = deps_with_array_method();
        let index = build_type_index_with_path(&program, "test.bsl", deps);

        let offset = source.find("F(Истина)").expect("call") as u32;
        let result = index.type_at_byte_offset(offset).expect("type at call");
        assert_eq!(result.type_name(), "Неопределено | Число");
    }

    #[test]
    fn mutual_recursion_is_deterministic_and_terminates() {
        let source = r#"Функция A()
    Возврат B();
КонецФункции

Функция B()
    Возврат A();
КонецФункции

Процедура Тест()
    x = A();
КонецПроцедуры
"#;
        let program = parse(source);
        let deps = deps_with_array_method();
        let index = build_type_index_with_path(&program, "test.bsl", deps);

        let offset = source.find("A();").expect("call") as u32;
        let result = index.type_at_byte_offset(offset).expect("type at call");
        assert!(
            result.is_unknown() && matches!(result.result, ResolutionResult::Dynamic),
            "expected Unknown/Dynamic, got: {:?}",
            result
        );
    }

    #[test]
    fn seeds_form_module_context_for_elements_property_access() {
        let repository_impl = Arc::new(InMemoryTypeRepository::new());
        repository_impl
            .load_types(vec![
                RawTypeData {
                    name: "Формы.Документы.Док1.Форма1".to_string(),
                    source: RawDataSource::Configuration,
                    ..Default::default()
                },
                RawTypeData {
                    name: "ЭлементыФормы.Документы.Док1.Форма1".to_string(),
                    source: RawDataSource::Configuration,
                    properties: vec![RawPropertyData {
                        name: "СчетФактураПросмотр".to_string(),
                        prop_type: "ГруппаФормы".to_string(),
                        is_readonly: false,
                    }],
                    ..Default::default()
                },
                RawTypeData {
                    name: "ГруппаФормы".to_string(),
                    source: RawDataSource::Platform,
                    ..Default::default()
                },
            ])
            .expect("load types");

        let repository =
            repository_impl.clone() as Arc<dyn bsl_shared::domain::repository::TypeRepository>;
        let resolver = Arc::new(TypeResolver::new(repository.clone()));

        let deps = Arc::new(SemanticDeps {
            repository,
            signature_index: SignatureIndex::new(),
            resolver: Some(resolver),
            platform_signatures_loaded: true,
        });

        let source = r#"Процедура Тест()
    x = Элементы.СчетФактураПросмотр;
КонецПроцедуры
"#;
        let program = parse(source);
        let file_path = "Documents/Док1/Forms/Форма1/Ext/Form/Module.bsl";
        let loc = CodeLocation::determine_from_path(Path::new(file_path)).expect("code location");
        assert!(
            matches!(loc.module_type, ModuleType::FormModule { .. }),
            "expected FormModule for seed path, got {:?}",
            loc.module_type
        );
        assert!(
            repository_impl
                .find_type("Формы.Документы.Док1.Форма1")
                .is_some(),
            "expected synthetic form type to be present"
        );
        assert!(
            repository_impl
                .find_type("ЭлементыФормы.Документы.Док1.Форма1")
                .is_some(),
            "expected synthetic form elements type to be present"
        );

        let index = build_type_index_with_path(&program, file_path, deps);

        let receiver_offset = source.find("Элементы").expect("Элементы") as u32;
        let receiver = index
            .type_at_byte_offset(receiver_offset)
            .expect("type at Элементы");
        assert_eq!(
            receiver.type_name(),
            "ЭлементыФормы.Документы.Док1.Форма1",
            "receiver should be seeded from form module context"
        );

        let member_offset = source.find("СчетФактураПросмотр").expect("member") as u32;
        let member = index
            .type_at_byte_offset(member_offset)
            .expect("type at member access");
        assert_eq!(member.type_name(), "ГруппаФормы");
    }

    #[test]
    fn seeds_form_module_context_for_this_object_and_parameters() {
        let repository_impl = Arc::new(InMemoryTypeRepository::new());
        repository_impl
            .load_types(vec![
                RawTypeData {
                    name: "Формы.Документы.Док1.Форма1".to_string(),
                    source: RawDataSource::Configuration,
                    ..Default::default()
                },
                RawTypeData {
                    name: "Документы.Док1".to_string(),
                    source: RawDataSource::Configuration,
                    facets: vec![FacetKind::Manager, FacetKind::Object, FacetKind::Reference],
                    kind: Some(MetadataKind::Document),
                    ..Default::default()
                },
            ])
            .expect("load types");

        let repository =
            repository_impl.clone() as Arc<dyn bsl_shared::domain::repository::TypeRepository>;
        let resolver = Arc::new(TypeResolver::new(repository.clone()));
        let deps = Arc::new(SemanticDeps {
            repository,
            signature_index: SignatureIndex::new(),
            resolver: Some(resolver),
            platform_signatures_loaded: true,
        });

        let source = r#"Процедура Тест()
    x = ЭтотОбъект;
    y = Параметры;
    z = Объект;
КонецПроцедуры
"#;
        let program = parse(source);
        let file_path = "Documents/Док1/Forms/Форма1/Ext/Form/Module.bsl";
        let index = build_type_index_with_path(&program, file_path, deps);

        let this_object_offset = source.find("ЭтотОбъект").expect("ЭтотОбъект") as u32;
        let this_object = index
            .type_at_byte_offset(this_object_offset)
            .expect("type at ЭтотОбъект");
        assert_eq!(this_object.type_name(), "Формы.Документы.Док1.Форма1");

        let params_offset = source.find("Параметры").expect("Параметры") as u32;
        let params = index
            .type_at_byte_offset(params_offset)
            .expect("type at Параметры");
        assert_eq!(params.type_name(), "Структура");

        let object_offset = source
            .find("z = Объект")
            .map(|idx| idx + "z = ".len())
            .expect("Объект") as u32;
        let object = index
            .type_at_byte_offset(object_offset)
            .expect("type at Объект");
        assert_eq!(object.type_name(), "ДокументОбъект.Док1");
    }

    #[test]
    fn resolves_form_module_object_link_property_from_object_facet() {
        let repository_impl = Arc::new(InMemoryTypeRepository::new());
        repository_impl
            .load_types(vec![
                RawTypeData {
                    name: "Документы.Док1".to_string(),
                    source: RawDataSource::Configuration,
                    facets: vec![FacetKind::Manager, FacetKind::Object, FacetKind::Reference],
                    kind: Some(MetadataKind::Document),
                    ..Default::default()
                },
                RawTypeData {
                    name: "ДокументОбъект".to_string(),
                    source: RawDataSource::Platform,
                    facets: vec![FacetKind::Object],
                    properties: vec![RawPropertyData {
                        name: "Ссылка".to_string(),
                        prop_type: "ДокументСсылка".to_string(),
                        is_readonly: true,
                    }],
                    ..Default::default()
                },
                RawTypeData {
                    name: "ДокументСсылка".to_string(),
                    source: RawDataSource::Platform,
                    facets: vec![FacetKind::Reference],
                    ..Default::default()
                },
                RawTypeData {
                    name: "Формы.Документы.Док1.Форма1".to_string(),
                    source: RawDataSource::Configuration,
                    ..Default::default()
                },
            ])
            .expect("load types");

        let repository =
            repository_impl.clone() as Arc<dyn bsl_shared::domain::repository::TypeRepository>;
        let resolver = Arc::new(TypeResolver::new(repository.clone()));
        let deps = Arc::new(SemanticDeps {
            repository,
            signature_index: SignatureIndex::new(),
            resolver: Some(resolver),
            platform_signatures_loaded: true,
        });

        let source = r#"Процедура Тест()
    x = Объект.Ссылка;
КонецПроцедуры
"#;
        let program = parse(source);
        let file_path = "Documents/Док1/Forms/Форма1/Ext/Form/Module.bsl";
        let index = build_type_index_with_path(&program, file_path, deps);

        let link_offset = source.find("Ссылка").expect("Ссылка") as u32;
        let link_type = index
            .type_at_byte_offset(link_offset)
            .expect("type at Объект.Ссылка");
        assert_eq!(link_type.type_name(), "ДокументСсылка");
    }

    #[test]
    fn seeds_manager_module_context_for_this_object_and_object() {
        let deps = deps_with_array_method();
        let source = r#"Процедура Тест()
    x = ЭтотОбъект;
    y = Объект;
КонецПроцедуры
"#;
        let program = parse(source);
        let file_path = "Documents/Док1/Ext/ManagerModule.bsl";
        let index = build_type_index_with_path(&program, file_path, deps);

        let this_object_offset = source.find("ЭтотОбъект").expect("ЭтотОбъект") as u32;
        let this_object = index
            .type_at_byte_offset(this_object_offset)
            .expect("type at ЭтотОбъект");
        assert_eq!(this_object.type_name(), "ДокументМенеджер.Док1");

        let object_offset = source.find("Объект").expect("Объект") as u32;
        let object = index
            .type_at_byte_offset(object_offset)
            .expect("type at Объект");
        assert_eq!(object.type_name(), "ДокументМенеджер.Док1");
    }

    #[test]
    fn seeds_object_module_context_for_this_object_and_object() {
        let deps = deps_with_array_method();
        let source = r#"Процедура Тест()
    x = ЭтотОбъект;
    y = Объект;
КонецПроцедуры
"#;
        let program = parse(source);
        let file_path = "Documents/Док1/Ext/ObjectModule.bsl";
        let index = build_type_index_with_path(&program, file_path, deps);

        let this_object_offset = source.find("ЭтотОбъект").expect("ЭтотОбъект") as u32;
        let this_object = index
            .type_at_byte_offset(this_object_offset)
            .expect("type at ЭтотОбъект");
        assert_eq!(this_object.type_name(), "ДокументОбъект.Док1");

        let object_offset = source.find("Объект").expect("Объект") as u32;
        let object = index
            .type_at_byte_offset(object_offset)
            .expect("type at Объект");
        assert_eq!(object.type_name(), "ДокументОбъект.Док1");
    }

    #[test]
    fn seeds_recordset_module_context_for_this_object_and_object() {
        let deps = deps_with_array_method();
        let source = r#"Процедура Тест()
    x = ЭтотОбъект;
    y = Объект;
КонецПроцедуры
"#;
        let program = parse(source);
        let file_path = "InformationRegisters/Регистр1/Ext/RecordSetModule.bsl";
        let index = build_type_index_with_path(&program, file_path, deps);

        let this_object_offset = source.find("ЭтотОбъект").expect("ЭтотОбъект") as u32;
        let this_object = index
            .type_at_byte_offset(this_object_offset)
            .expect("type at ЭтотОбъект");
        assert_eq!(
            this_object.type_name(),
            "РегистрСведенийНаборЗаписей.Регистр1"
        );

        let object_offset = source.find("Объект").expect("Объект") as u32;
        let object = index
            .type_at_byte_offset(object_offset)
            .expect("type at Объект");
        assert_eq!(object.type_name(), "РегистрСведенийНаборЗаписей.Регистр1");
    }

    #[test]
    fn no_context_directive_hides_form_context_symbols() {
        let deps = deps_with_array_method();
        let source = r#"&НаСервереБезКонтекста
Процедура Тест()
    x = ЭтотОбъект;
КонецПроцедуры
"#;
        let program = parse(source);
        let file_path = "Documents/Док1/Forms/Форма1/Ext/Form/Module.bsl";
        let index = build_type_index_with_path(&program, file_path, deps);

        let this_object_offset = source.find("ЭтотОбъект").expect("ЭтотОбъект") as u32;
        let this_object = index
            .type_at_byte_offset(this_object_offset)
            .expect("type at ЭтотОбъект");
        assert_eq!(
            this_object.is_undeclared_variable(),
            Some("ЭтотОбъект"),
            "expected ЭтотОбъект to be undeclared in *БезКонтекста"
        );
    }
}
