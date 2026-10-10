// Runs tests of the Rust test binary built for wasm32-wasip1 without the native feature (tests.html): every test in a
// fresh instance, since a panic aborts the instance (wasm has panic=abort). `is!` and friends compile and run their
// programs through warp_host (host.js), exactly like the playground.

importScripts("reader.js", "imports.js", "host.js");
importScripts(...HOST_PART_FILES, "components.js", "wasi.js");
prepareTaskPool(); // task Workers start while this worker is idle (host.js)

let module; // the compiled test binary

// an instance of the test binary with libtest arguments, printing through `hooks`; `awaited`: its runs may wait for
// promises (host-compiler.js warpHost), _start then called through awaitedEntry
function instantiate(args, hooks, awaited = false) {
	let instance;
	const memory = () => instance.exports.memory;
	instance = new WebAssembly.Instance(module, {
		wasi_snapshot_preview1: wasiImports(memory, ["tests", ...args], hooks.print),
		warp_host: warpHost(memory, hooks, awaited),
	});
	return instance;
}

// blocks known only at run time compile with the test binary itself: it exports the page's compiler entry points
// (src/web.rs web_eval_block), so the blocks run the code under test, with no warp.wasm to build
self.BLOCK_COMPILER = hooks => instantiate([], hooks).exports;

// run the binary with libtest arguments: {code, output, trapped}
async function runBinary(args) {
	let output = "";
	const print = text => { output += text; };
	const hooks = { print, panicked: print, sound: (samples, rate) => self.voicePlaced(samples.length / rate) }; // no page plays it: sound_queued() counts
	forgetBlockCompiler(); // a compiler instance of this test's own, printing into its output
	const instance = instantiate(args, hooks, true);
	try {
		await awaitedEntry(instance.exports._start)();
		return { code: 0, output };
	} catch (stop) {
		if (stop instanceof WasiExit) return { code: stop.code, output };
		return { code: null, output: `${output}\n${stop.message}`, trapped: true };
	}
}

const TEST_LINE = /^(.+): test$/;
const SKIPPED = /- should panic \.\.\. ignored/;
const listed = async args => (await runBinary(["--list", "--format", "terse", ...args])).output.split("\n").map(line => line.match(TEST_LINE)?.[1]).filter(Boolean);

let compiled; // resolves when the binary is compiled: messages that arrive meanwhile wait for it

// WARP_JSPI tells the tests that a program's main waits for a foreign call's promise here (host.js JSPI, tests/common jspi)
if (JSPI) ENVIRONMENT.push("WARP_JSPI=1");

// WARP_GPU_ADAPTER names the browser's WebGPU adapter to the tests, "(software)" marked: SwiftShader, CI's, computes
// sin and exp only as exactly as WGSL requires (tests/common gpu_tolerance, card browser-gpu)
async function announceGpuAdapter() {
	const adapter = await self.navigator.gpu?.requestAdapter();
	if (!adapter) return;
	const { vendor, architecture, description } = adapter.info ?? {};
	const software = adapter.info?.isFallbackAdapter || adapter.isFallbackAdapter || architecture === "swiftshader";
	ENVIRONMENT.push(`WARP_GPU_ADAPTER=${[vendor, architecture, description].filter(Boolean).join(" ")}${software ? " (software)" : ""}`);
}

self.onmessage = async ({ data }) => {
	if (data.type === "compile") {
		compiled = Promise.all([WebAssembly.compileStreaming(fetch(data.url)).then(binary => { module = binary; }), announceGpuAdapter()]);
		return;
	}
	await compiled;
	if (data.type === "list") {
		const tests = await listed(data.filters);
		const ignored = new Set(await listed(["--ignored", ...data.filters]));
		self.postMessage({ type: "listed", tests: tests.map(name => ({ name, ignored: ignored.has(name) })) });
	}
	if (data.type === "run") {
		await taskPoolReady(); // host.js: tasks run on loaded Workers, not inline
		const started = performance.now();
		const { code, output, trapped } = await runBinary([data.name, "--exact", "--nocapture", "--test-threads=1", ...(data.ignored ? ["--ignored"] : [])]);
		// libtest skips #[should_panic] tests when panics abort, as they do in wasm
		const skipped = SKIPPED.test(output);
		self.postMessage({ type: "result", name: data.name, passed: code === 0 && !skipped, skipped, trapped, output, milliseconds: performance.now() - started });
	}
};
