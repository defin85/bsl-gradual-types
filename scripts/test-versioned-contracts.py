#!/usr/bin/env python3
"""Regression tests for scripts/check-versioned-contracts.py."""

from __future__ import annotations

import importlib.util
import json
import shutil
import tempfile
import unittest
from pathlib import Path


class VersionedContractsScriptTest(unittest.TestCase):
    REPO_ROOT = Path(__file__).resolve().parents[1]
    CONTRACTS_SCRIPT = REPO_ROOT / "scripts" / "check-versioned-contracts.py"
    LSP_TIMELINE_CONTRACT = (
        REPO_ROOT
        / "contracts"
        / "lsp-completion-timeline"
        / "v16"
        / "contract.json"
    )

    @classmethod
    def load_script_module(cls):
        spec = importlib.util.spec_from_file_location(
            "check_versioned_contracts",
            cls.CONTRACTS_SCRIPT,
        )
        if spec is None or spec.loader is None:
            raise RuntimeError(f"failed to load script module from {cls.CONTRACTS_SCRIPT}")
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        return module

    @staticmethod
    def validate_surface_failure_message(module, surface_dir: Path) -> str | None:
        try:
            module.validate_surface_contract(surface_dir)
        except SystemExit as exc:  # pragma: no cover - defensive parity with script-style exits
            return str(exc)
        except Exception as exc:  # noqa: BLE001 - we assert only that validation fails
            return str(exc)
        return None

    def test_completion_timeline_latest_baseline_is_v16_and_response_version_19(self) -> None:
        module = self.load_script_module()
        contract = module.parse_json(self.LSP_TIMELINE_CONTRACT)

        self.assertEqual(module.REQUIRED_LATEST_MAJORS["lsp-completion-timeline"], 16)
        self.assertEqual(contract["major_version"], 16)
        self.assertEqual(contract["response"]["version"], 19)

    def test_completion_timeline_surface_validates_against_current_latest_major(self) -> None:
        module = self.load_script_module()
        module.validate_surface_contract(self.REPO_ROOT / "contracts" / "lsp-completion-timeline")

    def test_completion_timeline_v16_rejects_wrong_response_version(self) -> None:
        module = self.load_script_module()
        with tempfile.TemporaryDirectory(prefix="versioned-contracts-v16-version-") as tmp_dir:
            surface_dir = Path(tmp_dir) / "lsp-completion-timeline"
            shutil.copytree(
                self.REPO_ROOT / "contracts" / "lsp-completion-timeline",
                surface_dir,
            )
            contract_path = surface_dir / "v16" / "contract.json"
            payload = json.loads(contract_path.read_text(encoding="utf-8"))
            payload["response"]["version"] = 18
            contract_path.write_text(
                json.dumps(payload, ensure_ascii=False, indent=2) + "\n",
                encoding="utf-8",
            )

            failure = self.validate_surface_failure_message(module, surface_dir)
            self.assertIsNotNone(
                failure,
                "validate_surface_contract() must reject v16 contract with wrong response.version",
            )

    def test_completion_timeline_v16_rejects_non_exact_server_edge_field_sets(self) -> None:
        module = self.load_script_module()

        def mutate_contract(payload: dict[str, object], mutation: str) -> None:
            response = payload["response"]
            assert isinstance(response, dict)
            server_edge_fields = response["server_edge_details_fields"]
            assert isinstance(server_edge_fields, list)

            if mutation == "rename":
                server_edge_fields[:] = [
                    "adapter_read_at_ms" if field == "adapter_read_at_ms" else field
                    for field in server_edge_fields
                    if field != "adapter_read_at_ms"
                ]
                server_edge_fields.insert(0, "adapter_ingress_at_ms")
            elif mutation == "extra":
                server_edge_fields.append("unexpected_server_edge_field")
            elif mutation == "missing":
                response["server_edge_details_fields"] = [
                    field
                    for field in server_edge_fields
                    if field not in {"adapter_read_at_ms", "adapter_to_dispatch_wait_ms"}
                ]
            else:  # pragma: no cover - defensive guard for test data
                raise AssertionError(f"unknown mutation {mutation!r}")

        for mutation in ("rename", "extra", "missing"):
            with self.subTest(mutation=mutation):
                with tempfile.TemporaryDirectory(
                    prefix=f"versioned-contracts-v16-fields-{mutation}-"
                ) as tmp_dir:
                    surface_dir = Path(tmp_dir) / "lsp-completion-timeline"
                    shutil.copytree(
                        self.REPO_ROOT / "contracts" / "lsp-completion-timeline",
                        surface_dir,
                    )
                    contract_path = surface_dir / "v16" / "contract.json"
                    payload = json.loads(contract_path.read_text(encoding="utf-8"))
                    mutate_contract(payload, mutation)
                    contract_path.write_text(
                        json.dumps(payload, ensure_ascii=False, indent=2) + "\n",
                        encoding="utf-8",
                    )

                    failure = self.validate_surface_failure_message(module, surface_dir)
                    self.assertIsNotNone(
                        failure,
                        "validate_surface_contract() must reject non-exact v16 server edge field sets",
                    )


if __name__ == "__main__":
    unittest.main(verbosity=2)
