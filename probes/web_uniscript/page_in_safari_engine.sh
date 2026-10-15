#!/bin/bash
# The uniscript page's script in Safari's JavaScriptCore shell, with a stub DOM and fetch (page_in_node.mjs is the node twin).
# Usage: probes/web_uniscript/page_in_safari_engine.sh   (after web/uniscript/build.sh)
set -euo pipefail
JSC=/System/Library/Frameworks/JavaScriptCore.framework/Versions/A/Helpers/jsc
repository="$(cd "$(dirname "$0")/../.." && pwd)"
harness="$repository/scratch/page_in_safari_engine.js"
mkdir -p "$repository/scratch"
cat > "$harness" <<'JS'
const elements = new Map();
const element = id => elements.get(id) ?? elements.set(id, {
	value: "", textContent: "", dataset: {},
	classList: { toggle() {}, add() {} },
	addEventListener() {}, append(...parts) { this.textContent += parts.join(""); }, replaceChildren() {},
}).get(id);
const document = { getElementById: element, querySelectorAll: () => [], body: element("body"), createElement: () => element(Symbol()) };
const fetch = async path => {
	try {
		const bytes = read(path, "binary");
		return { ok: true, arrayBuffer: async () => bytes.buffer };
	} catch {
		return { ok: false, status: 404 };
	}
};
const console = { log: print, warn: print };
// the shell has no TextEncoder/TextDecoder: UTF-8 through the URI escapes
class TextEncoder { encode(text) { return Uint8Array.from(unescape(encodeURIComponent(text)), c => c.charCodeAt(0)); } }
class TextDecoder { decode(bytes) { return decodeURIComponent(escape(String.fromCharCode(...new Uint8Array(bytes)))); } }
load("imports.js");
load("uniscript.js");
let polls = 0;
const report = () => {
	if (!document.body.dataset.ready && polls++ < 200) return setTimeout(report, 20);
	print(`status: ${element("status").textContent}`);
	print(convert("uniscript", "<:alpha> <:fracture A> \\:infinity").text);
	print(convert("unicode_to_uniscript", "α 𝔄").text);
};
report();
JS
cd "$repository/web/uniscript" && "$JSC" "$harness"
