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
# standard_module_files <tree>: the standard module entry files, relative to the tree. The same list modules_in
# walks in uses.warp: a .warp file of lib/ or of lib/host/ (decision host-folder)
standard_module_files() {
	local tree=$1 file
	for file in "$tree"/lib/*.warp "$tree"/lib/host/*.warp; do
		[ -f "$file" ] && printf '%s\n' "${file#"$tree"/}"
	done
}
# module_file_of <tree> <path>: the entry a lib file belongs to, if any. A file under that module's folder
# (lib/host/native_host/webview.warp, webview_bridge.js, …) is that module (card smoke-sh-misses)
module_file_of() {
	local tree=$1 file=$2 entry stem
	for entry in $(standard_module_files "$tree"); do
		stem=${entry%.warp}
		if [ "$file" = "$entry" ] || [[ $file == "$stem"/* ]]; then
			printf '%s\n' "$entry"
			return 0
		fi
	done
	return 1
}
# modules_of_files <tree>: unique module entries of the paths on stdin
modules_of_files() {
	local tree=$1 file entry
	while IFS= read -r file; do
		[ -n "$file" ] || continue
		entry=$(module_file_of "$tree" "$file") && printf '%s\n' "$entry"
	done | sort -u
}

# modules_built <warp> <lib> <compiler> <folder> <runtime> [<count file>]: `warp rebuild`'s modules step in parallel (user 11-11, card
# lazy-module-prebuild). The compiler's plan (modules-plan) writes the library index and names each module with the
# modules it waits for; each module is then built by a process of its own (`modules <folder> <runtime> <name>`) once
# those are written. Run in a build slot (scripts/build-slot.sh), it adds a worker for each other slot free right now,
# never waiting for one. A module whose key (the native warp running the build, the compiler, the runtime, the index
# and sugar rules, what the plan says it is built of, its waited-for modules' keys) is its file's, <m>.wasm.built-of,
# is kept as it is. A refused lib/ module leaves no file, as the sequential step does; a refused lib/host/ one
# fails the step once every module is done. The instructions its processes retired, summed, go to the count file as
# /usr/bin/time -l says them, for the build's record (a timer counts only the process it runs, not its children)
MODULE_WAIT=0.2  # seconds a worker waits when no module is ready
digest() { { sha256sum 2>/dev/null || shasum -a 256; } | cut -c1-64; }
modules_built() {
	local warp=$1 lib=$2 compiler=$3 folder=$4 runtime=$5 count_file=${6-} work=$4/.modules-step name texts waits slot workers kept counted_file
	set -- "$1" "$2" "$3" "$4" "$5"
	# (a killed step's claims go: they'd leave its modules unclaimable)
	rm -rf "$work" && mkdir -p "$work" || return 1
	"${instruction_counter[@]}" "$work/plan.count" "$warp" --no-window --lib "$lib" "$compiler" modules-plan "$folder" >"$work/plan" || return 1
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
	[ -z "$count_file" ] || for counted_file in "$work"/*.count; do instructions_counted "$counted_file"; done | awk '{ total += $1 } END { printf "%d  instructions retired\n", total }' >"$count_file"
	[ ! -e "$work/refused" ] || { cat "$work/refused" >&2; return 1; }
	rm -rf "$work"
}
# a worker of modules_built: the modules in the plan whose waited-for ones are written or claimed before them in the
# same batch, claimed and built by one process (a process of its own each cost ~0.45 s, +30 s a build), until every
# module is; a batch stops at MODULE_BATCH, so a worker joining later finds modules left to build
MODULE_BATCH=16
modules_worker() {
	local warp=$1 lib=$2 compiler=$3 folder=$4 runtime=$5 work=$4/.modules-step name texts waits waited left batch stalled=
	touch "$work/worker-$$"
	while :; do
		left= batch=
		while IFS=$'\t' read -r name texts waits; do
			[ "$name" = library ] || [ -e "$work/$name.done" ] && continue
			left=yes
			for waited in $waits; do [ -e "$work/$waited.done" ] || [[ " $batch " == *" $waited "* ]] || continue 2; done
			mkdir "$work/$name.claimed" 2>/dev/null || continue
			if module_kept "$folder" "$work" "$name"; then touch "$work/$name.done"; else batch+="$name "; fi
			[ "$(wc -w <<<"$batch")" -lt $MODULE_BATCH ] || break
		done <"$work/plan"
		if [ -n "$batch" ]; then modules_made "$@" $batch; stalled=; continue; fi
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
# module_kept <folder> <work> <name>: its file is the one its key built, so it stays as it is
module_kept() {
	local file=$1/$3.wasm
	[ -f "$file" ] && [ "$(cat "$file.built-of" 2>/dev/null)" = "$(module_key "$2/$3.key" "$file")" ] || return 1
	touch "$file" "$file.built-of" "$2/$3.kept"
}
# modules_made <warp> <lib> <compiler> <folder> <runtime> <name>…: the modules built in order by one process
modules_made() {
	local warp=$1 lib=$2 compiler=$3 folder=$4 runtime=$5 work=$4/.modules-step name log
	shift 5
	log=$work/$1.log
	for name; do rm -f "$folder/$name.wasm" "$folder/$name.wasm.built-of"; done
	if "${instruction_counter[@]}" "$log.count" "$warp" --no-window --lib "$lib" "$compiler" modules "$folder" "$runtime" "$@" >"$log" 2>&1; then
		cat "$log"
		# (a lib/ module refusing to prebuild leaves no file: compiled from its source with each program, said again next step)
		for name; do [ ! -f "$folder/$name.wasm" ] || module_key "$work/$name.key" "$folder/$name.wasm" >"$folder/$name.wasm.built-of"; done
	else
		modules_failure $? "$log" "$@" >>"$work/refused"
	fi
	for name; do touch "$work/$name.done"; done
}
# modules_failure <status> <log> <name>…: a batch's failure said whole, never an empty refusal: its modules, its exit
# (a signal's name: a crash or a kill prints nothing itself), its output and what its timer saw (/usr/bin/time puts
# "terminated abnormally" in its own file, not the log)
modules_failure() {
	local status=$1 log=$2 signal=
	shift 2
	[ "$status" -gt 128 ] && signal=", signal $(kill -l $((status - 128)) 2>/dev/null || echo $((status - 128)))"
	echo "modules: the process building $* failed (exit $status$signal)$([ -s "$log" ] || echo ', printing nothing')"
	cat "$log"
	grep -v -E '^ *[0-9]' "$log.count" 2>/dev/null
}
# run, not sourced: the function named (a worker in another build slot, modules_built)
if [ "${BASH_SOURCE[0]}" = "$0" ]; then source "$(dirname "$0")/platform.sh"; "$@"; fi
