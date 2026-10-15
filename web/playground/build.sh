#!/bin/bash
# Builds web/playground: the warp compiler for the browser (no wasmtime: the `native` feature off, src/web/mod.rs) in two
# builds, samples.js (samples/*.warp for the example menu), keywords.js (the editor's hard and soft keywords) and
# version.js (the commit built, shown in the ⋯ menu). Serve the repository root and open the page:
#   web/playground/build.sh && python3 -m http.server 8000   →   http://localhost:8000/web/playground/  (?debug: debug build)
# Usage: build.sh [optimized|debug|components|libraries|served <site>|site <site>]   (all when omitted)
#   optimized → warp.wasm: release profile (opt-level z, fat LTO, one codegen unit, stripped), without the `validate`
#               feature (wasmparser's validator, a quarter of the module: the browser validates anyway), then wasm-opt
#   debug     → warp.debug.wasm: profile web-debug (opt-level 1, line tables, the name section kept), with `validate`, so
#               emitter bugs are named and stack traces and the browser's debugger show Rust functions and lines
#   components → components/<name>.js for every COMPONENTS component (`use <name>`, components.js), and their names in
#               components/names.txt: jco
#               transpiles it (npm i -g @bytecodealliance/jco), the core modules go into the script as base64, the
#               WIT signatures of its exports as JSON (wasm-tools component wit --json; cargo install wasm-tools)
#   site <site> → collects the built page into a fresh <site> folder, as deployed (pages.yml, scripts/browser-dry-run.sh)
#   served <site> → <site>/served-files.js for a collected site; every build writes the repository's
#   libraries → packages/<name>/<library>.wasm and .h: the release assets of each package packages.warp gives a `library`
#               (`use uniscript` calls uniscript_c.wasm; browsers cannot fetch release assets, so the page serves them)
# Measured 2026-10-03: optimized 1.30 MB (545 KB gzipped); debug 22 MB (4.8 MB gzipped; full DWARF would be 53 MB).
# Not taken: -Cpanic=immediate-abort with -Zbuild-std saves another 6% but needs nightly and loses the panic messages.
set -euo pipefail

COMPONENTS=(tests/fixtures/components/*.wasm)
# what `build.sh site <folder>` collects from web/playground into the deployed site (pages.yml)
SITE_FILES=(index.html playground.css playground.js deploy.js assistant.js completion.js shortcuts.js entities.tsv guide.js
	guide.md guide-expert.md primer.md markup.js markup-transitions.js markup-forms.js canvas.js worker.js task-worker.js
	task-workers.js js-members.js coi-serviceworker.js host.js "host-*.js" components.js components foreign_python.py
	reader.js imports.js exact.js std.js examples.js codemirror-warp-mode.js codemirror lib packages warp.wasm
	warp.debug.wasm samples.js keywords.js keywords.txt assistant.json version.js)
DOMAIN=warp.pannous.com

STACK_BYTES=8388608 # the parser and the emitter recurse deeply; wasm32's default stack is 1 MB
WASM_OPT_FLAGS=(-Oz --converge --strip-debug --strip-producers
	--enable-bulk-memory --enable-nontrapping-float-to-int --enable-sign-ext --enable-mutable-globals)

page="$(cd "$(dirname "$0")" && pwd)"
repository="$(cd "$page/../.." && pwd)"
cd "$repository"

offline=$([ "${CARGO_NET_OFFLINE:-true}" = false ] || echo --offline) # CI (.github/workflows/pages.yml) fetches crates
target_dir="$(cargo metadata $offline --format-version 1 --no-deps | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')"

# compile the cdylib with a cargo profile and features; prints the path of the module
compile() {
	local profile=$1; shift
	RUSTFLAGS="-C link-arg=-zstack-size=$STACK_BYTES" cargo rustc $offline --lib --crate-type cdylib --profile "$profile" \
		--target wasm32-unknown-unknown --no-default-features "$@" >&2 || return # set -e stops nothing inside $(compile …)
	echo "$target_dir/wasm32-unknown-unknown/$profile/warp.wasm"
}

build_optimized() {
	local compiled
	compiled=$(compile release)
	# without wasm-opt (or an older one refusing a feature) the module of the LTO build serves, a seventh larger
	wasm-opt "${WASM_OPT_FLAGS[@]}" "$compiled" -o "$page/warp.wasm" || cp "$compiled" "$page/warp.wasm"
	echo "built $page/warp.wasm ($(wc -c < "$page/warp.wasm") bytes)"
}

build_debug() {
	cp "$(compile web-debug --features validate)" "$page/warp.debug.wasm"
	echo "built $page/warp.debug.wasm ($(wc -c < "$page/warp.debug.wasm") bytes)"
}

# a classic script for importScripts: registerComponent(name, {core module: base64}, signatures, instantiate)
build_components() {
	mkdir -p "$page/components"
	for component in "${COMPONENTS[@]}"; do
		local name transpiled="$repository/scratch/components/$(basename "$component" .wasm)"
		name=$(basename "$component" .wasm)
		jco transpile "$component" --instantiation sync --no-namespaced-exports --quiet -o "$transpiled"
		wasm-tools component wit --json "$component" > "$transpiled/wit.json"
		python3 - "$transpiled" "$name" "$component" > "$page/components/$name.js" <<'PYTHON'
import base64, glob, json, os, re, sys
folder, name, source = sys.argv[1:]
cores = {os.path.basename(path): base64.b64encode(open(path, "rb").read()).decode() for path in sorted(glob.glob(f"{folder}/*.wasm"))}
# the exported functions' parameter names and types: the root's own and those of its exported interfaces
wit = json.load(open(f"{folder}/wit.json"))
world = next((world for world in wit["worlds"] if world["name"] == "root"), wit["worlds"][-1])
exported = [item["function"] for item in world["exports"].values() if "function" in item]
exported += [function for item in world["exports"].values() if "interface" in item for function in wit["interfaces"][item["interface"]["id"]]["functions"].values()]
signatures = {"functions": {function["name"]: {"params": [parameter["name"] for parameter in function["params"]], "result": function.get("result")} for function in exported},
	"types": [type_["kind"] for type_ in wit["types"]]}
script = re.sub(r"^export ", "", open(f"{folder}/{name}.js", encoding="utf-8").read(), flags=re.M)
print(f"// made by build.sh from {source} (jco transpile --instantiation sync)")
print(f"registerComponent({json.dumps(name)}, {json.dumps(cores)}, {json.dumps(signatures)}, (() => {{\n{script}\nreturn instantiate;\n}})());")
PYTHON
		echo "built $page/components/$name.js"
	done
	# the compiler's list of them (src/lowering/foreign/foreign_modules.rs PAGE_COMPONENTS): `use rust_demo` names one
	for component in "${COMPONENTS[@]}"; do basename "$component" .wasm; done > "$page/components/names.txt"
}

# the library assets of the pinned packages (packages.warp `library`), as the native loader downloads them
# (src/modules/package_tools.rs library_directory); a tag never changes, so a file already there stays
# a release tagged v<version> or <version>, as package_tools.rs download_release_asset tries them
fetch_release_asset() {
	local repository=$1 version=$2 file=$3 target=$4 tag
	for tag in "v$version" "$version"; do
		curl -fsSL -o "$target" "$repository/releases/download/$tag/$file" && return
	done
	echo "no release asset $file of $repository $version (tags v$version, $version)" >&2
	return 1
}

fetch_package_libraries() {
	local entry name repository version library file
	# the registry's curly entries with a library; repository: "https://…", "github:owner/repo" or a bare owner/repo
	grep -E '^[a-z_]+: \{.*library: "[^"]+"' packages.warp | while read -r entry; do
		name=${entry%%:*}
		repository=$(grep -oE 'repository: "?[^",}]+' <<< "$entry" | sed -E 's/^repository: "?//; s/^github://; s/\.git$//')
		[[ $repository == *://* ]] || repository="https://github.com/$repository"
		version=$(grep -oE 'version: [0-9.]+' <<< "$entry"); version=${version#version: }
		library=$(grep -oE 'library: "[^"]+"' <<< "$entry" | cut -d'"' -f2)
		mkdir -p "$page/packages/$name"
		for file in "$library.wasm" "$library.h"; do
			[ -s "$page/packages/$name/$file" ] || fetch_release_asset "$repository" "$version" "$file" "$page/packages/$name/$file"
		done
		echo "fetched $page/packages/$name/$library.wasm and .h ($name $version)"
	done
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
	# plus the built components and the fetched libraries, which git ignores (components/names.txt is what the compiler asks for)
	{ git ls-files --cached --others --exclude-standard; find web/playground/components web/playground/packages -type f 2>/dev/null || true; } | write_served_files "$page/served-files.js"
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
	# every host part the worker imports (host.js HOST_PART_FILES) is shipped, else no deploy
	for part in $(grep -o '"host-[a-z]*\.js"' "$page/host.js" | tr -d '"'); do
		[ -f "$site/$part" ] || { echo "::error::$part missing from the site" >&2; exit 1; }
	done
	# the C module the tour imports (examples.js "C libraries"), at its repository path: the page's file root is the site
	mkdir -p "$site/tests/fixtures/wasm" && cp tests/fixtures/wasm/zlib.wasm tests/fixtures/wasm/zlib.h "$site/tests/fixtures/wasm/"
	# the warp-compiled module samples/import_compiled.warp imports by its bare name (samples/shapes.wasm links it)
	mkdir -p "$site/samples" && cp tests/fixtures/compiled/shapes.wasm "$site/samples/"
	echo "$DOMAIN" > "$site/CNAME"
	# the files the site serves, so the compiler asks only for those (host-files.js): no 404 in the console
	write_site_served_files "$site"
	echo "collected $site"
}

case "${1:-all}" in
	served) write_site_served_files "$2"; exit ;;
	site) collect_site "$2"; exit ;;
	optimized) build_optimized ;;
	debug) build_debug ;;
	# test_in_browser.py builds only these, and the worker imports served-files.js (card browser-served)
	components) build_components; fetch_package_libraries; write_repository_served_files; exit ;;
	libraries) fetch_package_libraries; write_repository_served_files; exit ;;
	all) build_optimized; build_debug; build_components; fetch_package_libraries ;;
	*) echo "usage: $0 [optimized|debug|components|libraries|served <site>|site <site>]" >&2; exit 2 ;;
esac

write_repository_served_files

python3 - "$page/samples.js" "$page/excluded_samples.txt" samples/*.warp <<'PYTHON'
import json, os, sys
# the samples the page cannot run yet stay out of the menu, each named with its error in excluded_samples.txt
excluded = {line.split()[0] for line in open(sys.argv[2], encoding="utf-8") if line.strip() and not line.startswith("#")}
names = {os.path.basename(path)[:-len(".warp")]: path for path in sys.argv[3:]}
samples = {name: open(path, encoding="utf-8").read() for name, path in names.items() if name not in excluded}
with open(sys.argv[1], "w", encoding="utf-8") as script:
	script.write("// made by build.sh from samples/*.warp\nconst SAMPLES = " + json.dumps(samples, ensure_ascii=False, indent="\t") + ";\n")
PYTHON
echo "built $page/samples.js"

# the editor colors the keywords of P165 from their definitions (keywords.py)
python3 web/playground/keywords.py "$page/keywords.js"
echo "built $page/keywords.js and keywords.txt"
# the uniscript entities completion.js lists after \: and <:
cp src/text/uniscript_entities.tsv "$page/entities.tsv"

# the ⋯ menu names the commit the page was built from and when, so a deployed page tells which version it is
# (cards version-commit, g_oMw8)
echo "// made by build.sh: the commit built
const PLAYGROUND_VERSION = { commit: \"$(git rev-parse HEAD)\", date: \"$(git log -1 --format=%cs)\", built: \"$(date -u +%Y-%m-%dT%H:%M:%SZ)\" };" > "$page/version.js"
echo "built $page/version.js"
