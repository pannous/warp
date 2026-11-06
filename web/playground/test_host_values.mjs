// The host words that read a program's values in the page (host.js notify, dispatch_event) on a module of the
// self-hosted compiler, which has no reflect_* exports: read through its exports (values.js guestPlain), as natively.
// Before card drop-readnode-branch they read it with reader.js readNode and failed ("module.reflect_tag is not a
// function"). Needs `warp` on PATH: each program is built by `warp build` into its self-contained <name>.full.wasm.
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import vm from "node:vm";

const folder = dirname(fileURLToPath(import.meta.url));
// the run worker's scripts in one global scope, as worker.js imports them
const page = vm.createContext({ console, URL, TextDecoder, TextEncoder, WebAssembly, setTimeout, clearTimeout, location: { href: "http://localhost/web/playground/" }, navigator: {}, addEventListener: () => {}, postMessage: () => {} });
page.self = page.globalThis = page;
page.importScripts = (...files) => files.forEach(file => vm.runInContext(readFileSync(join(folder, file), "utf8"), page, { filename: file }));
page.importScripts("imports.js", "js-members.js", "std.js", "host.js");
page.importScripts(...vm.runInContext("HOST_PART_FILES", page));

const work = mkdtempSync(join(tmpdir(), "warp-host-values-"));
// what a program said to the page through the hooks, after its run
function ran(name, code) {
	const file = join(work, `${name}.warp`);
	writeFileSync(file, code);
	execFileSync("warp", ["build", file], { cwd: work, stdio: "pipe" });
	const said = { notified: [], dispatched: [], printed: "" };
	const hooks = { print: text => { said.printed += text; }, notify: text => said.notified.push(text), dispatch: event => said.dispatched.push(event) };
	const holder = page.runHolder({ warnings: [], hooks });
	const module = new WebAssembly.Module(readFileSync(join(work, `${name}.full.wasm`)));
	assert.equal(WebAssembly.Module.exports(module).some(entry => entry.name.startsWith("reflect_")), false, "a self-hosted module has no reflect_* exports");
	holder.exports = new WebAssembly.Instance(module, page.programImports(holder, hooks)).exports;
	holder.exports._start();
	return said;
}

try {
	assert.deepEqual(ran("notified", 'notify("hi there")\n1\n').notified, ["hi there"]);
	assert.deepEqual(ran("notified_list", "notify([1, 2])\n1\n").notified, ["[1,2]"]);
	const [event] = ran("dispatched", 'dispatch_event("saved", {n: 3, name: "x"})\n1\n').dispatched;
	assert.deepEqual({ ...event, detail: { ...event.detail } }, { name: "saved", detail: { n: 3, name: "x" } });
	console.log("host words read self-hosted values: all checks passed");
} finally {
	rmSync(work, { recursive: true, force: true });
}
