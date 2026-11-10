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
