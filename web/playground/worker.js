// The warp compiler (warp.wasm, built by build.sh) and the programs it compiles (host.js), run off the page's thread:
// a worker may compile any module synchronously and block on a synchronous fetch, which the host calls need.

importScripts("reader.js", "host.js", "components.js");
prepareTaskPool(); // task Workers start while this worker is idle (host.js)

// warp.wasm, the optimized build, or the one the page names (?compiler=warp.debug.wasm, build.sh)
const COMPILER_URL = new URL(self.location.href).searchParams.get("compiler") ?? "warp.wasm";
self.BLOCK_COMPILER_URL = COMPILER_URL; // run_block compiles with the same compiler (host.js blockCompiler)

let compiler; // the compiler instance's exports
let live; // the run whose page events are handled (host.js runProgram), until the next run
const PAGE_VALUE = "page·value";
let panicMessage; // the compiler's last panic message

const post = message => self.postMessage(message);
const hooks = {
	print: (text, stream) => post({ type: "print", text, stream }),
	module: bytes => post({ type: "module", bytes }),
	paint: (pixels, width, height) => post({ type: "paint", pixels, width, height }),
	listen: (holder, events) => {
		live = holder;
		post({ type: "listening", events });
	},
	panicked: message => { panicMessage = message; },
};

const compilerText = (pointer, length) => readText(compiler, pointer, length);

async function loadCompiler() {
	const response = await fetch(COMPILER_URL);
	if (!response.ok) throw new Error(`${COMPILER_URL}: HTTP ${response.status}; build it with web/playground/build.sh`);
	const { instance } = await WebAssembly.instantiate(await response.arrayBuffer(), { warp_host: warpHost(() => compiler.memory, hooks) });
	compiler = instance.exports;
}

function passText(text) {
	const bytes = utf8.encode(text);
	const pointer = compiler.web_alloc(bytes.length);
	new Uint8Array(compiler.memory.buffer, pointer, bytes.length).set(bytes);
	return [pointer, bytes.length];
}

function evaluate(code, acknowledged) {
	panicMessage = undefined;
	const codeText = passText(code);
	const acknowledgedText = passText(JSON.stringify(acknowledged));
	try {
		const length = compiler.web_evaluate(...codeText, ...acknowledgedText);
		const report = JSON.parse(compilerText(compiler.web_report(), length));
		compiler.web_free(...codeText);
		compiler.web_free(...acknowledgedText);
		return report;
	} catch (crash) {
		compiler = undefined; // a panic leaves the compiler's memory in an unknown state: load it again
		return { value: `compiler crashed: ${panicMessage ?? crash.message}`, error: true, crashed: true, warnings: [], runtime_warnings: [], hints: [], notes: [] };
	}
}

const ready = loadCompiler().then(() => post({ type: "ready" }), failure => post({ type: "failed", message: failure.message }));

// `use python` runs in Pyodide (host.js registers its call): loaded on first use, since a host call cannot wait for it
const PYODIDE_URL = "https://cdn.jsdelivr.net/pyodide/v0.29.5/full/";
const USES_PYTHON = /\buse\s+python\b/;
let python; // the loading or loaded Pyodide with the bridge (web/playground/foreign_python.py)
function loadPython() {
	python ??= (async () => {
		post({ type: "print", text: "loading Python (Pyodide, ~12 MB, once)…\n", stream: 2 });
		importScripts(PYODIDE_URL + "pyodide.js");
		const pyodide = await loadPyodide({ indexURL: PYODIDE_URL });
		const bridge = await (await fetch("foreign_python.py")).text();
		pyodide.globals.set("WARP_BRIDGE_LOOP", false);
		pyodide.runPython(bridge);
		self.pythonAnswer = pyodide.globals.get("answer");
	})();
	return python;
}
registerForeignRuntime("python", {
	prepare: code => USES_PYTHON.test(code) && loadPython().catch(failure => post({ type: "print", text: `Python could not load: ${failure.message}\n`, stream: 2 })),
});

// a page event (playground.js): the live run's handler, its value shown as the compiler shows a program's
function handleEvent({ event, detail }) {
	if (!live) return;
	const handled = runPageEvent(live, hooks, event, detail);
	// the output binding (src/lowering/event_signals.rs PAGE_VALUE): the program's last name read anew, else what the handler gave
	const binding = live.exports[PAGE_VALUE];
	const outcome = handled.result && binding ? outcomeOf(live, hooks, binding) : handled;
	const outcomeText = passText(JSON.stringify(outcome));
	const length = compiler.web_show(...outcomeText);
	const value = compilerText(compiler.web_report(), length);
	compiler.web_free(...outcomeText);
	post({ type: "handled", value, error: outcome.result === undefined });
}

self.onmessage = async ({ data }) => {
	await ready;
	if (data.event) return handleEvent(data);
	live = undefined;
	if (!compiler) await loadCompiler();
	await prepareForeignRuntimes(data.code); // host.js: a runtime that loads asynchronously loads before the run
	const started = performance.now();
	const report = evaluate(data.code, data.acknowledged ?? {});
	post({ type: "report", id: data.id, report, milliseconds: performance.now() - started });
};
