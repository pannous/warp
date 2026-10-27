#!/usr/bin/env bash
# scripts/compiler-inputs.sh <lib> [<rust fingerprint>] [-v]: the inputs hash of the compiler pair (warp.wasm,
# warp-self.wasm) built from <lib> (notes/compiler_rebuilds.md): 16 hex digits. Two trees with the same inputs give
# the same hash wherever they lie (paths are hashed relative to <lib>). -v lists the files on stderr.
# The inputs: core/**, the Rust build compiling the stages (WARP_SOURCE, the third line of a build file's
# warp.build section: scripts/rebuild.sh finds it), and every .warp module of lib/, which the compiler bundles
# (parser.warp LIBRARY_FILES, decision built-in-lib: a built compiler reads no lib/ from disk; a set WARP_LIB lets a
# module changed since then win over its bundled copy without a rebuild). Other lib/ files never change the hash.
# The seed building warp.wasm (WARP_SEED, its SHA-256): a refreshed seed builds anew (card scripts-rebuild).
set -euo pipefail
lib=$(cd "${1:?usage: scripts/compiler-inputs.sh <lib> [<rust fingerprint>] [-v]}" && pwd)
rust=${2-}
[ -d "$lib/compiler" ] || { echo "compiler-inputs.sh: no compiler/ in $lib" >&2; exit 1; }
files() {
	(cd "$lib" && find -H compiler -type f ! -name '.*' | LC_ALL=C sort
		find . -path ./compiler -prune -o -path ./data -prune -o -name '*.warp' -type f -print | sed 's|^\./||' | LC_ALL=C sort)
}
[ "${3-}" = -v ] && files >&2
{
	echo "rust $rust"
	echo "seed $([ -f "${WARP_SEED-}" ] && { sha256sum "$WARP_SEED" 2>/dev/null || shasum -a 256 "$WARP_SEED"; } | cut -c1-64)"
	files | while IFS= read -r file; do printf '%s\0' "$file"; cat "$lib/$file"; printf '\0'; done
} | { sha256sum 2>/dev/null || shasum -a 256; } | cut -c1-16
