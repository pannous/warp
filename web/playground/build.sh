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

# the installed compilers: lib/ beside the warp on $PATH (~/dev/bin/lib links to the current build), else WARP_LIB
INSTALLED_LIBRARY="${WARP_LIB:-$(dirname "$(command -v warp)")/lib}"
# what the browser cannot use of it: wasmtime's machine code, native libraries, kept values, stale compilers
NOT_FOR_THE_BROWSER=(--exclude='*.cwasm' --exclude='*.cwasm.key' --exclude='*.dylib' --exclude='*.so' --exclude='*.kept'
	--exclude='*.stale' --exclude='*.inputs' --exclude='warp-self.wasm*' --exclude='data/')
# what `build.sh site <folder>` collects from web/playground into the deployed site (pages.yml)
SITE_FILES=(index.html playground.css playground.js deploy.js assistant.js completion.js shortcuts.js entities.tsv guide.js
	guide.md guide-expert.md primer.md markup.js markup-transitions.js markup-forms.js canvas.js worker.js task-worker.js
	task-workers.js js-members.js coi-serviceworker.js host.js "host-*.js" values.js wasi.js driver.js components.js foreign_python.py
	reader.js imports.js exact.js std.js examples.js codemirror-warp-mode.js codemirror lib samples.js keywords.js
	keywords.txt assistant.json version.js)
DOMAIN=warp.pannous.com

page="$(cd "$(dirname "$0")" && pwd)"
repository="$(cd "$page/../.." && pwd)"
cd "$repository"

build_compiler() {
	[ -f "$INSTALLED_LIBRARY/warp.wasm" ] || { echo "error: no warp.wasm in $INSTALLED_LIBRARY (warp rebuild, or WARP_LIB=<lib>)" >&2; exit 1; }
	rm -rf "$page/warp-lib"
	rsync -aL "${NOT_FOR_THE_BROWSER[@]}" "$INSTALLED_LIBRARY/" "$page/warp-lib/"
	echo "built $page/warp-lib from $INSTALLED_LIBRARY ($(du -sh "$page/warp-lib" | cut -f1))"
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
	# plus the compiler's lib/, which git ignores
	{ git -c core.quotepath=off ls-files --cached --others --exclude-standard; find web/playground/warp-lib -type f 2>/dev/null || true; } | write_served_files "$page/served-files.js"
}

write_site_served_files() {
	(cd "$1" && find . -type f | sed 's|^\./||') | write_served_files "$1/served-files.js"
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
	git ls-files -z samples | while IFS= read -r -d '' file; do
		[ -L "$file" ] || { mkdir -p "$site/$(dirname "$file")" && cp "$file" "$site/$file"; }
	done
	echo "$DOMAIN" > "$site/CNAME"
	# the files the site serves, so the compiler asks only for those (host-files.js): no 404 in the console
	write_site_served_files "$site"
	echo "collected $site"
}

case "${1:-all}" in
	served) write_site_served_files "$2"; exit ;;
	site) collect_site "$2"; exit ;;
	compiler) build_compiler ;;
	all) build_compiler ;;
	*) echo "usage: $0 [compiler|served <site>|site <site>]" >&2; exit 2 ;;
esac

write_repository_served_files

# a folder per guide chapter (notes/samples.md); golf/ and wit/ hold no menu samples, native/ only natively built ones
python3 - "$page/samples.js" "$page/excluded_samples.txt" $(ls samples/*/*.warp | grep -v '^samples/\(golf\|wit\|native\)/') <<'PYTHON'
import collections, json, os, sys
# the samples the page cannot run yet stay out of the menu, each named with its error in excluded_samples.txt
excluded = {line.split()[0] for line in open(sys.argv[2], encoding="utf-8") if line.strip() and not line.startswith("#")}
files = collections.defaultdict(list)
for path in sys.argv[3:]: files[os.path.basename(path)[:-len(".warp")]].append(path)
# the menu and the address (#fizzbuzz) name a sample by its file name: one name, one file
twins = {name: paths for name, paths in files.items() if len(paths) > 1}
if twins: sys.exit(f"samples sharing a name: {twins}")
names = {name: paths[0] for name, paths in sorted(files.items()) if name not in excluded}
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

# the editor colors the keywords of P165 from their definitions (keywords.py)
# keywords.py still reads the Rust sources, gone since the compiler is warp: loud, and the page keeps the last keywords.js
python3 web/playground/keywords.py "$page/keywords.js" && echo "built $page/keywords.js and keywords.txt" ||
	echo "warning: keywords.js not rebuilt (keywords.py reads src/, card playground-keywords)" >&2
# the uniscript entities completion.js lists after \: and <:
cp core/entities.tsv "$page/entities.tsv"

# the ⋯ menu names the commit the page was built from and when, so a deployed page tells which version it is
# (cards version-commit, g_oMw8)
echo "// made by build.sh: the commit built
const PLAYGROUND_VERSION = { commit: \"$(git rev-parse HEAD)\", date: \"$(git log -1 --format=%cs)\", built: \"$(date -u +%Y-%m-%dT%H:%M:%SZ)\" };" > "$page/version.js"
echo "built $page/version.js"
