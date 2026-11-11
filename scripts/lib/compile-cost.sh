# The compile cost of three programs, in instructions, as gate.sh's tier 1 checks it (card compile-slowdown-gate:
# runtime-bridge-slots took `use units; 1` from 17.6 to 29.1 G unnoticed). A compiler's costs are measured once and kept
# in its store record beside its full-suite results (<store>/<hash>/warp.record/compile-cost-<lib digest>.txt), so main
# is never measured again: after-push.sh records each installed compiler's, a gate the compiler it finds none for.
# A tree more than COMPILE_COST_SLOWER_PERCENT slower than main's installed compiler on any program is red, unless its
# HEAD's message holds a `speed-ok: <card>` line (a slowdown the user accepted), which the gate prints. A red is
# measured once more before it counts (compile_costs_again): a busy Mac slows a measurement, never speeds it up.
# Sourced after platform.sh. Instructions are counted by /usr/bin/time -l (macOS) or perf stat (Linux); without
# either nothing is measured and the gate says so.
COMPILE_COST_SLOWER_PERCENT=20
COMPILE_COST_SECONDS=120
# a name and the program, as `warp <program>` runs it: hello, a library's whole compile, one that links sqlite
COMPILE_COST_PROGRAMS=(
	hello 'print "Hello"'
	units 'use units; 1'
	storage 'use storage; class U{n: int}; stored_add("u", U(5)); stored_table("u", U)#1.n'
)

# instructions_of <command…>: the instructions the command retired, its output dropped
instructions_of() {
	local counted status
	counted=$(mktemp "${TMPDIR:-/tmp}/compile-cost.XXXXXX")
	if [ $warp_platform = mac ]; then
		timeout "$COMPILE_COST_SECONDS" /usr/bin/time -l -o "$counted" "$@" >/dev/null 2>&1
		status=$?
		awk '/instructions retired/ { print $1 }' "$counted"
	else
		timeout "$COMPILE_COST_SECONDS" perf stat -x, -e instructions:u -o "$counted" "$@" >/dev/null 2>&1
		status=$?
		awk -F, '/instructions/ { print $1 }' "$counted"
	fi
	rm -f "$counted"
	return $status
}
compile_cost_counter() { [ $warp_platform = mac ] || command -v perf >/dev/null; }

# compile_costs_measured <lib> <binary>: `name<TAB>instructions` of each program, the lowest of two runs after one
# unmeasured (a fresh lib/ compiles its modules' machine code first)
compile_costs_measured() {
	local lib=$1 binary=$2 i name program first second
	for ((i = 0; i < ${#COMPILE_COST_PROGRAMS[@]}; i += 2)); do
		name=${COMPILE_COST_PROGRAMS[i]} program=${COMPILE_COST_PROGRAMS[i + 1]}
		export WARP_NO_WINDOW=1 WARP_HINTS=0
		WARP_LIB=$lib instructions_of "$binary" "$program" >/dev/null &&
			first=$(WARP_LIB=$lib instructions_of "$binary" "$program") &&
			second=$(WARP_LIB=$lib instructions_of "$binary" "$program") ||
			{ echo "compile-cost: $name failed to run with $lib" >&2; return 1; }
		printf '%s\t%s\n' "$name" $((first < second ? first : second))
	done
}

# compile_costs <lib> <binary>: the costs of that lib's compiler and library sources, measured the first time and kept
# in its store record; one measurer at a time (a lock folder holding its pid, a dead holder's taken over)
compile_costs() {
	local lib=$1 binary=$2 kept lock holder status
	kept=$(compile_costs_file "$lib") || return 1
	lock=${kept%.txt}.lock
	until [ -s "$kept" ] || mkdir "$lock" 2>/dev/null; do
		holder=$(cat "$lock/pid" 2>/dev/null)
		if [ -n "$holder" ] && ! kill -0 "$holder" 2>/dev/null; then rm -rf "$lock"; else sleep 2; fi
	done
	if [ ! -s "$kept" ]; then
		echo $$ >"$lock/pid"
		compile_costs_measured "$lib" "$binary" >"$kept.new" && mv "$kept.new" "$kept"
		status=$?
		rm -rf "$lock" "$kept.new"
		((status == 0)) || return 1
	fi
	cat "$kept"
}

# compile_costs_file <lib>: where that lib's compiler and library sources keep their costs, its folder made
compile_costs_file() {
	local lib=$1 hash digest record
	hash=$(cut -d' ' -f1 "$lib/warp.wasm.inputs" 2>/dev/null)
	[ -n "$hash" ] || { echo "compile-cost: no compiler hash in $lib/warp.wasm.inputs" >&2; return 1; }
	digest=$(cat "$lib"/*.warp | { sha256sum 2>/dev/null || shasum -a 256; } | cut -c1-12)
	record=$warp_store/$hash/warp.record
	mkdir -p "$record" && echo "$record/compile-cost-$digest.txt"
}
# compile_costs_again <lib> <binary>: the costs measured once more, each program's lower count kept in the record
# and printed (a red measured on a busy Mac)
compile_costs_again() {
	local lib=$1 binary=$2 kept again
	kept=$(compile_costs_file "$lib") && again=$(compile_costs_measured "$lib" "$binary") || return 1
	awk -F'\t' -v OFS='\t' 'NR == FNR { first[$1] = $2; next } { print $1, ($1 in first && first[$1] < $2 ? first[$1] : $2) }' \
		"$kept" <(echo "$again") >"$kept.new" && mv "$kept.new" "$kept" && cat "$kept"
}
# compile_costs_slower <tree costs> <main costs>: a line per program over the allowance: name, main's count, the tree's
compile_costs_slower() {
	awk -F'\t' -v allowed="$COMPILE_COST_SLOWER_PERCENT" '
		NR == FNR { main[$1] = $2; next }
		($1 in main) && $2 * 100 > main[$1] * (100 + allowed) {
			printf "%s %.2f G -> %.2f G (+%d%%)\n", $1, main[$1] / 1e9, $2 / 1e9, ($2 - main[$1]) * 100 / main[$1]
		}' <(echo "$2") <(echo "$1")
}
# compile_costs_shown <costs>: `hello 1.02 G, units 17.6 G, …`
compile_costs_shown() { echo "$1" | awk -F'\t' '{ printf "%s%s %.2f G", (NR > 1 ? ", " : ""), $1, $2 / 1e9 }'; }
