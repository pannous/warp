#!/bin/bash
# Probe (card browser-test): how many live Wasm memories one V8 process holds, and whether a Worker's instantiation fails
# when another thread of the process holds them (V8 reserves address space for Wasm memories per process, GCs per isolate).
# Usage: probes/wasm_memory_limit.sh [memories the main thread keeps alive] [pages each memory starts with]
node --input-type=commonjs - "${1:-100000}" "${2:-1}" <<'JS'
const { Worker, isMainThread, parentPort } = require("worker_threads");
const pages = Number(process.argv[3]); // (module (memory pages)), no maximum, like warp's; pages as a LEB128
const leb = [];
for (let rest = pages; ; rest >>= 7) { const low = rest & 0x7f; if (rest >> 7) leb.push(low | 0x80); else { leb.push(low); break; } }
const bytes = new Uint8Array([0, 97, 115, 109, 1, 0, 0, 0, 5, 2 + leb.length, 1, 0, ...leb]);
const module = new WebAssembly.Module(bytes);
const live = [];
function fill(limit) {
	try {
		while (live.length < limit) live.push(new WebAssembly.Instance(module));
		return `held ${live.length}`;
	} catch (failure) {
		return `failed after ${live.length}: ${failure.message}`;
	}
}
console.log("main:", fill(Number(process.argv[2])));
const worker = new Worker(`
	const { parentPort } = require("worker_threads");
	const module = new WebAssembly.Module(new Uint8Array([0, 97, 115, 109, 1, 0, 0, 0, 5, 3, 1, 0, 1]));
	try { new WebAssembly.Instance(module); parentPort.postMessage("instantiated"); }
	catch (failure) { parentPort.postMessage("failed: " + failure.message); }
`, { eval: true });
worker.on("message", message => { console.log("worker:", message); process.exit(0); });
JS
