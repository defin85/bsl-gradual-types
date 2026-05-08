use std::{fs, path::Path};

use zed::settings::LspSettings;
use zed_extension_api::{self as zed, LanguageServerId, Result};

const BSL_LSP_SERVER_BINARY: &str = "bsl-lsp-server";
const BSL_LSP_SERVER_BYTES: &[u8] = include_bytes!("../bsl-lsp-server");

struct BslExtension;

fn bundled_lsp_server_path() -> String {
    format!("./{}", BSL_LSP_SERVER_BINARY)
}

fn bundled_lsp_server_temp_path() -> String {
    format!("./{}.tmp", BSL_LSP_SERVER_BINARY)
}

fn install_bundled_lsp_server() -> Result<String> {
    let server_path = bundled_lsp_server_path();
    let temp_path = bundled_lsp_server_temp_path();

    fs::write(&temp_path, BSL_LSP_SERVER_BYTES).map_err(|err| {
        format!("failed to write bundled {BSL_LSP_SERVER_BINARY} to {temp_path}: {err}")
    })?;

    if Path::new(&server_path).exists() {
        fs::remove_file(&server_path).map_err(|err| {
            format!("failed to replace bundled {BSL_LSP_SERVER_BINARY} at {server_path}: {err}")
        })?;
    }

    fs::rename(&temp_path, &server_path).map_err(|err| {
        format!("failed to install bundled {BSL_LSP_SERVER_BINARY} at {server_path}: {err}")
    })?;
    zed::make_file_executable(&server_path)?;

    Ok(server_path)
}

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
        let command = install_bundled_lsp_server()?;

        Ok(zed::Command {
            command,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_lsp_server_path_is_extension_relative() {
        assert_eq!(bundled_lsp_server_path(), "./bsl-lsp-server");
    }

    #[test]
    fn bundled_lsp_server_bytes_are_embedded() {
        assert!(!BSL_LSP_SERVER_BYTES.is_empty());
    }

    #[test]
    fn initialization_options_convert_snake_case_to_camel_case() {
        let value = zed::serde_json::json!({
            "platform_docs_archive": "/tmp/docs",
            "nested_option": {
                "cache_enabled": true
            }
        });

        let converted = to_camel_case_keys(value);

        assert_eq!(
            converted,
            zed::serde_json::json!({
                "platformDocsArchive": "/tmp/docs",
                "nestedOption": {
                    "cacheEnabled": true
                }
            })
        );
    }
}
