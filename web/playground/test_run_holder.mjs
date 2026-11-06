// Regression for cors-blocked-request / forgotten-previous-message: while linked modules start, before the program's
// own Instance exists, holder.exports must already answer through runtime.wasm (and other linked modules) so WASI and
// host words can read memory. Live site 7c2c636f5 used a separate Map for linking, so holder.exports stayed undefined
// and Firefox said "can't access property memory, holder.exports is undefined" (or program() is undefined).
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import vm from "node:vm";

const folder = dirname(fileURLToPath(import.meta.url));
const sandbox = {
	console, TextEncoder, TextDecoder, URL, WebAssembly, Proxy, Map, Set, Object, Array, BigInt, Number, String,
	Boolean, Error, TypeError, RangeError, Math, Date, JSON, Uint8Array, DataView, Atomics, Reflect, Symbol,
	Infinity, NaN, undefined,
};
sandbox.self = sandbox;
sandbox.globalThis = sandbox;
sandbox.utf8Decoder = new TextDecoder();
vm.runInNewContext(readFileSync(join(folder, "reader.js"), "utf8"), sandbox);
vm.runInNewContext(readFileSync(join(folder, "host.js"), "utf8"), sandbox);

const { runHolder } = sandbox;
assert.equal(typeof runHolder, "function");
const RUNTIME_MODULE = "warp_runtime"; // host.js RUNTIME_MODULE (const, not on the script global)

const holder = runHolder({ warnings: [] });
assert.equal(holder.exports, undefined, "no instances yet: exports stay undefined");
assert.ok(holder.linked instanceof Map);

holder.linked.set(RUNTIME_MODULE, { memory: { buffer: new ArrayBuffer(8) }, new_text: () => 1n });
assert.ok(holder.exports, "runtime linked: exports answer before the program Instance exists");
assert.equal(holder.exports.memory.buffer.byteLength, 8);
assert.equal(typeof holder.exports.new_text, "function");

holder.linked.set("warp/draw", { page_render: () => 0, memory: { buffer: new ArrayBuffer(1) } });
assert.equal(typeof holder.exports.page_render, "function", "linked modules' exports join the run's");
assert.equal(holder.exports.memory.buffer.byteLength, 8, "runtime memory wins while the program is not set");

holder.exports = { memory: { buffer: new ArrayBuffer(16) }, main: () => {} };
assert.equal(holder.exports.memory.buffer.byteLength, 16, "the program's exports take over once set");
assert.equal(typeof holder.exports.main, "function");
assert.equal(typeof holder.exports.new_text, "function", "linked words stay reachable beside the program");
assert.equal(typeof holder.exports.page_render, "function");

console.log("runHolder linked exports: all checks passed");
