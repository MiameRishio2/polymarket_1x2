#!/usr/bin/env bash
set -euo pipefail

SOURCE_ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
TEMP_ROOT="$(mktemp -d)"
MISMATCH_PID=""

cleanup() {
    if [[ -n "$MISMATCH_PID" ]] && kill -0 "$MISMATCH_PID" 2>/dev/null; then
        kill "$MISMATCH_PID" 2>/dev/null || true
        wait "$MISMATCH_PID" 2>/dev/null || true
    fi
    rm -rf -- "$TEMP_ROOT"
}
trap cleanup EXIT

fail() {
    echo "FAIL: $*" >&2
    exit 1
}

assert_succeeds() {
    local description="$1"
    shift

    if ! "$@"; then
        fail "$description"
    fi
    echo "PASS: $description"
}

assert_fails() {
    local description="$1"
    shift

    if "$@"; then
        fail "$description"
    fi
    echo "PASS: $description"
}

for script in build.sh start.sh stop.sh; do
    [[ -f "$SOURCE_ROOT/scripts/$script" ]] ||
        fail "required source script is missing: scripts/$script"
done

repo="$TEMP_ROOT/repo"
mkdir -p "$repo/scripts" "$repo/target/release"
cp "$SOURCE_ROOT"/scripts/{build,start,stop}.sh "$repo/scripts/"
chmod +x "$repo"/scripts/*.sh

rm -f "$repo/target/release/polymarket-1x2"
assert_fails "start rejects a missing release binary" "$repo/scripts/start.sh"
assert_succeeds "stop treats a missing PID file as already stopped" \
    "$repo/scripts/stop.sh"

cat >"$TEMP_ROOT/fake_collector.c" <<'EOF'
#include <signal.h>
#include <unistd.h>

static volatile sig_atomic_t running = 1;

static void stop(int signal_number) {
    (void)signal_number;
    running = 0;
}

int main(void) {
    signal(SIGTERM, stop);
    while (running) {
        pause();
    }
    return 0;
}
EOF
cc "$TEMP_ROOT/fake_collector.c" -o "$repo/target/release/polymarket-1x2"

assert_succeeds "start launches the release binary" "$repo/scripts/start.sh"
pid="$(<"$repo/run/polymarket-1x2.pid")"
kill -0 "$pid" 2>/dev/null || fail "started process $pid is not alive"

assert_fails "start rejects a duplicate instance" "$repo/scripts/start.sh"
assert_succeeds "stop terminates the matching process" "$repo/scripts/stop.sh"
[[ ! -e "$repo/run/polymarket-1x2.pid" ]] ||
    fail "stop left the PID file behind"

(exit 0) &
stale_pid="$!"
wait "$stale_pid"
printf '%s\n' "$stale_pid" >"$repo/run/polymarket-1x2.pid"
assert_succeeds "stop removes a stale PID file" "$repo/scripts/stop.sh"
[[ ! -e "$repo/run/polymarket-1x2.pid" ]] ||
    fail "stale PID file was not removed"

sleep 30 &
MISMATCH_PID="$!"
printf '%s\n' "$MISMATCH_PID" >"$repo/run/polymarket-1x2.pid"
assert_fails "stop rejects a PID owned by another executable" \
    "$repo/scripts/stop.sh"
kill -0 "$MISMATCH_PID" 2>/dev/null ||
    fail "mismatched process was unexpectedly stopped"

echo "All deployment script tests passed."
