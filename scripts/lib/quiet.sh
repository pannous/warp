# Quiet-by-default output for the scripts agents run (token-lean): the command's full output goes to a log, the
# caller sees one summary line and, on failure, only the error lines. Source it from bash:
#   . "$(dirname "$0")/../scripts/lib/quiet.sh"
#   quiet_run <label> <log> <command> [<args>…]      returns the command's real exit status
# Full output instead (streamed, no log): VERBOSE=1 (or WARP_VERBOSE=1), or when stdout already is a regular file
# (`script > x.log`: the caller keeps its own log).
# Summary: `<label>: ok|FAIL rc=N <secs>s [N passed, M failed, K ignored] log=<path>`; on failure up to
# QUIET_ERROR_LINES (30) matching lines with 2 lines of context. A death by signal (rc > 128) is named
# ("killed: signal KILL"); rc 137 hints at out of memory or a kill by PID.

quiet_is_verbose() {
	[ "${VERBOSE:-${WARP_VERBOSE:-0}}" = 1 ] || [ -f /proc/self/fd/1 ] || [ -f /dev/stdout ]
}

QUIET_ERROR_PATTERN='^test .* FAILED$|panicked at|^error(\[E[0-9]+\])?:|^Error:|Error\(|FAIL|[Ff]ailed|refused|KILLED|^Killed$|SIGKILL|out of memory|command not found|No such file'

# "N passed, M failed, K ignored" summed over cargo's `test result:` lines, empty when there are none
quiet_cargo_counts() {
	awk '/^test result:/ { for (i = 1; i <= NF; i++) { n = $i + 0; w = $(i + 1); sub(/[;,].*/, "", w)
			if (w == "passed") p += n; else if (w == "failed") f += n; else if (w == "ignored") g += n }; seen = 1 }
		END { if (seen) printf "%d passed, %d failed, %d ignored", p, f, g }' "$1" 2>/dev/null
}

quiet_signal_note() { # rc → "killed: signal X" for rc > 128, else nothing
	[ "$1" -gt 128 ] 2>/dev/null || return 0
	local name; name=$(kill -l $(($1 - 128)) 2>/dev/null || echo $(($1 - 128)))
	if [ "$1" -eq 137 ]; then echo "killed: signal $name (likely out of memory, or killed by PID)"
	else echo "killed: signal $name"; fi
}

quiet_summary() { # label log rc seconds
	local label=$1 log=$2 rc=$3 secs=$4 status=ok counts note
	[ "$rc" -ne 0 ] && status=FAIL
	counts=$(quiet_cargo_counts "$log")
	echo "$label: $status rc=$rc ${secs}s${counts:+ [$counts]} log=$log" >&2
	note=$(quiet_signal_note "$rc"); [ -n "$note" ] && echo "$label: $note" >&2
	[ "$rc" -ne 0 ] && [ -s "$log" ] || return 0
	if grep -qE "$QUIET_ERROR_PATTERN" "$log"; then
		grep -nE -B2 -A2 "$QUIET_ERROR_PATTERN" "$log" | head -n "${QUIET_ERROR_LINES:-30}" >&2
	else
		echo "$label: no error lines matched; log tail:" >&2; tail -n 10 "$log" >&2
	fi
}

quiet_run() { # label log command…
	local label=$1 log=$2 rc t0; shift 2
	if quiet_is_verbose; then "$@"; return; fi
	mkdir -p "$(dirname "$log")" && : > "$log" || { echo "$label: cannot write log $log, streaming" >&2; "$@"; return; }
	t0=$(date +%s)
	"$@" > "$log" 2>&1
	rc=$?
	quiet_summary "$label" "$log" "$rc" $(($(date +%s) - t0))
	return "$rc"
}
