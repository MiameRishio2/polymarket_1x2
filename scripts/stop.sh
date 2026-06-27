#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
BINARY="$REPO_ROOT/target/release/polymarket-1x2"
PID_FILE="$REPO_ROOT/run/polymarket-1x2.pid"

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

if [[ ! -e "$PID_FILE" ]]; then
    echo "polymarket-1x2 is not running."
    exit 0
fi

pid="$(<"$PID_FILE")"
if [[ ! "$pid" =~ ^[0-9]+$ ]]; then
    echo "Invalid PID file: $PID_FILE" >&2
    exit 1
fi

if ! process_is_running "$pid"; then
    rm -f -- "$PID_FILE"
    echo "Removed stale PID file; polymarket-1x2 is not running."
    exit 0
fi

if ! process_matches_binary "$pid"; then
    echo "PID $pid belongs to another executable; refusing to stop it." >&2
    exit 1
fi

kill -TERM "$pid"

for _ in {1..100}; do
    if ! process_is_running "$pid"; then
        rm -f -- "$PID_FILE"
        echo "Stopped polymarket-1x2 (PID $pid)."
        exit 0
    fi
    sleep 0.1
done

echo "Timed out waiting for polymarket-1x2 (PID $pid) to stop." >&2
echo "The process was not sent SIGKILL; investigate it manually." >&2
exit 1
