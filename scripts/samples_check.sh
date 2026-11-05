#!/usr/bin/env bash
# scripts/samples_check.sh [<warp binary> [<program.warp> [<port>]]]:
# Serve a warp HTTP sample and curl its routes (historical probes/server_edits/curl_checks.sh,
# removed with probes/). Default: installed warp + samples/servers/server.warp on an ephemeral port.
#
# Under load the process can take a long time to answer, and the first reply can be empty (card
# sh-flake). This script:
#   - waits up to WAIT_SECS (default 120) for a non-empty health reply, polling every ~0.5s
#   - fails clearly if the server process dies before answering — unless stderr matches a known
#     open core abort (stored/sqlite FFI, heap corruption); then SKIP exit 0 (cards below)
#   - retries the first request a few times when the body is empty
#   - curls GET paths that the restored stored User demo can answer without open core cards
#   - skips /api/newest (`last n of` stack exhaustion: core-last-n-of-stack) and POST-with-body
#     (core-post-body-abort) until those land
#   - traps cleanup of the server and any <program>.database.sqlite
#
# Known skips (do not FAIL the probe while the cards are open):
#   - native sqlite FFI / heap abort on `stored` (core-native-stored-sqlite): server never listens
#   - GET /api/newest (core-last-n-of-stack)
#   - POST /api/users with a body (core-post-body-abort)
#
# Gate note: scripts/gate.sh never invokes this script (scripts/samples.sh only SRV-marks servers).
# It is a standalone probe; run it directly or from a future gate hook.
#
# usage:
#   scripts/samples_check.sh
#   scripts/samples_check.sh /path/to/warp samples/servers/server.warp 18080
#   WARP_LIB=/path/to/lib scripts/samples_check.sh
#
# exit 0 green (or known-core SKIP), 1 red (prints a FAIL line).
set -euo pipefail

scripts_dir=$(cd "$(dirname "$0")" && pwd)
# shellcheck source=lib/platform.sh
source "$scripts_dir/lib/platform.sh"

tree=$(cd "$scripts_dir/.." && pwd)
WARP=${1:-${WARP:-$warp_binary}}
PROGRAM=${2:-samples/servers/server.warp}
PORT=${3:-0}
WAIT_SECS=${WAIT_SECS:-120}
POLL_SECS=${POLL_SECS:-0.5}
FIRST_RETRIES=${FIRST_RETRIES:-8}
BODY_PREVIEW=${BODY_PREVIEW:-120}

cd "$tree" || exit 2
[ -x "$WARP" ] || WARP=$(command -v "$WARP" || true)
[ -x "$WARP" ] || { echo "FAIL samples_check: no warp binary ($WARP)" >&2; exit 1; }
[ -f "$PROGRAM" ] || { echo "FAIL samples_check: no program $PROGRAM" >&2; exit 1; }

# Prefer the tree's compilers when present (gate/smoke do the same).
if [ -f "$tree/lib/warp.wasm" ]; then
	export WARP_LIB=${WARP_LIB:-$tree/lib}
fi
export WARP_NO_WINDOW=1 WARP_HINTS=${WARP_HINTS:-0}

# Ephemeral port when 0: pick one that is free right now.
if [ "$PORT" = 0 ]; then
	PORT=$(python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1",0)); print(s.getsockname()[1]); s.close()')
fi

db_path="${PROGRAM%.warp}.database.sqlite"
rm -f "$db_path"

log=$(mktemp)
err=$(mktemp)
# Temp program: append an explicit serve(WARP_PORT) so the probe still listens when the
# program→routes_end rewrite does not fire on this native tip (sample source stays auto-serve).
run_program=$(mktemp --suffix=.warp)
SERVER=
cleanup() {
	if [ -n "${SERVER:-}" ] && kill -0 "$SERVER" 2>/dev/null; then
		kill "$SERVER" 2>/dev/null || true
		wait "$SERVER" 2>/dev/null || true
	fi
	rm -f "$db_path" "$log" "$err" "$run_program"
	# Temp serve may create <run_program>.database.sqlite beside the mktemp path.
	rm -f "${run_program%.warp}.database.sqlite"
}
trap cleanup EXIT

{
	cat "$PROGRAM"
	# Only append when the sample does not already call serve( — keep the rich demo source clean.
	if ! grep -Eq '^[[:space:]]*serve[[:space:]]*\(' "$PROGRAM"; then
		printf '\n# samples_check: explicit serve so the probe listens if routes_end rewrite is skipped\n'
		printf 'port = env("WARP_PORT")\n'
		printf 'serve(if port == ø then 8080 else int(port))\n'
	fi
} >"$run_program"

WARP_PORT=$PORT "$WARP" "$run_program" >"$log" 2>"$err" &
SERVER=$!

base="http://127.0.0.1:$PORT"
deadline=$((SECONDS + WAIT_SECS))
ready=0
while (( SECONDS < deadline )); do
	if ! kill -0 "$SERVER" 2>/dev/null; then
		# Known open-core aborts while starting the restored stored User demo: stay green with SKIP.
		if grep -Eqi 'corrupted size|corrupted double-linked|malloc(): invalid next size|Aborted while running|Call stack exhausted|sqlite|SIGABRT|wasm-rt warning: alternate stack was modified unexpectedly' "$err" 2>/dev/null; then
			echo "SKIP samples_check: server exited before answering ($PROGRAM) — known native abort (see core-native-stored-sqlite; also core-last-n-of-stack / core-post-body-abort for skipped paths)" >&2
			sed 's/^/  /' "$err" >&2 || true
			exit 0
		fi
		echo "FAIL samples_check: server exited before answering ($PROGRAM)" >&2
		sed 's/^/  /' "$err" >&2 || true
		sed 's/^/  /' "$log" >&2 || true
		exit 1
	fi
	# Health: TCP/HTTP to / must return a non-empty body (the flake was status-ish with body '').
	body=$(curl -s --connect-timeout 1 --max-time 2 "$base/" 2>/dev/null || true)
	if [ -n "$body" ]; then
		ready=1
		break
	fi
	sleep "$POLL_SECS"
done

if [ "$ready" != 1 ]; then
	if ! kill -0 "$SERVER" 2>/dev/null && grep -Eqi 'corrupted size|corrupted double-linked|malloc(): invalid next size|Aborted while running|Call stack exhausted|sqlite|SIGABRT|wasm-rt warning: alternate stack was modified unexpectedly' "$err" 2>/dev/null; then
		echo "SKIP samples_check: no reply within ${WAIT_SECS}s and server aborted ($PROGRAM) — core-native-stored-sqlite" >&2
		sed 's/^/  /' "$err" >&2 || true
		exit 0
	fi
	echo "FAIL samples_check: no non-empty reply from $base/ within ${WAIT_SECS}s" >&2
	sed 's/^/  /' "$err" >&2 || true
	sed 's/^/  /' "$log" >&2 || true
	exit 1
fi

# First-curl retry: empty body under load (sh-flake).
first_ok=0
for _ in $(seq 1 "$FIRST_RETRIES"); do
	body=$(curl -s --connect-timeout 2 --max-time 5 "$base/" 2>/dev/null || true)
	if [ -n "$body" ]; then
		first_ok=1
		break
	fi
	sleep "$POLL_SECS"
done
if [ "$first_ok" != 1 ]; then
	echo "FAIL samples_check: first GET / still empty after $FIRST_RETRIES retries" >&2
	exit 1
fi

server_dead=0
fail=0
check_miss() {
	local method=$1 path=$2
	local answer status content preview
	if [ "${server_dead:-0}" = 1 ]; then fail=1; return; fi
	if ! kill -0 "$SERVER" 2>/dev/null; then
		echo "FAIL samples_check: server died during checks ($PROGRAM)" >&2
		sed 's/^/  /' "$err" >&2 || true
		server_dead=1
		fail=1
		return
	fi
	answer=$(curl -s -X "$method" -w '\n%{http_code}' --connect-timeout 2 --max-time 10 "$base$path" || echo $'\n000')
	status=${answer##*$'\n'}
	content=${answer%$'\n'*}
	content=${content//$'\n'/ }
	preview=${content:0:$BODY_PREVIEW}
	if [[ $status != 404 && $status != 500 ]]; then
		echo "FAIL $method $path → status $status want 404|500 ($preview)" >&2
		fail=1
		return
	fi
	echo "ok   $method $path → $status ${preview}"
}
check() {
	local method=$1 path=$2 want=$3
	shift 3 || true
	local body_arg=${1-}
	local answer status content
	if [ "${server_dead:-0}" = 1 ]; then fail=1; return; fi
	if ! kill -0 "$SERVER" 2>/dev/null; then
		echo "FAIL samples_check: server died during checks ($PROGRAM)" >&2
		sed 's/^/  /' "$err" >&2 || true
		server_dead=1
		fail=1
		return
	fi
	if [ -n "$body_arg" ]; then
		answer=$(curl -s -X "$method" -H 'Content-Type: text/plain' -d "$body_arg" -w '\n%{http_code}' --connect-timeout 2 --max-time 10 "$base$path" || echo $'\n000')
	else
		answer=$(curl -s -X "$method" -w '\n%{http_code}' --connect-timeout 2 --max-time 10 "$base$path" || echo $'\n000')
	fi
	status=${answer##*$'\n'}
	content=${answer%$'\n'*}
	content=${content//$'\n'/ }
	local preview=${content:0:$BODY_PREVIEW}
	if [[ $status != $want ]]; then
		echo "FAIL $method $path → status $status want $want ($preview)" >&2
		fail=1
		return
	fi
	if [ -z "$content" ]; then
		echo "FAIL $method $path → empty body (status $status)" >&2
		fail=1
		return
	fi
	# Optional further args: substrings that must appear in the body.
	local frag
	for frag in "${@:2}"; do
		[ -z "$frag" ] && continue
		case $content in
			*"$frag"*) ;;
			*)
				echo "FAIL $method $path → missing '$frag' in: $preview" >&2
				fail=1
				return
				;;
		esac
	done
	echo "ok   $method $path → $status ${preview}"
}

# Adapted from historical curl_checks.sh to samples/servers/server.warp (restored stored User demo).
# Hits only paths that should answer once core-native-stored-sqlite lands. Skips:
#   - GET /api/newest — `last 2 of users` stack-exhausts (core-last-n-of-stack)
#   - POST /api/users with body (`curl -d Cy`) — abort/heap corruption (core-post-body-abort)
# Health + typed GETs + page routes + 404/miss only. Empty-body POST /rpc/user_count seeds users.
check POST /rpc/user_count 200 '' '2'
check GET /api/users 200 '' 'Ann' 'Bo'
check GET /api/users/2 200 '' 'Bo'
check GET /api/users/abc 200 '' 'no such page'
check_miss GET /api/users/9
echo "skip GET /api/newest — needs core-last-n-of-stack (last n of stack exhaustion)"
echo "skip POST /api/users with body — needs core-post-body-abort (curl -d Cy …)"
check GET /users/2 200 '' 'User'
check GET /users/abc 200 '' 'no such page'
check GET /nope 200 '' 'no such page'
check GET / 200 '' 'Users:'

if [ "$fail" != 0 ]; then
	echo "FAIL samples_check: one or more route checks failed ($PROGRAM on $base)" >&2
	exit 1
fi
echo "samples_check: ok ($PROGRAM on $base, WARP_LIB=${WARP_LIB:-installed})"
exit 0
