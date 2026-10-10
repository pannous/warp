#!/bin/bash
# Builds web/playground: the warp compiler for the browser (no wasmtime: the `native` feature off, src/web.rs) in two
# builds, samples.js (samples/*.warp for the example menu), keywords.js (the editor's hard and soft keywords) and
# version.js (the commit built, shown in the ⋯ menu). Serve the repository root and open the page:
#   web/playground/build.sh && python3 -m http.server 8000   →   http://localhost:8000/web/playground/  (?debug: debug build)
# Usage: build.sh [optimized|debug|components|served <site>]   (all when omitted)
#   optimized → warp.wasm: release profile (opt-level z, fat LTO, one codegen unit, stripped), without the `validate`
#               feature (wasmparser's validator, a quarter of the module: the browser validates anyway), then wasm-opt
#   debug     → warp.debug.wasm: profile web-debug (opt-level 1, line tables, the name section kept), with `validate`, so
#               emitter bugs are named and stack traces and the browser's debugger show Rust functions and lines
#   components → components/<name>.js for every COMPONENTS component (`use <name>`, components.js), and their names in
#               components/names.txt: jco
#               transpiles it (npm i -g @bytecodealliance/jco), the core modules go into the script as base64, the
#               WIT signatures of its exports as JSON (wasm-tools component wit --json; cargo install wasm-tools)
#   served <site> → <site>/served-files.js for a collected site (pages.yml); every build writes the repository's
# Measured 2026-10-03: optimized 1.30 MB (545 KB gzipped); debug 22 MB (4.8 MB gzipped; full DWARF would be 53 MB).
# Not taken: -Cpanic=immediate-abort with -Zbuild-std saves another 6% but needs nightly and loses the panic messages.
set -euo pipefail

COMPONENTS=(tests/fixtures/components/*.wasm)

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
	# the compiler's list of them (src/lowering/foreign_modules.rs PAGE_COMPONENTS): `use rust_demo` names one
	for component in "${COMPONENTS[@]}"; do basename "$component" .wasm; done > "$page/components/names.txt"
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
	# plus the built components, which git ignores (components/names.txt is what the compiler asks for)
	{ git ls-files --cached --others --exclude-standard; find web/playground/components -type f 2>/dev/null; } | write_served_files "$page/served-files.js"
}

case "${1:-all}" in
	served) (cd "$2" && find . -type f | sed 's|^\./||') | write_served_files "$2/served-files.js"; exit ;;
	optimized) build_optimized ;;
	debug) build_debug ;;
	# test_in_browser.py builds only these, and the worker imports served-files.js (card browser-served)
	components) build_components; write_repository_served_files; exit ;;
	all) build_optimized; build_debug; build_components ;;
	*) echo "usage: $0 [optimized|debug|components|served <site>]" >&2; exit 2 ;;
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

# the editor colors the keywords of P165 from their one definition, src/lowering/soft_keywords.rs
python3 - "$page/keywords.js" src/lowering/soft_keywords.rs src/modules.rs <<'PYTHON'
import json, re, sys
source = open(sys.argv[2], encoding="utf-8").read()
modules = open(sys.argv[3], encoding="utf-8").read()
# the standard modules `use` completes (completion.js): STD_MODULES' names, their aliases, the named constants'
std_names = re.findall(r'\(\s*(?:"([a-z0-9_]+)"|([A-Z_]+_MODULE)),\s*include_str!', modules)
constant = lambda name: re.search(rf'const {name}: &str = "([^"]+)"', modules).group(1)
module_names = [text or constant(name) for text, name in std_names] + re.findall(r'\("([a-z_]+)", "[a-z_]+"\)', re.search(r"STD_MODULE_ALIASES.*?\];", modules, re.S).group(0))
def words(name):
	return re.findall(r'"([^"]+)"', re.search(rf"const {name}: \[&str; \d+\] = \[(.*?)\];", source, re.S).group(1))
with open(sys.argv[1], "w", encoding="utf-8") as script:
	script.write("// made by build.sh from src/lowering/soft_keywords.rs\nconst KEYWORDS = " + json.dumps({"hard": words("HARD_KEYWORDS"), "soft": words("SOFT_KEYWORDS") + words("HIGHLIGHTED_WORDS"), "modules": module_names}, ensure_ascii=False) + ";\n")
PYTHON
echo "built $page/keywords.js"
# the uniscript entities completion.js lists after \: and <:
cp src/uniscript_entities.tsv "$page/entities.tsv"

# the ⋯ menu names the commit the page was built from and when, so a deployed page tells which version it is
# (cards version-commit, g_oMw8)
echo "// made by build.sh: the commit built
const PLAYGROUND_VERSION = { commit: \"$(git rev-parse HEAD)\", date: \"$(git log -1 --format=%cs)\", built: \"$(date -u +%Y-%m-%dT%H:%M:%SZ)\" };" > "$page/version.js"
echo "built $page/version.js"
