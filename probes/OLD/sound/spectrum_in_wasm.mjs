// The browser's spectrum is the native one (src/sound/spectrum.rs) through warp.wasm's web_spectrum, called as
// web/playground/host-compiler.js spectrumLevels calls it: a full-scale A4 lights its own band at about 1, the others
// far less; an empty window gives zeros.
// Run: node probes/sound/spectrum_in_wasm.mjs <warp.wasm> (a web-debug or release cdylib, web/playground/build.sh)
import { readFileSync } from "node:fs";

const RATE = 22050;
const WINDOW = 512;
const BANDS = 8;
const [LOW, HIGH] = [60, 5000];
const A4 = 440;
const wasm = readFileSync(process.argv[2] ?? "web/playground/warp.wasm");
const module = await WebAssembly.compile(wasm);
const stubs = {};
for (const { module: from, name, kind } of WebAssembly.Module.imports(module)) {
	if (kind === "function") (stubs[from] ??= {})[name] = () => 0;
}
const compiler = (await WebAssembly.instantiate(module, stubs)).exports;

const levels = amplitudes => {
	const bytes = new Uint8Array(Float32Array.from(amplitudes).buffer);
	const pointer = compiler.web_alloc(bytes.length);
	new Uint8Array(compiler.memory.buffer, pointer, bytes.length).set(bytes);
	const length = compiler.web_spectrum(pointer, bytes.length, RATE, BANDS, LOW, HIGH);
	compiler.web_free(pointer, bytes.length);
	const at = compiler.web_report();
	return Array.from(new Float64Array(compiler.memory.buffer.slice(at, at + length)));
};
const check = (label, holds) => {
	console.log(`${holds ? "ok" : "FAILED"}: ${label}`);
	if (!holds) process.exitCode = 1;
};

const nearestBand = Math.round(Math.log(A4 / LOW) / Math.log(HIGH / LOW) * (BANDS - 1));
const tone = levels(Array.from({ length: WINDOW }, (_, at) => 32767 * Math.sin(2 * Math.PI * A4 * at / RATE)));
check(`${BANDS} levels (${tone.map(level => level.toFixed(2))})`, tone.length === BANDS);
check(`A4 lights band ${nearestBand} most`, tone[nearestBand] === Math.max(...tone));
check("the far bands stay dark", tone[0] < 0.05 && tone[BANDS - 1] < 0.05);
check("an empty window is silent", levels([]).every(level => level === 0));
