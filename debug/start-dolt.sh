#!/usr/bin/env bash

set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BEADS_DIR="$PROJECT_ROOT/.beads"
METADATA_FILE="$BEADS_DIR/metadata.json"
LEGACY_DOLT_ROOT="$BEADS_DIR/dolt"
LEGACY_DATA_DIR="$LEGACY_DOLT_ROOT/beads"
GLOBAL_DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/beads/dolt-server"
BACKUP_ROOT="${XDG_DATA_HOME:-$HOME/.local/share}/beads/dolt-backups"
SERVICE_NAME="beads-dolt.service"

HOST="${BEADS_DOLT_SERVER_HOST:-127.0.0.1}"
PORT="${BEADS_DOLT_SERVER_PORT:-3307}"
USER_NAME="${BEADS_DOLT_SERVER_USER:-root}"
PASSWORD="${BEADS_DOLT_PASSWORD-}"

fail() {
  echo "ERROR: $*" >&2
  exit 1
}

require_file() {
  local path="$1"
  [[ -f "$path" ]] || fail "required file not found: $path"
}

read_metadata_value() {
  local key="$1"
  sed -n "s/.*\"${key}\":[[:space:]]*\"\\([^\"]*\\)\".*/\\1/p" "$METADATA_FILE" | head -n 1
}

sql_escape() {
  local value="$1"
  printf "%s" "${value//\'/\'\'}"
}

validate_dolt_access() {
  local password="$1"
  dolt --host "$HOST" --port "$PORT" --user "$USER_NAME" --password "$password" --no-tls --use-db "$DB_NAME" sql -q "show tables" >/dev/null 2>&1
}

wait_for_service() {
  local attempt
  for attempt in $(seq 1 20); do
    if systemctl --user is-active --quiet "$SERVICE_NAME" && ss -ltn | rg -q ":${PORT}[[:space:]]"; then
      return 0
    fi
    sleep 1
  done
  return 1
}

archive_legacy_storage() {
  local backup_path="$BACKUP_ROOT/${DB_NAME}-$(date +%Y%m%d-%H%M%S)"
  mkdir -p "$BACKUP_ROOT"
  mv "$LEGACY_DOLT_ROOT" "$backup_path"
  echo "$backup_path"
}

require_file "$METADATA_FILE"

DB_NAME="$(read_metadata_value "dolt_database")"
[[ -n "$DB_NAME" ]] || fail "dolt_database is missing in $METADATA_FILE"

DOLT_MODE="$(read_metadata_value "dolt_mode")"
[[ "$DOLT_MODE" == "server" ]] || fail "metadata dolt_mode must be 'server'"

GLOBAL_DB_DIR="$GLOBAL_DATA_DIR/$DB_NAME"
GLOBAL_DB_DOLT_DIR="$GLOBAL_DB_DIR/.dolt"
LEGACY_DB_DOLT_DIR="$LEGACY_DATA_DIR/$DB_NAME/.dolt"

mkdir -p "$GLOBAL_DATA_DIR"

if [[ ! -d "$GLOBAL_DB_DOLT_DIR" ]]; then
  [[ -d "$LEGACY_DB_DOLT_DIR" ]] || fail "global database is missing and no legacy local database was found at $LEGACY_DB_DOLT_DIR"
  mkdir -p "$GLOBAL_DB_DIR"
  cp -a "$LEGACY_DB_DOLT_DIR" "$GLOBAL_DB_DOLT_DIR"
  LEGACY_BACKUP_PATH="$(archive_legacy_storage)"
else
  if [[ -d "$LEGACY_DB_DOLT_DIR" ]]; then
    fail "split-brain risk: both global database ($GLOBAL_DB_DOLT_DIR) and legacy local database ($LEGACY_DB_DOLT_DIR) exist"
  fi
  LEGACY_BACKUP_PATH=""
fi

systemctl --user daemon-reload
systemctl --user enable --now "$SERVICE_NAME" >/dev/null
wait_for_service || fail "service $SERVICE_NAME did not become ready on ${HOST}:${PORT}"

if [[ -n "$PASSWORD" ]]; then
  if ! validate_dolt_access "$PASSWORD"; then
    if validate_dolt_access ""; then
      [[ "$USER_NAME" == "root" ]] || fail "bootstrap password flow supports only root user"
      ESCAPED_PASSWORD="$(sql_escape "$PASSWORD")"
      dolt --host "$HOST" --port "$PORT" --user "$USER_NAME" --password "" --no-tls sql -q "alter user 'root'@'localhost' identified by '$ESCAPED_PASSWORD'"
      validate_dolt_access "$PASSWORD" || fail "failed to validate dolt access after root password bootstrap"
    else
      fail "dolt is reachable, but authentication failed for both BEADS_DOLT_PASSWORD and empty password"
    fi
  fi
else
  validate_dolt_access "" || fail "dolt requires authentication; export BEADS_DOLT_PASSWORD before running this script"
fi

BEADS_DOLT_PASSWORD="$PASSWORD" bd doctor --server >/dev/null

echo "Service: $SERVICE_NAME"
echo "Server: ${HOST}:${PORT}"
echo "Database: $DB_NAME"
echo "Global storage: $GLOBAL_DB_DIR"
if [[ -n "${LEGACY_BACKUP_PATH:-}" ]]; then
  echo "Legacy backup: $LEGACY_BACKUP_PATH"
fi
echo "Status: ready"
