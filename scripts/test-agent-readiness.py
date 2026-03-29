#!/usr/bin/env python3
"""Regression tests for canonical agent-readiness validation assets."""

from __future__ import annotations

import re
import subprocess
import sys
import unittest
from pathlib import Path


class AgentReadinessValidationTest(unittest.TestCase):
    REPO_ROOT = Path(__file__).resolve().parents[1]
    CHECK_SCRIPT = REPO_ROOT / "scripts" / "check-agent-readiness.py"
    WRAPPER_SCRIPT = REPO_ROOT / "scripts" / "run-agent-readiness-checks.sh"
    LOCAL_ACT_WRAPPER = REPO_ROOT / "scripts" / "run-local-ci-with-act.sh"
    LOCAL_ACT_VSCODE_DOCKERFILE = REPO_ROOT / "scripts" / "act-vscode-runner.Dockerfile"
    VSCODE_TEST_RUNNER = REPO_ROOT / "vscode-extension" / "src" / "test" / "runTest.ts"
    DOCUMENT_SYMBOL_WRAPPER = (
        REPO_ROOT / "scripts" / "validate-document-symbol-interactive-isolation.sh"
    )
    PRE_DISPATCH_INGRESS_WRAPPER = (
        REPO_ROOT / "scripts" / "validate-isolate-completion-pre-dispatch-ingress.sh"
    )
    BACKEND_BUILD_SCRIPT = REPO_ROOT / "backend" / "build.rs"
    TURN_WAIT_WRAPPER = (
        REPO_ROOT / "scripts" / "validate-completion-turn-wait-lifecycle.sh"
    )
    FRONT_EDGE_WRAPPER = (
        REPO_ROOT / "scripts" / "validate-stabilize-completion-front-edge.sh"
    )
    TARGETS_FILE = REPO_ROOT / "scripts" / "doc-path-check-targets.txt"
    VERIFICATION_DOC = REPO_ROOT / "docs" / "agent" / "verification.md"
    CI_WORKFLOW = REPO_ROOT / ".github" / "workflows" / "ci.yml"
    REAL_MODULE_WORKFLOW = (
        REPO_ROOT / ".github" / "workflows" / "intellisense-real-module-gates.yml"
    )

    def test_required_targets_cover_primary_onboarding_and_agent_docs(self) -> None:
        content = self.TARGETS_FILE.read_text(encoding="utf-8")
        for required_entry in (
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
            "docs/agent/*.md",
        ):
            self.assertIn(required_entry, content)

    def test_verification_doc_exposes_local_agent_readiness_command(self) -> None:
        content = self.VERIFICATION_DOC.read_text(encoding="utf-8")
        self.assertIn("./scripts/run-agent-readiness-checks.sh", content)

    def test_verification_doc_exposes_local_act_wrapper(self) -> None:
        content = self.VERIFICATION_DOC.read_text(encoding="utf-8")
        self.assertIn("./scripts/run-local-ci-with-act.sh", content)

    def test_verification_doc_exposes_self_hosted_real_module_workflow(self) -> None:
        content = self.VERIFICATION_DOC.read_text(encoding="utf-8")
        self.assertIn("intellisense-real-module-gates.yml", content)
        self.assertIn("BSL_TEST_CONF_BIG_ROOT", content)

    def test_verification_doc_exposes_document_symbol_readiness_wrapper(self) -> None:
        content = self.VERIFICATION_DOC.read_text(encoding="utf-8")
        self.assertIn(
            "./scripts/validate-document-symbol-interactive-isolation.sh",
            content,
        )

    def test_verification_doc_exposes_pre_dispatch_ingress_readiness_wrapper(self) -> None:
        content = self.VERIFICATION_DOC.read_text(encoding="utf-8")
        self.assertIn(
            "./scripts/validate-isolate-completion-pre-dispatch-ingress.sh",
            content,
        )

    def test_verification_doc_exposes_turn_wait_readiness_wrapper(self) -> None:
        content = self.VERIFICATION_DOC.read_text(encoding="utf-8")
        self.assertIn(
            "./scripts/validate-completion-turn-wait-lifecycle.sh",
            content,
        )

    def test_document_symbol_wrapper_pins_change_id(self) -> None:
        content = self.DOCUMENT_SYMBOL_WRAPPER.read_text(encoding="utf-8")
        self.assertIn(
            "CHANGE_ID=refactor-document-symbol-interactive-isolation",
            content,
        )

    def test_turn_wait_wrapper_pins_change_id(self) -> None:
        content = self.TURN_WAIT_WRAPPER.read_text(encoding="utf-8")
        self.assertIn(
            "CHANGE_ID=refactor-completion-turn-wait-lifecycle",
            content,
        )

    def test_pre_dispatch_ingress_wrapper_pins_change_id(self) -> None:
        content = self.PRE_DISPATCH_INGRESS_WRAPPER.read_text(encoding="utf-8")
        self.assertIn(
            "CHANGE_ID=isolate-completion-pre-dispatch-ingress",
            content,
        )
        self.assertIn('REAL_MODULE_PROFILES="outline"', content)

    def test_front_edge_wrapper_pins_change_id(self) -> None:
        content = self.FRONT_EDGE_WRAPPER.read_text(encoding="utf-8")
        self.assertIn(
            "CHANGE_ID=stabilize-completion-front-edge",
            content,
        )

    def test_wrapper_runs_all_agent_readiness_checks(self) -> None:
        content = self.WRAPPER_SCRIPT.read_text(encoding="utf-8")
        self.assertIn("check-doc-paths.py", content)
        self.assertIn("check-agent-readiness.py", content)

    def test_local_act_wrapper_uses_named_volumes_and_bounded_storage(self) -> None:
        content = self.LOCAL_ACT_WRAPPER.read_text(encoding="utf-8")
        for required_snippet in (
            "docker volume create",
            "bsl-gradual-types-act",
            "--artifact-server-path",
            "--cache-server-path",
            "--container-options",
            "CARGO_HOME=/var/cache/cargo",
            "RUSTUP_HOME=/var/cache/rustup",
            "npm_config_cache=/var/cache/npm",
            'readonly CONTAINER_TARGET_DIR="${REPO_ROOT}/target"',
            'readonly BASE_RUNNER_IMAGE="${ACT_RUNNER_IMAGE:-catthehacker/ubuntu:act-latest}"',
            'readonly ACT_VSCODE_RUNNER_IMAGE="${ACT_VSCODE_RUNNER_IMAGE:-bsl-gradual-types-act-vscode:ubuntu-24.04}"',
            'readonly ACT_VSCODE_RUNNER_DOCKERFILE="${REPO_ROOT}/scripts/act-vscode-runner.Dockerfile"',
            'readonly EXTENSION_NODE_MODULES_DIR="${REPO_ROOT}/vscode-extension/node_modules"',
            'readonly EXTENSION_NODE_MODULES_VOLUME="${VOLUME_PREFIX}-vscode-extension-node-modules"',
            'readonly VSCODE_TEST_VOLUME="${VOLUME_PREFIX}-vscode-test"',
            'readonly VSCODE_TEST_DIR="${REPO_ROOT}/vscode-extension/.vscode-test"',
            "dst=${CONTAINER_TARGET_DIR}",
            "job_requires_vscode_runtime",
            "runner_image_for_job",
            "ensure_vscode_runner_image",
            "com.defin85.dockerfile-sha",
            "job_requires_extension_node_modules",
            "job_requires_vscode_test_volume",
            "container_options_for_job",
            "ensure_extension_node_modules",
            "npm --prefix ./vscode-extension ci",
            "type=volume,src=${EXTENSION_NODE_MODULES_VOLUME},dst=${EXTENSION_NODE_MODULES_DIR}",
            "type=volume,src=${VSCODE_TEST_VOLUME},dst=${VSCODE_TEST_DIR}",
            "prune_vscode_test_volume",
            "KEEP_VSCODE_TEST_LOGS",
            "KEEP_VSCODE_TEST_BUILDS",
            "prune_storage",
            "KEEP_LOGS",
            "KEEP_ARTIFACT_RUNS",
        ):
            self.assertIn(required_snippet, content)
        self.assertNotIn("CARGO_TARGET_DIR=/var/cache/target", content)

    def test_local_act_vscode_runner_installs_linux_runtime_dependencies(self) -> None:
        content = self.LOCAL_ACT_VSCODE_DOCKERFILE.read_text(encoding="utf-8")
        for required_snippet in (
            "FROM ${BASE_IMAGE}",
            "dbus-x11",
            "libasound2t64",
            "libatk-bridge2.0-0",
            "libatspi2.0-0",
            "libdrm2",
            "libgbm1",
            "libgtk-3-0",
            "libnspr4",
            "libnss3",
            "libx11-xcb1",
            "libxcb-dri3-0",
            "libxcomposite1",
            "libxdamage1",
            "libxfixes3",
            "libxrandr2",
            "libxshmfence1",
            "libxss1",
            "xauth",
            "xvfb",
        ):
            self.assertIn(required_snippet, content)

    def test_vscode_test_runner_accepts_headless_electron_launch_args(self) -> None:
        content = self.VSCODE_TEST_RUNNER.read_text(encoding="utf-8")
        for required_snippet in (
            "BSL_TEST_ELECTRON_LAUNCH_ARGS",
            "process.getuid() === 0",
            "--no-sandbox",
            "launchArgs",
            "runTests({ extensionDevelopmentPath, extensionTestsPath, launchArgs })",
        ):
            self.assertIn(required_snippet, content)

    def test_lsp_sources_use_explicit_client_index_imports(self) -> None:
        source_files = (
            self.REPO_ROOT / "vscode-extension" / "src" / "lsp" / "index.ts",
            self.REPO_ROOT / "vscode-extension" / "src" / "lsp" / "statsProvider.ts",
            self.REPO_ROOT / "vscode-extension" / "src" / "lsp" / "contextProvider.ts",
            self.REPO_ROOT / "vscode-extension" / "src" / "lsp" / "customRequests.ts",
            self.REPO_ROOT / "vscode-extension" / "src" / "test" / "suite" / "customRequests.test.ts",
        )
        for source_file in source_files:
            content = source_file.read_text(encoding="utf-8")
            self.assertNotIn("'./client'", content, source_file.as_posix())
            self.assertNotIn('"./client"', content, source_file.as_posix())
            self.assertNotIn("'../../lsp/client'", content, source_file.as_posix())
            self.assertNotIn('"../../lsp/client"', content, source_file.as_posix())

    def test_backend_build_script_rerun_if_changed_targets_exist(self) -> None:
        content = self.BACKEND_BUILD_SCRIPT.read_text(encoding="utf-8")
        watched_paths = re.findall(r'rerun_if_changed\("([^"]+)"\)', content)

        self.assertTrue(
            watched_paths,
            "backend/build.rs must declare at least one rerun-if-changed target",
        )

        missing = [
            relative_path
            for relative_path in watched_paths
            if not (self.BACKEND_BUILD_SCRIPT.parent / relative_path).exists()
        ]

        self.assertFalse(
            missing,
            (
                "backend/build.rs rerun-if-changed targets must exist relative to "
                f"backend/: {missing}"
            ),
        )

    def test_ci_workflow_keeps_hosted_perf_gate_generic_only(self) -> None:
        content = self.CI_WORKFLOW.read_text(encoding="utf-8")
        self.assertIn("Run IntelliSense perf gate (small|large|churn)", content)
        self.assertNotIn("Run real-module post-handoff readiness gate", content)
        self.assertNotIn("Build frontend (target/site) for representative gates", content)

    def test_self_hosted_real_module_workflow_requires_conf_big_fixture(self) -> None:
        content = self.REAL_MODULE_WORKFLOW.read_text(encoding="utf-8")
        for required_snippet in (
            "name: IntelliSense Real-Module Gates",
            "workflow_dispatch:",
            "runs-on: [self-hosted, linux, x64, conf-big]",
            "BSL_TEST_CONF_BIG_ROOT",
            "Validate self-hosted conf_big fixture",
            "Configuration.xml",
            "Build frontend (target/site) for representative gates",
            "Run real-module post-handoff readiness gate",
            "Run front-edge revision-churn representative gate",
        ):
            self.assertIn(required_snippet, content)

    def test_agent_readiness_checker_passes_for_repository_state(self) -> None:
        result = subprocess.run(
            [sys.executable, str(self.CHECK_SCRIPT)],
            cwd=self.REPO_ROOT,
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(result.returncode, 0, result.stderr or result.stdout)


if __name__ == "__main__":
    unittest.main(verbosity=2)
