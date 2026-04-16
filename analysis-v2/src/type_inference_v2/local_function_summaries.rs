use super::*;
use bsl_shared::ir::Span;

impl TypeInferencer<'_> {
    pub(super) fn infer_local_function_summaries(
        &self,
        program: &Program,
        base_env: &TypeEnv,
    ) -> LocalFunctionSummariesProfiled {
        #[derive(Clone, Copy, Debug)]
        struct Def<'a> {
            params: &'a [String],
            body: &'a [Statement],
            span: Span,
            is_function: bool,
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

        fn summary_for_idx(
            idx: usize,
            function_defs: &[(String, Def<'_>)],
            states: &[LocalFunctionState],
            may_fallthrough: &[bool],
        ) -> LocalFunctionSummary {
            let (_name_lower, def) = &function_defs[idx];
            LocalFunctionSummary {
                return_type: states[idx].return_type(),
                may_fallthrough: may_fallthrough[idx],
                params: def.params.to_vec(),
                declaration_span: def.span,
                is_function: def.is_function,
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
            inferencer: &TypeInferencer<'_>,
            body: &[Statement],
            env: &mut TypeEnv,
            facts: &mut SemanticFacts,
            out: &mut ReturnTypeSet,
        ) {
            for stmt in body {
                inferencer.cancellation_checkpoint();
                match stmt {
                    Statement::Return { value, .. } => {
                        if let Some(expr) = value {
                            let t = inferencer.infer_expr(expr, env, facts);
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
                        let _ = inferencer.infer_expr(condition, env, facts);
                        let mut then_env = env.clone();
                        collect_return_type_names(inferencer, then_body, &mut then_env, facts, out);
                        if let Some(else_body) = else_body {
                            let mut else_env = env.clone();
                            collect_return_type_names(
                                inferencer,
                                else_body,
                                &mut else_env,
                                facts,
                                out,
                            );
                        }
                    }
                    Statement::While {
                        condition, body, ..
                    } => {
                        let _ = inferencer.infer_expr(condition, env, facts);
                        let mut body_env = env.clone();
                        collect_return_type_names(inferencer, body, &mut body_env, facts, out);
                    }
                    Statement::For {
                        variable,
                        start,
                        end,
                        body,
                        ..
                    } => {
                        let _ = inferencer.infer_expr(start, env, facts);
                        let _ = inferencer.infer_expr(end, env, facts);
                        let mut body_env = env.clone();
                        body_env
                            .variables
                            .insert(variable.to_lowercase(), TypeResolution::primitive("Число"));
                        collect_return_type_names(inferencer, body, &mut body_env, facts, out);
                    }
                    Statement::ForEach {
                        variable,
                        collection,
                        body,
                        ..
                    } => {
                        let _ = inferencer.infer_expr(collection, env, facts);
                        let mut body_env = env.clone();
                        body_env
                            .variables
                            .insert(variable.to_lowercase(), TypeResolution::unknown());
                        collect_return_type_names(inferencer, body, &mut body_env, facts, out);
                    }
                    Statement::Try {
                        try_body,
                        except_body,
                        ..
                    } => {
                        let mut try_env = env.clone();
                        collect_return_type_names(inferencer, try_body, &mut try_env, facts, out);
                        let mut except_env = env.clone();
                        collect_return_type_names(
                            inferencer,
                            except_body,
                            &mut except_env,
                            facts,
                            out,
                        );
                    }
                    Statement::FunctionDecl { .. } | Statement::ProcedureDecl { .. } => {}
                    _ => inferencer.visit_statement(stmt, env, facts),
                }
            }
        }

        let prep_started = Instant::now();
        let mut function_defs: Vec<(String, Def<'_>)> = Vec::new();
        for stmt in &program.statements {
            self.cancellation_checkpoint();
            match stmt {
                Statement::FunctionDecl {
                    name,
                    params,
                    body,
                    span,
                    ..
                } => {
                    function_defs.push((
                        name.to_lowercase(),
                        Def {
                            params,
                            body,
                            span: *span,
                            is_function: true,
                        },
                    ));
                }
                Statement::ProcedureDecl {
                    name,
                    params,
                    body,
                    span,
                    ..
                } => {
                    function_defs.push((
                        name.to_lowercase(),
                        Def {
                            params,
                            body,
                            span: *span,
                            is_function: false,
                        },
                    ));
                }
                _ => {}
            }
        }
        if function_defs.is_empty() {
            return LocalFunctionSummariesProfiled {
                summaries: HashMap::new(),
                profile: LocalFunctionSummariesProfile::default(),
            };
        }

        // Stable node ordering: appearance in file.
        let n = function_defs.len();
        let mut name_to_idx: HashMap<String, usize> = HashMap::new();
        for (idx, (name_lower, _)) in function_defs.iter().enumerate() {
            self.cancellation_checkpoint();
            name_to_idx.insert(name_lower.clone(), idx);
        }

        let mut may_fallthrough: Vec<bool> = Vec::with_capacity(n);
        for (_, def) in &function_defs {
            self.cancellation_checkpoint();
            may_fallthrough.push(!block_always_exits(def.body));
        }

        // Call graph edges: caller -> callee (by index), only for local functions in this file.
        let mut edges: Vec<Vec<usize>> = vec![Vec::new(); n];
        for (caller_idx, (_, def)) in function_defs.iter().enumerate() {
            self.cancellation_checkpoint();
            let mut called = BTreeSet::<String>::new();
            for stmt in def.body {
                self.cancellation_checkpoint();
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
        let prep_ms = prep_started.elapsed().as_millis();

        // Tarjan SCC
        fn scc_tarjan(edges: &[Vec<usize>]) -> Vec<Vec<usize>> {
            let n = edges.len();
            struct TarjanCtx<'a> {
                index: usize,
                stack: Vec<usize>,
                on_stack: Vec<bool>,
                indices: Vec<Option<usize>>,
                lowlink: Vec<usize>,
                edges: &'a [Vec<usize>],
                sccs: Vec<Vec<usize>>,
            }

            impl<'a> TarjanCtx<'a> {
                fn strongconnect(&mut self, v: usize) {
                    self.indices[v] = Some(self.index);
                    self.lowlink[v] = self.index;
                    self.index += 1;
                    self.stack.push(v);
                    self.on_stack[v] = true;

                    let neighbors = self.edges[v].clone();
                    for w in neighbors {
                        if self.indices[w].is_none() {
                            self.strongconnect(w);
                            self.lowlink[v] = self.lowlink[v].min(self.lowlink[w]);
                        } else if self.on_stack[w] {
                            self.lowlink[v] =
                                self.lowlink[v].min(self.indices[w].expect("index set"));
                        }
                    }

                    if self.lowlink[v] == self.indices[v].expect("index set") {
                        let mut component = Vec::new();
                        loop {
                            let w = self.stack.pop().expect("stack pop");
                            self.on_stack[w] = false;
                            component.push(w);
                            if w == v {
                                break;
                            }
                        }
                        component.sort_unstable();
                        self.sccs.push(component);
                    }
                }
            }

            let mut ctx = TarjanCtx {
                index: 0,
                stack: Vec::new(),
                on_stack: vec![false; n],
                indices: vec![None; n],
                lowlink: vec![0; n],
                edges,
                sccs: Vec::new(),
            };

            for v in 0..n {
                if ctx.indices[v].is_none() {
                    ctx.strongconnect(v);
                }
            }

            ctx.sccs
        }

        let fixed_point_setup_started = Instant::now();
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
        let scc_count = sccs.len() as u64;
        let fixed_point_setup_ms = fixed_point_setup_started.elapsed().as_millis();

        let mut states: Vec<LocalFunctionState> = (0..n)
            .map(|_| LocalFunctionState {
                return_types: ReturnTypeSet::default(),
            })
            .collect();

        // Process SCCs in reverse topo order so that callees are stabilized first.
        let stable_summaries =
            Rc::new(RefCell::new(HashMap::<String, LocalFunctionSummary>::new()));
        let mut fixed_point_ms = 0_u128;
        let mut snapshot_build_ms = 0_u128;
        let mut body_infer_ms = 0_u128;
        let mut fixed_point_iteration_count = 0_u64;
        let mut singleton_fast_path_count = 0_u64;
        let mut recursive_scc_count = 0_u64;
        for &scc_id in topo.iter().rev() {
            let nodes = &sccs[scc_id];
            let singleton_non_recursive = nodes.len() == 1 && !edges[nodes[0]].contains(&nodes[0]);
            if singleton_non_recursive {
                singleton_fast_path_count = singleton_fast_path_count.saturating_add(1);
                let node_idx = nodes[0];
                let (_name_lower, def) = &function_defs[node_idx];

                let mut fn_env = base_env.clone();
                fn_env.local_function_summaries =
                    LocalFunctionSummaryLookup::stable(stable_summaries.clone());
                for p in def.params {
                    fn_env
                        .variables
                        .insert(p.to_lowercase(), TypeResolution::unknown());
                }

                let mut scratch_facts = SemanticFacts::default();
                let mut return_types = ReturnTypeSet::default();
                let body_infer_started = Instant::now();
                collect_return_type_names(
                    self,
                    def.body,
                    &mut fn_env,
                    &mut scratch_facts,
                    &mut return_types,
                );
                body_infer_ms =
                    body_infer_ms.saturating_add(body_infer_started.elapsed().as_millis());

                if return_types.is_empty() || may_fallthrough[node_idx] {
                    return_types.insert_named("Неопределено");
                }

                states[node_idx].return_types = return_types;
                stable_summaries.borrow_mut().insert(
                    function_defs[node_idx].0.clone(),
                    summary_for_idx(node_idx, &function_defs, &states, &may_fallthrough),
                );
                continue;
            }

            recursive_scc_count = recursive_scc_count.saturating_add(1);
            let recursive_started = Instant::now();
            loop {
                fixed_point_iteration_count = fixed_point_iteration_count.saturating_add(1);
                let mut changed = false;

                // Expose stable out-of-SCC summaries plus current-SCC overlay to expression inference.
                let snapshot_build_started = Instant::now();
                let mut overlay: HashMap<String, LocalFunctionSummary> =
                    HashMap::with_capacity(nodes.len());
                for &idx in nodes {
                    overlay.insert(
                        function_defs[idx].0.clone(),
                        summary_for_idx(idx, &function_defs, &states, &may_fallthrough),
                    );
                }
                let lookup = LocalFunctionSummaryLookup::overlay(stable_summaries.clone(), overlay);
                snapshot_build_ms =
                    snapshot_build_ms.saturating_add(snapshot_build_started.elapsed().as_millis());

                for &node_idx in nodes {
                    let (_name_lower, def) = &function_defs[node_idx];

                    let mut fn_env = base_env.clone();
                    fn_env.local_function_summaries = lookup.clone();
                    for p in def.params {
                        fn_env
                            .variables
                            .insert(p.to_lowercase(), TypeResolution::unknown());
                    }

                    let mut scratch_facts = SemanticFacts::default();
                    let mut return_types = ReturnTypeSet::default();
                    let body_infer_started = Instant::now();
                    collect_return_type_names(
                        self,
                        def.body,
                        &mut fn_env,
                        &mut scratch_facts,
                        &mut return_types,
                    );
                    body_infer_ms =
                        body_infer_ms.saturating_add(body_infer_started.elapsed().as_millis());

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
            fixed_point_ms = fixed_point_ms.saturating_add(recursive_started.elapsed().as_millis());
            let mut stable_summaries_mut = stable_summaries.borrow_mut();
            for &node_idx in nodes {
                stable_summaries_mut.insert(
                    function_defs[node_idx].0.clone(),
                    summary_for_idx(node_idx, &function_defs, &states, &may_fallthrough),
                );
            }
        }

        let mut out: HashMap<String, LocalFunctionSummary> = HashMap::new();
        for (idx, (name_lower, _def)) in function_defs.iter().enumerate() {
            out.insert(
                name_lower.clone(),
                summary_for_idx(idx, &function_defs, &states, &may_fallthrough),
            );
        }

        LocalFunctionSummariesProfiled {
            summaries: out,
            profile: LocalFunctionSummariesProfile {
                prep_ms: prep_ms.saturating_add(fixed_point_setup_ms),
                fixed_point_ms,
                snapshot_build_ms,
                body_infer_ms,
                function_count: n as u64,
                scc_count,
                fixed_point_iteration_count,
                singleton_fast_path_count,
                recursive_scc_count,
            },
        }
    }
}
