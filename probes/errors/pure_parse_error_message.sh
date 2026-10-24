#!/bin/bash
# A compiler.wasm run away from the lib/ it was built from (an installed pair, a copied build, a removed worktree)
# says a parse error with its message from the lib/ it is given (card pure-parse-error-message). Before the fix it said
# "no message for the diagnostic (parse_error …) in lib/compiler/errors.data, …": diagnostics.warp read errors.data
# from module_directory, which the build bakes in as the build's own lib/compiler.
# Expected last line: Error("`]` is missing: the group opened at 1:5 runs to the end of the input at …")
# usage: probes/errors/pure_parse_error_message.sh [<warp binary>, default target/release/warp]   (a heavy run: slot.sh)
cd "$(dirname "$0")/../.." || exit 1
warp=$(realpath "${1:-target/release/warp}")
scratch=$(mktemp -d)
cp -a lib "$scratch/lib"
WARP_NO_WINDOW=1 WARP_LIB="$scratch/lib" "$warp" rebuild pure > /dev/null 2>&1 || { echo "rebuild failed"; exit 1; }
mv "$scratch/lib" "$scratch/moved-lib"
said=$(WARP_NO_WINDOW=1 WARP_HINTS=0 WARP_MODULE_CACHE="$scratch/cache" "$warp" "$scratch/data/compiler.wasm" -- \
	probes/errors/pure_parse_error_message.warp "$scratch/out.wasm" probes/errors "$scratch/moved-lib" 2>&1 | tail -1)
rm -rf "$scratch"
echo "$said"
[[ $said == *'`]` is missing: the group opened at 1:5'* ]]
