#!/usr/bin/env sh
# Runs the agent loop by hand on a copy of the shop fixture:
# scan, apply the "harder" patch, compare, apply the "fix" patch, compare.
# Usage: scripts/agent-loop.sh [path-to-stem-binary]
set -eu

here=$(cd "$(dirname "$0")/.." && pwd)
stem=${1:-"$here/target/debug/stem"}
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

cp -R "$here/tests/fixtures/projects/shop/." "$work/"
cd "$work"

"$stem" scan -o baseline.json > /dev/null

cp -R "$here/tests/fixtures/agent-loop/harder/." "$work/"
set +e
"$stem" compare baseline.json --json > harder.json
status=$?
set -e
if [ "$status" -ne 1 ]; then
  echo "expected exit 1 after the harder patch, got $status" >&2
  exit 1
fi
grep -q '"name": "applyDiscounts"' harder.json
grep -q 'Nesting adds' harder.json

cp -R "$here/tests/fixtures/agent-loop/fix/." "$work/"
"$stem" compare baseline.json --json > fix.json
grep -q '"result": "pass"' fix.json
echo "agent loop: ok"
