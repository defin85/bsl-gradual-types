#!/usr/bin/env bash

set -euo pipefail

readonly REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly WORKFLOW_PATH="${REPO_ROOT}/.github/workflows/ci.yml"
readonly CACHE_ROOT="${REPO_ROOT}/cache/act"
readonly ACTION_CACHE_PATH="${CACHE_ROOT}/action-cache"
readonly CACHE_SERVER_PATH="${CACHE_ROOT}/cache-server"
readonly ARTIFACTS_ROOT="${CACHE_ROOT}/artifacts"
readonly LOGS_ROOT="${CACHE_ROOT}/logs"
readonly RUNNER_IMAGE="${ACT_RUNNER_IMAGE:-catthehacker/ubuntu:act-latest}"
readonly VOLUME_PREFIX="${ACT_VOLUME_PREFIX:-bsl-gradual-types-act}"
readonly KEEP_LOGS="${ACT_KEEP_LOGS:-5}"
readonly KEEP_ARTIFACT_RUNS="${ACT_KEEP_ARTIFACT_RUNS:-3}"
readonly MAX_LOG_AGE_DAYS="${ACT_MAX_LOG_AGE_DAYS:-7}"
readonly MAX_ARTIFACT_AGE_DAYS="${ACT_MAX_ARTIFACT_AGE_DAYS:-7}"
readonly CARGO_HOME_VOLUME="${VOLUME_PREFIX}-cargo-home"
readonly RUSTUP_HOME_VOLUME="${VOLUME_PREFIX}-rustup-home"
readonly NPM_CACHE_VOLUME="${VOLUME_PREFIX}-npm-cache"
readonly TARGET_VOLUME="${VOLUME_PREFIX}-cargo-target"

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

container_mount_options() {
  cat <<EOF
--mount type=volume,src=${CARGO_HOME_VOLUME},dst=/var/cache/cargo \
--mount type=volume,src=${RUSTUP_HOME_VOLUME},dst=/var/cache/rustup \
--mount type=volume,src=${NPM_CACHE_VOLUME},dst=/var/cache/npm \
--mount type=volume,src=${TARGET_VOLUME},dst=/var/cache/target
EOF
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
  prune_storage

  local timestamp
  timestamp="$(date +%Y%m%d-%H%M%S)"
  local artifact_dir="${ARTIFACTS_ROOT}/${timestamp}-${job_id}"
  local log_file="${LOGS_ROOT}/${timestamp}-${job_id}.log"
  mkdir -p "${artifact_dir}"

  local -a act_args=(
    workflow_dispatch
    -W "${WORKFLOW_PATH}"
    -j "${job_id}"
    -P "ubuntu-latest=${RUNNER_IMAGE}"
    --rm
    --pull=false
    --rebuild=false
    --action-cache-path "${ACTION_CACHE_PATH}"
    --cache-server-path "${CACHE_SERVER_PATH}"
    --artifact-server-path "${artifact_dir}"
    --container-options "$(container_mount_options)"
    --env "CARGO_HOME=/var/cache/cargo"
    --env "RUSTUP_HOME=/var/cache/rustup"
    --env "npm_config_cache=/var/cache/npm"
    --env "CARGO_TARGET_DIR=/var/cache/target"
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
    "${TARGET_VOLUME}"; do
    local size
    size="$(docker run --rm -v "${volume_name}:/inspect" "${helper_image}" sh -lc 'du -sh /inspect 2>/dev/null | cut -f1')"
    echo "${volume_name}: ${size:-0}"
  done

  du -sh "${CACHE_ROOT}" 2>/dev/null || true
}

main() {
  require_cmd act
  require_cmd docker

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
