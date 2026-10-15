// card abysmally-slow: where the time of a drawing loop goes in the playground, measured in node with the playground's
// own worker scripts (web/playground/worker.js and its host parts) and the page's canvas.js drawing into a stub canvas.
// Prints per frame: the worker's time between two paint messages (the program's stroke + show()), the time inside the
// host's paint word, the bytes posted, and the page's time to draw the frame. Node's vm context makes global lookups
// slow, so compare the numbers with each other, not with a browser's.
// (In node every run's value ends as "type incompatibility when transforming from/to JS", after its frames.)
// Run after web/playground/build.sh optimized: node probes/paint_speed.mjs [program.warp]
import { readFileSync } from "node:fs";
import vm from "node:vm";

const PAGE = new URL("../web/playground/", import.meta.url);
const PROGRAM = process.argv[2] ?? new URL("paint_speed.warp", import.meta.url).pathname;

const posted = [];
const context = vm.createContext({
	console, performance, TextEncoder, TextDecoder, WebAssembly, URL, Headers, Response, structuredClone, queueMicrotask,
	setTimeout, clearTimeout, setInterval, clearInterval, Atomics, SharedArrayBuffer, BigInt64Array, atob, btoa, Blob,
	crypto, navigator,
	crossOriginIsolated: false,
	location: { href: new URL("worker.js", PAGE).href },
	fetch: async url => new Response(readFileSync(new URL(url, PAGE))),
	addEventListener() {},
	postMessage: message => posted.push({ at: performance.now(), message }),
});
context.self = context;
context.globalThis = context;
context.importScripts = (...files) => files.forEach(file => vm.runInContext(readFileSync(new URL(file, PAGE), "utf8"), context, { filename: file }));
context.importScripts("worker.js");

// the host's paint word, timed: wrapped where host-media.js registered it
const timedPaints = [];
vm.runInContext(`{
	const part = hostParts.find(part => part.words?.toString().includes("paint:"));
	const words = part.words;
	part.words = (...given) => {
		const made = words(...given);
		const paint = made.paint;
		made.paint = (...arguments_) => { const start = performance.now(); paint(...arguments_); timedPaintsPush(performance.now() - start); };
		return made;
	};
}`, Object.assign(context, { timedPaintsPush: milliseconds => timedPaints.push(milliseconds) }));

// the two steps of the paint word, timed: reading the list, shading and comparing
let workerPainting; // the worker's whole last painting, which the page's must equal
const stepTimes = { intHalvesOfList: [], paintFrame: [] };
for (const step of Object.keys(stepTimes)) {
	const original = context[step];
	context[step] = (...given) => {
		const begin = performance.now();
		const made = original(...given);
		stepTimes[step].push(performance.now() - begin);
		if (step === "paintFrame") workerPainting = given[0].paintedFrame.words.slice();
		return made;
	};
}

// the page side: canvas.js keeps each painting whole and draws what changed into a canvas whose 2D context does nothing
vm.runInContext(readFileSync(new URL("canvas.js", PAGE), "utf8"), context, { filename: "canvas.js" });
const screenFrames = [];
Object.assign(context, {
	ImageData: class { constructor(data, width, height) { Object.assign(this, { data, width, height }); } },
	requestAnimationFrame: draw => screenFrames.push(draw),
});
const stubCanvas = { getContext: () => ({ putImageData() {} }) };

const ready = new Promise(resolve => {
	const watch = setInterval(() => posted.some(({ message }) => message.type === "ready" || message.type === "failed") && (clearInterval(watch), resolve()), 20);
});
await ready;
const failed = posted.find(({ message }) => message.type === "failed");
if (failed) throw new Error(failed.message.message);
posted.length = 0;
const start = performance.now();
await context.onmessage({ data: { id: 1, code: readFileSync(PROGRAM, "utf8") } });
const total = performance.now() - start;

const paints = posted.filter(({ message }) => message.type === "paint");
const report = posted.find(({ message }) => message.type === "report")?.message.report;
const sizeOf = frame => frame?.words.byteLength ?? 0;
let pagePainting;
const pageTimes = paints.map(({ message }) => {
	const begin = performance.now();
	pagePainting = context.wholePainting(message);
	context.drawn(stubCanvas, pagePainting);
	screenFrames.splice(0).forEach(draw => draw());
	return performance.now() - begin;
});
const gaps = paints.slice(1).map((paint, index) => paint.at - paints[index].at);
const average = values => values.reduce((sum, value) => sum + value, 0) / Math.max(values.length, 1);
const line = (label, values) => console.log(`${label}: average ${average(values).toFixed(2)} ms, max ${Math.max(...values).toFixed(2)} ms`);
console.log(`value: ${report?.value}  frames: ${paints.length}  run: ${total.toFixed(0)} ms`);
line("worker frame (stroke + show + sleep)", gaps);
line("host paint word", timedPaints);
Object.entries(stepTimes).forEach(([step, times]) => line(`  ${step}`, times));
line("page draw (canvas.js drawn)", pageTimes);
const sizes = paints.map(({ message }) => sizeOf(message.frame));
console.log(`posted: first frame ${(sizes[0] / 1e6).toFixed(2)} MB, then average ${(average(sizes.slice(1)) / 1e3).toFixed(1)} KB`);
const pageWords = pagePainting?.words;
const same = pageWords?.length === workerPainting?.length && pageWords.every((word, at) => word === workerPainting[at]);
console.log(same ? "the page's painting equals the worker's" : "WRONG: the page's painting differs from the worker's");
process.exit(same ? 0 : 1);
