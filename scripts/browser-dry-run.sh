#!/bin/bash
# The Playground workflow's tours (.github/workflows/pages.yml), dry run on pannous.com over SSH: no browser starts on
# the Mac and a trial run mails nothing. Builds the site here as the workflow does (web/playground/build.sh with the
# installed warp's lib/, or WARP_LIB), copies it to the server and runs the same tour commands there:
#   scripts/browser-dry-run.sh                     the Chrome and the Firefox tour of the collected site
#   scripts/browser-dry-run.sh --chrome [name…]    only the Chrome tour · --firefox [name…] only the Firefox one
#   scripts/browser-dry-run.sh --guide             the guide's snippets (test_in_browser.py --guide), in Chrome
#   scripts/browser-dry-run.sh --samples [sample…] the samples in the page against data/samples/out, the native outputs
#                                                  (scripts/samples.sh); brings back test/samples-web.txt, data/samples-web
#   scripts/browser-dry-run.sh --again …           no build, no copy: the server checks what the last run copied
# Names pick examples, as the Safari job names them. It checks the files as they are, committed or not, and prints the
# page's first-load size (the compiler and its lib). Exit code 0 when every chosen check passed. The server side is
# set up as notes/ci.md "Browser dry run on pannous.com" describes: runs take turns by a lock, niced and in a
# memory-capped scope, so the sites the server hosts stay responsive.
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
local_dir="$repository/data/browser-dry-run"
site="$local_dir/_site"
branch=$({ git symbolic-ref --short -q HEAD || git rev-parse --short HEAD; } | tr '/' '-')
run_dir="$SERVER_HOME/runs/$branch"

again=""
if [ "${1:-}" = --again ]; then again=1; shift; fi
checks=(chrome firefox)
case "${1:-}" in
	--chrome | --firefox | --guide | --samples) checks=("${1#--}"); shift ;;
	-*) sed -n '2,12p' "$0" >&2; exit 2 ;;
esac

# a value of the workflow's env (AGENT_BROWSER_ARGS, FIREFOX_PREFS), so both run the browsers alike
workflow_value() {
	grep -m1 -E "^ +$1: " "$WORKFLOW" | sed -E "s/^ +$1: //; s/ +#.*//; s/^\"(.*)\"$/\1/"
}

# what a visitor loads before the first run: the page's own files, the compiler and the runtime it links
first_load_size() {
	local lib="$site/web/playground/warp-lib"
	echo "first load: warp.wasm $(du -h "$lib/warp.wasm" | cut -f1), runtime.wasm $(du -h "$lib/runtime.wasm" | cut -f1);" \
		"warp-lib $(du -sh "$lib" | cut -f1) on the server (modules and sources fetched as a program uses them), the whole site $(du -sh "$site" | cut -f1)"
}

build() {
	mkdir -p "$local_dir"
	web/playground/build.sh
	web/playground/build.sh site "$site"
	first_load_size
}

# ownership is set on the server afterwards: macOS system rsync (openrsync) has no --chown
copy() { rsync -a --delete "$@"; }

copy_to_server() {
	echo "copying to $SERVER:$run_dir"
	ssh "$SERVER" "mkdir -p $run_dir/data/samples && chown $SERVER_USER: $SERVER_HOME/runs $run_dir"
	# the repository's files as they are, new ones too: the tours run the runner's scripts from them
	git ls-files -z --cached --others --exclude-standard | copy --from0 --files-from=- --ignore-missing-args . "$SERVER:$run_dir/"
	copy "$site/" "$SERVER:$run_dir/_site/"
	# the samples check serves the tree itself: what git ignores there too, the page's built files (the repository's
	# served-files.js, not the site's), the packages samples use by path and the native outputs it compares with
	if [ "${checks[*]}" = samples ]; then
		copy --exclude warp-lib web/playground/ "$SERVER:$run_dir/web/playground/"
		[ -d packages ] && copy -L --exclude .git packages/ "$SERVER:$run_dir/packages/"
		[ -d data/samples/out ] && copy data/samples/out/ "$SERVER:$run_dir/data/samples/out/"
	fi
	ssh "$SERVER" "chown -R $SERVER_USER: $run_dir"
}

# separate commands: set -e stops at neither half of an && list, so a failed build went on to check an empty site
if [ -z "$again" ]; then
	build
	copy_to_server
fi

remote_arguments=$(printf '%q ' "$run_dir" "$(workflow_value AGENT_BROWSER_ARGS)" "$(workflow_value FIREFOX_PREFS)" "${checks[*]}" "$@")
# the token goes in on stdin, never on a command line the server's ps shows (api.github.com's limit, samples/web/live_region)
{
	echo "export GITHUB_TOKEN=$(printf '%q' "$(gh auth token 2>/dev/null || true)")"
	echo "SERVER_HOME=$SERVER_HOME SERVER_USER=$SERVER_USER MEMORY_MAX=$MEMORY_MAX CHECK_TIME_LIMIT=$CHECK_TIME_LIMIT RUNS_KEPT_DAYS=$RUNS_KEPT_DAYS"
	cat <<'SERVER_SIDE'
set -uo pipefail
run_dir=$1 agent_browser_args=$2 firefox_prefs=$3 checks=$4; shift 4
find "$SERVER_HOME/runs" -mindepth 1 -maxdepth 1 -mtime +"$RUNS_KEPT_DAYS" ! -path "$run_dir" -exec rm -rf {} +
exec 9> "$SERVER_HOME/run.lock"
flock -n 9 || { echo "another dry run holds the server ($SERVER_HOME/run.lock): waiting"; flock 9; }
# one scope per browser start: a check's scope still loaded (its browser lingering) made the next start fail
units=() started=0
trap 'systemctl stop "${units[@]/%/.scope}" 2>/dev/null' EXIT
cd "$run_dir"
# the user's own, low priority, memory-capped: a runaway browser is killed in its scope, not the server's sites; a
# clean environment (root's XDG_RUNTIME_DIR kept agent-browser from making its socket)
as_browser_user() {
	local unit=browser-dry-run-$$-$((++started))
	units+=("$unit")
	systemd-run --quiet --scope --unit="$unit" --uid="$SERVER_USER" --gid="$SERVER_USER" -p MemoryMax="$MEMORY_MAX" \
		-p MemorySwapMax=0 -p CPUWeight=20 -p IOWeight=20 \
		env -i HOME="$SERVER_HOME" USER="$SERVER_USER" LANG=C.UTF-8 PATH="$SERVER_HOME/npm/bin:/usr/local/bin:/usr/bin:/bin" CI=1 GITHUB_TOKEN="$GITHUB_TOKEN" \
		AGENT_BROWSER_ARGS="$agent_browser_args" FIREFOX="$SERVER_HOME/firefox/firefox" FIREFOX_PREFS="$firefox_prefs" \
		timeout "$CHECK_TIME_LIMIT" nice -n 15 ionice -c 3 "$@"
}
tour_chrome() { as_browser_user python3 web/playground/test_in_browser.py --examples --site _site "$@"; }
# as the workflow: a failed first attempt runs once more, loudly (card tour-firefox-stall)
tour_firefox() {
	as_browser_user python3 web/playground/test_in_browser.py --examples --firefox --site _site "$@" && return
	echo "warning: the Firefox tour failed once, running it again (card tour-firefox-stall)"
	as_browser_user python3 web/playground/test_in_browser.py --examples --firefox --site _site "$@"
}
guide() { as_browser_user python3 web/playground/test_in_browser.py --examples --site _site --guide; }
# the page served from the tree, the site's compiler
samples() {
	ln -sfn "$run_dir/_site/web/playground/warp-lib" web/playground/warp-lib
	as_browser_user python3 web/playground/test_in_browser.py --samples "$@"
}
declare -A command=([chrome]=tour_chrome [firefox]=tour_firefox [guide]=guide [samples]=samples)
verdicts=() failed=0
for check in $checks; do
	echo "==== $check on $(hostname) ===="
	if "${command[$check]}" "$@"; then verdicts+=("$check: ok"); else verdicts+=("$check: FAILED"); failed=1; fi
done
echo "==== browser dry run of $(basename "$run_dir") ===="
printf '  %s\n' "${verdicts[@]}"
exit $failed
SERVER_SIDE
} | ssh "$SERVER" "bash -s -- $remote_arguments" && status=0 || status=$?
# what the samples check wrote, brought home: the results only when all samples ran, the report always
if [ "${checks[*]}" = samples ]; then
	copy --ignore-missing-args "$SERVER:$run_dir/test/samples-web.txt" test/ 2>/dev/null || true
	copy "$SERVER:$run_dir/data/samples-web/" data/samples-web/
fi
exit $status
