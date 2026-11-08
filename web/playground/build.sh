#!/bin/bash
# Builds web/playground: the self-hosted compiler for the browser (warp.wasm with the lib/ it compiles with, copied from
# the installed warp beside ~/dev/bin/warp into warp-lib/, driver.js runs it), samples.js (samples/<chapter>/*.warp for
# the example menu), keywords.js (the editor's hard and soft keywords) and version.js (the commit built, shown in the ⋯
# menu). Serve the repository root and open the page:
#   web/playground/build.sh && python3 -m http.server 8000   →   http://localhost:8000/web/playground/
# Usage: build.sh [compiler|served <site>|site <site>]   (all when omitted)
#   compiler  → warp-lib/: the installed lib/ (its warp.wasm, runtime.wasm, modules/ and sources), no machine code
#   site <site> → collects the built page into a fresh <site> folder, as deployed (pages.yml, scripts/browser-dry-run.sh)
#   served <site> → <site>/served-files.js for a collected site; every build writes the repository's
set -euo pipefail

page="$(cd "$(dirname "$0")" && pwd)"
repository="$(cd "$page/../.." && pwd)"
source "$repository/scripts/lib/packages.sh"
# the compilers: WARP_LIB, else this checkout's own (scripts/rebuild.sh pure <tree> writes them into lib/), else the
# installed ones beside the warp on $PATH (~/dev/bin/lib links to the current build)
OWN_LIBRARY="$repository/lib"
[ -f "$OWN_LIBRARY/warp.wasm.inputs" ] || OWN_LIBRARY="$(dirname "$(command -v warp)")/lib"
INSTALLED_LIBRARY="${WARP_LIB:-$OWN_LIBRARY}"
# the sources a compiler bakes in: one built from a commit where they differ ships its lag to the page
COMPILER_SOURCES=(core 'lib/*.warp')
# what the browser cannot use of it: wasmtime's machine code, native libraries, kept values, stale compilers
NOT_FOR_THE_BROWSER=(--exclude='*.cwasm' --exclude='*.cwasm.key' --exclude='*.dylib' --exclude='*.so' --exclude='*.kept'
	--exclude='*.stale' --exclude='*.inputs' --exclude='warp-self.wasm*' --exclude='data/')
# what `build.sh site <folder>` collects from web/playground into the deployed site (pages.yml)
SITE_FILES=(index.html playground.css playground.js deploy.js assistant.js completion.js shortcuts.js entities.tsv guide.js
	guide.md guide-expert.md primer.md markup.js markup-transitions.js markup-forms.js canvas.js worker.js task-worker.js
	task-workers.js js-members.js coi-serviceworker.js host.js "host-*.js" values.js wasi.js driver.js components.js foreign_python.py python_capi.py
	imports.js std.js examples.js codemirror-warp-mode.js codemirror lib samples.js keywords.js
	keywords.txt assistant.json version.js)
DOMAIN=warp.pannous.com
# the digits of a file's content hash its deployed URL carries (stamp_assets)
STAMP_DIGITS=10
# the library sources a first compile reads, sent to the page in one file, warp-lib/sources.json (wasi.js preloadSources)
PRELOADED_SOURCES=('*.warp' 'extensions/sugar/*.warp' 'modules/index.tsv' 'modules/sugar_rules.packed')
# the exports of warp.wasm the page calls (driver.js, values.js): the browser's copy keeps only these, and Binaryen drops
# what they never reach and the name section (1.35 → 1.26 MB gzipped); an export missing here fails the tour loudly
COMPILER_EXPORTS=(main _start memory text_heap trap_detail trap_position trap_name show trap_message shown_at shown_from returned_error
	error_of new_empty new_int new_bool new_float new_codepoint new_text new_symbol new_key new_list new_type new_bytes new_ints
	new_floats new_spelled_key new_spelled_list item_count item_at int_item_at float_item_at gpu_list_place gpu_list_cell
	is_textual)
# core/commands/commands.warp WASM_OPT's features: Binaryen's -all would add exact references Firefox refuses
WASM_OPT_FEATURES=(--enable-gc --enable-reference-types --enable-bulk-memory --enable-multivalue --enable-sign-ext
	--enable-nontrapping-float-to-int --enable-mutable-globals --enable-tail-call --enable-simd --enable-extended-const
	--enable-multimemory --enable-exception-handling)

cd "$repository"

# the modules a lib/ source's use lines name (core/parsing/parser.warp used_modules: `use x` leading a line)
used_modules() { [ -f "$1" ] && sed -nE 's/^(use|import|require) ([a-z_0-9]+)[[:space:]]*$/\2/p' "$1" | sort -u; }

# built_of_changed <module> <seen>: whether lib/<module>.warp or one of the modules it uses, transitively, differs
# from the installed library's, the text its prebuilt image was built of
built_of_changed() {
	local name=$1 seen=$2 used
	# (a used library of no lib/ source, `use sqlite3`'s lib/native/sqlite3.warp, is not built in)
	[ -f "$repository/lib/$name.warp" ] || return 1
	cmp -s "$repository/lib/$name.warp" "$INSTALLED_LIBRARY/$name.warp" || return 0
	for used in $(used_modules "$repository/lib/$name.warp"); do
		[[ $seen == *" $used "* ]] && continue
		built_of_changed "$used" "$seen$name " && return 0
	done
	return 1
}

build_compiler() {
	[ -f "$INSTALLED_LIBRARY/warp.wasm" ] || { echo "error: no warp.wasm in $INSTALLED_LIBRARY (warp rebuild, or WARP_LIB=<lib>)" >&2; exit 1; }
	refuse_lagging_compiler
	rm -rf "$page/warp-lib"
	rsync -aL "${NOT_FOR_THE_BROWSER[@]}" "$INSTALLED_LIBRARY/" "$page/warp-lib/"
	# the tree's library over it, so the page runs this checkout's lib/ before an install: a module whose source or
	# the source of a module it uses (transitively, as core/passes/library.warp built_of_texts) changed is compiled
	# from its source (its prebuilt modules/<name>.wasm dropped)
	rsync -aL "${NOT_FOR_THE_BROWSER[@]}" --exclude=compiler/ --exclude=modules/ --exclude="*.wasm" "$repository/lib/" "$page/warp-lib/"
	local module name
	for module in "$page/warp-lib/modules/"*.wasm; do
		[ -f "$module" ] || continue
		name=$(basename "$module" .wasm)
		# (an image of no lib/ source, a module's .splice.wasm, the page never reads)
		[ -f "$repository/lib/$name.warp" ] || { rm "$module"; continue; }
		built_of_changed "$name" " " || continue
		rm "$module"
		echo "lib/$name.warp or a module it uses changed: compiled from its source"
	done
	write_sources "$page/warp-lib"
	slim_compiler "$page/warp-lib/warp.wasm"
	pack_package_libraries
	echo "built $page/warp-lib from $INSTALLED_LIBRARY and lib/ ($(du -sh "$page/warp-lib" | cut -f1))"
}

# each package's C library as the page has it, lib/<name>.wasm (host-foreign.js packedLibraryImports): a proxy's
# `library: "dir/name"` prebuilt as <clone>/dir/name.wasm; natively scripts/link-packages.sh builds it beside warp
pack_package_libraries() {
	local manifest clone library
	# a fresh worktree has no packages/ (git ignores it): linked first, else the page lacks uniscript_c.wasm and
	# samples/text/uniscript_text.warp fails "uniscript_c.uniscript_error is not available in the browser"; the served
	# list, written after, then holds the packages samples use by path (samples/modules/packages.warp)
	compgen -G "packages/*/package.warp" > /dev/null || "$repository/scripts/link-packages.sh" "$repository"
	for manifest in packages/*/package.warp; do
		[ -f "$manifest" ] || continue
		library=$(manifest_field "$manifest" library)
		[ -n "$library" ] && clone=$(package_clone "$manifest") && [ -f "$clone/$library.wasm" ] || continue
		cmp -s "$clone/$library.wasm" "$page/lib/$(basename "$library").wasm" ||
			{ cp "$clone/$library.wasm" "$page/lib/" && echo "packed $page/lib/$(basename "$library").wasm"; }
	done
}

# the compiler's commit (warp.wasm.inputs) must have this checkout's COMPILER_SOURCES, else the page runs an older
# language than its lib/ (card playground-gaps: ~30 samples failed on "no field coefficient"); WARP_LAGGING_COMPILER=1
# takes it anyway
refuse_lagging_compiler() {
	local commit
	commit=$(sed -n 's/.* commit=\([^ ]*\).*/\1/p' "$INSTALLED_LIBRARY/warp.wasm.inputs" 2>/dev/null || true)
	if [ -z "$commit" ]; then echo "warning: $INSTALLED_LIBRARY/warp.wasm.inputs names no commit: the compiler may lag lib/" >&2; return; fi
	git diff --quiet "$commit" -- "${COMPILER_SOURCES[@]}" 2>/dev/null && return
	[ "${WARP_LAGGING_COMPILER:-}" = 1 ] && { echo "warning: $INSTALLED_LIBRARY/warp.wasm lags this checkout (built from $commit), taken anyway" >&2; return; }
	echo "error: $INSTALLED_LIBRARY/warp.wasm was built from $commit, whose core/ or lib/ differs from this checkout's: the page would run its lag. scripts/rebuild.sh pure $repository builds this checkout's own, which build.sh takes (or WARP_LAGGING_COMPILER=1)" >&2
	exit 1
}

# the compiler with only the COMPILER_EXPORTS, without what they never reach and without names; without wasm-opt as it is
slim_compiler() {
	command -v wasm-opt >/dev/null || { echo "warning: no wasm-opt (brew install binaryen): $1 keeps all its exports" >&2; return; }
	python3 -c 'import sys
module, keep = sys.argv[1], set(sys.argv[2:])
data = open(module, "rb").read()
def number(at):
	value = shift = 0
	while True:
		byte = data[at]; at += 1; value |= (byte & 0x7f) << shift; shift += 7
		if byte < 0x80: return value, at
def encoded(value):
	out = bytearray()
	while True:
		byte, value = value & 0x7f, value >> 7
		out.append(byte | (0x80 if value else 0))
		if not value: return bytes(out)
EXPORT_SECTION = 7
out, at = bytearray(data[:8]), 8
while at < len(data):
	section, (size, body) = data[at], number(at + 1)
	if section == EXPORT_SECTION:
		count, entry = number(body)
		kept = []
		for _ in range(count):
			length, name = number(entry)
			_, after = number(name + length + 1)
			if data[name:name + length].decode() in keep: kept.append(data[entry:after])
			entry = after
		exports = encoded(len(kept)) + b"".join(kept)
		out += bytes([section]) + encoded(len(exports)) + exports
	else:
		out += data[at:body + size]
	at = body + size
open(module, "wb").write(out)' "$1" "${COMPILER_EXPORTS[@]}"
	wasm-opt "${WASM_OPT_FEATURES[@]}" --remove-unused-module-elements --strip-debug "$1" -o "$1"
}

# <lib>/sources.json: {name: text} of the PRELOADED_SOURCES, each byte for byte: newline="" keeps a lone \r, which
# Python's universal newlines made \n, and the packed sugar rules (112 of them) then unpacked wrong: every tour example
# trapped 'illegal cast' (card sugar-packed-page-cast)
write_sources() {
	(cd "$1" && python3 -c 'import glob, json, sys
names = sorted(name for pattern in sys.argv[1:] for name in glob.glob(pattern))
json.dump({name: open(name, encoding="utf-8", newline="").read() for name in names}, open("sources.json", "w"), ensure_ascii=False)' "${PRELOADED_SOURCES[@]}")
}

# served-files.js: the files the server has, so the compiler's module and header searches ask only for those that exist
# (host-files.js; a missing one is a red 404 in the console, card console-errors). Paths are relative to the served root
write_served_files() {
	python3 -c 'import json, sys
names = sorted(line for line in sys.stdin.read().split("\n") if line)
print("// made by build.sh: the files the server has (host-files.js isUnserved)\nconst SERVED_FILES = new Set(" + json.dumps(names) + ");")' > "$1"
	echo "built $1"
}

# the repository's files, as the dev server and test_in_browser.py serve them from the repository root
write_repository_served_files() {
	# plus the compiler's lib/ and the packages, which git ignores
	{ git -c core.quotepath=off ls-files --cached --others --exclude-standard; find web/playground/warp-lib -type f 2>/dev/null || true; package_files; find web/playground/lib -name '*.wasm'; } | write_served_files "$page/served-files.js"
}

# the files of packages/ (scripts/link-packages.sh, the registry's folders linked in), which samples use by path,
# `use "../../packages/colors/colors.warp"` (decision package-use-by-path)
package_files() {
	[ -d packages ] && find -L packages -type f -not -path '*/.git/*' -not -name .DS_Store || true
}

write_site_served_files() {
	(cd "$1" && find . -type f | sed 's|^\./||') | write_served_files "$1/served-files.js"
}

# the repository's tracked files of these paths copied into <site> at their paths; the regular ones, not the links
shipped_tracked() {
	local site=$1 file
	shift
	git ls-files -z -- "$@" | while IFS= read -r -d '' file; do
		[ -L "$file" ] || { mkdir -p "$site/$(dirname "$file")" && cp "$file" "$site/$file"; }
	done
}

# the deployed site from a built playground (pages.yml, scripts/browser-dry-run.sh): a fresh <site> folder
collect_site() {
	local site part
	rm -rf "$1" && mkdir -p "$1" && site=$(cd "$1" && pwd)
	(cd "$page" && cp -r ${SITE_FILES[@]} "$site/") # unquoted: host-*.js expands in the page folder
	# the language primer for agents at the conventional path (llmstxt.org)
	cp "$page/primer.md" "$site/llms.txt"
	# the compiler at its repository path (driver.js COMPILER_LIBRARY)
	mkdir -p "$site/web/playground" && cp -r "$page/warp-lib" "$site/web/playground/"
	# every host part the worker imports (host.js HOST_PART_FILES) is shipped, else no deploy
	for part in $(grep -o '"host-[a-z]*\.js"' "$page/host.js" | tr -d '"'); do
		[ -f "$site/$part" ] || { echo "::error::$part missing from the site" >&2; exit 1; }
	done
	# the samples' neighbours at their repository paths, as natively: `play "tune.mp3"` beside samples/sound/play_mp3.warp
	# (card open-mozart; tune.mp3 is our own half-second C-E-G, mozart.mp3 left for its unclear copyright), the files a sample reads, writes over or includes; the regular ones, not the links
	# (samples/tests, samples/wasm/shapes.wasm)
	shipped_tracked "$site" samples
	# the files the tour's examples read, at their repository paths too: `use wasm "test/fixtures/…/rust_demo.wasm"`,
	# `import test/fixtures/wasm/zlib` (its .wasm and .warp), else the page's compile meets an unknown name
	for named in $(grep -oE '(test|lib|data)/[A-Za-z0-9_./-]+' "$page/examples.js" | sort -u); do
		shipped_tracked "$site" "$named" "$named.*"
	done
	package_files | while IFS= read -r file; do mkdir -p "$site/$(dirname "$file")" && cp -L "$file" "$site/$file"; done
	echo "$DOMAIN" > "$site/CNAME"
	# the files the site serves, so the compiler asks only for those (host-files.js): no 404 in the console
	write_site_served_files "$site"
	stamp_assets "$site"
	echo "collected $site"
}

# each script and style sheet the site's index.html links, its URL stamped with its content hash, `playground.js?v=3f…`
# (card stale-cached-assets): GitHub Pages lets a browser keep any file 10 minutes, so a reloaded page met the build
# before's CSS; a changed file is a new URL, fetched at once. version.js is one, so the workers' ?build= (host.js
# ofThisBuild) is the new build's too
stamp_assets() {
	local index=$1/index.html asset stamp
	for asset in $(grep -oE '(src|href)="[^":?#]+\.(js|css)"' "$index" | sed -E 's/^[a-z]+="(.*)"$/\1/' | sort -u); do
		[ -f "$1/$asset" ] || continue
		stamp=$(shasum -a 256 "$1/$asset" | cut -c1-$STAMP_DIGITS)
		ASSET="$asset" STAMP="$stamp" perl -pi -e 's{((?:src|href)=")\Q$ENV{ASSET}\E"}{$1$ENV{ASSET}?v=$ENV{STAMP}"}g' "$index"
	done
}

case "${1:-all}" in
	served) write_site_served_files "$2"; exit ;;
	site) collect_site "$2"; exit ;;
	compiler) build_compiler ;;
	all) build_compiler ;;
	*) echo "usage: $0 [compiler|served <site>|site <site>]" >&2; exit 2 ;;
esac

# the compiled module samples/wasm/import_compiled.warp links (samples/wasm/shapes.wasm, a link to it), built behind the
# page's runtime.wasm, as scripts/samples.sh builds it natively
WARP_LIB="$INSTALLED_LIBRARY" warp build --module test/fixtures/compiled/shapes.warp || echo "warning: no test/fixtures/compiled/shapes.wasm: wasm/import_compiled fails in the page" >&2
write_repository_served_files

# a folder per guide chapter (notes/samples.md); golf/ and wit/ hold no menu samples, native/ only natively built ones
python3 - "$page/samples.js" $(ls samples/*/*.warp | grep -v '^samples/\(golf\|wit\|native\)/') <<'PYTHON'
import collections, json, os, re, sys
# a sample a browser cannot run says so on a line of its own, `# host: native: <why>` (or none), and stays out of the
# menu (test_in_browser.py HOST_LINE)
NOT_IN_PAGE = re.compile(r"^# host: (native|none)\b", re.M)
files = collections.defaultdict(list)
for path in sys.argv[2:]: files[os.path.basename(path)[:-len(".warp")]].append(path)
# the menu and the address (#fizzbuzz) name a sample by its file name: one name, one file
twins = {name: paths for name, paths in files.items() if len(paths) > 1}
if twins: sys.exit(f"samples sharing a name: {twins}")
names = {name: paths[0] for name, paths in sorted(files.items()) if not NOT_IN_PAGE.search(open(paths[0], encoding="utf-8").read())}
samples = {name: open(path, encoding="utf-8").read() for name, path in names.items()}
with open(sys.argv[1], "w", encoding="utf-8") as script:
	script.write("// made by build.sh from samples/<chapter>/*.warp\nconst SAMPLES = " + json.dumps(samples, ensure_ascii=False, indent="\t") + ";\n")
	script.write("// each sample's file, which its runs name their neighbours beside (playground.js programFile)\n")
	script.write("const SAMPLE_FILES = " + json.dumps(names, indent="\t") + ";\n")
	# a sample's <name>.args beside it, one argument a line, as `use os; args` gives them natively in the tests
	# (tests/common arguments_beside): a song visualizer plays its first seconds only (card playground-browser)
	beside = {name: path[:-len(".warp")] + ".args" for name, path in names.items()}
	arguments = {name: open(file, encoding="utf-8").read().splitlines() for name, file in beside.items() if os.path.exists(file)}
	script.write("const SAMPLE_ARGUMENTS = " + json.dumps(arguments, ensure_ascii=False) + ";\n")
PYTHON
echo "built $page/samples.js"

# the editor colors the keywords of P165 (keywords.py)
python3 web/playground/keywords.py "$page/keywords.js"
echo "built $page/keywords.js and keywords.txt"
# the uniscript entities completion.js lists after \: and <:
cp lib/entities.tsv "$page/entities.tsv"

# the ⋯ menu names the commit the page was built from and when, so a deployed page tells which version it is
# (cards version-commit, g_oMw8)
echo "// made by build.sh: the commit built
const PLAYGROUND_VERSION = { commit: \"$(git rev-parse HEAD)\", date: \"$(git log -1 --format=%cs)\", built: \"$(date -u +%Y-%m-%dT%H:%M:%SZ)\" };" > "$page/version.js"
echo "built $page/version.js"
