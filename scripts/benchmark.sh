#!/usr/bin/env bash
# Scans a project and fails when time or peak memory exceed the targets:
# under 10 seconds and under 2 GB.
# Usage: scripts/benchmark.sh <stem-binary> <project-folder>
set -euo pipefail

stem=$(cd "$(dirname "$1")" && pwd)/$(basename "$1")
project=$2
log=$(mktemp)
trap 'rm -f "$log"' EXIT

lines=$(find "$project" -name node_modules -prune -o -type f \
  \( -name '*.ts' -o -name '*.tsx' -o -name '*.mts' -o -name '*.cts' \) \
  ! -name '*.d.ts' ! -name '*.d.mts' ! -name '*.d.cts' -print0 | xargs -0 cat | wc -l)

(cd "$project" && /usr/bin/time -v "$stem" scan > /dev/null) 2> "$log"
seconds=$(awk -F': ' '/Elapsed \(wall clock\)/ { n = split($2, p, ":"); s = 0; for (i = 1; i <= n; i++) s = s * 60 + p[i]; print s }' "$log")
peak_kb=$(awk -F': ' '/Maximum resident set size/ { print $2 }' "$log")

echo "lines=$lines seconds=$seconds peak_kb=$peak_kb"
awk -v s="$seconds" -v m="$peak_kb" 'BEGIN { exit !(s < 10 && m < 2 * 1024 * 1024) }' \
  || { echo "benchmark target missed" >&2; exit 1; }
