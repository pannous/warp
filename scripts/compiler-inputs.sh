#!/usr/bin/env bash
# scripts/compiler-inputs.sh <lib> [<rust fingerprint>] [-v]: the inputs hash of the compiler pair (compiler.wasm,
# compiler-self.wasm) built from <lib> (notes/compiler_rebuilds.md): 16 hex digits. Two trees with the same inputs give
# the same hash wherever they lie (paths are hashed relative to <lib>). -v lists the files on stderr.
# The inputs: lib/compiler/**, the Rust build compiling the stages (WARP_SOURCE, the third line of a build file's
# warp.build section: scripts/rebuild.sh finds it), and the std modules the compiler still embeds as text at build time
# through module_source(…) (prelude, math, list, regex). Once those are read from the run's lib/ (card
# compiler-library-source) they leave this list and the hash is lib/compiler's alone. Any other change of lib/ (a std
# module, a new host verb in os.warp, sugar) never changes the hash: the compiler reads those at run time.
set -euo pipefail
EMBEDDED_MODULES=(prelude math list regex)
lib=$(cd "${1:?usage: scripts/compiler-inputs.sh <lib> [<rust fingerprint>] [-v]}" && pwd)
rust=${2-}
[ -d "$lib/compiler" ] || { echo "compiler-inputs.sh: no compiler/ in $lib" >&2; exit 1; }
files() {
	(cd "$lib" && find compiler -type f ! -name '.*' | LC_ALL=C sort
		for module in "${EMBEDDED_MODULES[@]}"; do [ -f "$module.warp" ] && echo "$module.warp"; done)
}
[ "${3-}" = -v ] && files >&2
{
	echo "rust $rust"
	files | while IFS= read -r file; do printf '%s\0' "$file"; cat "$lib/$file"; printf '\0'; done
} | { sha256sum 2>/dev/null || shasum -a 256; } | cut -c1-16
