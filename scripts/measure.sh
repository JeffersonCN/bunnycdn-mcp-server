#!/usr/bin/env bash
# Measures the C1 lightness targets of bunnycdn-mcp (specs/constitution.md):
#   - release binary size  (target <= 8 MB)
#   - idle RSS after `initialize` over stdio (target <= 10 MB)
#   - startup: spawn -> `initialize` response on stdout (target <= 20 ms)
#
# Usage: scripts/measure.sh [--no-build] [--runs N] [--bin PATH]
#
# Portable across macOS (/usr/bin/time -l) and Linux (/proc/<pid>/status).
# The server is started with a dummy API key; the real BUNNY_API_KEY is never read nor printed.
# `initialize` does not touch the network, so the dummy key is enough.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN="$ROOT/target/release/bunnycdn-mcp"
BUILD=1
RUNS=20
IDLE_SECS=1

while [ $# -gt 0 ]; do
    case "$1" in
        --no-build) BUILD=0 ;;
        --runs) RUNS="$2"; shift ;;
        --bin) BIN="$2"; BUILD=0; shift ;;
        -h|--help) sed -n '2,11p' "$0"; exit 0 ;;
        *) echo "unknown argument: $1" >&2; exit 64 ;;
    esac
    shift
done

command -v python3 >/dev/null || { echo "python3 is required for startup timing" >&2; exit 69; }

if [ "$BUILD" -eq 1 ]; then
    echo "building release binary..." >&2
    (cd "$ROOT" && cargo build --release --locked --bin bunnycdn-mcp >&2)
fi
[ -x "$BIN" ] || { echo "binary not found: $BIN" >&2; exit 66; }

# Isolated environment for the server: dummy key, no inherited secrets or logging.
run_env=(env -i "PATH=$PATH" "BUNNY_API_KEY=measure-dummy-key" "RUST_LOG=off")

INIT='{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"measure","version":"0"}}}'
INITIALIZED='{"jsonrpc":"2.0","method":"notifications/initialized"}'

OS="$(uname -s)"
ARCH="$(uname -m)"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

# --- 1. Binary size ------------------------------------------------------------------------
if [ "$OS" = "Darwin" ]; then
    size_bytes="$(stat -f %z "$BIN")"
else
    size_bytes="$(stat -c %s "$BIN")"
fi

# --- 2. Idle RSS -------------------------------------------------------------------------
# The server receives `initialize` + `initialized`, stays idle for IDLE_SECS, then stdin closes
# and it exits.
feed() { printf '%s\n%s\n' "$INIT" "$INITIALIZED"; sleep "$IDLE_SECS"; }

if [ "$OS" = "Darwin" ]; then
    # Max RSS over the whole (idle) lifetime, in bytes.
    feed | /usr/bin/time -l "${run_env[@]}" "$BIN" >/dev/null 2>"$TMP/time.txt"
    rss_kb=$(( $(awk '/maximum resident set size/ {print $1}' "$TMP/time.txt") / 1024 ))
    rss_src="/usr/bin/time -l (maximum resident set size)"
else
    mkfifo "$TMP/in"
    "${run_env[@]}" "$BIN" <"$TMP/in" >"$TMP/out" 2>/dev/null &
    pid=$!
    exec 3>"$TMP/in"
    printf '%s\n%s\n' "$INIT" "$INITIALIZED" >&3
    sleep "$IDLE_SECS"
    # VmHWM = peak RSS, VmRSS = current RSS (kB).
    rss_kb="$(awk '/^VmHWM:/ {print $2}' "/proc/$pid/status")"
    rss_now_kb="$(awk '/^VmRSS:/ {print $2}' "/proc/$pid/status")"
    exec 3>&-
    wait "$pid" || true
    rss_src="/proc/<pid>/status (VmHWM; VmRSS=${rss_now_kb} kB)"
fi

# --- 3. Startup ----------------------------------------------------------------------------
# Wall time from spawn until the full `initialize` response line is read; RUNS samples.
startup="$(python3 -I - "$BIN" "$RUNS" "$INIT" <<'PY'
import os, statistics, subprocess, sys, time

binary, runs, init = sys.argv[1], int(sys.argv[2]), sys.argv[3]
env = {"PATH": os.environ.get("PATH", ""), "BUNNY_API_KEY": "measure-dummy-key", "RUST_LOG": "off"}
samples = []
for _ in range(runs):
    t0 = time.perf_counter()
    p = subprocess.Popen([binary], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                         stderr=subprocess.DEVNULL, env=env)
    p.stdin.write((init + "\n").encode())
    p.stdin.flush()
    line = p.stdout.readline()
    dt = (time.perf_counter() - t0) * 1000
    p.stdin.close()
    p.wait(timeout=5)
    if b'"result"' not in line:
        sys.exit("unexpected initialize response")
    samples.append(dt)
samples.sort()
print(f"{samples[0]:.1f} {statistics.median(samples):.1f} {samples[-1]:.1f}")
PY
)"
read -r st_min st_med st_max <<<"$startup"

# --- Report --------------------------------------------------------------------------------
verdict() { awk -v v="$1" -v lim="$2" 'BEGIN { print (v <= lim) ? "OK" : "FAIL" }'; }
size_mb="$(awk -v b="$size_bytes" 'BEGIN { printf "%.2f", b / 1048576 }')"
rss_mb="$(awk -v k="$rss_kb" 'BEGIN { printf "%.2f", k / 1024 }')"

echo "bunnycdn-mcp lightness ($OS $ARCH, $(date -u +%Y-%m-%dT%H:%MZ))"
printf '%-10s %-28s %-10s %s\n' "metric" "measured" "target" "result"
printf '%-10s %-28s %-10s %s\n' "binary" "$size_mb MB ($size_bytes B)" "<= 8 MB" "$(verdict "$size_mb" 8)"
printf '%-10s %-28s %-10s %s\n' "idle_rss" "$rss_mb MB" "<= 10 MB" "$(verdict "$rss_mb" 10)"
printf '%-10s %-28s %-10s %s\n' "startup" "${st_med} ms (median of $RUNS)" "<= 20 ms" "$(verdict "$st_med" 20)"
echo "startup min/median/max: ${st_min}/${st_med}/${st_max} ms"
echo "rss source: $rss_src"
