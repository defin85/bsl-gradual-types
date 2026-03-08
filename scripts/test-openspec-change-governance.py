#!/usr/bin/env python3
"""Regression tests for scripts/check-openspec-change-governance.py."""

from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


class OpenSpecGovernanceScriptTest(unittest.TestCase):
    REPO_ROOT = Path(__file__).resolve().parents[1]
    SCRIPT = REPO_ROOT / "scripts" / "check-openspec-change-governance.py"
    CHANGE_ID = "test-governance-change"

    def seed_valid_change(self, repo_root: Path) -> Path:
        change_root = repo_root / "openspec" / "changes" / self.CHANGE_ID
        governance_root = change_root / "governance"
        validation_root = change_root / "validation"
        spec_root = change_root / "specs" / "dev-workflow"
        governance_root.mkdir(parents=True, exist_ok=True)
        validation_root.mkdir(parents=True, exist_ok=True)
        spec_root.mkdir(parents=True, exist_ok=True)

        (change_root / "proposal.md").write_text("# Proposal\n", encoding="utf-8")
        (change_root / "tasks.md").write_text(
            "\n".join(
                [
                    "## 1. Tasks",
                    "- [x] 2.1 ADR template",
                    "- [x] 2.2 Protected assets",
                    "- [x] 2.3 Contract schema",
                    "- [x] 2.4 Ownership model",
                    "- [x] 2.6 Evaluator boundary",
                    "- [x] 3.1 Process gates",
                    "- [x] 3.2 Instrumentation",
                    "- [x] 3.3 Dedicated evaluator module",
                    "- [x] 3.4 Contract pipeline integration",
                    "- [x] 3.5 Extended perf gate",
                    "- [x] 3.6 Blocking mode",
                    "",
                    "## Dependencies / Parallelism",
                    "- [x] D1 Пункты 2.1 и 2.2 блокируют 3.1.",
                    "- [x] D2 Пункты 2.3 и 2.6 блокируют 3.3 и 3.5.",
                    "- [x] D3 Пункт 3.2 может выполняться параллельно с 3.1 после завершения 2.4.",
                    "- [x] D4 Пункт 3.4 блокирует 3.6.",
                    "",
                ]
            ),
            encoding="utf-8",
        )
        (change_root / "design.md").write_text("# Design\n", encoding="utf-8")
        (spec_root / "spec.md").write_text("## ADDED Requirements\n", encoding="utf-8")
        (validation_root / "acceptance_matrix.md").write_text(
            "Pass criteria\nFail criteria\n",
            encoding="utf-8",
        )
        (validation_root / "failing.md").write_text(
            "failing evidence: fail before implementation\n",
            encoding="utf-8",
        )
        (validation_root / "passing.md").write_text(
            "passing evidence: pass after implementation\n",
            encoding="utf-8",
        )
        (validation_root / "review-ownership-signoff.md").write_text(
            "role review evidence\n",
            encoding="utf-8",
        )

        (governance_root / "change_criticality.json").write_text(
            json.dumps(
                {
                    "schema_version": "v1",
                    "change_id": self.CHANGE_ID,
                    "change_criticality": "perf_critical",
                    "rule_id": "criticality.rules.v1/perf_hot_path",
                    "reason": "touches hot path",
                },
                ensure_ascii=False,
                indent=2,
            )
            + "\n",
            encoding="utf-8",
        )
        (governance_root / "test_first_evidence.json").write_text(
            json.dumps(
                {
                    "schema_version": "v1",
                    "change_id": self.CHANGE_ID,
                    "scope": "backend/runtime",
                    "rule_id": "test_first.rules.v1/backend_runtime",
                    "failing_ref": f"openspec/changes/{self.CHANGE_ID}/validation/failing.md",
                    "passing_ref": f"openspec/changes/{self.CHANGE_ID}/validation/passing.md",
                    "reason_codes": [],
                },
                ensure_ascii=False,
                indent=2,
            )
            + "\n",
            encoding="utf-8",
        )
        (governance_root / "protected_assets_manifest.txt").write_text(
            "contracts/intellisense-perf-gate/**\n",
            encoding="utf-8",
        )
        (governance_root / "bootstrap_policy.json").write_text(
            json.dumps(
                {
                    "schema_version": "v1",
                    "change_id": self.CHANGE_ID,
                    "required_profiles": ["small", "large", "churn"],
                    "sample_size_min": 5,
                    "aggregation_rule": "median",
                    "approval_ref": (
                        f"openspec/changes/{self.CHANGE_ID}/governance/adr.md"
                    ),
                },
                ensure_ascii=False,
                indent=2,
            )
            + "\n",
            encoding="utf-8",
        )
        (governance_root / "dependency_checks.json").write_text(
            json.dumps(
                {
                    "schema_version": "v1",
                    "change_id": self.CHANGE_ID,
                    "dependencies": [
                        {"id": "D1", "requires": ["2.1", "2.2"], "blocked": ["3.1"]},
                        {
                            "id": "D2",
                            "requires": ["2.3", "2.6"],
                            "blocked": ["3.3", "3.5"],
                        },
                        {"id": "D3", "requires": ["2.4"], "blocked": ["3.2"]},
                        {"id": "D4", "requires": ["3.4"], "blocked": ["3.6"]},
                    ],
                },
                ensure_ascii=False,
                indent=2,
            )
            + "\n",
            encoding="utf-8",
        )
        (governance_root / "ownership_signoff.json").write_text(
            json.dumps(
                {
                    "schema_version": "v1",
                    "change_id": self.CHANGE_ID,
                    "signoffs": [
                        {
                            "role": "analysis_v2_owner",
                            "status": "approved",
                            "reviewed_on": "2026-03-02",
                            "evidence_ref": (
                                f"openspec/changes/{self.CHANGE_ID}/validation/review-ownership-signoff.md"
                            ),
                        },
                        {
                            "role": "runtime_owner",
                            "status": "approved",
                            "reviewed_on": "2026-03-02",
                            "evidence_ref": (
                                f"openspec/changes/{self.CHANGE_ID}/validation/review-ownership-signoff.md"
                            ),
                        },
                        {
                            "role": "lsp_owner",
                            "status": "approved",
                            "reviewed_on": "2026-03-02",
                            "evidence_ref": (
                                f"openspec/changes/{self.CHANGE_ID}/validation/review-ownership-signoff.md"
                            ),
                        },
                        {
                            "role": "process_owner",
                            "status": "approved",
                            "reviewed_on": "2026-03-02",
                            "evidence_ref": (
                                f"openspec/changes/{self.CHANGE_ID}/validation/review-ownership-signoff.md"
                            ),
                        },
                    ],
                },
                ensure_ascii=False,
                indent=2,
            )
            + "\n",
            encoding="utf-8",
        )
        (governance_root / "adr.md").write_text(
            "\n".join(
                [
                    "# ADR",
                    "",
                    "## Status",
                    "accepted",
                    "",
                    "## Change ID and Criticality",
                    f"- change_id: `{self.CHANGE_ID}`",
                    "",
                    "## Options Considered",
                    "- option a",
                    "- option b",
                    "",
                    "## Budgets",
                    "- p95/p99",
                    "",
                    "## Rollback",
                    "- rollback plan",
                    "",
                    "## Owners and Approvers",
                    "- owner",
                    "",
                ]
            ),
            encoding="utf-8",
        )
        return change_root

    def run_gate(
        self,
        repo_root: Path,
        expected_exit: int,
        *,
        env: dict[str, str] | None = None,
    ) -> subprocess.CompletedProcess[str]:
        completed = subprocess.run(
            [
                sys.executable,
                str(self.SCRIPT),
                "--repo-root",
                str(repo_root),
                "--change-id",
                self.CHANGE_ID,
            ],
            check=False,
            capture_output=True,
            text=True,
            env=env,
        )
        self.assertEqual(
            completed.returncode,
            expected_exit,
            msg=(
                f"unexpected exit code: stdout={completed.stdout}\n"
                f"stderr={completed.stderr}"
            ),
        )
        return completed

    def make_fake_bd(self, repo_root: Path, statuses: dict[str, str]) -> dict[str, str]:
        bin_dir = repo_root / "bin"
        bin_dir.mkdir(parents=True, exist_ok=True)
        fake_bd = bin_dir / "bd"
        fake_bd.write_text(
            "\n".join(
                [
                    "#!/usr/bin/env python3",
                    "import json",
                    "import sys",
                    "",
                    f"STATUSES = {json.dumps(statuses, ensure_ascii=False)}",
                    "",
                    "if sys.argv[1:3] != ['show', '--json']:",
                    "    sys.exit(2)",
                    "",
                    "issues = []",
                    "for issue_id in sys.argv[3:]:",
                    "    issues.append({'id': issue_id, 'status': STATUSES.get(issue_id, 'open')})",
                    "print(json.dumps(issues))",
                    "",
                ]
            ),
            encoding="utf-8",
        )
        fake_bd.chmod(0o755)
        env = os.environ.copy()
        env["PATH"] = f"{bin_dir}:{env.get('PATH', '')}"
        return env

    def enable_readiness_gate(self, change_root: Path) -> None:
        spec_path = change_root / "specs" / "dev-workflow" / "spec.md"
        spec_path.write_text(
            "\n".join(
                [
                    "## ADDED Requirements",
                    "",
                    "### Requirement: Change completion MUST NOT завышать readiness относительно MUST backlog",
                    "Gate MUST сверять связанный критический Beads backlog.",
                    "",
                    "### Requirement: Traceability и review artifacts MUST отражать реальные gaps без optimistic overclaim",
                    "Traceability и review artifacts MUST отражать реальные gaps без optimistic overclaim.",
                    "",
                ]
            ),
            encoding="utf-8",
        )

    def write_readiness_status(
        self,
        change_root: Path,
        *,
        declared_status: str = "complete",
        review_verdict: str = "resolved",
        review_ref: str | None = None,
        traceability_status: str = "resolved",
        traceability_ref: str | None = None,
        critical_backlog: list[str] | None = None,
        superseding_delivery_path: str | None = None,
    ) -> None:
        change_rel = f"openspec/changes/{self.CHANGE_ID}"
        payload: dict[str, object] = {
            "schema_version": "v1",
            "change_id": self.CHANGE_ID,
            "declared_status": declared_status,
            "review_verdict": review_verdict,
            "review_ref": review_ref or f"{change_rel}/validation/readiness-review.md",
            "traceability_status": traceability_status,
            "traceability_ref": traceability_ref or f"{change_rel}/validation/traceability.md",
            "critical_backlog": critical_backlog or ["test-epic-1"],
        }
        if superseding_delivery_path is not None:
            payload["superseding_delivery_path"] = superseding_delivery_path
        (change_root / "governance" / "readiness_status.json").write_text(
            json.dumps(payload, ensure_ascii=False, indent=2) + "\n",
            encoding="utf-8",
        )

    def test_passes_for_valid_change(self) -> None:
        with tempfile.TemporaryDirectory(prefix="governance-pass-") as tmp_dir:
            repo_root = Path(tmp_dir)
            self.seed_valid_change(repo_root)
            completed = self.run_gate(repo_root, expected_exit=0)
            self.assertIn("OpenSpec governance gate passed", completed.stdout)

    def test_fails_when_change_criticality_missing(self) -> None:
        with tempfile.TemporaryDirectory(prefix="governance-missing-criticality-") as tmp_dir:
            repo_root = Path(tmp_dir)
            change_root = self.seed_valid_change(repo_root)
            (change_root / "governance" / "change_criticality.json").unlink()
            completed = self.run_gate(repo_root, expected_exit=1)
            self.assertIn("change_criticality_missing_or_unknown", completed.stderr)

    def test_fails_when_evidence_ref_missing(self) -> None:
        with tempfile.TemporaryDirectory(prefix="governance-missing-ref-") as tmp_dir:
            repo_root = Path(tmp_dir)
            change_root = self.seed_valid_change(repo_root)
            evidence_path = change_root / "governance" / "test_first_evidence.json"
            payload = json.loads(evidence_path.read_text(encoding="utf-8"))
            payload["failing_ref"] = (
                f"openspec/changes/{self.CHANGE_ID}/validation/does-not-exist.md"
            )
            evidence_path.write_text(
                json.dumps(payload, ensure_ascii=False, indent=2) + "\n",
                encoding="utf-8",
            )
            completed = self.run_gate(repo_root, expected_exit=1)
            self.assertIn("test_first_evidence_missing_or_invalid", completed.stderr)

    def test_fails_when_acceptance_matrix_missing(self) -> None:
        with tempfile.TemporaryDirectory(prefix="governance-missing-matrix-") as tmp_dir:
            repo_root = Path(tmp_dir)
            change_root = self.seed_valid_change(repo_root)
            (change_root / "validation" / "acceptance_matrix.md").unlink()
            completed = self.run_gate(repo_root, expected_exit=1)
            self.assertIn("doc_first_contract_missing", completed.stderr)

    def test_fails_when_adr_not_accepted(self) -> None:
        with tempfile.TemporaryDirectory(prefix="governance-adr-status-") as tmp_dir:
            repo_root = Path(tmp_dir)
            change_root = self.seed_valid_change(repo_root)
            adr_path = change_root / "governance" / "adr.md"
            adr_text = adr_path.read_text(encoding="utf-8").replace("accepted", "proposed", 1)
            adr_path.write_text(adr_text, encoding="utf-8")
            completed = self.run_gate(repo_root, expected_exit=1)
            self.assertIn("adr_missing_or_not_approved", completed.stderr)

    def test_fails_when_rule_id_does_not_match_scope(self) -> None:
        with tempfile.TemporaryDirectory(prefix="governance-rule-scope-") as tmp_dir:
            repo_root = Path(tmp_dir)
            change_root = self.seed_valid_change(repo_root)
            evidence_path = change_root / "governance" / "test_first_evidence.json"
            payload = json.loads(evidence_path.read_text(encoding="utf-8"))
            payload["rule_id"] = "test_first.rules.v1/frontend_ui"
            evidence_path.write_text(
                json.dumps(payload, ensure_ascii=False, indent=2) + "\n",
                encoding="utf-8",
            )
            completed = self.run_gate(repo_root, expected_exit=1)
            self.assertIn("test_first_evidence_missing_or_invalid", completed.stderr)

    def test_fails_when_bootstrap_policy_has_invalid_sample_size(self) -> None:
        with tempfile.TemporaryDirectory(prefix="governance-bootstrap-invalid-") as tmp_dir:
            repo_root = Path(tmp_dir)
            change_root = self.seed_valid_change(repo_root)
            bootstrap_path = change_root / "governance" / "bootstrap_policy.json"
            payload = json.loads(bootstrap_path.read_text(encoding="utf-8"))
            payload["sample_size_min"] = 3
            bootstrap_path.write_text(
                json.dumps(payload, ensure_ascii=False, indent=2) + "\n",
                encoding="utf-8",
            )
            completed = self.run_gate(repo_root, expected_exit=1)
            self.assertIn("initial_budget_not_fixed", completed.stderr)

    def test_fails_when_dependency_checks_missing_declared_dependency(self) -> None:
        with tempfile.TemporaryDirectory(prefix="governance-deps-missing-") as tmp_dir:
            repo_root = Path(tmp_dir)
            change_root = self.seed_valid_change(repo_root)
            deps_path = change_root / "governance" / "dependency_checks.json"
            payload = json.loads(deps_path.read_text(encoding="utf-8"))
            payload["dependencies"] = [
                dep for dep in payload["dependencies"] if dep["id"] != "D3"
            ]
            deps_path.write_text(
                json.dumps(payload, ensure_ascii=False, indent=2) + "\n",
                encoding="utf-8",
            )
            completed = self.run_gate(repo_root, expected_exit=1)
            self.assertIn("doc_first_contract_missing", completed.stderr)

    def test_fails_when_ownership_signoff_missing_required_role(self) -> None:
        with tempfile.TemporaryDirectory(prefix="governance-owner-missing-role-") as tmp_dir:
            repo_root = Path(tmp_dir)
            change_root = self.seed_valid_change(repo_root)
            signoff_path = change_root / "governance" / "ownership_signoff.json"
            payload = json.loads(signoff_path.read_text(encoding="utf-8"))
            payload["signoffs"] = [
                signoff
                for signoff in payload["signoffs"]
                if signoff["role"] != "process_owner"
            ]
            signoff_path.write_text(
                json.dumps(payload, ensure_ascii=False, indent=2) + "\n",
                encoding="utf-8",
            )
            completed = self.run_gate(repo_root, expected_exit=1)
            self.assertIn("adr_missing_or_not_approved", completed.stderr)

    def test_fails_when_complete_verdict_has_open_critical_backlog(self) -> None:
        with tempfile.TemporaryDirectory(prefix="governance-open-backlog-") as tmp_dir:
            repo_root = Path(tmp_dir)
            change_root = self.seed_valid_change(repo_root)
            self.enable_readiness_gate(change_root)
            (change_root / "validation" / "readiness-review.md").write_text(
                "partial review verdict with open backlog\n",
                encoding="utf-8",
            )
            self.write_readiness_status(
                change_root,
                review_verdict="partial",
                traceability_status="partial",
                traceability_ref=f"openspec/changes/{self.CHANGE_ID}/validation/readiness-review.md",
            )

            env = self.make_fake_bd(repo_root, {"test-epic-1": "open"})
            completed = self.run_gate(repo_root, expected_exit=1, env=env)
            self.assertIn("readiness_gate_conflict", completed.stderr)

    def test_passes_when_open_backlog_has_approved_superseding_delivery_path(self) -> None:
        with tempfile.TemporaryDirectory(prefix="governance-approved-superseding-") as tmp_dir:
            repo_root = Path(tmp_dir)
            change_root = self.seed_valid_change(repo_root)
            self.enable_readiness_gate(change_root)
            (change_root / "validation" / "readiness-review.md").write_text(
                "# Readiness Review\n\n## Verdict\ncomplete\n",
                encoding="utf-8",
            )
            (change_root / "validation" / "traceability.md").write_text(
                "# Traceability\n\n## Requirement: Example\n\nStatus: `covered`\n",
                encoding="utf-8",
            )
            superseding_rel = f"openspec/changes/{self.CHANGE_ID}/validation/superseding-delivery.md"
            (change_root / "validation" / "superseding-delivery.md").write_text(
                "# Superseding Delivery\n\nApproved handoff evidence.\n",
                encoding="utf-8",
            )
            self.write_readiness_status(
                change_root,
                review_verdict="complete",
                traceability_status="covered",
                superseding_delivery_path=superseding_rel,
            )

            env = self.make_fake_bd(repo_root, {"test-epic-1": "open"})
            completed = self.run_gate(repo_root, expected_exit=0, env=env)
            self.assertIn("OpenSpec governance gate passed", completed.stdout)

    def test_fails_when_open_backlog_has_unapproved_superseding_delivery_path(self) -> None:
        with tempfile.TemporaryDirectory(prefix="governance-unapproved-superseding-") as tmp_dir:
            repo_root = Path(tmp_dir)
            change_root = self.seed_valid_change(repo_root)
            self.enable_readiness_gate(change_root)
            (change_root / "validation" / "readiness-review.md").write_text(
                "# Readiness Review\n\n## Verdict\ncomplete\n",
                encoding="utf-8",
            )
            (change_root / "validation" / "traceability.md").write_text(
                "# Traceability\n\n## Requirement: Example\n\nStatus: `covered`\n",
                encoding="utf-8",
            )
            superseding_rel = f"openspec/changes/{self.CHANGE_ID}/validation/superseding-delivery.md"
            (change_root / "validation" / "superseding-delivery.md").write_text(
                "# Superseding Delivery\n\nPending handoff evidence.\n",
                encoding="utf-8",
            )
            self.write_readiness_status(
                change_root,
                review_verdict="complete",
                traceability_status="covered",
                superseding_delivery_path=superseding_rel,
            )

            env = self.make_fake_bd(repo_root, {"test-epic-1": "open"})
            completed = self.run_gate(repo_root, expected_exit=1, env=env)
            self.assertIn("superseding_delivery_path", completed.stderr)
            self.assertIn("readiness_gate_conflict", completed.stderr)

    def test_fails_when_review_ref_evidence_is_weaker_than_declared_success(self) -> None:
        with tempfile.TemporaryDirectory(prefix="governance-review-mismatch-") as tmp_dir:
            repo_root = Path(tmp_dir)
            change_root = self.seed_valid_change(repo_root)
            self.enable_readiness_gate(change_root)
            (change_root / "validation" / "readiness-review.md").write_text(
                "# Readiness Review\n\n## Verdict\npartial\n",
                encoding="utf-8",
            )
            (change_root / "validation" / "traceability.md").write_text(
                "# Traceability\n\n## Requirement: Example\n\nStatus: `covered`\n",
                encoding="utf-8",
            )
            self.write_readiness_status(change_root)

            env = self.make_fake_bd(repo_root, {"test-epic-1": "closed"})
            completed = self.run_gate(repo_root, expected_exit=1, env=env)
            self.assertIn("review_ref", completed.stderr)
            self.assertIn("readiness_gate_conflict", completed.stderr)

    def test_fails_when_traceability_ref_evidence_is_weaker_than_declared_success(self) -> None:
        with tempfile.TemporaryDirectory(prefix="governance-traceability-mismatch-") as tmp_dir:
            repo_root = Path(tmp_dir)
            change_root = self.seed_valid_change(repo_root)
            self.enable_readiness_gate(change_root)
            (change_root / "validation" / "readiness-review.md").write_text(
                "# Readiness Review\n\n## Verdict\ncomplete\n",
                encoding="utf-8",
            )
            (change_root / "validation" / "traceability.md").write_text(
                "# Traceability\n\n## Requirement: Example\n\nStatus: `gap`\n",
                encoding="utf-8",
            )
            self.write_readiness_status(change_root)

            env = self.make_fake_bd(repo_root, {"test-epic-1": "closed"})
            completed = self.run_gate(repo_root, expected_exit=1, env=env)
            self.assertIn("readiness_gate_conflict", completed.stderr)
            self.assertIn("traceability_ref", completed.stderr)

    def test_fails_when_review_and_traceability_artifacts_disagree(self) -> None:
        with tempfile.TemporaryDirectory(prefix="governance-artifact-conflict-") as tmp_dir:
            repo_root = Path(tmp_dir)
            change_root = self.seed_valid_change(repo_root)
            self.enable_readiness_gate(change_root)
            (change_root / "validation" / "readiness-review.md").write_text(
                "# Readiness Review\n\n## Verdict\nresolved\n",
                encoding="utf-8",
            )
            (change_root / "validation" / "traceability.md").write_text(
                "# Traceability\n\n## Requirement: Example\n\nStatus: `partial`\n",
                encoding="utf-8",
            )
            self.write_readiness_status(
                change_root,
                declared_status="partial",
                traceability_status="partial",
            )

            env = self.make_fake_bd(repo_root, {"test-epic-1": "closed"})
            completed = self.run_gate(repo_root, expected_exit=1, env=env)
            self.assertIn("review/traceability evidence disagree", completed.stderr)


if __name__ == "__main__":
    unittest.main(verbosity=2)
