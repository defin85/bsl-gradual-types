#!/usr/bin/env bash
set -euo pipefail

mode="tracked"
if [[ "${1-}" == "--staged" ]]; then
  mode="staged"
  shift
fi

if [[ "${1-}" != "" ]]; then
  echo "Usage: $0 [--staged]" >&2
  exit 2
fi

declare -a bad_paths=()

read_paths() {
  local cmd=("$@")
  while IFS= read -r -d '' path; do
    case "$path" in
      vscode-extension/out/*|vscode-extension/*.vsix) bad_paths+=("$path") ;;
    esac
  done < <("${cmd[@]}")
}

if [[ "$mode" == "tracked" ]]; then
  read_paths git ls-files -z
else
  read_paths git diff --cached --name-only -z
fi

if (( ${#bad_paths[@]} == 0 )); then
  exit 0
fi

echo "ERROR: VSCode extension build artifacts must not be committed." >&2
if [[ "$mode" == "staged" ]]; then
  echo "Found in staged changes:" >&2
else
  echo "Found as tracked files:" >&2
fi
for p in "${bad_paths[@]}"; do
  echo " - $p" >&2
done
echo >&2
echo "Fix:" >&2
echo " - Remove them from the index (keep working tree):" >&2
echo "   git rm -r --cached vscode-extension/out && git rm --cached vscode-extension/*.vsix" >&2
echo " - Make sure ignores exist in .gitignore and vscode-extension/.gitignore" >&2
echo >&2
exit 1

