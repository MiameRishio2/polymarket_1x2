#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
BINARY="$REPO_ROOT/target/release/polymarket-1x2"
PID_FILE="$REPO_ROOT/run/polymarket-1x2.pid"
LOG_FILE="$REPO_ROOT/logs/polymarket-1x2.out.log"

process_is_running() {
    local pid="$1"
    local state

    kill -0 "$pid" 2>/dev/null || return 1
    state="$(awk '{print $3}' "/proc/$pid/stat" 2>/dev/null || true)"
    [[ "$state" != "Z" ]]
}

process_matches_binary() {
    local pid="$1"
    local actual
    local expected

    actual="$(readlink -f "/proc/$pid/exe" 2>/dev/null || true)"
    expected="$(readlink -f "$BINARY" 2>/dev/null || true)"
    [[ -n "$actual" && -n "$expected" && "$actual" == "$expected" ]]
}

if [[ ! -x "$BINARY" ]]; then
    echo "Release binary not found: $BINARY" >&2
    echo "Run ./scripts/build.sh first." >&2
    exit 1
fi

if [[ -e "$PID_FILE" ]]; then
    pid="$(<"$PID_FILE")"
    if [[ ! "$pid" =~ ^[0-9]+$ ]]; then
        echo "Invalid PID file: $PID_FILE" >&2
        exit 1
    fi

    if process_is_running "$pid"; then
        if process_matches_binary "$pid"; then
            echo "polymarket-1x2 is already running with PID $pid." >&2
        else
            echo "PID $pid belongs to another executable; refusing to start." >&2
        fi
        exit 1
    fi

    rm -f -- "$PID_FILE"
fi

mkdir -p -- "$(dirname -- "$PID_FILE")" "$(dirname -- "$LOG_FILE")"

(
    cd -- "$REPO_ROOT"
    nohup "$BINARY" >>"$LOG_FILE" 2>&1 &
    printf '%s\n' "$!" >"$PID_FILE"
)

pid="$(<"$PID_FILE")"
sleep 0.2

if ! process_is_running "$pid" || ! process_matches_binary "$pid"; then
    rm -f -- "$PID_FILE"
    echo "polymarket-1x2 failed to start; inspect $LOG_FILE." >&2
    exit 1
fi

echo "Started polymarket-1x2 with PID $pid."
echo "Log: $LOG_FILE"
