#!/bin/bash
# The Playground workflow's browser checks (.github/workflows/pages.yml), dry run on pannous.com over SSH: no browser
# starts on the Mac (user 2026-10-09) and trial runs no longer mail "Run failed" (user 2026-10-15, notes/ci.md).
# Builds here exactly as the workflow does, copies the result to the server and runs the same commands there:
#   scripts/browser-dry-run.sh                    tour-chrome, tour-firefox and the browser suite (the whole workflow)
#   scripts/browser-dry-run.sh --tours [name…]    both tours, only those examples when named (as the Safari job names them)
#   scripts/browser-dry-run.sh --chrome [name…]   the Chrome tour   · --firefox [name…]  the Firefox tour
#   scripts/browser-dry-run.sh --suite [filter…]  `cargo browser-test [filter…]`, the suite in headless Chrome
#   scripts/browser-dry-run.sh --again --chrome …  no build, no copy: the server checks what the last run copied
# Run it in the checkout or worktree to check; it checks the files as they are, committed or not. Exit code 0 when
# every chosen check passed. Safari has no server: it stays in the workflow (macos runner).
# The server side (user browsercheck, /srv/browser-checks: Chrome of agent-browser, Mozilla's Firefox) is set up as
# notes/ci.md "Browser dry run on pannous.com" describes; runs take turns by a lock, niced and in a memory-capped scope,
# so the sites the server hosts stay responsive.
set -euo pipefail

SERVER=pannous.com
SERVER_HOME=/srv/browser-checks
SERVER_USER=browsercheck
MEMORY_MAX=2500M # of 3.7 GB; the server hosts warp.pannous.com and more
CHECK_TIME_LIMIT=45m
RUNS_KEPT_DAYS=2
WORKFLOW=.github/workflows/pages.yml

repository=$(git rev-parse --show-toplevel)
cd "$repository"
local_dir="$repository/scratch/browser-dry-run"
branch=$({ git symbolic-ref --short -q HEAD || git rev-parse --short HEAD; } | tr '/' '-')
run_dir="$SERVER_HOME/runs/$branch"

again=""
if [ "${1:-}" = --again ]; then again=1; shift; fi
checks=(chrome firefox suite)
case "${1:-}" in
	--tours) checks=(chrome firefox); shift ;;
	--chrome | --firefox | --suite) checks=("${1#--}"); shift ;;
	-*) sed -n '2,14p' "$0" >&2; exit 2 ;;
esac
names=("$@")
wants() { [[ " ${checks[*]} " == *" $1 "* ]]; }
wants_tour() { wants chrome || wants firefox; }

# a value of the workflow's env (AGENT_BROWSER_ARGS, FIREFOX_PREFS), so both run the browsers alike
workflow_value() {
	grep -m1 -E "^ +$1: " "$WORKFLOW" | sed -E "s/^ +$1: //; s/ +#.*//; s/^\"(.*)\"$/\1/"
}

# the test binary `cargo browser-test` would run: built for wasm32-wasip1 without the native feature
build_suite() {
	# only built here, so no turn in tests/queue.sh: the test binary runs on the server
	cargo --offline build --target wasm32-wasip1 --no-default-features --test tests --message-format=json-render-diagnostics |
		python3 -c 'import json, sys
print(next(message["executable"] for line in sys.stdin if (message := json.loads(line)).get("executable")))' > "$local_dir/tests_path"
	web/playground/build.sh components
	web/playground/build.sh pinned
}

build() {
	mkdir -p "$local_dir"
	if wants_tour; then
		web/playground/build.sh
		web/playground/build.sh site "$local_dir/_site"
	fi
	if wants suite; then build_suite; fi
}

# ownership is set on the server afterwards: macOS system rsync (openrsync) has no --chown
copy() { rsync -a --delete "$@"; }

copy_to_server() {
	echo "copying to $SERVER:$run_dir"
	ssh "$SERVER" "mkdir -p $run_dir && chown $SERVER_USER: $SERVER_HOME/runs $run_dir"
	# the repository's files as they are, new ones too: the suite serves them (tests.html, samples, fixtures), the tours
	# read the runner's scripts
	git ls-files -z --cached --others --exclude-standard | copy --from0 --files-from=- --ignore-missing-args . "$SERVER:$run_dir/"
	# what the builds made for the page, which git ignores
	copy web/playground/{components,served-files.js} "$SERVER:$run_dir/web/playground/"
	if wants_tour; then copy "$local_dir/_site/" "$SERVER:$run_dir/_site/"; fi
	if wants suite; then
		copy "$(cat "$local_dir/tests_path")" "$SERVER:$run_dir/tests.wasm"
		# the pinned packages (build.sh pinned): their contents, not the links into ~/.cache
		copy --copy-links --exclude .git packages/ "$SERVER:$run_dir/packages/"
	fi
	ssh "$SERVER" "chown -R $SERVER_USER: $run_dir"
}

[ -n "$again" ] || { build && copy_to_server; }

remote_arguments=$(printf '%q ' "$run_dir" "$(workflow_value AGENT_BROWSER_ARGS)" "$(workflow_value FIREFOX_PREFS)" "${checks[*]}" "${names[@]+"${names[@]}"}")
# the token goes in on stdin, never on a command line the server's ps shows (api.github.com's limit, samples/live_region)
{
	echo "export GITHUB_TOKEN=$(printf '%q' "$(gh auth token 2>/dev/null || true)")"
	echo "SERVER_HOME=$SERVER_HOME SERVER_USER=$SERVER_USER MEMORY_MAX=$MEMORY_MAX CHECK_TIME_LIMIT=$CHECK_TIME_LIMIT RUNS_KEPT_DAYS=$RUNS_KEPT_DAYS"
	cat <<'SERVER_SIDE'
set -uo pipefail
run_dir=$1 agent_browser_args=$2 firefox_prefs=$3 checks=$4; shift 4
find "$SERVER_HOME/runs" -mindepth 1 -maxdepth 1 -mtime +"$RUNS_KEPT_DAYS" ! -path "$run_dir" -exec rm -rf {} +
exec 9> "$SERVER_HOME/run.lock"
flock -n 9 || { echo "another dry run holds the server ($SERVER_HOME/run.lock): waiting"; flock 9; }
unit=browser-dry-run-$$
trap 'systemctl stop "$unit".scope 2>/dev/null' EXIT
cd "$run_dir"
# the user's own, low priority, memory-capped: a runaway browser is killed in its scope, not the server's sites; a
# clean environment (root's XDG_RUNTIME_DIR kept agent-browser from making its socket)
as_browser_user() {
	systemd-run --quiet --scope --unit="$unit" --uid="$SERVER_USER" --gid="$SERVER_USER" -p MemoryMax="$MEMORY_MAX" \
		-p MemorySwapMax=0 -p CPUWeight=20 -p IOWeight=20 \
		env -i HOME="$SERVER_HOME" USER="$SERVER_USER" LANG=C.UTF-8 PATH="$SERVER_HOME/npm/bin:/usr/local/bin:/usr/bin:/bin" CI=1 GITHUB_TOKEN="$GITHUB_TOKEN" \
		AGENT_BROWSER_ARGS="$agent_browser_args" FIREFOX="$SERVER_HOME/firefox/firefox" FIREFOX_PREFS="$firefox_prefs" \
		WARP_COMPONENTS_BUILT=1 timeout "$CHECK_TIME_LIMIT" nice -n 15 ionice -c 3 "$@"
}
tour_chrome() { as_browser_user python3 web/playground/test_in_browser.py --examples --site _site "$@"; }
# as the workflow: a failed first attempt runs once more, loudly (card tour-firefox-stall)
tour_firefox() {
	as_browser_user python3 web/playground/test_in_browser.py --examples --firefox --site _site "$@" && return
	echo "warning: the Firefox tour failed once, running it again (card tour-firefox-stall)"
	as_browser_user python3 web/playground/test_in_browser.py --examples --firefox --site _site "$@"
}
suite() { as_browser_user python3 web/playground/test_in_browser.py tests.wasm "$@"; }
declare -A command=([chrome]=tour_chrome [firefox]=tour_firefox [suite]=suite)
verdicts=() failed=0
for check in $checks; do
	echo "==== $check on $(hostname) ===="
	if "${command[$check]}" "$@"; then verdicts+=("$check: ok"); else verdicts+=("$check: FAILED"); failed=1; fi
done
echo "==== browser dry run of $(basename "$run_dir") ===="
printf '  %s\n' "${verdicts[@]}"
exit $failed
SERVER_SIDE
} | ssh "$SERVER" "bash -s -- $remote_arguments"
