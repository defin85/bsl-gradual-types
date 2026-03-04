#!/usr/bin/env python3
"""Fail-closed gate for production Rust file size and LLM-friendly budgets."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

EXCLUDED_DIR_NAMES = {
    "third_party",
    "target",
    "node_modules",
    "tests",
    "benches",
    "examples",
    "fixtures",
    "mocks",
}

TARGET_FILES = [
    "backend/src/bin/lsp_server/server/core.rs",
    "backend/src/bin/lsp_server/server/language_server.rs",
    "bsl-agent/src/session/mod.rs",
    "bsl-runtime/src/application/type_system/services/completion_service.rs",
    "bsl-runtime/src/system/basic_observability.rs",
    "analysis-v2/src/lib.rs",
    "bsl-runtime/src/application/intellisense_v2/facade.rs",
    "analysis-v2/src/type_inference_v2.rs",
    "bsl-runtime/src/system/system_coordinator/config_loader.rs",
    "backend/src/bin/lsp_server/handlers/references_and_rename.rs",
    "bsl-runtime/src/system/disk_cache.rs",
    "bsl-runtime/src/application/intellisense_v2/policy.rs",
    "bsl-agent/src/server/mod.rs",
    "backend/src/bin/lsp_server/handlers/completion.rs",
    "bsl-runtime/src/system/runtime_config.rs",
    "bsl-runtime/src/system/parser_coordinator.rs",
    "backend/src/bin/lsp_server/server/completion_dispatcher.rs",
    "bsl-runtime/src/system/system_coordinator/lifecycle.rs",
    "semantic-diagnostics/src/visitor.rs",
    "bsl-repository/src/repository.rs",
    "backend/src/bin/lsp_server/commands/configuration.rs",
    "bsl-runtime/src/application/type_system/services/completion_ranking.rs",
    "bsl-runtime/src/system/system_coordinator/coordinator.rs",
    "backend/src/presentation/web/handlers.rs",
    "bsl-runtime/src/data/loaders/config_metadata_parser/discovery.rs",
    "bsl-runtime/src/data/loaders/config_metadata_parser/converter.rs",
    "backend/src/bin/intellisense_perf.rs",
    "backend/src/perf_gate_evaluator.rs",
]


def repo_root() -> Path:
    return Path(__file__).resolve().parents[1]


def line_count(text: str) -> int:
    if not text:
        return 0
    return text.count("\n") + (0 if text.endswith("\n") else 1)


def is_excluded(rel_path: Path) -> bool:
    if any(part in EXCLUDED_DIR_NAMES for part in rel_path.parts):
        return True
    name = rel_path.name
    if name == "tests.rs" or name.endswith("_test.rs"):
        return True
    return False


def list_production_rs_files(root: Path) -> list[Path]:
    files = []
    for path in sorted(root.rglob("*.rs")):
        rel = path.relative_to(root)
        if is_excluded(rel):
            continue
        files.append(path)
    return files


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Validate production Rust file LOC and LLM-friendly budgets."
    )
    parser.add_argument(
        "--repo-root",
        type=Path,
        default=repo_root(),
        help="Repository root (defaults to script parent root).",
    )
    parser.add_argument(
        "--max-production-loc",
        type=int,
        default=1000,
        help="Hard LOC limit for any production Rust file.",
    )
    parser.add_argument(
        "--max-target-loc",
        type=int,
        default=800,
        help="LLM-friendly LOC limit for target files.",
    )
    parser.add_argument(
        "--max-target-bytes",
        type=int,
        default=80 * 1024,
        help="LLM-friendly bytes limit for target files.",
    )
    parser.add_argument(
        "--max-target-tokens",
        type=int,
        default=12000,
        help="LLM-friendly token limit for target files.",
    )
    parser.add_argument(
        "--tokenizer",
        default="o200k_base",
        help="tiktoken encoding name for token counting (default: o200k_base).",
    )
    parser.add_argument(
        "--report",
        type=Path,
        default=None,
        help="Optional JSON report output path.",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="Print machine-readable JSON report to stdout.",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    root = args.repo_root.resolve()

    try:
        import tiktoken
    except ImportError:
        print(
            "ERROR: tiktoken is required for token budget checks. "
            "Run with: uv run --with tiktoken python3 scripts/check-rust-file-llm-budget.py",
            file=sys.stderr,
        )
        return 2

    try:
        encoder = tiktoken.get_encoding(args.tokenizer)
    except Exception as exc:  # pragma: no cover - defensive
        print(f"ERROR: cannot load tokenizer '{args.tokenizer}': {exc}", file=sys.stderr)
        return 2

    production_files = list_production_rs_files(root)
    production_metrics: dict[str, dict[str, Any]] = {}
    hard_loc_violations: list[dict[str, Any]] = []

    for file_path in production_files:
        rel = file_path.relative_to(root).as_posix()
        text = file_path.read_text(encoding="utf-8")
        loc = line_count(text)
        byte_size = len(text.encode("utf-8"))
        production_metrics[rel] = {
            "loc": loc,
            "bytes": byte_size,
        }
        if loc > args.max_production_loc:
            hard_loc_violations.append(
                {
                    "path": rel,
                    "loc": loc,
                    "max_loc": args.max_production_loc,
                }
            )

    target_missing: list[str] = []
    target_budget_violations: list[dict[str, Any]] = []

    for rel in TARGET_FILES:
        path = root / rel
        if not path.exists():
            target_missing.append(rel)
            continue
        text = path.read_text(encoding="utf-8")
        loc = line_count(text)
        byte_size = len(text.encode("utf-8"))
        tokens = len(encoder.encode(text))
        breaches = []
        if loc > args.max_target_loc:
            breaches.append("loc")
        if byte_size > args.max_target_bytes:
            breaches.append("bytes")
        if tokens > args.max_target_tokens:
            breaches.append("tokens")
        if breaches:
            target_budget_violations.append(
                {
                    "path": rel,
                    "loc": loc,
                    "bytes": byte_size,
                    "tokens": tokens,
                    "breaches": breaches,
                    "limits": {
                        "max_loc": args.max_target_loc,
                        "max_bytes": args.max_target_bytes,
                        "max_tokens": args.max_target_tokens,
                    },
                }
            )

    report = {
        "pass": not hard_loc_violations and not target_missing and not target_budget_violations,
        "tokenizer": args.tokenizer,
        "limits": {
            "max_production_loc": args.max_production_loc,
            "max_target_loc": args.max_target_loc,
            "max_target_bytes": args.max_target_bytes,
            "max_target_tokens": args.max_target_tokens,
        },
        "counts": {
            "production_files_scanned": len(production_files),
            "target_files_expected": len(TARGET_FILES),
            "target_files_missing": len(target_missing),
            "hard_loc_violations": len(hard_loc_violations),
            "target_budget_violations": len(target_budget_violations),
        },
        "violations": {
            "hard_loc": hard_loc_violations,
            "target_missing": target_missing,
            "target_budget": target_budget_violations,
        },
    }

    if args.report is not None:
        report_path = args.report if args.report.is_absolute() else (root / args.report)
        report_path.parent.mkdir(parents=True, exist_ok=True)
        report_path.write_text(
            json.dumps(report, ensure_ascii=False, indent=2) + "\n",
            encoding="utf-8",
        )

    if args.json:
        print(json.dumps(report, ensure_ascii=False, indent=2))
    else:
        print("Rust file LLM budget gate")
        print(f"- Production files scanned: {report['counts']['production_files_scanned']}")
        print(f"- Hard LOC violations: {report['counts']['hard_loc_violations']}")
        print(f"- Target files missing: {report['counts']['target_files_missing']}")
        print(f"- Target budget violations: {report['counts']['target_budget_violations']}")
        if hard_loc_violations:
            print("\nHard LOC violations:")
            for item in hard_loc_violations:
                print(
                    f"  - {item['path']}: loc={item['loc']} > {item['max_loc']}"
                )
        if target_missing:
            print("\nMissing target files:")
            for rel in target_missing:
                print(f"  - {rel}")
        if target_budget_violations:
            print("\nTarget budget violations:")
            for item in target_budget_violations:
                print(
                    "  - "
                    f"{item['path']}: loc={item['loc']}, bytes={item['bytes']}, "
                    f"tokens={item['tokens']} (breaches: {', '.join(item['breaches'])})"
                )

    return 0 if report["pass"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
