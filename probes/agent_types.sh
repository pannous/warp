#!/bin/zsh
# card int-let: a typed agent call answers a value of its type, with real requests (ANTHROPIC_API_KEY, or _VISION)
#   probes/agent_types.sh [warp binary]
WARP=${1:-scratch/warp}
export ANTHROPIC_API_KEY=${ANTHROPIC_API_KEY:-$ANTHROPIC_API_KEY_VISION}
export WARP_NO_WINDOW=1
[[ -n $ANTHROPIC_API_KEY ]] || { echo "FAIL no ANTHROPIC_API_KEY"; exit 1 }

failures=0
expect() { # program, expected result
	local got=$($WARP "$1" 2>&1 | tail -1)
	got=${got#» }
	if [[ $got == "$2" ]]; then echo "ok   $1 → $got"; else echo "FAIL $1 → $got (want $2)"; failures=$((failures + 1)); fi
}
expect 'x:int = agent "33/3"; x + 1' 12
expect 'float y = agent "half of 5"; y * 2' 5
expect 'agent "is 7 a prime number" as bool' yes
exit $failures
