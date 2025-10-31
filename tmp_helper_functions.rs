/// Рекурсивный поиск в узле SemanticNodeDto
fn find_in_node(
    node: &bsl_shared::api::semantic_dtos::SemanticNodeDto,
    line: u32,
    character: u32,
) -> Option<(String, String, Vec<String>, Option<String>)> {
    // Проверяем, что позиция внутри range узла (если есть)
    if let Some(ref range) = node.range {
        if !range_contains(range, line, character) {
            return None;
        }
    } else if !location_matches(&node.location, line, character) {
        // Если range отсутствует, проверяем location (точное совпадение)
        return None;
    }

    // Проверяем тип узла
    match node.kind.as_str() {
        "FunctionDeclaration" => {
            let name = node
                .name
                .clone()
                .unwrap_or_else(|| "<anonymous>".to_string());
            // Извлекаем параметры из metadata (если есть)
            let params = extract_params_from_node(node);
            let return_type = extract_return_type_from_node(node);

            return Some((name, "function".to_string(), params, return_type));
        }
        "ProcedureDeclaration" => {
            let name = node
                .name
                .clone()
                .unwrap_or_else(|| "<anonymous>".to_string());
            let params = extract_params_from_node(node);

            return Some((name, "procedure".to_string(), params, None));
        }
        _ => {
            // Рекурсивно ищем в детях
            for child in &node.children {
                if let Some(result) = find_in_node(child, line, character) {
                    return Some(result);
                }
            }
        }
    }

    None
}

/// Проверяет, содержит ли range позицию
fn range_contains(
    range: &bsl_shared::api::semantic_dtos::SourceRangeDto,
    line: u32,
    character: u32,
) -> bool {
    let start = &range.start;
    let end = &range.end;

    line >= start.line
        && line <= end.line
        && (line > start.line || character >= start.column)
        && (line < end.line || character <= end.column)
}

/// Проверяет, совпадает ли location с позицией
fn location_matches(
    location: &bsl_shared::api::semantic_dtos::SourceLocationDto,
    line: u32,
    character: u32,
) -> bool {
    location.line == line && location.column == character
}
