# scripts/lib/modules.sh, sourced: modules_kept_only <folder> keeps in a modules/ folder only the files named on stdin
# and the run-time machine code of those modules (X.wasm.driver.cwasm, X.wasm.cwasm, X.splice.wasm …); everything else
# goes. A module that left the standard set stayed beside the new ones and was linked: reals.wasm after 827309223b,
# "won't link: built behind another runtime.wasm" on installed main (card stale-modules-kept). Pruned, not
# emptied: a live install keeps the machine code of the modules that stay, and warp.wasm links modules/unicode.wasm
# while it builds the runtime, so a tree's folder is never empty during a build.
module_of() { basename "$1" | sed -E 's/\.splice\.wasm$/.wasm/; s/(\.wasm).+$/\1/'; }
modules_kept_only() {
	local folder=$1 kept file
	[ -d "$folder" ] || return 0
	kept=$(cat)
	for file in "$folder"/*; do
		[ -e "$file" ] || continue
		grep -qxF "$(basename "$file")" <<<"$kept" || grep -qxF "$(module_of "$file")" <<<"$kept" || rm -rf "$file"
	done
}
# companion_files <compiler file name>: the library a compiler links its NFC from, beside it (core/native/driver.warp
# compiler_library): the runtime and unicode module its builder made, of the GC types it has (card asciitext-compile)
companion_files() { echo "$1.library/runtime.wasm $1.library/modules/unicode.wasm"; }
# companion_copied <from compiler> <to compiler>: that library, when the first has one, beside the second
# each file moved into place whole (a run beside this finds the old or the new one)
companion_copied() {
	local file
	for file in $(companion_files ""); do
		[ -f "$1$file" ] || continue
		mkdir -p "$(dirname "$2$file")" && cp "$1$file" "$2$file.new-$$" && mv -f "$2$file.new-$$" "$2$file" || return 1
	done
}

# modules_built <warp> <lib> <compiler> <folder> <runtime>: `warp rebuild`'s modules step in parallel (user 11-11, card
# lazy-module-prebuild). The compiler's plan (modules-plan) writes the library index and names each module with the
# modules it waits for; each module is then built by a process of its own (`modules <folder> <runtime> <name>`) once
# those are written. Run in a build slot (scripts/build-slot.sh), it adds a worker for each other slot free right now,
# never waiting for one. A module whose key (the native warp running the build, the compiler, the runtime, the index
# and sugar rules, what the plan says it is built of, its waited-for modules' keys) is its file's, <m>.wasm.built-of,
# is kept as it is. A refused lib/ module leaves an empty file, as the sequential step does; a refused lib/host/ one
# fails the step once every module is done
MODULE_WAIT=0.2  # seconds a worker waits when no module is ready
digest() { { sha256sum 2>/dev/null || shasum -a 256; } | cut -c1-64; }
modules_built() {
	local warp=$1 lib=$2 compiler=$3 folder=$4 runtime=$5 work=$4/.modules-step name texts waits slot workers kept
	# (a killed step's claims go: they'd leave its modules unclaimable)
	rm -rf "$work" && mkdir -p "$work" || return 1
	"$warp" --no-window --lib "$lib" "$compiler" modules-plan "$folder" >"$work/plan" || return 1
	{ cat "$warp" "$compiler" "$runtime" "$folder/sugar_rules.packed"; grep -v -e '^hash	' -e '^slice	' "$folder/index.tsv"; grep '^library	' "$work/plan"; } | digest >"$work/common"
	while IFS=$'\t' read -r name texts waits; do
		[ "$name" = library ] && continue
		{ cat "$work/common"; echo "$texts"; for name_waited in $waits; do cat "$work/$name_waited.key"; done; } | digest >"$work/$name.key"
	done <"$work/plan"
	if command -v lockf >/dev/null; then
		for slot in $(seq "$(build_slots)"); do
			lockf -s -t 0 "$warp_store/build-slots/$slot" bash "${BASH_SOURCE[0]}" modules_worker "$@" &
		done
	fi
	modules_worker "$@"
	wait
	workers=$(ls "$work" | grep -c '^worker-')
	kept=$(ls "$work" | grep -c '\.kept$')
	echo "modules: $(($(grep -vc '^library	' "$work/plan") - kept)) built by $workers worker$([ "$workers" = 1 ] || echo s)$(command -v lockf >/dev/null || echo ' (no lockf: one)'), $kept kept as they were"
	[ ! -e "$work/refused" ] || { cat "$work/refused" >&2; return 1; }
	rm -rf "$work"
}
# a worker of modules_built: the first module in the plan whose waited-for ones are written, claimed and built, until
# every module is
modules_worker() {
	local warp=$1 lib=$2 compiler=$3 folder=$4 runtime=$5 work=$4/.modules-step name texts waits waited left stalled=
	touch "$work/worker-$$"
	while :; do
		left=
		while IFS=$'\t' read -r name texts waits; do
			[ "$name" = library ] || [ -e "$work/$name.done" ] && continue
			left=yes
			for waited in $waits; do [ -e "$work/$waited.done" ] || continue 2; done
			mkdir "$work/$name.claimed" 2>/dev/null || continue
			module_made "$@" "$name"
			stalled=
			continue 2
		done <"$work/plan"
		[ -n "$left" ] || return 0
		# (twice: a module done between the scan and this check makes another ready)
		if modules_in_flight "$work"; then stalled=; elif [ -z "$stalled" ]; then stalled=yes; continue; else
			echo "modules: none ready and none building: the plan waits for a module it never builds ($work/plan)" | tee -a "$work/refused" >&2
			return 1
		fi
		sleep $MODULE_WAIT
	done
}
# whether a module is claimed and not yet done (being built by some worker)
modules_in_flight() {
	local claim
	for claim in "$1"/*.claimed; do [ -e "${claim%.claimed}.done" ] || [ ! -e "$claim" ] || return 0; done
	return 1
}
# (a module's key file holds its key and its own file's sha: one written by anything else, a store hit, a sequential
# `warp rebuild`, is never taken for the key's)
module_key() { cat "$1"; digest <"$2"; }
module_made() {
	local warp=$1 lib=$2 compiler=$3 folder=$4 runtime=$5 name=$6 work=$4/.modules-step file=$4/$6.wasm
	if [ -f "$file" ] && [ "$(cat "$file.built-of" 2>/dev/null)" = "$(module_key "$work/$name.key" "$file")" ]; then
		touch "$file" "$file.built-of" "$work/$name.kept"
	elif rm -f "$file.built-of" && "$warp" --no-window --lib "$lib" "$compiler" modules "$folder" "$runtime" "$name" >"$work/$name.log" 2>&1; then
		cat "$work/$name.log"
		module_key "$work/$name.key" "$file" >"$file.built-of"
	else
		cat "$work/$name.log" >>"$work/refused"
	fi
	touch "$work/$name.done"
}
# run, not sourced: the function named (a worker in another build slot, modules_built)
if [ "${BASH_SOURCE[0]}" = "$0" ]; then source "$(dirname "$0")/platform.sh"; "$@"; fi
