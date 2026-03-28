#!/usr/bin/env python3
"""Fail-closed validation for canonical agent-facing docs and instruction layering."""

from __future__ import annotations

import json
import sys
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]

REQUIRED_FILES = (
    "AGENTS.md",
    "README.md",
    "CONTRIBUTING.md",
    ".github/copilot-instructions.md",
    "backend/AGENTS.md",
    "bsl-agent/AGENTS.md",
    "bsl-agent/README.md",
    "vscode-extension/AGENTS.md",
    "docs/README.md",
    "docs/BUILD_GUIDE.md",
    "docs/guides/development-workflow.md",
    "docs/agent/index.md",
    "docs/agent/architecture-map.md",
    "docs/agent/verification.md",
    "docs/agent/task-artifacts.md",
    "docs/agent/codex-setup.md",
    ".mcp.json",
    ".agents/skills/verify-workspace/SKILL.md",
    ".agents/skills/bsl-agent-mcp-smoke/SKILL.md",
    ".agents/skills/openspec-delivery-matrix/SKILL.md",
    ".agents/skills/docs-drift-audit/SKILL.md",
    "scripts/run-agent-readiness-checks.sh",
    "scripts/validate-document-symbol-interactive-isolation.sh",
    "scripts/validate-isolate-completion-pre-dispatch-ingress.sh",
    "scripts/validate-completion-turn-wait-lifecycle.sh",
)

REQUIRED_SNIPPETS = {
    "AGENTS.md": (
        "docs/agent/index.md",
        "docs/agent/verification.md",
        "docs/agent/codex-setup.md",
        "backend/AGENTS.md",
        "bsl-agent/AGENTS.md",
        "vscode-extension/AGENTS.md",
    ),
    "docs/agent/index.md": (
        "docs/agent/architecture-map.md",
        "docs/agent/verification.md",
        "docs/agent/task-artifacts.md",
        "AGENTS.override.md",
    ),
    "docs/agent/verification.md": (
        "./scripts/run-agent-readiness-checks.sh",
        "cargo run -p bsl-cli -- --help",
        "cargo run -p bsl-backend --bin bsl-web-server -- --help",
        "cargo run -p bsl-backend --bin bsl-lsp-server -- --help",
        "cargo run -p bsl-agent -- --help",
        "./scripts/run-intellisense-tests.sh smoke",
        "./scripts/validate-document-symbol-interactive-isolation.sh",
        "./scripts/validate-isolate-completion-pre-dispatch-ingress.sh",
        "./scripts/validate-completion-turn-wait-lifecycle.sh",
    ),
    "docs/agent/codex-setup.md": (
        ".mcp.json",
        ".agents/skills/",
        "cargo run -p bsl-agent -- --help",
    ),
    "README.md": (
        "docs/agent/index.md",
        "docs/agent/verification.md",
        "./scripts/run-agent-readiness-checks.sh",
    ),
    ".github/copilot-instructions.md": (
        "../AGENTS.md",
        "../docs/agent/index.md",
        "../docs/agent/verification.md",
    ),
    "docs/guides/development-workflow.md": (
        "./scripts/run-agent-readiness-checks.sh",
        "cargo run -p bsl-cli -- --help",
        "cargo run -p bsl-backend --bin bsl-web-server -- --help",
        "cargo run -p bsl-backend --bin bsl-lsp-server -- --help",
        "cargo run -p bsl-agent -- --help",
    ),
}

STALE_SNIPPETS = {
    "bsl-type-check": (
        "README.md",
        "docs/README.md",
        "docs/BUILD_GUIDE.md",
        "docs/guides/development-workflow.md",
        "CONTRIBUTING.md",
    ),
    "bsl-analyzer": (
        "README.md",
        "docs/README.md",
        "docs/BUILD_GUIDE.md",
        "docs/guides/development-workflow.md",
        "CONTRIBUTING.md",
    ),
    "cargo run -p cli": (
        "README.md",
        "docs/README.md",
        "docs/BUILD_GUIDE.md",
        "docs/guides/development-workflow.md",
    ),
    "--bin lsp-server": (
        "README.md",
        "docs/README.md",
        "docs/BUILD_GUIDE.md",
        "docs/guides/development-workflow.md",
    ),
    "target/release/lsp-server": (
        "README.md",
        "docs/README.md",
        "docs/BUILD_GUIDE.md",
        "docs/guides/development-workflow.md",
    ),
    "SOURCEBOT_API_KEY": (".mcp.json",),
    "C:\\1CProject": (".mcp.json", "docs/agent/codex-setup.md", "bsl-agent/README.md"),
    ".exe": (".mcp.json",),
}


def read_text(path: str) -> str:
    return (REPO_ROOT / path).read_text(encoding="utf-8")


def check_required_files(errors: list[str]) -> None:
    for rel_path in REQUIRED_FILES:
        if not (REPO_ROOT / rel_path).exists():
            errors.append(f"missing required file: {rel_path}")


def check_required_snippets(errors: list[str]) -> None:
    for rel_path, snippets in REQUIRED_SNIPPETS.items():
        content = read_text(rel_path)
        for snippet in snippets:
            if snippet not in content:
                errors.append(f"{rel_path}: missing required snippet `{snippet}`")


def check_stale_snippets(errors: list[str]) -> None:
    for stale_snippet, docs in STALE_SNIPPETS.items():
        for rel_path in docs:
            content = read_text(rel_path)
            if stale_snippet in content:
                errors.append(f"{rel_path}: stale snippet still present `{stale_snippet}`")


def check_mcp_json(errors: list[str]) -> None:
    payload = json.loads(read_text(".mcp.json"))
    servers = payload.get("mcpServers")
    if not isinstance(servers, dict):
        errors.append(".mcp.json: `mcpServers` must be an object")
        return

    bsl_agent = servers.get("bsl-agent")
    if not isinstance(bsl_agent, dict):
        errors.append(".mcp.json: missing `bsl-agent` server example")
        return

    if bsl_agent.get("command") != "cargo":
        errors.append(".mcp.json: `bsl-agent.command` must use portable `cargo` launch")

    expected_args = ["run", "-p", "bsl-agent", "--"]
    if bsl_agent.get("args") != expected_args:
        errors.append(
            ".mcp.json: `bsl-agent.args` must be ['run', '-p', 'bsl-agent', '--']"
        )

    env = bsl_agent.get("env")
    if not isinstance(env, dict):
        errors.append(".mcp.json: `bsl-agent.env` must exist")
        return

    if env.get("BSL_AGENT_HTTP_ADDR") != "127.0.0.1:0":
        errors.append(".mcp.json: `BSL_AGENT_HTTP_ADDR` must stay portable (`127.0.0.1:0`)")


def main() -> int:
    errors: list[str] = []
    check_required_files(errors)
    if errors:
        for error in errors:
            print(error, file=sys.stderr)
        return 1

    check_required_snippets(errors)
    check_stale_snippets(errors)
    check_mcp_json(errors)

    if errors:
        print("agent readiness validation failed:", file=sys.stderr)
        for error in errors:
            print(f"- {error}", file=sys.stderr)
        return 1

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
