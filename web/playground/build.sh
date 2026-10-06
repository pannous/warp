#!/bin/bash
# Builds web/playground: the warp compiler for the browser (no wasmtime: the `native` feature off, src/web.rs) in two
# builds, samples.js (samples/*.wasp for the example menu) and keywords.js (the editor's hard and soft keywords). Serve the repository root and open the page:
#   web/playground/build.sh && python3 -m http.server 8000   →   http://localhost:8000/web/playground/  (?debug: debug build)
# Usage: build.sh [optimized|debug|components]   (all when omitted)
#   optimized → warp.wasm: release profile (opt-level z, fat LTO, one codegen unit, stripped), without the `validate`
#               feature (wasmparser's validator, a quarter of the module: the browser validates anyway), then wasm-opt
#   debug     → warp.debug.wasm: profile web-debug (opt-level 1, line tables, the name section kept), with `validate`, so
#               emitter bugs are named and stack traces and the browser's debugger show Rust functions and lines
#   components → components/<name>.js for every COMPONENTS component (`use <name>.wasm`, components.js): jco
#               transpiles it (npm i -g @bytecodealliance/jco), the core modules go into the script as base64, the
#               WIT signatures of its exports as JSON (wasm-tools component wit --json; cargo install wasm-tools)
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
		--target wasm32-unknown-unknown --no-default-features "$@" >&2
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
}

case "${1:-all}" in
	optimized) build_optimized ;;
	debug) build_debug ;;
	components) build_components; exit ;;
	all) build_optimized; build_debug; build_components ;;
	*) echo "usage: $0 [optimized|debug|components]" >&2; exit 2 ;;
esac

python3 - "$page/samples.js" samples/*.wasp <<'PYTHON'
import json, os, sys
samples = {os.path.basename(path)[:-len(".wasp")]: open(path, encoding="utf-8").read() for path in sys.argv[2:]}
with open(sys.argv[1], "w", encoding="utf-8") as script:
	script.write("// made by build.sh from samples/*.wasp\nconst SAMPLES = " + json.dumps(samples, ensure_ascii=False, indent="\t") + ";\n")
PYTHON
echo "built $page/samples.js"

# the editor colors the keywords of P165 from their one definition, src/lowering/soft_keywords.rs
python3 - "$page/keywords.js" src/lowering/soft_keywords.rs <<'PYTHON'
import json, re, sys
source = open(sys.argv[2], encoding="utf-8").read()
def words(name):
	return re.findall(r'"([^"]+)"', re.search(rf"const {name}: \[&str; \d+\] = \[(.*?)\];", source, re.S).group(1))
with open(sys.argv[1], "w", encoding="utf-8") as script:
	script.write("// made by build.sh from src/lowering/soft_keywords.rs\nconst KEYWORDS = " + json.dumps({"hard": words("HARD_KEYWORDS"), "soft": words("SOFT_KEYWORDS")}, ensure_ascii=False) + ";\n")
PYTHON
echo "built $page/keywords.js"
