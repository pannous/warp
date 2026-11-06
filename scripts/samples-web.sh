#!/usr/bin/env bash
# scripts/samples-web.sh [<tree>, default this checkout] [samples/….warp…]: every sample in the headless playground page
# (agent-browser's Chrome, the page served from the tree), its output against the native run's, as scripts/samples.sh
# made it (data/samples/out: the tree's, else the main checkout's). Writes test/samples-web.txt, tracked (status and
# sample, one a line: ok, drew a C library's window drawn in the page (raylib, SDL: natively a window, no output to
# compare), DIFF the page printed otherwise, FAIL an error, TIME no answer, ran no native output) when all ran, and data/samples-web/report.txt (each difference and error) and out/ (what the page
# printed). The page runs one sample at a time (notes/playground_plan.md step 2).
set -uo pipefail
source "$(dirname "$0")/lib/platform.sh"
source "$(dirname "$0")/lib/results-header.sh"
tree=$(cd "${1:-$(git rev-parse --show-toplevel)}" && pwd) || exit 2
shift $(( $# > 0 ))
main_checkout=$(git -C "$tree" worktree list --porcelain | sed -n '1s/^worktree //p')
for outputs in "$tree/data/samples/out" "$main_checkout/data/samples/out"; do
	[ -d "$outputs" ] && break
done
[ -f "$tree/web/playground/warp-lib/warp.wasm" ] || { echo "error: $tree/web/playground/warp-lib/warp.wasm is missing: web/playground/build.sh" >&2; exit 2; }
WARP_NATIVE_OUTPUTS=$outputs results=$tree/test/samples-web.txt
before=$(mtime_of "$results" 2>/dev/null)
python3 "$tree/web/playground/test_in_browser.py" --samples "$@"
status=$?
# the page script writes the results only when all ran: a header only for a file it wrote anew
[ "$(mtime_of "$results" 2>/dev/null)" != "$before" ] && results_header "$results" "$(git -C "$tree" rev-parse --short=10 HEAD)" "ok|drew"
exit $status
