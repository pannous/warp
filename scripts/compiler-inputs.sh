#!/usr/bin/env bash
# scripts/compiler-inputs.sh <lib> [-v] [--baked]: the inputs hash of the compiler pair (warp.wasm,
# warp-self.wasm) built from <lib> (notes/compiler_rebuilds.md): 16 hex digits. Two trees with the same inputs give
# the same hash wherever they lie (paths are hashed relative to <lib>). -v lists the files on stderr.
# The inputs: core/** and every .warp module of lib/, which the compiler bundles
# (parser.warp LIBRARY_FILES, decision built-in-lib: a built compiler reads no lib/ from disk; a set WARP_LIB lets a
# module changed since then win over its bundled copy without a rebuild). Other lib/ files never change the hash.
# The seed building warp.wasm (WARP_SEED, its SHA-256): a refreshed seed builds anew (card scripts-rebuild).
# core/native/ is no input: the native warp's own sources, which no compiler embeds.
# --baked: the hash of what the compiler bakes in at its own build (module_source: core/** and BAKED below) and the
# seed, the rest of lib/ left out: two lib/ trees of one baked hash compile compiler.warp alike, so a self test passed
# for one holds for the other (scripts/rebuild.sh self). --baked-files: BAKED, one a line (scripts/smoke.sh).
set -euo pipefail
BAKED=(prelude.warp aliases.warp math.warp rule_words.warp wgsl.warp unit_names.warp)
[ "${1-}" = --baked-files ] && { printf '%s\n' "${BAKED[@]}"; exit 0; }
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
modules() {
	if [ -n "$baked" ]; then (cd "$lib" && ls "${BAKED[@]}" 2>/dev/null); return; fi
	(cd "$lib" && find . -path ./compiler -prune -o -path ./data -prune -o -name '*.warp' -type f -print | sed 's|^\./||' | LC_ALL=C sort)
}
files() {
	(cd "$lib" && find -H compiler -path compiler/native -prune -o -type f ! -name '.*' -print | LC_ALL=C sort)
	modules
}
[ -n "$listed" ] && files >&2
{
	echo "seed $({ sha256sum "$WARP_SEED" 2>/dev/null || shasum -a 256 "$WARP_SEED"; } | cut -c1-64)"
	files | while IFS= read -r file; do printf '%s\0' "$file"; cat "$lib/$file"; printf '\0'; done
} | { sha256sum 2>/dev/null || shasum -a 256; } | cut -c1-16
