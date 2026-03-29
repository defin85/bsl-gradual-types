#!/usr/bin/env bash

set -euo pipefail

readonly REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly WORKFLOW_PATH="${REPO_ROOT}/.github/workflows/ci.yml"
readonly CACHE_ROOT="${REPO_ROOT}/cache/act"
readonly ACTION_CACHE_PATH="${CACHE_ROOT}/action-cache"
readonly CACHE_SERVER_PATH="${CACHE_ROOT}/cache-server"
readonly ARTIFACTS_ROOT="${CACHE_ROOT}/artifacts"
readonly LOGS_ROOT="${CACHE_ROOT}/logs"
readonly CONTAINER_TARGET_DIR="${REPO_ROOT}/target"
readonly BASE_RUNNER_IMAGE="${ACT_RUNNER_IMAGE:-catthehacker/ubuntu:act-latest}"
readonly ACT_VSCODE_RUNNER_IMAGE="${ACT_VSCODE_RUNNER_IMAGE:-bsl-gradual-types-act-vscode:ubuntu-24.04}"
readonly ACT_VSCODE_RUNNER_DOCKERFILE="${REPO_ROOT}/scripts/act-vscode-runner.Dockerfile"
readonly VOLUME_PREFIX="${ACT_VOLUME_PREFIX:-bsl-gradual-types-act}"
readonly KEEP_LOGS="${ACT_KEEP_LOGS:-5}"
readonly KEEP_ARTIFACT_RUNS="${ACT_KEEP_ARTIFACT_RUNS:-3}"
readonly MAX_LOG_AGE_DAYS="${ACT_MAX_LOG_AGE_DAYS:-7}"
readonly MAX_ARTIFACT_AGE_DAYS="${ACT_MAX_ARTIFACT_AGE_DAYS:-7}"
readonly KEEP_VSCODE_TEST_LOGS="${ACT_KEEP_VSCODE_TEST_LOGS:-5}"
readonly KEEP_VSCODE_TEST_BUILDS="${ACT_KEEP_VSCODE_TEST_BUILDS:-2}"
readonly CARGO_HOME_VOLUME="${VOLUME_PREFIX}-cargo-home"
readonly RUSTUP_HOME_VOLUME="${VOLUME_PREFIX}-rustup-home"
readonly NPM_CACHE_VOLUME="${VOLUME_PREFIX}-npm-cache"
readonly TARGET_VOLUME="${VOLUME_PREFIX}-cargo-target"
readonly EXTENSION_NODE_MODULES_DIR="${REPO_ROOT}/vscode-extension/node_modules"
readonly EXTENSION_NODE_MODULES_VOLUME="${VOLUME_PREFIX}-vscode-extension-node-modules"
readonly VSCODE_TEST_VOLUME="${VOLUME_PREFIX}-vscode-test"
readonly VSCODE_TEST_DIR="${REPO_ROOT}/vscode-extension/.vscode-test"

usage() {
  cat <<'EOF'
Usage:
  ./scripts/run-local-ci-with-act.sh list
  ./scripts/run-local-ci-with-act.sh prune
  ./scripts/run-local-ci-with-act.sh [job-id] [--offline] [--dryrun] [--verbose] [-- <extra act args>]

Examples:
  ./scripts/run-local-ci-with-act.sh list
  ./scripts/run-local-ci-with-act.sh agent_readiness_docs_gate
  ./scripts/run-local-ci-with-act.sh intellisense_smoke_gate --offline
  ./scripts/run-local-ci-with-act.sh intellisense_perf_gate -- --env OPENSPEC_CHANGE_ID=foo

Notes:
  - Heavy Rust/npm caches live in Docker named volumes instead of the repo worktree.
  - IntelliSense smoke uses a local act runner image with VS Code runtime libraries.
  - VS Code test downloads/logs live in a dedicated named volume instead of ./vscode-extension/.vscode-test.
  - Logs and uploaded artifacts go under ./cache/act/ with bounded retention.
EOF
}

die() {
  echo "error: $*" >&2
  exit 1
}

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || die "missing required command: $1"
}

ensure_host_dirs() {
  mkdir -p "${ACTION_CACHE_PATH}" "${CACHE_SERVER_PATH}" "${ARTIFACTS_ROOT}" "${LOGS_ROOT}"
}

create_named_volume() {
  local volume_name="$1"
  docker volume create \
    --label "com.defin85.repo=bsl-gradual-types" \
    --label "com.defin85.role=local-act-cache" \
    "${volume_name}" >/dev/null
}

ensure_named_volumes() {
  create_named_volume "${CARGO_HOME_VOLUME}"
  create_named_volume "${RUSTUP_HOME_VOLUME}"
  create_named_volume "${NPM_CACHE_VOLUME}"
  create_named_volume "${TARGET_VOLUME}"
  create_named_volume "${EXTENSION_NODE_MODULES_VOLUME}"
  create_named_volume "${VSCODE_TEST_VOLUME}"
}

job_requires_vscode_runtime() {
  local job_id="$1"
  [[ "${job_id}" == "intellisense_smoke_gate" ]]
}

job_requires_extension_node_modules() {
  local job_id="$1"
  job_requires_vscode_runtime "${job_id}"
}

job_requires_vscode_test_volume() {
  local job_id="$1"
  job_requires_vscode_runtime "${job_id}"
}

runner_image_for_job() {
  local job_id="$1"
  if job_requires_vscode_runtime "${job_id}"; then
    printf '%s\n' "${ACT_VSCODE_RUNNER_IMAGE}"
    return 0
  fi

  printf '%s\n' "${BASE_RUNNER_IMAGE}"
}

ensure_vscode_runner_image() {
  local job_id="$1"
  if ! job_requires_vscode_runtime "${job_id}"; then
    return 0
  fi

  [[ -f "${ACT_VSCODE_RUNNER_DOCKERFILE}" ]] || die "missing Dockerfile: ${ACT_VSCODE_RUNNER_DOCKERFILE}"

  local expected_sha
  expected_sha="$(sha256sum "${ACT_VSCODE_RUNNER_DOCKERFILE}" | awk '{print $1}')"
  local current_sha=""
  current_sha="$(
    docker image inspect \
      --format '{{ index .Config.Labels "com.defin85.dockerfile-sha" }}' \
      "${ACT_VSCODE_RUNNER_IMAGE}" 2>/dev/null || true
  )"

  if [[ "${current_sha}" == "${expected_sha}" ]]; then
    return 0
  fi

  echo "==> building local act runner image: ${ACT_VSCODE_RUNNER_IMAGE}"
  docker build \
    --build-arg "BASE_IMAGE=${BASE_RUNNER_IMAGE}" \
    --label "com.defin85.dockerfile-sha=${expected_sha}" \
    --tag "${ACT_VSCODE_RUNNER_IMAGE}" \
    --file "${ACT_VSCODE_RUNNER_DOCKERFILE}" \
    "${REPO_ROOT}" >/dev/null
}

ensure_extension_node_modules() {
  local job_id="$1"
  if ! job_requires_extension_node_modules "${job_id}"; then
    return 0
  fi

  if docker run --rm \
    -v "${EXTENSION_NODE_MODULES_VOLUME}:/node_modules" \
    alpine:3.22 \
    sh -lc 'test -x /node_modules/.bin/tsc' >/dev/null 2>&1; then
    return 0
  fi

  if [[ -x "${EXTENSION_NODE_MODULES_DIR}/.bin/tsc" ]]; then
    docker run --rm \
      -v "${EXTENSION_NODE_MODULES_VOLUME}:/dest" \
      -v "${EXTENSION_NODE_MODULES_DIR}:/src:ro" \
      alpine:3.22 \
      sh -lc 'mkdir -p /dest && cp -a /src/. /dest/' >/dev/null
    return 0
  fi

  cat >&2 <<EOF
error: ${job_id} requires local vscode-extension dependencies in ${EXTENSION_NODE_MODULES_VOLUME}.
Run:
  npm --prefix ./vscode-extension ci
EOF
  exit 1
}

safe_remove_path() {
  local target="$1"
  [[ -n "${target}" ]] || die "refusing to remove empty path"
  [[ "${target}" == "${CACHE_ROOT}/"* ]] || die "refusing to remove path outside ${CACHE_ROOT}: ${target}"
  rm -rf -- "${target}"
}

prune_by_age() {
  local root="$1"
  local max_days="$2"
  [[ -d "${root}" ]] || return 0
  while IFS= read -r stale_path; do
    [[ -n "${stale_path}" ]] || continue
    safe_remove_path "${stale_path}"
  done < <(find "${root}" -mindepth 1 -maxdepth 1 -mtime "+${max_days}" -print)
}

prune_by_count() {
  local root="$1"
  local keep_count="$2"
  [[ -d "${root}" ]] || return 0

  mapfile -t entries < <(find "${root}" -mindepth 1 -maxdepth 1 -printf '%T@ %p\n' | sort -nr | awk '{print $2}')
  local index=0
  for entry in "${entries[@]}"; do
    index=$((index + 1))
    if (( index > keep_count )); then
      safe_remove_path "${entry}"
    fi
  done
}

prune_storage() {
  ensure_host_dirs
  prune_by_age "${LOGS_ROOT}" "${MAX_LOG_AGE_DAYS}"
  prune_by_age "${ARTIFACTS_ROOT}" "${MAX_ARTIFACT_AGE_DAYS}"
  prune_by_count "${LOGS_ROOT}" "${KEEP_LOGS}"
  prune_by_count "${ARTIFACTS_ROOT}" "${KEEP_ARTIFACT_RUNS}"
}

prune_vscode_test_volume() {
  local job_id="$1"
  if ! job_requires_vscode_test_volume "${job_id}"; then
    return 0
  fi

  local helper_image="alpine:3.22"
  local log_cutoff=$((KEEP_VSCODE_TEST_LOGS + 1))
  local build_cutoff=$((KEEP_VSCODE_TEST_BUILDS + 1))

  docker run --rm \
    -v "${VSCODE_TEST_VOLUME}:/vscode-test" \
    "${helper_image}" \
    sh -lc "
      set -eu
      mkdir -p /vscode-test/user-data/logs
      cd /vscode-test/user-data/logs
      ls -1dt */ 2>/dev/null | sed -n '${log_cutoff},\$p' | while read -r entry; do
        [ -n \"\${entry}\" ] || continue
        rm -rf -- \"/vscode-test/user-data/logs/\${entry}\"
      done
      cd /vscode-test
      ls -1dt vscode-* 2>/dev/null | sed -n '${build_cutoff},\$p' | while read -r entry; do
        [ -n \"\${entry}\" ] || continue
        rm -rf -- \"/vscode-test/\${entry}\"
      done
    " >/dev/null
}

container_mount_options() {
  cat <<EOF
--mount type=volume,src=${CARGO_HOME_VOLUME},dst=/var/cache/cargo \
--mount type=volume,src=${RUSTUP_HOME_VOLUME},dst=/var/cache/rustup \
--mount type=volume,src=${NPM_CACHE_VOLUME},dst=/var/cache/npm \
--mount type=volume,src=${TARGET_VOLUME},dst=${CONTAINER_TARGET_DIR}
EOF
}

extension_node_modules_mount_option() {
  if ! job_requires_extension_node_modules "${1:-}"; then
    return 0
  fi

  printf '%s\n' \
    "--mount type=volume,src=${EXTENSION_NODE_MODULES_VOLUME},dst=${EXTENSION_NODE_MODULES_DIR}"
}

vscode_test_mount_option() {
  local job_id="$1"
  if ! job_requires_vscode_test_volume "${job_id}"; then
    return 0
  fi

  printf '%s\n' \
    "--mount type=volume,src=${VSCODE_TEST_VOLUME},dst=${VSCODE_TEST_DIR}"
}

container_options_for_job() {
  local job_id="$1"
  local options
  options="$(container_mount_options)"

  local extension_node_modules_mount=""
  extension_node_modules_mount="$(extension_node_modules_mount_option "${job_id}")"
  if [[ -n "${extension_node_modules_mount}" ]]; then
    options="${options} ${extension_node_modules_mount}"
  fi

  local vscode_test_mount=""
  vscode_test_mount="$(vscode_test_mount_option "${job_id}")"
  if [[ -n "${vscode_test_mount}" ]]; then
    options="${options} ${vscode_test_mount}"
  fi

  printf '%s\n' "${options}"
}

run_job() {
  local job_id="$1"
  shift

  local offline_mode=0
  local dryrun_mode=0
  local verbose_mode=0
  local pull_images=0
  local -a passthrough_args=()

  while (($# > 0)); do
    case "$1" in
      --offline)
        offline_mode=1
        ;;
      --dryrun)
        dryrun_mode=1
        ;;
      --verbose)
        verbose_mode=1
        ;;
      --pull)
        pull_images=1
        ;;
      --)
        shift
        passthrough_args+=("$@")
        break
        ;;
      *)
        passthrough_args+=("$1")
        ;;
    esac
    shift
  done

  ensure_host_dirs
  ensure_named_volumes
  ensure_vscode_runner_image "${job_id}"
  ensure_extension_node_modules "${job_id}"
  prune_storage
  prune_vscode_test_volume "${job_id}"

  local timestamp
  timestamp="$(date +%Y%m%d-%H%M%S)"
  local artifact_dir="${ARTIFACTS_ROOT}/${timestamp}-${job_id}"
  local log_file="${LOGS_ROOT}/${timestamp}-${job_id}.log"
  mkdir -p "${artifact_dir}"
  local runner_image
  runner_image="$(runner_image_for_job "${job_id}")"
  local container_options
  container_options="$(container_options_for_job "${job_id}")"

  local -a act_args=(
    workflow_dispatch
    -W "${WORKFLOW_PATH}"
    -j "${job_id}"
    -P "ubuntu-latest=${runner_image}"
    --rm
    --pull=false
    --rebuild=false
    --action-cache-path "${ACTION_CACHE_PATH}"
    --cache-server-path "${CACHE_SERVER_PATH}"
    --artifact-server-path "${artifact_dir}"
    --container-options "${container_options}"
    --env "CARGO_HOME=/var/cache/cargo"
    --env "RUSTUP_HOME=/var/cache/rustup"
    --env "npm_config_cache=/var/cache/npm"
  )

  if (( offline_mode )); then
    act_args+=(--action-offline-mode)
  fi
  if (( dryrun_mode )); then
    act_args+=(--dryrun)
  fi
  if (( verbose_mode )); then
    act_args+=(-v)
  fi
  if (( pull_images )); then
    act_args+=(--pull=true)
  fi
  if ((${#passthrough_args[@]} > 0)); then
    act_args+=("${passthrough_args[@]}")
  fi

  echo "==> act job: ${job_id}"
  echo "==> named volumes:"
  echo "    - ${CARGO_HOME_VOLUME}"
  echo "    - ${RUSTUP_HOME_VOLUME}"
  echo "    - ${NPM_CACHE_VOLUME}"
  echo "    - ${TARGET_VOLUME}"
  echo "    - ${EXTENSION_NODE_MODULES_VOLUME}"
  echo "    - ${VSCODE_TEST_VOLUME}"
  echo "==> runner image: ${runner_image}"
  echo "==> log file: ${log_file}"
  echo "==> artifact dir: ${artifact_dir}"

  (
    cd "${REPO_ROOT}"
    set -o pipefail
    act "${act_args[@]}" 2>&1 | tee "${log_file}"
  )
}

show_volume_sizes() {
  ensure_named_volumes
  local helper_image="alpine:3.22"
  docker image inspect "${helper_image}" >/dev/null 2>&1 || docker pull "${helper_image}" >/dev/null

  for volume_name in \
    "${CARGO_HOME_VOLUME}" \
    "${RUSTUP_HOME_VOLUME}" \
    "${NPM_CACHE_VOLUME}" \
    "${TARGET_VOLUME}" \
    "${EXTENSION_NODE_MODULES_VOLUME}" \
    "${VSCODE_TEST_VOLUME}"; do
    local size
    size="$(docker run --rm -v "${volume_name}:/inspect" "${helper_image}" sh -lc 'du -sh /inspect 2>/dev/null | cut -f1')"
    echo "${volume_name}: ${size:-0}"
  done

  if docker image inspect "${ACT_VSCODE_RUNNER_IMAGE}" >/dev/null 2>&1; then
    docker image ls "${ACT_VSCODE_RUNNER_IMAGE}" --format 'runner-image {{.Repository}}:{{.Tag}} {{.Size}}'
  fi

  du -sh "${CACHE_ROOT}" 2>/dev/null || true
}

main() {
  require_cmd act
  require_cmd docker
  require_cmd sha256sum

  local subcommand="${1:-agent_readiness_docs_gate}"
  case "${subcommand}" in
    -h|--help|help)
      usage
      ;;
    list)
      shift || true
      act --list --workflows "${WORKFLOW_PATH}" "$@"
      ;;
    prune)
      shift || true
      prune_storage
      prune_vscode_test_volume intellisense_smoke_gate
      echo "Pruned ${CACHE_ROOT}"
      ;;
    du)
      shift || true
      show_volume_sizes
      ;;
    *)
      local job_id="${subcommand}"
      shift || true
      run_job "${job_id}" "$@"
      ;;
  esac
}

main "$@"
