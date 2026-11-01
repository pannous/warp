// The warp compiler (warp.wasm, the self-hosted one build.sh copies, run by driver.js as the native warp runs it) and
// the programs it compiles (host.js), off the page's thread: a worker may compile any module synchronously and block
// on a synchronous fetch, which WASI's file reads and the host calls need.

// where a message waits: the page keeps the last stage for its stall warning and the Firefox driver's timeout report
// (cards firefox-hello-again, tour-firefox)
const stage = name => self.postMessage({ type: "stage", stage: name });

stage("loading its scripts");
importScripts("reader.js", "imports.js", "exact.js", "js-members.js", "std.js", "host.js");
importScripts(...HOST_PART_FILES, "components.js", "served-files.js", "wasi.js", "driver.js");
stage("starting the task workers");
prepareTaskPool(); // task Workers start while this worker is idle (host.js)

// ?slow_start=<ms> (playground.js passes the page's one): ready that much later, as on a slow CI runner
const SLOW_START_MS = Number(new URL(self.location.href).searchParams.get("slow_start") ?? 0);

let compiler; // the self-hosted compiler loaded (driver.js), undefined until then
// the messages use the compiler one after another: an evaluation whose program waits for a promise (host.js JSPI) holds
// the compiler's memory until it ends
let compilerTurn = Promise.resolve();
function inTurn(work) {
	const done = compilerTurn.then(work);
	compilerTurn = done.catch(() => {});
	return done;
}
let live; // the run whose page events are handled (host.js runProgram), until the next run
const PAGE_VALUE = "page·value";
// after a handler that patched holes, the value text waits this long for the next event before it is read whole
const VALUE_DELAY_MILLISECONDS = 50;
let panicMessage; // the compiler's last panic message

let warming = false; // the warm-up's run says nothing to the page
const post = (message, transfer) => warming || self.postMessage(message, transfer);
// the first markup program compiles the markup renderer (lib/markup.warp's to_html) once: ~0.5 s in Chrome, seconds in
// Safari. The page asks for it while idle after its first run (card guide-warmup), so a guide's first ▶ is quick too
const WARM_UP_CODE = 'p{ "" }';
self.keepStored = (name, value, file) => post({ type: "stored", name, value, file }); // host-files.js STD_ADAPTERS.store
self.writeClipboard = text => post({ type: "clipboard", text }); // host-files.js STD_ADAPTERS.clipboard
self.askPageClipboard = shared => self.postMessage({ type: "clipboard read", shared }); // host.js clipboard_text
self.askPageDecode = (shared, url, bytes) => self.postMessage({ type: "sound decode", shared, url, bytes }); // host-sound.js pageDecoded
self.askPageCall = (shared, request) => self.postMessage({ type: "page call", shared, request }); // host-foreign.js callOnPage
self.keepFile = (path, bytes) => post({ type: "file", path, bytes }); // save sound's WAV, for the page's download link
// a program serving the page's requests (host-routes.js PAGE_SERVING): waiting for the next in `shared`, its answer
self.servingPage = (shared, port) => self.postMessage({ type: "serving", shared, port });
self.servedPage = response => self.postMessage({ type: "served", response });
addPageSoundWords(post); // host-tasks.js
const hooks = {
	renders: true, // each outcome carries its HTML by the program's own renderer (host.js renderedHtml)
	print: (text, stream) => post({ type: "print", text, stream }),
	module: bytes => post({ type: "module", bytes }),
	// host-media.js paintFrame: its words move to the page, not copied
	paint: (frame, width, height, canvas, shaded) => post({ type: "paint", frame, width, height, canvas, shaded }, frame && [frame.words.buffer]),
	gpuCanvas: canvas => post({ type: "gpu canvas", ...canvas }), // host-gpu.js gpuPainted
	savePicture: (path, words, width, height) => post({ type: "picture", path, words, width, height }, [words.buffer]), // host-media.js save_picture
	sound: sound => self.playSamples(sound),
	sleeping: milliseconds => post({ type: "sleep", milliseconds }),
	tasksInline: reason => post({ type: "tasks inline", reason }),
	notify: text => post({ type: "notify", text }),
	resizeWindow: (width, height) => post({ type: "window", width, height }), // window.width = 800 (canvas.js)
	dispatch: event => post({ type: "dispatch", event }), // playground.js dispatchToPage
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

// the response's bytes, each chunk told to the page as {type: "loading", loaded, total} (the page shows the progress,
// card firefox-hello-hang); a compressed response's Content-Length counts other bytes, so its total stays unknown
async function downloaded(response) {
	const total = response.headers.get("Content-Encoding") ? 0 : Number(response.headers.get("Content-Length") ?? 0);
	const chunks = [];
	let loaded = 0;
	for (const reader = response.body.getReader(); ;) {
		const { done, value } = await reader.read();
		if (done) break;
		chunks.push(value);
		loaded += value.length;
		post({ type: "loading", loaded, total });
	}
	const bytes = new Uint8Array(loaded);
	chunks.reduce((offset, chunk) => (bytes.set(chunk, offset), offset + chunk.length), 0);
	return bytes;
}

async function loadCompiler() {
	stage(`downloading ${COMPILER_URL}`);
	await loadDriverCompiler(response => (post({ type: "compiling" }), downloaded(response)));
	compiler = true;
}

// the code compiled and run as the native warp runs it (driver.js); a crash of the compiler itself loads it anew
async function evaluate(code) {
	panicMessage = undefined;
	try {
		return evaluatedNatively(code, self.programFile, hooks);
	} catch (crash) {
		compiler = undefined;
		return { value: `compiler crashed: ${crash.message}`, error: true, crashed: true, warnings: [], runtime_warnings: [], hints: [], notes: [] };
	}
}

// not over a live run (its listeners and timers stay), nor without a compiler (after a crash: the next run loads it)
async function warmUp() {
	if (live || !compiler) return;
	warming = true;
	try {
		await evaluate(WARM_UP_CODE, {});
	} finally {
		warming = false;
	}
}

const ready = loadCompiler().then(() => new Promise(done => setTimeout(done, SLOW_START_MS)))
	.then(() => post({ type: "ready" }), failure => post({ type: "failed", message: failure.message }));

// `use python` runs in Pyodide (host.js registers its call): loaded on first use, since a host call cannot wait for it
const PYODIDE_URL = "https://cdn.jsdelivr.net/pyodide/v0.29.5/full/";
const USES_PYTHON = /\buse\s+python\b/;
let python; // the loading or loaded Pyodide with the bridge (web/playground/foreign_python.py)
function loadPython() {
	python ??= (async () => {
		post({ type: "print", text: "loading Python (Pyodide, ~12 MB, once)…\n", stream: 2 });
		importScripts(PYODIDE_URL + "pyodide.js");
		const pyodide = await loadPyodide({ indexURL: PYODIDE_URL });
		const [bridge, capi] = await Promise.all(["foreign_python.py", "python_capi.py"].map(async file => (await fetch(file)).text()));
		pyodide.globals.set("WARP_BRIDGE_LOOP", false);
		pyodide.runPython(bridge);
		self.pythonAnswer = pyodide.globals.get("answer");
		pyodide.runPython(capi); // `use python`'s C API (host-foreign.js pythonLibrary)
		self.pythonCapi = pyodide.globals.get("capi");
		// Python's print goes to the run's output, as natively libpython's stdout is the process's
		pyodide.setStdout({ batched: line => post({ type: "print", text: `${line}\n`, stream: 1 }) });
		pyodide.setStderr({ batched: line => post({ type: "print", text: `${line}\n`, stream: 2 }) });
		return pyodide;
	})();
	return python;
}
// the modules a program imports (python_import("numpy")): Pyodide's packages among them loaded before the run, as
// natively they are installed beside libpython
const PYTHON_IMPORT = /python_import\("([\w.]+)"\)/g;
const pythonImports = code => [...code.matchAll(PYTHON_IMPORT)].map(([, name]) => `import ${name}`).join("\n");
registerForeignRuntime("python", {
	prepare: code => USES_PYTHON.test(code) && loadPython().then(pyodide => pyodide.loadPackagesFromImports(pythonImports(code), { messageCallback: () => {} }))
		.catch(failure => post({ type: "print", text: `Python could not load: ${failure.message}\n`, stream: 2 })),
});

// a link in the shown markup was followed (playground.js followLink): the live run shows the page at that path
function handleNavigation(path) {
	if (!live) return;
	navigate(live, hooks, path);
	post({ type: "address", path: addressOf(live) });
	showHandled(live, { result: true });
}

// the page's path for the playground's address bar: only a program with routes has one (lowering/web/routes.rs)
const addressOf = holder => holder.exports[PAGE_ROUTES_EXPORT] ? holder.pagePath ?? ROOT_PATH : undefined;

// a form of the page was sent (playground.js submitForm): the live run's route answers it, then its page shows anew
function handleSubmit(request) {
	if (live) showHandled(live, runSubmitted(live, hooks, request));
}

// what the Rust compiler of the page answered and the self-hosted one has no word for yet (card playground-compiles)
const NOT_SELF_HOSTED = "not in the self-hosted playground yet";

// the playground's Deploy (deploy.js): the program's Worker module and the query naming the scripts warp-hosting
// composes its Worker of, `{query, module}` or `{error}`
const workerBundle = () => ({ error: `deploy: ${NOT_SELF_HOSTED}` });

// the members to offer after the code and a dot (completion.js): a list of names
const memberNames = () => [];

// a page event (playground.js): the live run's handler
function handleEvent({ event, detail }) {
	if (live?.exports.page_event) return showPageEvent(live, pageEventHandled(live, event, detail));
	if (live) showHandled(live, runPageEvent(live, hooks, event, detail));
}

// a page of markup after its handler (driver.js pageEventHandled): the value, the page, the handlers it has now
function showPageEvent(holder, { value, html, error, events }) {
	post({ type: "handled", value, html, error });
	if (events) post({ type: "listening", events: events.map(event => `on ${event}`), address: addressOf(holder) });
}

// what a handler gave, shown as the compiler shows a program's value: the output binding (src/lowering/signals/event_signals.rs
// PAGE_VALUE), the program's last name read anew, else the handler's own value (a timer's only when it failed)
function showHandled(holder, handled, timer = false) {
	const binding = holder.exports[PAGE_VALUE] ?? holder.exports[PAGE_ROUTED_EXPORT];
	if (timer && !binding && handled.result) return;
	if (handled.result && binding && showHoles(holder)) return;
	const outcome = handled.result && binding ? outcomeOf(holder, hooks, binding) : handled;
	const failed = outcome.result === undefined;
	const { value, html } = shownOf(outcome);
	// a failing handler (a route's raise refusing an empty title) leaves the page as it was and says why beside it
	const page = failed && binding ? shownOf(outcomeOf(holder, hooks, binding)).html : html;
	post({ type: "handled", value, html: page, error: failed });
}

// the value and HTML the compiler shows for a run outcome: a page event's handler, not run by the self-hosted page yet
function shownOf(outcome) {
	return { value: outcome.failure ?? `page events: ${NOT_SELF_HOSTED}` };
}

// markup with holes (PAGE_HOLE·<path>, src/lowering/signals/event_signals.rs, card web-fine-holes): only the elements holding
// computed parts, or their computed texts alone between children (TEXT_HOLE), are read, the page gets those whose HTML changed; the whole value follows once the events pause.
// False when there are none, or one fails (the whole value then shows the error)
function showHoles(holder) {
	const holes = pageHoles(holder.exports);
	if (!holes.length) return false;
	const rendering = holder.exports[PAGE_HOLES];
	if (rendering && outcomeOf(holder, hooks, rendering).result === undefined) return false;
	holder.holeHtml ??= new Map();
	const patches = [];
	for (const { name, path, after } of holes) {
		const outcome = outcomeOf(holder, hooks, holder.exports[name]);
		if (outcome.result === undefined) return false;
		const shown = after === undefined ? { html: outcome.html ?? shownOf(outcome).html } : { text: String(plainOfTree(outcome.result)), after };
		const seen = shown.html ?? shown.text;
		if (holder.holeHtml.get(name) === seen) continue;
		holder.holeHtml.set(name, seen);
		patches.push({ path, ...shown });
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
	if (data.pointer) return self.pagePointer = sharedPointer(data.pointer); // host.js system_value
	if (data.system) return Object.assign(self.pageSystemValues ??= {}, data.system); // host.js system_value
	if (data.environment) return Object.assign(self, { pageEnvironment: data.environment, pageSecrets: data.secrets }); // host-files.js os.env, withPageSecret
	if (data.stored) return Object.assign(storedValues, data.stored) && Object.assign(sessionValues, data.session); // host-files.js STD_ADAPTERS.store
	stage("waiting for the compiler");
	await ready;
	stage("loading the database");
	await globalThis.loadKept?.(); // host-files.js: `database[k]`'s values and the written files, once
	return inTurn(() => handleMessage(data));
};

async function handleMessage(data) {
	if (data.warm) return warmUp();
	if (data.event) return handleEvent(data);
	if (data.navigate) return handleNavigation(data.navigate);
	if (data.submit) return handleSubmit(data.submit);
	if (data.bundle !== undefined) { // the running program keeps running
		if (!compiler) await loadCompiler();
		return post({ type: "bundle", bundle: workerBundle(data.bundle) });
	}
	if (data.members !== undefined) { // the running program keeps running
		if (!compiler) await loadCompiler();
		return post({ type: "members", id: data.id, names: memberNames(data.members) });
	}
	if (live) stopListening(live);
	live = undefined;
	stage("checking the compiler");
	if (!compiler) await loadCompiler();
	stage("preparing foreign runtimes");
	await prepareForeignRuntimes(data.code); // host.js: a runtime that loads asynchronously loads before the run
	stage("waiting for the task workers");
	await taskPoolReady(); // host.js: tasks run on loaded Workers, not inline
	stage(`evaluating run ${data.id}`);
	self.newSoundHandles(); // host-tasks.js
	self.liveHandover = data.liveHandover ?? {}; // the page's live loops of the run before (host-tasks.js liveCycleStarted)
	self.programFile = data.file; // host-files.js besideProgram
	self.programArguments = data.arguments ?? []; // host-files.js os.args
	const started = performance.now();
	const report = await evaluate(data.code, data.acknowledged ?? {});
	post({ type: "report", id: data.id, report, milliseconds: performance.now() - started });
}
