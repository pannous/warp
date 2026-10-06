// Runs tests of the Rust test binary built for wasm32-wasip1 without the native feature (tests.html): every test in a
// fresh instance, since a panic aborts the instance (wasm has panic=abort). `is!` and friends compile and run their
// programs through warp_host (host.js), exactly like the playground.

importScripts("reader.js", "host.js", "components.js", "wasi.js");
prepareTaskPool(); // task Workers start while this worker is idle (host.js)

let module; // the compiled test binary

// an instance of the test binary with libtest arguments, printing through `hooks`
function instantiate(args, hooks) {
	let instance;
	const memory = () => instance.exports.memory;
	instance = new WebAssembly.Instance(module, {
		wasi_snapshot_preview1: wasiImports(memory, ["tests", ...args], hooks.print),
		warp_host: warpHost(memory, hooks),
	});
	return instance;
}

// blocks known only at run time compile with the test binary itself: it exports the page's compiler entry points
// (src/web.rs web_eval_block), so the blocks run the code under test, with no warp.wasm to build
self.BLOCK_COMPILER = hooks => instantiate([], hooks).exports;

// run the binary with libtest arguments: {code, output, trapped}
function runBinary(args) {
	let output = "";
	const print = text => { output += text; };
	const hooks = { print, panicked: print };
	forgetBlockCompiler(); // a compiler instance of this test's own, printing into its output
	const instance = instantiate(args, hooks);
	try {
		instance.exports._start();
		return { code: 0, output };
	} catch (stop) {
		if (stop instanceof WasiExit) return { code: stop.code, output };
		return { code: null, output: `${output}\n${stop.message}`, trapped: true };
	}
}

const TEST_LINE = /^(.+): test$/;
const SKIPPED = /- should panic \.\.\. ignored/;
const listed = args => runBinary(["--list", "--format", "terse", ...args]).output.split("\n").map(line => line.match(TEST_LINE)?.[1]).filter(Boolean);

let compiled; // resolves when the binary is compiled: messages that arrive meanwhile wait for it

self.onmessage = async ({ data }) => {
	if (data.type === "compile") {
		compiled = WebAssembly.compileStreaming(fetch(data.url)).then(binary => { module = binary; });
		return;
	}
	await compiled;
	if (data.type === "list") {
		const tests = listed(data.filters);
		const ignored = new Set(listed(["--ignored", ...data.filters]));
		self.postMessage({ type: "listed", tests: tests.map(name => ({ name, ignored: ignored.has(name) })) });
	}
	if (data.type === "run") {
		await taskPoolReady(); // host.js: tasks run on loaded Workers, not inline
		const started = performance.now();
		const { code, output, trapped } = runBinary([data.name, "--exact", "--nocapture", "--test-threads=1", ...(data.ignored ? ["--ignored"] : [])]);
		// libtest skips #[should_panic] tests when panics abort, as they do in wasm
		const skipped = SKIPPED.test(output);
		self.postMessage({ type: "result", name: data.name, passed: code === 0 && !skipped, skipped, trapped, output, milliseconds: performance.now() - started });
	}
};
