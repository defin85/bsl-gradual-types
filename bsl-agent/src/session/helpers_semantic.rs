fn settings_id_v2(diagnostics_detail_level: DetailLevel) -> SettingsId {
    SettingsId::from_hash(format!(
        "bsl-agent;schema={};diagnostics.detail_level={:?}",
        bsl_analysis_v2::SETTINGS_SCHEMA_VERSION,
        diagnostics_detail_level
    ))
}

#[allow(clippy::too_many_arguments)]
fn prepare_ephemeral_mcp_operation(
    operation: SemanticOperation,
    flow_sensitive: bool,
    deps_id: bsl_analysis_v2::DepsSnapshotId,
    deps: Arc<bsl_analysis_v2::SemanticDeps>,
    index_snapshot: Arc<bsl_runtime::system::IndexSnapshot>,
    text: Arc<str>,
    version: i32,
    path: Arc<str>,
    diagnostics_detail_level: DetailLevel,
    coordinator: &bsl_runtime::system::SystemCoordinator,
) -> Result<(ExecutionContext, PreparedOperationSnapshot), bsl_runtime::application::SemanticOutcome>
{
    let context = ExecutionContext {
        origin: ObservabilityOrigin::Agent,
        operation,
        completion_mode: None,
        completion_large_churn_active: false,
        file_id: FileId(1),
        min_file_version: Some(version),
        expected_deps_id: Some(deps_id.clone()),
        flow_sensitive,
        settings: ExecutionSettings {
            settings_id: settings_id_v2(diagnostics_detail_level),
            diagnostics_detail_level,
        },
        cancellation: CancellationPolicy::BestEffort,
    };

    let prepared = IntellisenseV2Facade::prepare_ephemeral_operation(
        &context,
        deps_id,
        deps,
        index_snapshot,
        text,
        version,
        path,
        Some(coordinator),
    )?;

    Ok((context, prepared))
}

fn span_to_range_with_index(
    text: &str,
    line_index: &bsl_analysis_v2::LineIndex,
    span: bsl_shared::ir::Span,
) -> RangeDto {
    let (start_line, start_character) =
        line_index.byte_offset_to_utf16_position(text, span.start as usize);
    let (end_line, end_character) =
        line_index.byte_offset_to_utf16_position(text, span.end as usize);

    RangeDto {
        start: PositionDto {
            line: start_line,
            character: start_character,
        },
        end: PositionDto {
            line: end_line,
            character: end_character,
        },
    }
}

fn span_to_range_with_analysis(
    analysis: &bsl_analysis_v2::AnalysisV2,
    file_id: bsl_analysis_v2::FileId,
    span: bsl_shared::ir::Span,
) -> RangeDto {
    let Some(text) = analysis.file_text(file_id).ok().flatten() else {
        return RangeDto::default();
    };
    let Some(line_index) = analysis.line_index(file_id).ok().flatten() else {
        return RangeDto::default();
    };

    span_to_range_with_index(text.as_ref(), line_index.as_ref(), span)
}

fn type_at_utf16_position(
    analysis: &bsl_analysis_v2::AnalysisV2,
    file_id: bsl_analysis_v2::FileId,
    line: u32,
    character: u32,
    include_flow_sensitive: bool,
    coordinator: Option<&bsl_runtime::system::SystemCoordinator>,
) -> Option<bsl_shared::domain::types::TypeResolution> {
    let byte_offset = analysis
        .utf16_position_to_byte_offset(file_id, line, character)
        .ok()
        .flatten()? as u32;

    if include_flow_sensitive {
        analysis
            .flow_type_at_byte_offset(file_id, byte_offset)
            .ok()
            .flatten()
    } else {
        serve_only_type_at_byte_offset_with_reason(analysis, file_id, byte_offset, coordinator)
    }
}

fn member_access_owner_type_hint_at_position(
    analysis: &bsl_analysis_v2::AnalysisV2,
    file_id: bsl_analysis_v2::FileId,
    file_content: &str,
    line: u32,
    character: u32,
    include_flow_sensitive: bool,
    coordinator: Option<&bsl_runtime::system::SystemCoordinator>,
) -> Option<bsl_shared::domain::types::TypeResolution> {
    let line_text = file_content.lines().nth(line as usize)?;
    let cursor_byte = bsl_analysis_v2::utf16_to_byte_offset(line_text, character);
    let line_prefix = line_text.get(..cursor_byte)?;
    let dot_in_line = line_prefix.rfind('.')?;
    let receiver = line_prefix.get(..dot_in_line)?.trim_end();
    if receiver.is_empty() {
        return None;
    }
    let probe_utf16 = bsl_analysis_v2::byte_offset_to_utf16(line_text, receiver.len());
    let offset = analysis
        .utf16_position_to_byte_offset(file_id, line, probe_utf16)
        .ok()
        .flatten()?
        .saturating_sub(1);
    let offset = offset.min(u32::MAX as usize) as u32;
    if include_flow_sensitive {
        analysis
            .flow_type_at_byte_offset(file_id, offset)
            .ok()
            .flatten()
    } else {
        serve_only_type_at_byte_offset_with_reason(analysis, file_id, offset, coordinator)
    }
}

fn definition_receiver_type_hint_at_position(
    analysis: &bsl_analysis_v2::AnalysisV2,
    program: &bsl_shared::ir::SemanticProgram,
    file_id: bsl_analysis_v2::FileId,
    file_content: &str,
    line: u32,
    character: u32,
    coordinator: Option<&bsl_runtime::system::SystemCoordinator>,
) -> Option<bsl_shared::domain::types::TypeResolution> {
    let offset = analysis
        .utf16_position_to_byte_offset(file_id, line, character)
        .ok()
        .flatten()
        .map(|offset| offset.min(u32::MAX as usize) as u32)?;
    let node = program.find_node_at_byte_offset(offset)?;

    let object_span = match &node.kind {
        bsl_shared::ir::SemanticNodeKind::MemberAccess {
            object_node,
            object_span,
            ..
        } => {
            object_span.or_else(|| object_node.and_then(|idx| program.nodes.get(idx).map(|node| node.span)))
        }
        bsl_shared::ir::SemanticNodeKind::FunctionCall {
            object_node,
            object_span,
            ..
        } => {
            object_span.or_else(|| object_node.and_then(|idx| program.nodes.get(idx).map(|node| node.span)))
        }
        _ => None,
    }?;

    let mut fallback = None;
    let mut probes = Vec::with_capacity(2);
    if object_span.end > object_span.start {
        probes.push(object_span.end.saturating_sub(1));
    }
    probes.push(object_span.start);

    for probe in probes {
        let Some(resolution) =
            serve_only_type_at_byte_offset_with_reason(analysis, file_id, probe, coordinator)
        else {
            continue;
        };
        if !resolution.is_unknown() && !resolution.is_dynamic() {
            return Some(resolution);
        }
        if fallback.is_none() {
            fallback = Some(resolution);
        }
    }

    let line_based = member_access_owner_type_hint_at_position(
        analysis,
        file_id,
        file_content,
        line,
        character,
        false,
        coordinator,
    );
    if let Some(resolution) = line_based {
        if !resolution.is_unknown() && !resolution.is_dynamic() {
            return Some(resolution);
        }
        if fallback.is_none() {
            fallback = Some(resolution);
        }
    }

    fallback
}

fn serve_only_type_at_byte_offset_with_reason(
    analysis: &bsl_analysis_v2::AnalysisV2,
    file_id: bsl_analysis_v2::FileId,
    byte_offset: u32,
    coordinator: Option<&bsl_runtime::system::SystemCoordinator>,
) -> Option<bsl_shared::domain::types::TypeResolution> {
    let profiled = analysis
        .type_at_byte_offset_serve_only_profiled(file_id, byte_offset)
        .ok()?;
    if let Some(coordinator) = coordinator {
        coordinator.record_intellisense_v2_type_index_reason(profiled.serve_reason_code.as_str());
    }
    profiled.resolution
}

fn node_at_utf16_position<'a>(
    analysis: &bsl_analysis_v2::AnalysisV2,
    program: &'a bsl_shared::ir::SemanticProgram,
    file_id: bsl_analysis_v2::FileId,
    line: u32,
    character: u32,
) -> Option<&'a bsl_shared::ir::SemanticNode> {
    let byte_offset = analysis
        .utf16_position_to_byte_offset(file_id, line, character)
        .ok()
        .flatten()? as u32;

    program.find_node_at_byte_offset(byte_offset)
}

fn map_path_to_root(roots: &[RootEntry], target: &Path) -> Option<(String, String)> {
    for root in roots {
        let Ok(rel) = target.strip_prefix(&root.path) else {
            continue;
        };
        let rel_path = normalize_path_components(rel);
        return Some((root.root_id.clone(), rel_path));
    }
    None
}

fn sort_symbols(symbols: &mut [SymbolDto]) {
    symbols.sort_by(|a, b| {
        (
            a.file.root_id.as_str(),
            a.file.path.as_str(),
            sort::range_sort_key(&a.range),
            a.kind.as_str(),
            a.name.as_str(),
            a.symbol_id.as_str(),
        )
            .cmp(&(
                b.file.root_id.as_str(),
                b.file.path.as_str(),
                sort::range_sort_key(&b.range),
                b.kind.as_str(),
                b.name.as_str(),
                b.symbol_id.as_str(),
            ))
    });
}

fn sort_references(references: &mut [ReferenceDto]) {
    references.sort_by(|a, b| {
        (
            a.file.root_id.as_str(),
            a.file.path.as_str(),
            sort::range_sort_key(&a.range),
        )
            .cmp(&(
                b.file.root_id.as_str(),
                b.file.path.as_str(),
                sort::range_sort_key(&b.range),
            ))
    });
}

fn sort_pack_items(items: &mut [ContextPackItemDto]) {
    items.sort_by(|a, b| {
        let (a_root, a_path) = a
            .file
            .as_ref()
            .map(|file| (file.root_id.as_str(), file.path.as_str()))
            .unwrap_or(("", ""));
        let (b_root, b_path) = b
            .file
            .as_ref()
            .map(|file| (file.root_id.as_str(), file.path.as_str()))
            .unwrap_or(("", ""));
        let a_range = a
            .range
            .as_ref()
            .map(sort::range_sort_key)
            .unwrap_or((0, 0, 0, 0));
        let b_range = b
            .range
            .as_ref()
            .map(sort::range_sort_key)
            .unwrap_or((0, 0, 0, 0));
        (a.kind.as_str(), a_root, a_path, a_range, a.item_id.as_str()).cmp(&(
            b.kind.as_str(),
            b_root,
            b_path,
            b_range,
            b.item_id.as_str(),
        ))
    });
}

struct SnippetRender {
    text: String,
    range: RangeDto,
    truncated: bool,
}

fn render_snippet(text: &str, center_line: u32, context_lines: u32) -> SnippetRender {
    let lines: Vec<&str> = text.lines().collect();
    let total_lines = lines.len() as u32;

    let start_line = center_line.saturating_sub(context_lines);
    let end_line = if total_lines == 0 {
        0
    } else {
        center_line
            .saturating_add(context_lines)
            .min(total_lines.saturating_sub(1))
    };

    let mut rendered = String::new();
    for line_idx in start_line..=end_line {
        let raw = lines
            .get(line_idx as usize)
            .copied()
            .unwrap_or_default()
            .trim_end_matches('\r');
        let marker = if line_idx == center_line { ">" } else { " " };
        rendered.push_str(&format!(
            "{marker}{:>5} | {raw}\n",
            line_idx.saturating_add(1)
        ));
    }

    SnippetRender {
        text: rendered,
        range: RangeDto {
            start: PositionDto {
                line: start_line,
                character: 0,
            },
            end: PositionDto {
                line: end_line,
                character: 0,
            },
        },
        truncated: false,
    }
}

struct TextBudget {
    budget_chars: usize,
    used_chars: usize,
    text: String,
    truncated: bool,
}

impl TextBudget {
    fn new(budget_chars: usize) -> Self {
        Self {
            budget_chars,
            used_chars: 0,
            text: String::new(),
            truncated: false,
        }
    }

    fn push_str(&mut self, value: &str) {
        if self.truncated || self.budget_chars == 0 {
            if !value.is_empty() {
                self.truncated = true;
            }
            return;
        }

        let mut chars = value.chars();
        while self.used_chars < self.budget_chars {
            let Some(ch) = chars.next() else {
                return;
            };
            self.text.push(ch);
            self.used_chars += 1;
        }

        if chars.next().is_some() {
            self.truncated = true;
        }
    }

    fn push_line(&mut self, value: &str) {
        self.push_str(value);
        self.push_str("\n");
    }

    fn finish(self) -> String {
        self.text
    }
}

fn compute_budget_chars(budget_chars: Option<u32>, budget_tokens: Option<u32>) -> usize {
    if let Some(chars) = budget_chars {
        return chars as usize;
    }
    if let Some(tokens) = budget_tokens {
        return (tokens as usize).saturating_mul(CHARS_PER_TOKEN);
    }
    DEFAULT_BUDGET_CHARS
}

fn include_fingerprint(include: &crate::server::types::ContextInclude) -> String {
    format!(
        "snippets={};diagnostics={};types={};members={};references={};symbols={}",
        if include.snippets { 1 } else { 0 },
        if include.diagnostics { 1 } else { 0 },
        if include.types { 1 } else { 0 },
        if include.members { 1 } else { 0 },
        if include.references { 1 } else { 0 },
        if include.symbols { 1 } else { 0 },
    )
}

fn scope_key_for_pack(
    roots: &[RootEntry],
    scope: &WorkspaceScopeTagged,
) -> Result<String, rmcp::ErrorData> {
    match scope {
        WorkspaceScopeTagged::Project => Ok("project".to_string()),
        WorkspaceScopeTagged::Hot => Ok("hot".to_string()),
        WorkspaceScopeTagged::File { document } => {
            let key = document_key_from_ref(roots, document)?;
            let document_id = ids::document_id(&key.root_id, &key.path);
            Ok(format!("file:{document_id}"))
        }
    }
}

fn focus_key_for_pack(
    roots: &[RootEntry],
    focus: &ContextFocus,
) -> Result<String, rmcp::ErrorData> {
    match focus {
        ContextFocus::Diagnostic { diagnostic_id } => Ok(format!("diagnostic:{diagnostic_id}")),
        ContextFocus::Symbol { symbol_id } => Ok(format!("symbol:{symbol_id}")),
        ContextFocus::Query { query } => Ok(format!("query:{}", query.trim())),
        ContextFocus::Position { file, position } => {
            let key = document_key_from_ref(roots, &file.doc)?;
            let document_id = ids::document_id(&key.root_id, &key.path);
            Ok(format!(
                "position:{document_id}:{}:{}",
                position.line, position.character
            ))
        }
    }
}
