#!/bin/bash
# Evaluate each argument as a wasp program with the freshly built CLI: probes/night/try.sh 'code' 'code' …
repo="$(cd "$(dirname "$0")/../.." && pwd)"
WARP="$("$repo/scripts/own-warp.sh")"
for code in "$@"; do
	printf '%s\n  → ' "$code"
	"$WARP" eval "$code" 2>&1 | tail -4 | tr '\n' ' ' | cut -c1-300
	echo
done
