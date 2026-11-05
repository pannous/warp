#!/usr/bin/env bash
# test/modules/check_program_mode.sh [--all] [<tree>] [<module>…]: each lib module's pairs cases (`'use <m>; …'`) run as a
# standalone program file twice: through the bundled precompiled module (WARP_LIB=<tree>/lib) and from source
# (WARP_LIB=a copy of lib/ whose <m>.warp has one extra trailing newline, so its hash misses and it compiles from
# source). A module without pairs cases runs `use <m>` alone. Prints ok/DIFF per module with the first diverging case
# (a fresh case.database.sqlite each run) and exits 1 if any diverges (card test-module-program-mode). PROGRAM_MODE_CASES=<n> cases per module (default 3, 0 = all);
# --all runs every case and reports each diverging one, not only the first (card program-mode-full).
all=no; [ "$1" == --all ] && { all=yes; PROGRAM_MODE_CASES=0; shift; }
tree=$(cd "${1:-$(dirname "$0")/../..}" && pwd -P); shift
cases=${PROGRAM_MODE_CASES:-3}
work=$(mktemp -d); trap 'rm -rf "$work"' EXIT
cp -r "$tree/lib" "$work/lib"
run() { rm -f "$work"/case.database.sqlite*; (cd "$tree" && WARP_LIB=$1 WARP_NO_WINDOW=1 timeout 120 "${WARP:-warp}" "$2" 2>&1 | grep -vE 'alternate stack|^warning: .*compiled from source$' | sed "s#$work/##g; s#$tree/##g"); }
modules=("$@"); [ ${#modules[@]} -eq 0 ] && modules=($(cd "$tree/lib" && ls *.warp | sed 's/\.warp$//'))
diverged=()
for m in "${modules[@]}"; do
	mapfile -t codes < <(grep -rhE "^'use $m;" "$tree/test" --include='*.pairs.warp' | cut -f1 | sed "s/^'//; s/'\$//")
	[ "$cases" -gt 0 ] && codes=("${codes[@]:0:$cases}")
	[ ${#codes[@]} -eq 0 ] && codes=("use $m; \"loaded\"")
	echo >> "$work/lib/$m.warp"
	bad=""
	for code in "${codes[@]}"; do
		printf '%s\n' "$code" > "$work/case.warp"
		bundled=$(run "$tree/lib" "$work/case.warp"); source=$(run "$work/lib" "$work/case.warp")
		if [ "$bundled" != "$source" ]; then
			bad+="${bad:+$'\n'     }$code"$'\n'"  bundled: ${bundled:0:200}"$'\n'"  source:  ${source:0:200}"
			[ $all == yes ] || break
		fi
	done
	cp "$tree/lib/$m.warp" "$work/lib/$m.warp"
	if [ -z "$bad" ]; then echo "ok   $m (${#codes[@]})"; else echo "DIFF $m: $bad"; diverged+=("$m"); fi
done
[ ${#diverged[@]} -eq 0 ] && exit 0
echo "diverging: ${diverged[*]}"; exit 1
