// The warp compiler (warp.wasm, built by build.sh) and the programs it compiles (host.js), run off the page's thread:
// a worker may compile any module synchronously and block on a synchronous fetch, which the host calls need.

importScripts("reader.js", "imports.js", "host.js");
importScripts(...HOST_PART_FILES, "components.js", "served-files.js");
prepareTaskPool(); // task Workers start while this worker is idle (host.js)

// warp.wasm, the optimized build, or the one the page names (?compiler=warp.debug.wasm, build.sh)
const COMPILER_URL = new URL(self.location.href).searchParams.get("compiler") ?? "warp.wasm";
self.BLOCK_COMPILER_URL = COMPILER_URL; // run_block compiles with the same compiler (host.js blockCompiler)

let compiler; // the compiler instance's exports
let live; // the run whose page events are handled (host.js runProgram), until the next run
const PAGE_VALUE = "page·value";
const PAGE_HOLE = "page·hole";
const PATH_JOINER = "·";
// after a handler that patched holes, the value text waits this long for the next event before it is read whole
const VALUE_DELAY_MILLISECONDS = 50;
let panicMessage; // the compiler's last panic message

let warming = false; // the warm-up's run says nothing to the page
const post = message => warming || self.postMessage(message);
// the first markup program compiles the markup renderer (lib/markup.warp's to_html) once: ~0.5 s in Chrome, seconds in
// Safari. The page asks for it while idle after its first run (card guide-warmup), so a guide's first ▶ is quick too
const WARM_UP_CODE = 'p{ "" }';
self.keepStored = (name, value, file) => post({ type: "stored", name, value, file }); // host-files.js STD_ADAPTERS.store
self.writeClipboard = text => post({ type: "clipboard", text }); // host-files.js STD_ADAPTERS.clipboard
const hooks = {
	renders: true, // each outcome carries its HTML by the program's own renderer (host.js renderedHtml)
	print: (text, stream) => post({ type: "print", text, stream }),
	module: bytes => post({ type: "module", bytes }),
	paint: (pixels, width, height) => post({ type: "paint", pixels, width, height }),
	sleeping: () => post({ type: "sleep" }),
	tasksInline: reason => post({ type: "tasks inline", reason }),
	notify: text => post({ type: "notify", text }),
	listen: (holder, events) => {
		live = holder;
		startTimers(holder, handler => runHandler(holder, handler));
		const fetches = [...(holder.fetches?.values() ?? [])].map(({ url }) => `fetch ${url}`);
		post({ type: "listening", events: [...events.map(event => `on ${event}`), ...(holder.timers ?? []).map(timer => timerLabel(holder, timer)), ...fetches], address: addressOf(holder) });
	},
	// a fetch's reply arrived (host.js startFetch): its handler runs like a timer's, and the page shows the outcome
	arrived: (holder, handler) => holder === live && runHandler(holder, handler),
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

// not over a live run (its listeners and timers stay), nor without a compiler (after a crash: the next run loads it)
function warmUp() {
	if (live || !compiler) return;
	warming = true;
	try {
		evaluate(WARM_UP_CODE, {});
	} finally {
		warming = false;
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

// a link in the shown markup was followed (playground.js followLink): the live run shows the page at that path
function handleNavigation(path) {
	if (!live) return;
	navigate(live, hooks, path);
	post({ type: "address", path: addressOf(live) });
	showHandled(live, { result: true });
}

// the page's path for the playground's address bar: only a program with routes has one (lowering/routes.rs)
const addressOf = holder => holder.exports[PAGE_ROUTES_EXPORT] ? holder.pagePath ?? ROOT_PATH : undefined;

// a page event (playground.js): the live run's handler
function handleEvent({ event, detail }) {
	if (live) showHandled(live, runPageEvent(live, hooks, event, detail));
}

// what a handler gave, shown as the compiler shows a program's value: the output binding (src/lowering/event_signals.rs
// PAGE_VALUE), the program's last name read anew, else the handler's own value (a timer's only when it failed)
function showHandled(holder, handled, timer = false) {
	const binding = holder.exports[PAGE_VALUE] ?? holder.exports[PAGE_ROUTED_EXPORT];
	if (timer && !binding && handled.result) return;
	if (handled.result && binding && showHoles(holder)) return;
	const outcome = handled.result && binding ? outcomeOf(holder, hooks, binding) : handled;
	const { value, html } = shownOf(outcome);
	post({ type: "handled", value, html, error: outcome.result === undefined });
}

// the value and HTML the compiler shows for a run outcome (src/web.rs shown)
function shownOf(outcome) {
	const outcomeText = passText(JSON.stringify(outcome));
	const length = compiler.web_show(...outcomeText);
	const shown = JSON.parse(compilerText(compiler.web_report(), length));
	compiler.web_free(...outcomeText);
	return shown;
}

// markup with holes (PAGE_HOLE·<path>, src/lowering/event_signals.rs, card web-fine-holes): only the elements holding
// computed parts are read, the page gets those whose HTML changed; the whole value follows once the events pause.
// False when there are none, or one fails (the whole value then shows the error)
function showHoles(holder) {
	const holes = Object.keys(holder.exports).filter(name => name.startsWith(PAGE_HOLE + PATH_JOINER));
	if (!holes.length) return false;
	holder.holeHtml ??= new Map();
	const patches = [];
	for (const name of holes) {
		const outcome = outcomeOf(holder, hooks, holder.exports[name]);
		if (outcome.result === undefined) return false;
		const html = outcome.html ?? shownOf(outcome).html;
		if (holder.holeHtml.get(name) === html) continue;
		holder.holeHtml.set(name, html);
		patches.push({ path: name.split(PATH_JOINER).slice(PAGE_HOLE.split(PATH_JOINER).length).map(Number), html });
	}
	post({ type: "handled", patches });
	clearTimeout(holder.valueTimer);
	holder.valueTimer = setTimeout(() => {
		if (live !== holder) return;
		const outcome = outcomeOf(holder, hooks, holder.exports[PAGE_VALUE]);
		post({ type: "handled", value: shownOf(outcome).value, error: outcome.result === undefined });
	}, VALUE_DELAY_MILLISECONDS);
	return true;
}

// a timer's or fetch's handler run and its outcome shown; a failing handler stops the timers
function runHandler(holder, handler) {
	const handled = runTimer(holder, hooks, handler);
	showHandled(holder, handled, true);
	if (handled.result === undefined) holder.stopTimers();
}

// the run's timers (host.js addTimer), each running its handler until the next run
self.onmessage = async ({ data }) => {
	if (data.pointer) return self.pagePointer = { values: new Int32Array(data.pointer.buffer), names: data.pointer.names }; // host.js system_value
	if (data.system) return Object.assign(self.pageSystemValues ??= {}, data.system); // host.js system_value
	if (data.stored) return Object.assign(storedValues, data.stored) && Object.assign(sessionValues, data.session); // host-files.js STD_ADAPTERS.store
	await ready;
	if (data.warm) return warmUp();
	if (data.event) return handleEvent(data);
	if (data.navigate) return handleNavigation(data.navigate);
	if (live) stopListening(live);
	live = undefined;
	if (!compiler) await loadCompiler();
	await prepareForeignRuntimes(data.code); // host.js: a runtime that loads asynchronously loads before the run
	await taskPoolReady(); // host.js: tasks run on loaded Workers, not inline
	const started = performance.now();
	const report = evaluate(data.code, data.acknowledged ?? {});
	post({ type: "report", id: data.id, report, milliseconds: performance.now() - started });
};
