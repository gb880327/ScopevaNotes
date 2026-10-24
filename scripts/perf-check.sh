#!/bin/bash
# Startup-time and idle-residency measurement for the bundled app (task 8.4).
#
#   scripts/perf-check.sh startup [app-path]   launch twice, report milestones
#   scripts/perf-check.sh soak <hours> [app]   sample CPU/RSS until stopped
#
# Milestones: process exists, library DB file opened, WebKit content process
# spawned. The last one is the closest observable proxy for "ready to show";
# the app starts hidden, so window-shown time can only be timed by hand.
set -euo pipefail

DEFAULT_APP="src-tauri/target/release/bundle/macos/Notes.app"
CPU_SECONDS='
import sys
raw = sys.argv[1].strip().replace(",", ".")
frac = 0.0
if "." in raw:
    raw, _, f = raw.rpartition(".")
    frac = float("0." + f)
parts = [float(p) for p in raw.split(":")]
while len(parts) < 3:
    parts.insert(0, 0.0)
h, m, s = parts
print(f"{h * 3600 + m * 60 + s + frac:.2f}")
'

cputime() { python3 -c "$CPU_SECONDS" "$(ps -o time= -p "$1")"; }

find_pid() {
  pgrep -f "Notes.app/Contents/MacOS/macos-notes" | head -1 || true
}

startup() {
  local app
  app=$(cd "$(dirname "$1")" && pwd)/$(basename "$1")
  for run in cold warm; do
    # "cold" = first launch of this build; "warm" = launch after the binary and
    # webview frameworks are already in the page cache. Neither drops caches:
    # that needs sudo, so treat the first number as an upper bound.
    local start pid db_at web_at
    start=$(python3 -c 'import time; print(time.time())')
    open -a "$app"
    pid=""
    for _ in $(seq 1 600); do
      pid=$(find_pid)
      [ -n "$pid" ] && break
      sleep 0.02
    done
    [ -n "$pid" ] || { echo "$run: process never appeared" >&2; exit 1; }
    local t_proc
    t_proc=$(python3 -c "import time; print(round(time.time()-$start, 3))")

    db_at=""
    for _ in $(seq 1 600); do
      if lsof -p "$pid" 2>/dev/null | grep -q "notes.db"; then db_at=1; break; fi
      sleep 0.02
    done
    local t_db
    t_db=$(python3 -c "import time; print(round(time.time()-$start, 3))")

    web_at=""
    for _ in $(seq 1 600); do
      if pgrep -P "$pid" -f "com.apple.WebKit" >/dev/null 2>&1 ||
         pgrep -f "com.apple.WebKit.WebContent" >/dev/null 2>&1; then web_at=1; break; fi
      sleep 0.02
    done
    local t_web
    t_web=$(python3 -c "import time; print(round(time.time()-$start, 3))")

    echo "$run: pid=$pid process=${t_proc}s db_open=$([ -n "$db_at" ] && echo "${t_db}s" || echo n/a) webkit=$([ -n "$web_at" ] && echo "${t_web}s" || echo n/a)"

    # Quit gracefully so the warm run starts from the same state.
    kill -TERM "$pid" 2>/dev/null || true
    for _ in $(seq 1 200); do [ -z "$(find_pid)" ] && break; sleep 0.05; done
  done
}

soak() {
  local hours="$1" app
  app=$(cd "$(dirname "$2")" && pwd)/$(basename "$2")
  local end=$(( $(date +%s) + $(python3 -c "print(int(float('$hours') * 3600))") ))
  local pid
  pid=$(find_pid)
  if [ -z "$pid" ]; then
    open -a "$app"
    for _ in $(seq 1 600); do pid=$(find_pid); [ -n "$pid" ] && break; sleep 0.05; done
  fi
  [ -n "$pid" ] || { echo "app is not running" >&2; exit 1; }

  echo "t_s,cpu_pct,rss_mb"
  local prev prev_epoch now cur wall rss i=0
  prev=$(cputime "$pid")
  prev_epoch=$(date +%s)
  while [ "$(date +%s)" -lt "$end" ]; do
    sleep 20
    now=$(date +%s)
    cur=$(cputime "$pid")
    wall=$(( now - prev_epoch ))
    rss=$(ps -o rss= -p "$pid" | tr -d ' ')
    python3 -c "print(f'{$i + 20},{(($cur - $prev) / $wall * 100):.2f},{$rss / 1024:.1f}')"
    prev=$cur; prev_epoch=$now; i=$((i + 20))
  done
}

case "${1:-}" in
  startup) startup "${2:-$DEFAULT_APP}" ;;
  soak)    soak "${2:?hours}" "${3:-$DEFAULT_APP}" ;;
  *)       sed -n '2,10p' "$0"; exit 1 ;;
esac
