// The uniscript page's own script (web/uniscript/uniscript.js with imports.js) run in node, against the built page:
// uniscript.wasm and the C library it imports, modules/uniscript_c.wasm. No browser on this Mac, so the DOM is a stub.
// Usage: web/uniscript/build.sh (or its compile and modules steps), then node probes/web_uniscript/page_in_node.mjs
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import vm from "node:vm";

const PAGE = fileURLToPath(new URL("../../web/uniscript/", import.meta.url));
const EXPECTED = [
	["uniscript", "<:alpha> <:fracture A> \\:infinity", "α 𝔄 ∞"],
	["uniscript", "<:nosuchthing>", "unknown uniscript entity: nosuchthing"],
	["unicode_to_uniscript", "α 𝔄", "\\:alpha \\:fracture-A"],
];

const elements = new Map();
const element = id => elements.get(id) ?? elements.set(id, {
	value: "", textContent: "", dataset: {},
	classList: { toggle() {}, add() {} },
	addEventListener() {}, append(...parts) { this.textContent += parts.join(""); }, replaceChildren() {},
}).get(id);
const document = { getElementById: element, querySelectorAll: () => [], body: element("body"), createElement: () => element(Symbol()) };
const fetch = async path => {
	try {
		return new Response(readFileSync(PAGE + path));
	} catch {
		return new Response(null, { status: 404 });
	}
};

const context = vm.createContext({ document, fetch, WebAssembly, TextEncoder, TextDecoder, console, setTimeout, clearTimeout, Response });
vm.runInContext(readFileSync(PAGE + "imports.js", "utf8") + readFileSync(PAGE + "uniscript.js", "utf8"), context);
for (let wait = 0; !document.body.dataset.ready && wait < 100; wait++) await new Promise(done => setTimeout(done, 50));
if (!document.body.dataset.ready) throw new Error(`the page did not start: ${element("status").textContent}`);

let failures = 0;
for (const [functionName, input, expected] of EXPECTED) {
	const { text } = vm.runInContext("convert", context)(functionName, input);
	const ok = text === expected;
	failures += !ok;
	console.log(`${ok ? "ok  " : "FAIL"} ${functionName}(${JSON.stringify(input)}) = ${JSON.stringify(text)}${ok ? "" : `, expected ${JSON.stringify(expected)}`}`);
}
console.log(`status: ${element("status").textContent}`);
process.exit(failures ? 1 : 0);
