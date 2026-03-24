#!/usr/bin/env python3
"""
Проверка ссылок на пути в документации «инструкция к действию».

Идея: в выбранных markdown-файлах извлекать кандидаты на пути (в первую очередь из inline code и ссылок),
и проверять, что эти пути существуют в репозитории (на чистом checkout).

Запуск:
  python3 scripts/check-doc-paths.py
  python3 scripts/check-doc-paths.py --targets scripts/doc-path-check-targets.txt
"""

from __future__ import annotations

import argparse
import re
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable


RE_INLINE_CODE = re.compile(r"`([^`\n]+)`")
RE_MD_LINK = re.compile(r"\]\(([^)]+)\)")


SKIP_PREFIXES = (
    "http://",
    "https://",
    "file://",
    "vscode://",
    "cargo ",
    "rg ",
    "grep ",
    "find ",
    "Read ",
    "read ",
)

SKIP_PATH_PREFIXES = (
    "feature/",
    "bugfix/",
    "hotfix/",
    "release/",
    "target/",
    "node_modules/",
    "workspace/",
)

SKIP_PATH_SUFFIXES = (
    ".log",
)


@dataclass(frozen=True)
class Hit:
    doc: Path
    raw: str
    path: str


def repo_root() -> Path:
    return Path(__file__).resolve().parents[1]


def load_targets(root: Path, targets_path: Path) -> list[Path]:
    patterns: list[str] = []
    for line in targets_path.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        patterns.append(line)

    files: list[Path] = []
    for pattern in patterns:
        matches = sorted(root.glob(pattern))
        if not matches:
            raise SystemExit(f"targets: pattern has no matches: {pattern}")
        for m in matches:
            if m.is_file():
                files.append(m)
    # unique + stable
    uniq: dict[str, Path] = {}
    for f in files:
        uniq[str(f)] = f
    return list(uniq.values())


def normalize_candidate(token: str) -> str | None:
    token = token.strip().strip(",;:")  # basic punctuation
    token = token.strip("\"'")

    if not token:
        return None

    if any(token.startswith(p) for p in SKIP_PREFIXES):
        return None

    if token.startswith("~/") or token.startswith("/"):
        return None

    if "<" in token or ">" in token:
        return None

    if "=" in token:
        return None

    token = token.strip("()[]{}")

    if token.startswith("@/"):
        token = token[2:]

    # Drop anchors and line refs: path#L10C5, path:10:5
    if "#" in token:
        token = token.split("#", 1)[0]

    # path:line[:col] (but avoid "C:\\"-like Windows paths by ignoring those altogether)
    m = re.match(r"^(.+?):(\d+)(?::(\d+))?$", token)
    if m:
        token = m.group(1)

    token = token.strip()
    if not token or " " in token:
        return None

    if "*" in token or "..." in token:
        return None

    if any(token.startswith(p) for p in SKIP_PATH_PREFIXES):
        return None

    if any(token.endswith(s) for s in SKIP_PATH_SUFFIXES):
        return None

    # Heuristic: path should look like repo-relative filesystem path
    if "/" not in token:
        return None

    return token


def extract_candidates(text: str) -> Iterable[str]:
    # Inline code spans
    for m in RE_INLINE_CODE.finditer(text):
        yield m.group(1)
    # Markdown links
    for m in RE_MD_LINK.finditer(text):
        yield m.group(1)


def tokenise(candidate: str) -> Iterable[str]:
    # Split on whitespace; also unwrap common patterns like diagnostics(path) or Read path
    parts = candidate.split()
    for p in parts:
        # Strip common wrappers
        p = p.strip()
        p = p.removeprefix("Read")
        p = p.removeprefix("read")
        p = p.strip()
        # diagnostics(file.rs), hover(file.rs, ...)
        if "(" in p and ")" in p and "/" in p:
            inner = p[p.find("(") + 1 : p.rfind(")")]
            for inner_part in inner.split(","):
                yield inner_part.strip()
            continue
        yield p


def find_missing_paths(root: Path, doc: Path) -> list[Hit]:
    text = doc.read_text(encoding="utf-8")
    hits: list[Hit] = []
    for cand in extract_candidates(text):
        for token in tokenise(cand):
            path = normalize_candidate(token)
            if path is None:
                continue
            hits.append(Hit(doc=doc, raw=cand, path=path))

    def is_within_root(path: Path) -> bool:
        try:
            path.relative_to(root)
            return True
        except ValueError:
            return False

    missing: list[Hit] = []
    for hit in hits:
        # 1) relative to document directory (e.g. backend/src/README.md -> application/ means backend/src/application/)
        doc_rel = (doc.parent / hit.path).resolve()
        if is_within_root(doc_rel) and doc_rel.exists():
            continue

        # 2) relative to repo root (e.g. shared/src/..., docs/..., scripts/...)
        root_rel = (root / hit.path).resolve()
        if is_within_root(root_rel) and root_rel.exists():
            continue

        missing.append(hit)
    return missing


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--targets",
        default="scripts/doc-path-check-targets.txt",
        help="Путь к файлу со списком документов (glob-паттерны от корня репозитория).",
    )
    args = parser.parse_args()

    root = repo_root()
    targets_path = root / args.targets
    if not targets_path.exists():
        print(f"targets file not found: {targets_path}", file=sys.stderr)
        return 2

    docs = load_targets(root, targets_path)
    all_missing: list[Hit] = []
    for doc in docs:
        all_missing.extend(find_missing_paths(root, doc))

    if not all_missing:
        return 0

    print("Missing paths referenced from docs:", file=sys.stderr)
    for hit in all_missing:
        rel_doc = hit.doc.relative_to(root)
        print(f"- {rel_doc}: `{hit.path}` (from `{hit.raw}`)", file=sys.stderr)
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
