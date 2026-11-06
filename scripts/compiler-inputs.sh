#!/usr/bin/env bash
# scripts/compiler-inputs.sh <lib> [-v] [--baked]: the inputs hash of the compiler pair (warp.wasm,
# warp-self.wasm) built from <lib> (notes/compiler_rebuilds.md): 16 hex digits. Two trees with the same inputs give
# the same hash wherever they lie (paths are hashed relative to <lib>). -v lists the files on stderr.
# The inputs: core/**, extensions/** and every .warp module of lib/: the compiler's code, and the runtime, modules and
# library index built beside it from lib/ (it bakes no library since L4, notes/library_zero_bake.md: it reads lib/ from
# disk). Other lib/ files never change the hash.
# The seed building warp.wasm (WARP_SEED, its SHA-256): a refreshed seed builds anew (card scripts-rebuild).
# core/native/ is no input: the native warp's own sources, which no compiler embeds.
# --baked: the hash of the compiler's code alone: core/**, extensions/**, the lib/ modules they use (`use os`) with
# those these use in turn, compiled in, and the lib/ data it reads at its own build (CODE_DATA: the prelude, the
# aliases, module_source's files), and the seed; the rest of lib/ left out: two lib/ trees of one baked hash compile
# compiler.warp alike, so a self test passed for one holds for the other (scripts/rebuild.sh self), and a lib/-only
# change rebakes a stored compiler of that code (rebuild.sh rebaked_stored). --baked-files [<lib>]: those lib/ files,
# one a line, of <lib> or this tree's (scripts/smoke.sh, integrate/land.sh)
set -euo pipefail
CODE_DATA=(prelude.warp aliases.warp entities.tsv modes.data)
USE_LINE='s/^use \([a-z_]*\)$/\1/p'
# code_files <lib>: the lib/ files of the compiler's code, its modules found by their use lines, never listed by hand
code_files() {
	local lib=$1 module seen=" " queue
	queue=($(cd "$lib" && find -H compiler extensions -path compiler/native -prune -o -name '*.warp' -print | env LC_ALL=C xargs sed -n "$USE_LINE" | env LC_ALL=C sort -u))
	while [ ${#queue[@]} -gt 0 ]; do
		module=${queue[0]} queue=("${queue[@]:1}")
		[[ $seen == *" $module "* ]] && continue
		seen+="$module "
		# (a use of an extension's module, `use rule_words`, is extensions/ already)
		[ -f "$lib/$module.warp" ] || continue
		echo "$module.warp"
		queue+=($(sed -n "$USE_LINE" "$lib/$module.warp"))
	done
	printf '%s\n' "${CODE_DATA[@]}"
}
if [ "${1-}" = --baked-files ]; then code_files "${2:-$(cd "$(dirname "$0")/../lib" && pwd)}" | env LC_ALL=C sort -u; exit 0; fi
lib=$(cd "${1:?usage: scripts/compiler-inputs.sh <lib> [-v] [--baked] | --baked-files}" && pwd)
shift
listed= baked=
for option in "$@"; do
	case $option in
		-v) listed=1 ;;
		--baked) baked=1 ;;
		*) echo "compiler-inputs.sh: unknown option $option" >&2; exit 2 ;;
	esac
done
[ -d "$lib/compiler" ] || { echo "compiler-inputs.sh: no compiler/ in $lib" >&2; exit 1; }
# a hash without the seed is no tree's: refused, never a quiet wrong hash (2026-10-30)
[ -f "${WARP_SEED-}" ] || { echo "compiler-inputs.sh: WARP_SEED names no seed (${WARP_SEED-unset}); rebuild.sh sets it, by hand: WARP_SEED=~/dev/bin/warp-seed.wasm" >&2; exit 1; }
# `env LC_ALL=C cmd`, never bash's `LC_ALL=C cmd`: in a subshell, bash restoring its locale after the command calls
# Homebrew libintl's setlocale, CoreFoundation in a forked child, which segfaults now and then (39 bash crash reports
# 11-01..05, all in reset_locale_vars; promote-self RED 07:40 11-05). env changes only the command's environment
modules() {
	if [ -n "$baked" ]; then code_files "$lib" | env LC_ALL=C sort -u; return; fi
	{ (cd "$lib" && find . -path ./compiler -prune -o -path ./data -prune -o -name '*.warp' -type f -print | sed 's|^\./||'); printf '%s\n' "${CODE_DATA[@]}" | grep -v '\.warp$'; } | env LC_ALL=C sort
}
files() {
	(cd "$lib" && find -H compiler extensions -path compiler/native -prune -o -type f ! -name '.*' -print | env LC_ALL=C sort)
	modules
}
[ -n "$listed" ] && files >&2
{
	echo "seed $({ sha256sum "$WARP_SEED" 2>/dev/null || shasum -a 256 "$WARP_SEED"; } | cut -c1-64)"
	files | while IFS= read -r file; do printf '%s\0' "$file"; cat "$lib/$file"; printf '\0'; done
} | { sha256sum 2>/dev/null || shasum -a 256; } | cut -c1-16
