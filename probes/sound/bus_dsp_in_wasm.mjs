// card shared-buses stage 2: the browser's bus effects are the native DSP (src/sound/effects.rs) through warp.wasm's
// web_bus_processed, called as web/playground/worker.js busProcessed calls it. An impulse through a delay bus echoes
// 0.3 s later at half its level, the tail added; a chain it cannot read gives 0.
// Run: node probes/sound/bus_dsp_in_wasm.mjs <warp.wasm> (a web-debug or release cdylib, web/playground/build.sh)
import { readFileSync } from "node:fs";

const RATE = 1000;
const wasm = readFileSync(process.argv[2] ?? "web/playground/warp.wasm");
const module = await WebAssembly.compile(wasm);
const stubs = {};
for (const { module: from, name, kind } of WebAssembly.Module.imports(module)) {
	if (kind === "function") (stubs[from] ??= {})[name] = () => 0;
}
const compiler = (await WebAssembly.instantiate(module, stubs)).exports;

const passBytes = bytes => {
	const pointer = compiler.web_alloc(bytes.length);
	new Uint8Array(compiler.memory.buffer, pointer, bytes.length).set(bytes);
	return [pointer, bytes.length];
};
const busProcessed = (chain, floats, rate, channels) => {
	const chainText = passBytes(new TextEncoder().encode(JSON.stringify(chain)));
	const samples = passBytes(new Uint8Array(floats.buffer, floats.byteOffset, floats.byteLength));
	const length = compiler.web_bus_processed(...chainText, ...samples, rate, channels);
	compiler.web_free(...chainText);
	compiler.web_free(...samples);
	const at = compiler.web_report();
	return length ? new Float32Array(compiler.memory.buffer.slice(at, at + length)) : undefined;
};

const impulse = new Float32Array(RATE / 10);
impulse[0] = 1;
const echoed = busProcessed([["delay", 0.3, 0.5, 1]], impulse, RATE, 1);
const check = (label, holds) => {
	console.log(`${holds ? "ok" : "FAILED"}: ${label}`);
	if (!holds) process.exitCode = 1;
};
check(`the dry impulse stays (${echoed[0]})`, Math.abs(echoed[0] - 1) < 1e-6);
check(`its echo 0.3 s later at half (${echoed[300]})`, Math.abs(echoed[300] - 0.5) < 1e-6);
check(`the tail is added (${echoed.length} samples)`, echoed.length > 600);
check("an unknown effect gives nothing", busProcessed([["wobble", 1]], impulse, RATE, 1) === undefined);
