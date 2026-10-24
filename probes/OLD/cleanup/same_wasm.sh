#!/bin/bash
# A behaviour-preserving emitter change emits the same bytes: compile every given program with two warp binaries
# (`warp build --wasm`) and compare the modules.
# Usage: probes/cleanup/same_wasm.sh <warp before> <warp after> <program.warp>…
# Prints one line per program and exits 1 when any module differs:
#   same       both emit identical bytes
#   unstable   the emitter is not deterministic for the program (card emitter-deterministic), but some run of each
#              binary emits the same bytes
#   DIFFERENT  no run of the after binary emits bytes a run of the before binary emitted
#   no module  a binary emitted nothing
set -u
RUNS_WHEN_UNSTABLE=6
before=$1 after=$2
shift 2
# the worktree and commit each binary was built from: another session's build can replace a copied-too-late binary
for binary in "$before" "$after"; do echo "binary     $binary: $("$binary" --version 2>&1 | tr '\n' ' ')"; done
work=$(mktemp -d "${PWD}/scratch/same_wasm.XXXX")
differing=0

# the md5 of the module `binary` emits for the program copied to $work/program.warp, empty when it emits none
module_hash() {
	rm -f "$work/program.wasm"
	WARP_NO_WINDOW=1 "$1" build --wasm "$work/program.warp" >/dev/null 2>&1
	[ -f "$work/program.wasm" ] && md5 -q "$work/program.wasm"
}

for program in "$@"; do
	cp "$program" "$work/program.warp"
	first_before=$(module_hash "$before")
	first_after=$(module_hash "$after")
	if [ -z "$first_before" ] || [ -z "$first_after" ]; then
		echo "no module  $program"
		continue
	fi
	if [ "$first_before" = "$first_after" ]; then
		echo "same       $program"
		continue
	fi
	befores=$first_before afters=$first_after
	for _ in $(seq $RUNS_WHEN_UNSTABLE); do
		befores="$befores $(module_hash "$before")"
		afters="$afters $(module_hash "$after")"
	done
	shared=$(comm -12 <(tr ' ' '\n' <<<"$befores" | sort -u) <(tr ' ' '\n' <<<"$afters" | sort -u))
	if [ -n "$shared" ]; then
		echo "unstable   $program"
	else
		echo "DIFFERENT  $program"
		differing=1
	fi
done
rm -r "$work"
exit $differing
