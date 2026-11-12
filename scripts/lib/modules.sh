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
