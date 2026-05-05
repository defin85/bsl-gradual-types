use zed::settings::LspSettings;
use zed_extension_api::{self as zed, LanguageServerId, Result};

struct BslExtension;

/// Convert snake_case JSON keys to camelCase for LspConfig compatibility.
fn to_camel_case_keys(value: zed::serde_json::Value) -> zed::serde_json::Value {
    match value {
        zed::serde_json::Value::Object(map) => {
            let converted: zed::serde_json::Map<String, zed::serde_json::Value> = map
                .into_iter()
                .map(|(k, v)| {
                    let camel = to_camel_case(&k);
                    (camel, to_camel_case_keys(v))
                })
                .collect();
            zed::serde_json::Value::Object(converted)
        }
        other => other,
    }
}

fn to_camel_case(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '_' {
            if let Some(&next) = chars.peek() {
                result.push(next.to_ascii_uppercase());
                chars.next();
            }
        } else {
            result.push(c);
        }
    }
    result
}

impl zed::Extension for BslExtension {
    fn new() -> Self {
        Self
    }

    fn language_server_command(
        &mut self,
        _language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        let path = worktree
            .which("bsl-lsp-server")
            .ok_or_else(|| "bsl-lsp-server not found in PATH".to_string())?;
        Ok(zed::Command {
            command: path,
            args: vec![],
            env: worktree.shell_env(),
        })
    }

    fn language_server_initialization_options(
        &mut self,
        server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<Option<zed::serde_json::Value>> {
        let settings = LspSettings::for_worktree(server_id.as_ref(), worktree)
            .ok()
            .and_then(|lsp_settings| lsp_settings.settings.clone())
            .unwrap_or_default();
        Ok(Some(to_camel_case_keys(settings)))
    }

    fn language_server_workspace_configuration(
        &mut self,
        server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<Option<zed::serde_json::Value>> {
        let settings = LspSettings::for_worktree(server_id.as_ref(), worktree)
            .ok()
            .and_then(|lsp_settings| lsp_settings.settings.clone())
            .unwrap_or_default();
        Ok(Some(settings))
    }
}

zed::register_extension!(BslExtension);
