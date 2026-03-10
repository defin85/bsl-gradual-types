use super::*;

pub(super) fn is_implicit_context_symbol(name: &str) -> bool {
    let lowered = name.to_lowercase();
    IMPLICIT_CONTEXT_SYMBOL_KEYS.contains(&lowered.as_str())
}

pub(super) fn resolve_loop_body_scope(
    ir_program: &SemanticProgram,
    parent_scope: ScopeId,
    body: &[usize],
    loop_variable: Option<&str>,
    loop_span_start: u32,
) -> Option<ScopeId> {
    if let Some(scope_id) = body
        .iter()
        .filter_map(|idx| ir_program.nodes.get(*idx).map(|node| node.scope_id))
        .next()
    {
        return Some(scope_id);
    }

    let parent = ir_program.get_scope(parent_scope)?;
    match loop_variable {
        Some(variable) => parent.children.iter().copied().find(|child| {
            ir_program
                .get_scope(*child)
                .and_then(|scope| scope.variables.get(variable))
                .map(|state| state.declaration_span.start == loop_span_start)
                .unwrap_or(false)
        }),
        None => parent.children.first().copied(),
    }
}

pub(super) fn scope_from_body_nodes(
    ir_program: &SemanticProgram,
    body: &[usize],
) -> Option<ScopeId> {
    body.iter()
        .filter_map(|idx| ir_program.nodes.get(*idx).map(|node| node.scope_id))
        .next()
}

pub(super) fn body_bounds(ir_program: &SemanticProgram, body: &[usize]) -> Option<(u32, u32)> {
    let mut start: Option<u32> = None;
    let mut end: Option<u32> = None;

    for node_index in body {
        let Some(node) = ir_program.nodes.get(*node_index) else {
            continue;
        };
        start = Some(start.map_or(node.span.start, |value| value.min(node.span.start)));
        end = Some(end.map_or(node.span.end, |value| value.max(node.span.end)));
    }

    match (start, end) {
        (Some(start), Some(end)) => Some((start, end)),
        _ => None,
    }
}

pub(super) fn completion_scope_for_enclosing_node(
    ir_program: &SemanticProgram,
    node: &bsl_shared::ir::SemanticNode,
    byte_offset: u32,
) -> ScopeId {
    fn in_bounds(byte_offset: u32, start: u32, end: u32) -> bool {
        start <= byte_offset && byte_offset < end
    }

    match &node.kind {
        SemanticNodeKind::FunctionDeclaration { body_scope, .. }
        | SemanticNodeKind::ProcedureDeclaration { body_scope, .. } => *body_scope,
        SemanticNodeKind::BlockScope { scope_id, .. } => *scope_id,
        SemanticNodeKind::IfStatement {
            then_branch,
            else_branch,
            ..
        } => {
            let then_scope = scope_from_body_nodes(ir_program, then_branch);
            let then_bounds = body_bounds(ir_program, then_branch);
            let else_scope = else_branch
                .as_ref()
                .and_then(|body| scope_from_body_nodes(ir_program, body));
            let else_bounds = else_branch
                .as_ref()
                .and_then(|body| body_bounds(ir_program, body));

            if let (Some(scope), Some((else_start, _))) = (else_scope, else_bounds) {
                if node.span.contains(byte_offset) && byte_offset >= else_start {
                    return scope;
                }
            }

            if let (Some(scope), Some((then_start, then_end))) = (then_scope, then_bounds) {
                if in_bounds(byte_offset, then_start, then_end) {
                    return scope;
                }

                if let Some(else_scope) = else_scope {
                    if node.span.contains(byte_offset) && byte_offset > then_end {
                        return else_scope;
                    }
                    if node.span.contains(byte_offset) {
                        return node.scope_id;
                    }
                    return node.scope_id;
                }

                if node.span.contains(byte_offset) {
                    return scope;
                }
                return node.scope_id;
            }

            if node.span.contains(byte_offset) {
                return then_scope.or(else_scope).unwrap_or(node.scope_id);
            }

            node.scope_id
        }
        SemanticNodeKind::TryExcept {
            try_body,
            except_body,
        } => {
            let try_scope = scope_from_body_nodes(ir_program, try_body);
            let try_bounds = body_bounds(ir_program, try_body);
            let except_scope = scope_from_body_nodes(ir_program, except_body);
            let except_bounds = body_bounds(ir_program, except_body);

            if let (Some(scope), Some((except_start, _))) = (except_scope, except_bounds) {
                if node.span.contains(byte_offset) && byte_offset >= except_start {
                    return scope;
                }
            }

            if let (Some(scope), Some((try_start, try_end))) = (try_scope, try_bounds) {
                if in_bounds(byte_offset, try_start, try_end) {
                    return scope;
                }

                if let Some(except_scope) = except_scope {
                    if node.span.contains(byte_offset) && byte_offset > try_end {
                        return except_scope;
                    }
                    if node.span.contains(byte_offset) {
                        return node.scope_id;
                    }
                    return node.scope_id;
                }

                if node.span.contains(byte_offset) {
                    return scope;
                }
                return node.scope_id;
            }

            if node.span.contains(byte_offset) {
                return try_scope.or(except_scope).unwrap_or(node.scope_id);
            }

            node.scope_id
        }
        SemanticNodeKind::ForLoop { variable, body, .. }
        | SemanticNodeKind::ForEachLoop { variable, body, .. } => resolve_loop_body_scope(
            ir_program,
            node.scope_id,
            body,
            Some(variable.as_str()),
            node.span.start,
        )
        .unwrap_or(node.scope_id),
        SemanticNodeKind::WhileLoop { body, .. } => {
            resolve_loop_body_scope(ir_program, node.scope_id, body, None, node.span.start)
                .unwrap_or(node.scope_id)
        }
        _ => node.scope_id,
    }
}

pub(super) fn resolve_completion_scope_position(
    ir_program: &SemanticProgram,
    file_content: &str,
    line: u32,
    column: u32,
) -> Option<CompletionScopePosition> {
    let line_index = LineIndex::new(file_content);
    let byte_offset = line_index.utf16_position_to_byte_offset(file_content, line, column);
    let byte_offset: u32 = byte_offset.try_into().ok()?;

    let scope_id = {
        let from_node = (0u32..=32)
            .filter_map(|delta| byte_offset.checked_sub(delta))
            .find_map(|offset| ir_program.find_node_at_byte_offset(offset))
            .map(|node| completion_scope_for_enclosing_node(ir_program, node, byte_offset));

        let from_enclosing_decl = || {
            ir_program
                .nodes
                .iter()
                .filter(|node| node.span.contains(byte_offset))
                .filter_map(|node| match &node.kind {
                    SemanticNodeKind::FunctionDeclaration { body_scope, .. }
                    | SemanticNodeKind::ProcedureDeclaration { body_scope, .. } => {
                        Some((node.span.len(), *body_scope))
                    }
                    _ => None,
                })
                .min_by_key(|(len, _)| *len)
                .map(|(_, scope_id)| scope_id)
        };

        let from_prev_node = || {
            ir_program
                .nodes
                .iter()
                .filter(|node| node.span.start < byte_offset)
                .max_by_key(|node| node.span.start)
                .map(|node| match &node.kind {
                    SemanticNodeKind::FunctionDeclaration { body_scope, .. }
                    | SemanticNodeKind::ProcedureDeclaration { body_scope, .. } => *body_scope,
                    SemanticNodeKind::BlockScope { scope_id, .. } => *scope_id,
                    _ => node.scope_id,
                })
        };

        from_node
            .or_else(from_enclosing_decl)
            .or_else(from_prev_node)?
    };

    let mut visible_scopes = Vec::new();
    let mut current_scope_id = Some(scope_id);
    while let Some(sid) = current_scope_id {
        visible_scopes.push(sid);
        current_scope_id = ir_program.get_scope(sid).and_then(|scope| scope.parent);
    }

    let scope_rank = visible_scopes
        .iter()
        .copied()
        .enumerate()
        .map(|(idx, sid)| (sid, idx))
        .collect();

    Some(CompletionScopePosition {
        byte_offset,
        scope_rank,
    })
}

pub(super) fn collect_local_candidates_from_ir(
    ir_program: &SemanticProgram,
    scope_position: &CompletionScopePosition,
) -> Vec<LocalSymbolCandidate> {
    let mut best_by_name: HashMap<String, LocalSymbolCandidate> = HashMap::new();
    {
        let mut push_candidate = |name: &str, scope_id: ScopeId, span_start: u32| {
            push_local_candidate_if_visible(
                ir_program,
                scope_position,
                &mut best_by_name,
                name,
                scope_id,
                span_start,
                false,
            );
        };

        let enclosing_function_scope = scope_position
            .scope_rank
            .iter()
            .filter_map(|(scope_id, rank)| {
                let scope = ir_program.get_scope(*scope_id)?;
                matches!(scope.kind, ScopeKind::Function).then_some((*rank, *scope_id))
            })
            .min_by_key(|(rank, _)| *rank)
            .map(|(_, scope_id)| scope_id);

        let mut collected_from_routine = false;
        if let Some(function_scope_id) = enclosing_function_scope {
            if let Some(decl_node) = ir_program.nodes.iter().find(|node| match &node.kind {
                SemanticNodeKind::FunctionDeclaration { body_scope, .. }
                | SemanticNodeKind::ProcedureDeclaration { body_scope, .. } => {
                    *body_scope == function_scope_id
                }
                _ => false,
            }) {
                match &decl_node.kind {
                    SemanticNodeKind::FunctionDeclaration { params, body, .. }
                    | SemanticNodeKind::ProcedureDeclaration { params, body, .. } => {
                        for param in params {
                            push_candidate(&param.name, function_scope_id, decl_node.span.start);
                        }
                        collect_local_candidates_from_body(
                            ir_program,
                            scope_position,
                            body,
                            &mut push_candidate,
                        );
                        collected_from_routine = true;
                    }
                    _ => {}
                }
            }
        }

        if !collected_from_routine {
            for node in ir_program.nodes.iter() {
                match &node.kind {
                    SemanticNodeKind::VariableDeclaration { name, .. } => {
                        push_candidate(name, node.scope_id, node.span.start);
                    }
                    SemanticNodeKind::Assignment { variable, .. } => {
                        push_candidate(variable, node.scope_id, node.span.start);
                    }
                    SemanticNodeKind::FunctionDeclaration {
                        params, body_scope, ..
                    }
                    | SemanticNodeKind::ProcedureDeclaration {
                        params, body_scope, ..
                    } => {
                        for param in params {
                            push_candidate(&param.name, *body_scope, node.span.start);
                        }
                    }
                    SemanticNodeKind::ForLoop { variable, body, .. }
                    | SemanticNodeKind::ForEachLoop { variable, body, .. } => {
                        if let Some(loop_scope) = resolve_loop_body_scope(
                            ir_program,
                            node.scope_id,
                            body,
                            Some(variable.as_str()),
                            node.span.start,
                        ) {
                            push_candidate(variable, loop_scope, node.span.start);
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    // Дополняем кандидатов только implicit symbols из SymbolTable:
    // они могут не иметь отдельных AST/IR-узлов.
    for scope_id in scope_position.scope_rank.keys().copied() {
        let Some(scope) = ir_program.get_scope(scope_id) else {
            continue;
        };
        for (name, state) in scope.variables.iter() {
            if is_implicit_context_symbol(name) {
                push_local_candidate_if_visible(
                    ir_program,
                    scope_position,
                    &mut best_by_name,
                    name,
                    scope_id,
                    state.declaration_span.start,
                    true,
                );
            }
        }
    }

    let mut out: Vec<LocalSymbolCandidate> = best_by_name.into_values().collect();
    out.sort_by(|left, right| {
        let left_rank = scope_position
            .scope_rank
            .get(&left.scope_id)
            .copied()
            .unwrap_or(usize::MAX);
        let right_rank = scope_position
            .scope_rank
            .get(&right.scope_id)
            .copied()
            .unwrap_or(usize::MAX);
        left_rank
            .cmp(&right_rank)
            .then_with(|| right.span_start.cmp(&left.span_start))
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
    });

    out
}

pub(super) fn collect_local_candidates_from_body<F>(
    ir_program: &SemanticProgram,
    scope_position: &CompletionScopePosition,
    body: &[usize],
    push_candidate: &mut F,
) where
    F: FnMut(&str, ScopeId, u32),
{
    for node_index in body {
        collect_local_candidates_from_node(ir_program, scope_position, *node_index, push_candidate);
    }
}

pub(super) fn collect_local_candidates_from_node<F>(
    ir_program: &SemanticProgram,
    scope_position: &CompletionScopePosition,
    node_index: usize,
    push_candidate: &mut F,
) where
    F: FnMut(&str, ScopeId, u32),
{
    let Some(node) = ir_program.nodes.get(node_index) else {
        return;
    };
    if node.span.start > scope_position.byte_offset {
        return;
    }

    match &node.kind {
        SemanticNodeKind::VariableDeclaration { name, .. } => {
            push_candidate(name, node.scope_id, node.span.start);
        }
        SemanticNodeKind::Assignment { variable, .. } => {
            push_candidate(variable, node.scope_id, node.span.start);
        }
        SemanticNodeKind::IfStatement {
            then_branch,
            else_branch,
            ..
        } => {
            collect_local_candidates_from_body(
                ir_program,
                scope_position,
                then_branch,
                push_candidate,
            );
            if let Some(else_branch) = else_branch.as_ref() {
                collect_local_candidates_from_body(
                    ir_program,
                    scope_position,
                    else_branch,
                    push_candidate,
                );
            }
        }
        SemanticNodeKind::TryExcept {
            try_body,
            except_body,
        } => {
            collect_local_candidates_from_body(
                ir_program,
                scope_position,
                try_body,
                push_candidate,
            );
            collect_local_candidates_from_body(
                ir_program,
                scope_position,
                except_body,
                push_candidate,
            );
        }
        SemanticNodeKind::WhileLoop { body, .. } => {
            collect_local_candidates_from_body(ir_program, scope_position, body, push_candidate);
        }
        SemanticNodeKind::ForLoop { variable, body, .. }
        | SemanticNodeKind::ForEachLoop { variable, body, .. } => {
            if let Some(loop_scope) = resolve_loop_body_scope(
                ir_program,
                node.scope_id,
                body,
                Some(variable.as_str()),
                node.span.start,
            ) {
                push_candidate(variable, loop_scope, node.span.start);
            }
            collect_local_candidates_from_body(ir_program, scope_position, body, push_candidate);
        }
        SemanticNodeKind::BlockScope { statements, .. } => {
            collect_local_candidates_from_body(
                ir_program,
                scope_position,
                statements,
                push_candidate,
            );
        }
        _ => {}
    }
}

pub(super) fn push_local_candidate_if_visible(
    ir_program: &SemanticProgram,
    scope_position: &CompletionScopePosition,
    best_by_name: &mut HashMap<String, LocalSymbolCandidate>,
    name: &str,
    scope_id: ScopeId,
    span_start: u32,
    allow_global: bool,
) {
    if span_start > scope_position.byte_offset {
        return;
    }

    let Some(candidate_rank) = scope_position.scope_rank.get(&scope_id).copied() else {
        return;
    };
    let Some(scope) = ir_program.get_scope(scope_id) else {
        return;
    };
    if !allow_global && matches!(scope.kind, ScopeKind::Global) {
        return;
    }

    let candidate = LocalSymbolCandidate {
        name: name.to_string(),
        scope_id,
        span_start,
    };
    let key = name.to_lowercase();

    let should_replace = match best_by_name.get(&key) {
        None => true,
        Some(existing) => {
            let existing_rank = scope_position
                .scope_rank
                .get(&existing.scope_id)
                .copied()
                .unwrap_or(usize::MAX);
            candidate_rank < existing_rank
                || (candidate_rank == existing_rank && span_start > existing.span_start)
        }
    };

    if should_replace {
        best_by_name.insert(key, candidate);
    }
}

pub(super) fn add_local_symbols_from_ir(
    analysis: Option<&CompletionAnalysisContext<'_>>,
    file_content: &str,
    line: u32,
    column: u32,
    target: &mut Vec<Candidate>,
    priority: u8,
) {
    let Some(ctx) = analysis else {
        return;
    };
    let Some(ir_program) = ctx.ir_program.as_deref() else {
        return;
    };
    let Some(scope_position) =
        resolve_completion_scope_position(ir_program, file_content, line, column)
    else {
        return;
    };

    for local in collect_local_candidates_from_ir(ir_program, &scope_position) {
        target.push(Candidate::new(
            CompletionItem::new(local.name, CompletionKind::Variable),
            priority,
            None,
            None,
            Some(SymbolScope::Local),
        ));
    }
}

pub(super) fn completion_scope_contains_local_symbol(
    analysis: Option<&CompletionAnalysisContext<'_>>,
    file_content: &str,
    line: u32,
    column: u32,
    name: &str,
) -> bool {
    let Some(ctx) = analysis else {
        return false;
    };
    let Some(ir_program) = ctx.ir_program.as_deref() else {
        return false;
    };
    let Some(scope_position) =
        resolve_completion_scope_position(ir_program, file_content, line, column)
    else {
        return false;
    };

    let target = name.to_lowercase();
    collect_local_candidates_from_ir(ir_program, &scope_position)
        .into_iter()
        .any(|local| local.name.to_lowercase() == target)
}

pub(super) fn add_symbols(
    snapshot: &IndexSnapshot,
    file_uri: Option<&str>,
    target: &mut Vec<Candidate>,
    priority: u8,
    include_local: bool,
) {
    let Some(uri) = file_uri else {
        return;
    };
    let Some(items) = snapshot.symbol_index.get(uri) else {
        return;
    };

    for item in items.iter() {
        if matches!(item.scope, Some(SymbolScope::Local)) {
            if !include_local {
                continue;
            }
            let allow_unbound_local_routine = matches!(
                item.kind,
                IndexItemKind::Symbol(SymbolKind::Function | SymbolKind::Procedure)
            );
            if item.uri.as_deref() != Some(uri) && !allow_unbound_local_routine {
                continue;
            }
        }
        let kind = completion_kind_from_index_item(item);
        target.push(Candidate::new(
            CompletionItem::new(item.name.clone(), kind),
            priority,
            None,
            None,
            item.scope,
        ));
    }
}

pub(super) fn add_module_symbols(
    snapshot: &IndexSnapshot,
    target: &mut Vec<Candidate>,
    priority: u8,
) {
    for items in snapshot.module_index.values() {
        for item in items.iter() {
            let kind = completion_kind_from_index_item(item);
            target.push(Candidate::new(
                CompletionItem::new(item.name.clone(), kind),
                priority,
                None,
                None,
                item.scope,
            ));
        }
    }
}

pub(super) fn add_metadata_items(
    snapshot: &IndexSnapshot,
    kind: Option<MetadataKind>,
    target: &mut Vec<Candidate>,
    priority: u8,
) {
    fn format_metadata_detail(kind: MetadataKind, facets: &[FacetKind]) -> String {
        if facets.is_empty() {
            return kind.to_russian_name().to_string();
        }
        let facets = facets
            .iter()
            .map(|facet| facet.display_name())
            .collect::<Vec<_>>()
            .join(", ");
        format!("{} ({})", kind.to_russian_name(), facets)
    }

    match kind {
        Some(kind) => {
            if let Some(items) = snapshot.metadata_index.get(&kind) {
                for item in items.iter() {
                    let item_kind = completion_kind_from_index_item(item);
                    let detail = match item.kind {
                        IndexItemKind::Metadata(kind) => {
                            Some(format_metadata_detail(kind, &item.facets))
                        }
                        _ => None,
                    };
                    target.push(Candidate::new(
                        CompletionItem::with_details(item.name.clone(), item_kind, detail, None),
                        priority,
                        None,
                        None,
                        None,
                    ));
                }
            }
        }
        None => {
            for items in snapshot.metadata_index.values() {
                for item in items.iter() {
                    let item_kind = completion_kind_from_index_item(item);
                    let detail = match item.kind {
                        IndexItemKind::Metadata(kind) => {
                            Some(format_metadata_detail(kind, &item.facets))
                        }
                        _ => None,
                    };
                    target.push(Candidate::new(
                        CompletionItem::with_details(item.name.clone(), item_kind, detail, None),
                        priority,
                        None,
                        None,
                        None,
                    ));
                }
            }
        }
    }
}

pub(super) fn completion_kind_from_index_item(item: &crate::system::IndexItem) -> CompletionKind {
    match &item.kind {
        IndexItemKind::Keyword => CompletionKind::Keyword,
        IndexItemKind::Type(_) => CompletionKind::Type,
        IndexItemKind::Metadata(kind) => CompletionKind::from_metadata_kind(*kind),
        IndexItemKind::Symbol(symbol) => match symbol {
            crate::system::SymbolKind::Function => CompletionKind::Function,
            crate::system::SymbolKind::Procedure => CompletionKind::Function,
            crate::system::SymbolKind::Method => CompletionKind::Method,
            crate::system::SymbolKind::Field => CompletionKind::Field,
            crate::system::SymbolKind::Variable => CompletionKind::Variable,
            crate::system::SymbolKind::Parameter => CompletionKind::Variable,
            crate::system::SymbolKind::Constant => CompletionKind::Constant,
            crate::system::SymbolKind::Module => CompletionKind::Module,
        },
    }
}
