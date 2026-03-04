use crate::types::McpHelpResponse;

pub(super) fn build_mcp_help_response(
    tool_name: Option<String>,
) -> Result<McpHelpResponse, rmcp::ErrorData> {
    let mut quickstart = vec![
        "workspace_open(roots[], platform_docs_archive?, configuration_path?, platform_version?, mode?)".to_string(),
        "workspace_status(session_id) poll until ready=true".to_string(),
        "workspace_get_settings/workspace_update_settings use camelCase overrides payload (legacy snake_case accepted)".to_string(),
        "workspace_get_observability_metrics(session_id)".to_string(),
        "workspace_documents_set(session_id, files[], mark_hot=true)".to_string(),
        "bsl_diagnostics_start(...) / bsl_symbol_search_start(...) / context_pack_start(...)".to_string(),
        "bsl_types_list_start(...) / bsl_types_search_start(...) / bsl_type_get_start(...)".to_string(),
        "job_wait(job_id, timeout_ms) until state=succeeded".to_string(),
        "job_result(job_id)".to_string(),
    ];

    let mut notes = vec![
        "Multi-root: prefer absolute paths; server resolves via deterministic longest-prefix match against roots[].".to_string(),
        "If configuration_path is set and platform_version is omitted, bsl-agent tries to infer platform_version from config dump; otherwise INVALID_PARAMS.".to_string(),
        "Async: all semantic tools are *_start and return job_id; fetch result via job_result.".to_string(),
    ];

    let mut examples: Vec<serde_json::Value> = Vec::new();
    if let Some(name) = tool_name.as_deref() {
        match name {
            "workspace_open" => {
                examples.push(serde_json::json!({
                    "name": "workspace_open",
                    "arguments": { "roots": ["/abs/path/to/workspace"], "mode": "default" }
                }));
                examples.push(serde_json::json!({
                    "name": "workspace_open",
                    "arguments": { "roots": ["/ws/config", "/ws/ext1"], "configuration_path": "/ws/config", "platform_version": "8.3.25" }
                }));
                notes.push("Single-session: calling workspace_open again with different params requires workspace_close first.".to_string());
            }
            "workspace_documents_set" => {
                examples.push(serde_json::json!({
                    "name": "workspace_documents_set",
                    "arguments": {
                        "session_id": "<session_id>",
                        "files": ["/ws/ext1/src/CommonModules/Foo/Module.bsl"],
                        "mark_hot": true
                    }
                }));
                examples.push(serde_json::json!({
                    "name": "workspace_documents_set",
                    "arguments": {
                        "session_id": "<session_id>",
                        "files": [
                            { "doc": { "path": "/ws/ext1/src/CommonModules/Foo/Module.bsl" }, "text": "Procedure P()\nEndProcedure\n", "version": 1 }
                        ],
                        "mark_hot": true
                    }
                }));
                notes.push("When text is provided, version is required.".to_string());
            }
            "workspace_documents_clear" => {
                examples.push(serde_json::json!({
                    "name": "workspace_documents_clear",
                    "arguments": {
                        "session_id": "<session_id>",
                        "documents": [{ "path": "/ws/ext1/src/CommonModules/Foo/Module.bsl" }],
                        "clear_hot": true
                    }
                }));
            }
            "bsl_diagnostics_start" => {
                examples.push(serde_json::json!({
                    "name": "bsl_diagnostics_start",
                    "arguments": { "session_id": "<session_id>", "scope": "hot", "limit": 200 }
                }));
                examples.push(serde_json::json!({
                    "name": "bsl_diagnostics_start",
                    "arguments": { "session_id": "<session_id>", "scope": { "kind": "project" }, "limit": 200 }
                }));
                examples.push(serde_json::json!({
                    "name": "bsl_diagnostics_start",
                    "arguments": {
                        "session_id": "<session_id>",
                        "scope": { "kind": "file", "document": { "path": "/ws/ext1/src/CommonModules/Foo/Module.bsl" } },
                        "limit": 200
                    }
                }));
                examples.push(serde_json::json!({
                    "name": "bsl_diagnostics_start",
                    "arguments": {
                        "session_id": "<session_id>",
                        "scope": "hot",
                        "limit": 200,
                        "include_flow_sensitive": true
                    }
                }));
                notes.push("scope string supports only: project|hot. For a single file use tagged: {kind:\"file\",document:...}.".to_string());
                notes.push("Flow-sensitive is opt-in: pass include_flow_sensitive=true. Responses include flow_sensitive_enabled (bool).".to_string());
            }
            "bsl_type_at_position_start" => {
                examples.push(serde_json::json!({
                    "name": "bsl_type_at_position_start",
                    "arguments": {
                        "session_id": "<session_id>",
                        "file": { "doc": { "path": "/ws/ext1/src/CommonModules/Foo/Module.bsl" } },
                        "position": { "line": 10, "character": 15 },
                        "include_flow_sensitive": false
                    }
                }));
                examples.push(serde_json::json!({
                    "name": "bsl_type_at_position_start",
                    "arguments": {
                        "session_id": "<session_id>",
                        "file": { "doc": { "path": "/ws/ext1/src/CommonModules/Foo/Module.bsl" } },
                        "position": { "line": 10, "character": 15 },
                        "include_flow_sensitive": true
                    }
                }));
                notes.push("Flow-sensitive is opt-in: include_flow_sensitive defaults to false. Responses include flow_sensitive_enabled (bool).".to_string());
            }
            "bsl_members_start" => {
                examples.push(serde_json::json!({
                    "name": "bsl_members_start",
                    "arguments": {
                        "session_id": "<session_id>",
                        "file": { "doc": { "path": "/ws/ext1/src/CommonModules/Foo/Module.bsl" } },
                        "position": { "line": 10, "character": 15 },
                        "limit": 200,
                        "include_flow_sensitive": false
                    }
                }));
                examples.push(serde_json::json!({
                    "name": "bsl_members_start",
                    "arguments": {
                        "session_id": "<session_id>",
                        "file": { "doc": { "path": "/ws/ext1/src/CommonModules/Foo/Module.bsl" } },
                        "position": { "line": 10, "character": 15 },
                        "limit": 200,
                        "include_flow_sensitive": true
                    }
                }));
                notes.push("Flow-sensitive is opt-in: include_flow_sensitive defaults to false. Responses include flow_sensitive_enabled (bool).".to_string());
            }
            "job_wait" => {
                examples.push(serde_json::json!({
                    "name": "job_wait",
                    "arguments": { "job_id": "<job_id>", "timeout_ms": 5000 }
                }));
            }
            "bsl_types_list_start" => {
                examples.push(serde_json::json!({
                    "name": "bsl_types_list_start",
                    "arguments": { "session_id": "<session_id>", "page": 1, "limit": 50, "view": "names_only" }
                }));
                examples.push(serde_json::json!({
                    "name": "bsl_types_list_start",
                    "arguments": { "session_id": "<session_id>", "page": 1, "limit": 50, "source": "configuration", "view": "summary" }
                }));
            }
            "bsl_types_search_start" => {
                examples.push(serde_json::json!({
                    "name": "bsl_types_search_start",
                    "arguments": { "session_id": "<session_id>", "query": "Документ", "limit": 200, "view": "summary" }
                }));
                examples.push(serde_json::json!({
                    "name": "bsl_types_search_start",
                    "arguments": { "session_id": "<session_id>", "query": "Документы.", "limit": 200, "source": "configuration", "view": "names_only" }
                }));
            }
            "bsl_type_get_start" => {
                examples.push(serde_json::json!({
                    "name": "bsl_type_get_start",
                    "arguments": { "session_id": "<session_id>", "type_name": "Документы.ЗаказНаряд", "source": "configuration", "include_methods": false }
                }));
                notes.push("bsl_type_get_start returns a TypeDto with properties[] and tabularSections[] for configuration objects.".to_string());
            }
            "workspace_update_settings" => {
                examples.push(serde_json::json!({
                    "name": "workspace_update_settings",
                    "arguments": {
                        "session_id": "<session_id>",
                        "envOverrides": { "BSL_CACHE_DISABLE": true },
                        "allowDevOverrides": true,
                        "devEnvOverrides": { "BSL_COMPLETION_TRACE": true }
                    }
                }));
                examples.push(serde_json::json!({
                    "name": "workspace_update_settings",
                    "arguments": {
                        "session_id": "<session_id>",
                        "env_overrides": { "BSL_CACHE_DISABLE": true }
                    }
                }));
            }
            "workspace_get_observability_metrics" => {
                examples.push(serde_json::json!({
                    "name": "workspace_get_observability_metrics",
                    "arguments": { "session_id": "<session_id>" }
                }));
            }
            other => {
                return Err(rmcp::ErrorData::invalid_params(
                    format!("unknown tool_name: {other}"),
                    None,
                ));
            }
        }
    } else {
        quickstart.insert(
            0,
            "mcp_help(tool_name?) for examples (read-only)".to_string(),
        );
        notes.push("Pass tool_name to get examples: workspace_open, workspace_update_settings, workspace_get_observability_metrics, workspace_documents_set, workspace_documents_clear, bsl_diagnostics_start, bsl_type_at_position_start, bsl_members_start, bsl_types_list_start, bsl_types_search_start, bsl_type_get_start, job_wait.".to_string());
    }

    Ok(McpHelpResponse {
        summary: "bsl-agent MCP help (read-only)".to_string(),
        quickstart,
        tool_name,
        notes,
        examples,
    })
}
