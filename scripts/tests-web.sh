#!/usr/bin/env bash
# scripts/tests-web.sh [<tree>, default this checkout] [test/<topic>…]: every case of test/ (each pair of a *.pairs.warp,
# each is! line) run in the headless playground page (agent-browser's Chrome, the page served from the tree), judged as
# the native `warp test` judges (warp.wasm test-list and test-verdicts), against the stored native baseline
# (scripts/baseline.sh). Writes test/tests-web.txt, tracked (status and case, one a line: ok, PAGE fails only in the
# page, BOTH fails natively too, NONE fails with no native verdict stored) when all of test/ ran, and
# data/tests-web/report.txt. The page runs one case at a time, beside scripts/samples-web.sh (notes/playground_plan.md).
set -uo pipefail
tree=$(cd "${1:-$(git rev-parse --show-toplevel)}" && pwd) || exit 2
shift $(( $# > 0 ))
[ -f "$tree/web/playground/warp-lib/warp.wasm" ] || { echo "error: $tree/web/playground/warp-lib/warp.wasm is missing: web/playground/build.sh" >&2; exit 2; }
exec python3 "$tree/web/playground/test_in_browser.py" --tests "$@"
