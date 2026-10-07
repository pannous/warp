// The page: the editor, the worker that compiles and runs (worker.js), and the report shown like the CLI prints it:
// the value, printed output, errors, warnings, hints. An ambiguity is a warning naming its explicit form; "got it"
// silences a warning's topic in this browser (it never changes the value) until "show again". Each reading the user
// might have meant is an "I meant: …" button that rewrites the code at the warning and runs it again (notes/fixits.md).

const ACKNOWLEDGED_KEY = "warp-playground-acknowledged";
const OLD_ANSWERS_KEY = "warp-playground-answers"; // the Ask era kept {topic: form, "ack:<topic>": "acknowledged"}
const RUN_TIMEOUT_MS = 10000;
const TYPING_DELAY_MS = 300;
const DEFAULT_EXAMPLE = "welcome";
const DEBUG_PARAMETER = "debug"; // ?debug runs warp.debug.wasm: Rust names and lines in traces and the debugger
const DEBUG_COMPILER = "warp.debug.wasm";
const ACKNOWLEDGED = "acknowledged";
const ACKNOWLEDGED_PREFIX = "ack:";
const STDERR = 2;
// a page event, or one element's (`on click·1`, src/lowering/element_events.rs)
const PAGE_EVENT = /^on ((?:click|key|input)(?:·\d+)?)$/;
const DARK_MODE_QUERY = "(prefers-color-scheme: dark)";
// the gray levels of paint: a nonzero pixel, a zero pixel; from PAINT_COLOR_FROM on a value is a color 0xAARRGGBB
// (src/paint.rs shade, std/draw.wasp)
const PAINT_INK = 29;
const PAINT_PAPER = 250;
const PAINT_COLOR_FROM = 2 ** 24;
const PAINT_SHOWN_SIDE = 288; // a small painting is shown this wide (or high), scaled by a whole factor so pixels stay square

const $ = id => document.getElementById(id);
function element(tag, properties = {}, ...children) {
	const node = Object.assign(document.createElement(tag), properties);
	node.append(...children);
	return node;
}

let worker;
let workerReady;
let nextRunId = 0;
let pending; // {id, resolve, printed, timer} of the run in the worker
let runs = Promise.resolve(); // the evaluations, one after another
let showing = false; // the page is evaluating its editor's code
let queued; // the editor's code to show once the current run is shown
let typingTimer;
const debugBuild = new URLSearchParams(location.search).has(DEBUG_PARAMETER);
let lastModule; // the bytes of the last module the compiler emitted, for the download button

// ---- acknowledged topics, remembered in this browser ---------------------------------------------------------

function loadAcknowledged() {
	try {
		const saved = JSON.parse(localStorage.getItem(ACKNOWLEDGED_KEY));
		if (Array.isArray(saved)) return saved;
		const old = JSON.parse(localStorage.getItem(OLD_ANSWERS_KEY)) ?? {};
		return Object.keys(old).filter(key => key.startsWith(ACKNOWLEDGED_PREFIX)).map(key => key.slice(ACKNOWLEDGED_PREFIX.length));
	} catch { return []; }
}

function saveAcknowledged(topics) {
	acknowledged = [...new Set(topics)].sort();
	try { localStorage.setItem(ACKNOWLEDGED_KEY, JSON.stringify(acknowledged)); } catch { /* private window: lasts for this page */ }
}

let acknowledged = loadAcknowledged();

// what web_evaluate takes: `ack:<topic>` keys (the newer compiler also takes the plain list of topics)
const acknowledgements = () => Object.fromEntries(acknowledged.map(topic => [ACKNOWLEDGED_PREFIX + topic, ACKNOWLEDGED]));

function acknowledge(topic) {
	saveAcknowledged([...acknowledged, topic]);
	runNow();
}

function showAgain(topic) {
	saveAcknowledged(acknowledged.filter(known => known !== topic));
	runNow();
}

// ---- the worker ---------------------------------------------------------------------------------------------

function startWorker() {
	worker = new Worker(debugBuild ? `worker.js?compiler=${DEBUG_COMPILER}` : "worker.js");
	workerReady = new Promise((resolve, reject) => {
		worker.onmessage = ({ data }) => {
			if (data.type === "ready") return resolve();
			if (data.type === "stored") return keepStored(data.name, data.value);
			if (data.type === "failed") return reject(new Error(data.message));
			if (!pending) return showEventOutput(data);
			if (data.type === "listening") pending.listening = data.events;
			if (data.type === "print") pending.printed.push(data);
			if (data.type === "sleep") pending.slept = true;
			if (data.type === "paint") painted(pending, data);
			if (data.type === "module") lastModule = data.bytes;
			if (data.type === "report" && data.id === pending.id) finish(pending, { ...data.report, printed: pending.printed, paintings: pending.paintings, listening: pending.listening ?? [], milliseconds: data.milliseconds });
		};
	});
	workerReady.then(() => setStatus("ready"), failure => setStatus(failure.message, true));
	tellSystemValues();
	sharePointer();
	worker.postMessage({ stored: storedValues() });
}

// `stored theme = "dark"` (src/lowering/stored_values.rs): the page keeps each stored value in localStorage as JSON,
// the worker gets them all when it starts and sends each change back (host.js STD_ADAPTERS.store)
const STORED_PREFIX = "wasp stored ";
function storedValues() {
	try {
		const names = Object.keys(localStorage).filter(key => key.startsWith(STORED_PREFIX));
		return Object.fromEntries(names.map(key => [key.slice(STORED_PREFIX.length), JSON.parse(localStorage.getItem(key))]));
	} catch (failure) {
		console.error("stored values could not be read from localStorage:", failure);
		return {};
	}
}
function keepStored(name, value) {
	try {
		localStorage.setItem(STORED_PREFIX + name, JSON.stringify(value));
	} catch (failure) {
		console.error(`stored ${name} could not be kept in localStorage:`, failure);
	}
}

// the system values a Worker cannot read itself (host.js system_value), sent again when they change
const darkMode = matchMedia(DARK_MODE_QUERY);
const tellSystemValues = () => worker.postMessage({ system: { "dark mode": darkMode.matches } });
darkMode.addEventListener("change", tellSystemValues);

// `loop { …; show(); sleep(16) }`: a paint after a sleep is an animation's next frame, shown at once in place of the
// last one; frames keep the run alive past RUN_TIMEOUT_MS, the next run stops it (card drawing-frames)
function painted(run, painting) {
	if (!run.slept) return run.paintings.push(painting);
	run.slept = false;
	run.animating = true;
	run.paintings = [painting];
	showFrame(painting);
	stopAfterTimeout(run);
}

function stopAfterTimeout(run) {
	clearTimeout(run.timer);
	run.timer = setTimeout(() => stopRun(run, `stopped after ${RUN_TIMEOUT_MS / 1000} s: the program may not terminate`), RUN_TIMEOUT_MS);
}

// a program that does not stop blocks the worker: replace it
function stopRun(run, message) {
	worker.terminate();
	startWorker();
	finish(run, { value: message, error: !run.animating, printed: run.printed, paintings: run.paintings,
		warnings: [], runtime_warnings: [], hints: [], notes: [] });
}

function finish(run, report) {
	clearTimeout(run.timer);
	if (pending === run) pending = undefined;
	run.resolve(report);
}

// evaluate code in the worker: the report of src/web.rs evaluate plus `printed` [{text, stream}]. The worker runs one
// program at a time, so each call waits for the calls before it (typing while the compiler still loads makes several)
function evaluate(code) {
	const run = runs.then(() => runInWorker(code));
	runs = run.catch(() => {});
	return run;
}

async function runInWorker(code) {
	await workerReady;
	return new Promise(resolve => {
		const run = { id: ++nextRunId, resolve, printed: [], paintings: [] };
		stopAfterTimeout(run);
		pending = run;
		worker.postMessage({ id: run.id, code, acknowledged: acknowledgements() });
	});
}

// ---- showing a report ---------------------------------------------------------------------------------------

function setStatus(text, failed = false) {
	$("status").textContent = text;
	$("status").classList.toggle("failed", failed);
}

const at = (line, column) => line ? `${line}:${column}` : "";

function diagnostic(kind, position, ...content) {
	return element("li", { className: kind }, element("span", { className: "label" }, kind), position ? element("span", { className: "position" }, position) : "", ...content);
}

const code = text => element("code", {}, text);

// apply a fix's edits to the editor's text (UTF-16 ranges, src/web.rs fixes_json) and run the changed code
function applyFix(fix) {
	// every edit of the fix, the last in the text first, so the earlier offsets stay valid
	for (const edit of fix.edits) editor.replaceRange(edit.replacement, editor.posFromIndex(edit.start), editor.posFromIndex(edit.end));
	clearTimeout(typingTimer);
	runNow();
}

// the "I meant: …" buttons of a warning, error or hint; a fix whose text the code does not show cannot be applied
function fixButtons(fixes = []) {
	if (fixes.length === 0) return "";
	return element("span", { className: "fixes" }, ...fixes.map(fix => fix.start === null
		? element("button", { disabled: true, title: `${fix.meaning} (could not find \`${fix.written}\` in the code)` }, fix.label)
		: element("button", { className: "apply-fix", title: fix.meaning, onclick: () => applyFix(fix) }, fix.label)));
}

const GOT_IT_COMMENT = " // got it";

// a `// got it` comment at the end of the warning's line silences that line's warnings in the code itself
function commentGotIt(line) {
	const index = line - 1;
	editor.replaceRange(GOT_IT_COMMENT, { line: index, ch: editor.getLine(index).length });
	clearTimeout(typingTimer);
	runNow();
}

// "got it" (user, 2026-10-05): for this expression (its `topic@expression` key), for all of the kind (the topic), or
// as a `// got it` comment on its line; each keeps the code and never changes the value
function gotIt(topic, expression, line) {
	const button = (label, title, onclick) => element("button", { className: "got-it", title, onclick }, label);
	return element("span", { className: "got-it-choices" },
		expression ? button("got it", "stop warning about this expression", () => acknowledge(expression)) : "",
		button(`got it: all ${topic}`, `stop warning about every ${topic}`, () => acknowledge(topic)),
		line ? button("// got it", "say it in the code: a // got it comment on this line", () => commentGotIt(line)) : "");
}

function showAcknowledged() {
	$("acknowledged").replaceChildren(...acknowledged.map(topic => element("li", {}, `${topic} `,
		element("button", { className: "forget", title: "warn about it again", onclick: () => showAgain(topic) }, "show again"))));
	$("silenced").hidden = acknowledged.length === 0;
}

function showReport(report) {
	$("value").textContent = report.value;
	$("value").classList.toggle("error", report.error);
	const printed = report.printed.map(chunk => chunk.stream === STDERR ? "" : chunk.text).join("");
	$("printed").textContent = printed;
	$("printed").hidden = printed === "";
	showRendered(report.html);
	showPaintings(report.paintings ?? []);
	listenTo(report.listening ?? []);
	const notes = report.notes ?? [];
	const inline = new Set((report.warnings ?? []).map(warning => warning.topic).filter(topic => notes.includes(topic)));
	const expressionOf = topic => (report.got_it ?? []).find(offer => offer.topic === topic)?.expression;
	const shown = (kind, problem, extra = "") => diagnostic(kind, at(problem.line, problem.column), problem.message,
		problem.fix ? element("span", { className: "fix" }, "fix: ", code(problem.fix)) : "", fixButtons(problem.fixes), extra);
	const items = [
		...(report.errors ?? []).map(error => shown("error", error)),
		...report.warnings.map(warning => shown("warning", warning, inline.has(warning.topic) ? gotIt(warning.topic, warning.expression_key, warning.line) : "")),
		...report.runtime_warnings.map(message => diagnostic("warning", "runtime", message)),
		...report.hints.map(hint => diagnostic("hint", hint.position, "prefer ", code(hint.canonical), " over ", code(hint.original),
			element("span", { className: "reason" }, hint.reason), fixButtons(hint.fixes))),
		...notes.filter(topic => !inline.has(topic)).map(topic => diagnostic("note", "", `the ${topic} note above shows until you say `, gotIt(topic, expressionOf(topic)))),
	];
	$("diagnostics").replaceChildren(...items);
	markPositions(report);
	showAcknowledged();
	setStatus(report.crashed ? "the compiler crashed; reloaded" : `${Math.round(report.milliseconds ?? 0)} ms`, report.crashed);
}

let positionMarks = []; // the editor's underlines of the shown report

// the word at each error and warning position underlined in the editor, its message on hover (src/web.rs error_at is
// the position of an error no diagnostic made); hints are not marked: their positions can lag behind (card hint-positions)
function markPositions(report) {
	positionMarks.forEach(mark => mark.clear());
	const errors = report.errors?.length ? report.errors : [report.error_at && { ...report.error_at, message: report.value }].filter(Boolean);
	const places = [...errors.map(error => ["error", error]), ...(report.warnings ?? []).map(warning => ["warning", warning])];
	positionMarks = places.filter(([, { line, column }]) => line > 0 && column > 0 && line <= editor.lineCount())
		.map(([kind, { line, column, message }]) => markWord(kind, line - 1, column - 1, message)).filter(Boolean);
}

// the word (or the one character) at a position in characters, as the compiler counts, not UTF-16 units
function markWord(kind, line, column, message) {
	const characters = Array.from(editor.getLine(line));
	const rest = characters.slice(column).join("");
	const marked = rest.match(/^[\p{L}\p{N}_]+/u)?.[0] ?? Array.from(rest)[0];
	if (!marked) return null;
	const ch = characters.slice(0, column).join("").length;
	return editor.markText({ line, ch }, { line, ch: ch + marked.length }, { className: `marked-${kind}`, title: message ?? "" });
}

// paint(pixels, width, height): one canvas per call, a pixel dark where its value is nonzero (true), light where 0
// what a pixel value shows, as src/paint.rs shade: paper for 0, its color for a value with an alpha byte, else ink
function paintShade(value) {
	const number = Number(value);
	if (!value) return [PAINT_PAPER, PAINT_PAPER, PAINT_PAPER];
	if (number >= PAINT_COLOR_FROM) return [Math.floor(number / 65536) % 256, Math.floor(number / 256) % 256, number % 256];
	return [PAINT_INK, PAINT_INK, PAINT_INK];
}

// a markup value as DOM (std/markup.wasp, card web-dom), in a shadow root so its own style cannot restyle the page.
// Markup shown anew after a handler changes only the text nodes and attributes that differ (card web-fine): the
// elements stay, with their focus, input and scroll state.
function showRendered(html) {
	const host = $("rendered");
	host.hidden = !html;
	const template = document.createElement("template");
	template.innerHTML = html ?? "";
	morphChildren(host.shadowRoot ?? host.attachShadow({ mode: "open" }), template.content);
}

function showPaintings(paintings) {
	$("paintings").replaceChildren(...paintings.map(painting => {
		const canvas = element("canvas", { width: painting.width, height: painting.height, className: "painting" });
		canvas.style.width = `${painting.width * Math.max(1, Math.floor(PAINT_SHOWN_SIDE / Math.max(painting.width, painting.height, 1)))}px`;
		return drawn(canvas, painting);
	}));
}

// an animation's next frame drawn into the canvas shown, which keeps the pointer over it; another size is a new canvas
function showFrame(painting) {
	const shown = $("paintings").querySelectorAll("canvas");
	const [canvas] = shown;
	if (shown.length === 1 && canvas.width === painting.width && canvas.height === painting.height) drawn(canvas, painting);
	else showPaintings([painting]);
}

function drawn(canvas, { pixels, width, height }) {
	const image = canvas.getContext("2d").createImageData(width, height);
	for (let index = 0; index < width * height; index++) {
		image.data.set([...paintShade(pixels[index]), 255], index * 4);
	}
	canvas.getContext("2d").putImageData(image, 0, 0);
	return canvas;
}

// ---- page events (notes/signals.md phase 7): `on click {…}`, `on key {…}` of the program shown --------------------

let listening = new Set(); // the page events the shown program handles: "click", "key"

// what the shown program listens to: "on click", "on key" (page events, sent from here), "every 1 s", "at 09:00",
// "message from …" (timers and channels, run by the worker)
function listenTo(labels) {
	listening = new Set(labels.filter(label => PAGE_EVENT.test(label)).map(label => label.replace(PAGE_EVENT, "$1")));
	$("output").classList.toggle("listening", listening.size > 0);
	$("listening").hidden = labels.length === 0;
	const hint = listening.has("key") ? "; click here, then type" : listening.size ? "; click here" : "";
	$("listening").textContent = `listening: ${labels.join(", ")}${hint}`;
}

// a click on the output (on a canvas: its pixel), a key typed while the output has the focus
function sendPageEvent(event, detail) {
	if (listening.has(event)) worker.postMessage({ event, detail });
}

// an event inside the shown markup, for its element's handler (markup.js elementEvent)
function sendElementEvent(event, happened, detail) {
	const found = elementEvent(event, happened, detail);
	if (found) sendPageEvent(found.event, found.detail);
}

// mouse_x, mouse_y, mouse_down (host.js system_value): the pointer over a canvas in shared memory, which a running
// animation reads at once (its worker takes no message while it runs); the worker gets the buffer and these names
const POINTER_NAMES = ["mouse_x", "mouse_y", "mouse_down"];
const pointer = globalThis.SharedArrayBuffer ? new Int32Array(new SharedArrayBuffer(POINTER_NAMES.length * Int32Array.BYTES_PER_ELEMENT)) : undefined;
const sharePointer = () => pointer && worker.postMessage({ pointer: { buffer: pointer.buffer, names: POINTER_NAMES } });

function trackPointer(event) {
	if (!pointer || !event.target.closest?.("canvas")) return;
	const { x, y } = clickDetail(event);
	[x, y, event.buttons & 1].forEach((value, index) => Atomics.store(pointer, index, value));
}

function clickDetail(click) {
	const target = click.target.closest("canvas") ?? $("output");
	const bounds = target.getBoundingClientRect();
	const scale = target.width ? target.width / bounds.width : 1;
	return { x: Math.floor((click.clientX - bounds.left) * scale), y: Math.floor((click.clientY - bounds.top) * scale) };
}

// what a handler printed, painted and gave, while no run is pending
function showEventOutput(data) {
	if (data.type === "print" && data.stream !== STDERR) {
		$("printed").textContent += data.text;
		$("printed").hidden = false;
	}
	if (data.type === "paint") showPaintings([data]);
	if (data.type !== "handled") return;
	if (data.value !== undefined) {
		$("value").textContent = data.value;
		$("value").classList.toggle("error", data.error);
	}
	if (data.html !== undefined) showRendered(data.html);
	(data.patches ?? []).forEach(showPatch);
}

// one element of the shown markup anew (card web-fine-holes): `path` its element indices from the root down
function showPatch({ path, html }) {
	const root = $("rendered").shadowRoot;
	const shown = path.reduce((element, index) => element?.children[index], root?.children[0]);
	const template = document.createElement("template");
	template.innerHTML = html;
	const wanted = template.content.children[0];
	if (shown && wanted && shown.nodeName === wanted.nodeName) morphElement(shown, wanted);
	else console.error(`markup hole ${path.join("·")}: no ${wanted?.nodeName} there to update`, shown);
}

async function show(code) {
	if (showing) {
		queued = code; // only the newest code is worth running next
		if (pending?.animating) stopRun(pending, "animation stopped");
		return;
	}
	showing = true;
	setStatus("running…");
	try {
		showReport(await evaluate(code));
	} catch (failure) {
		setStatus(failure.message, true);
	}
	showing = false;
	if (queued !== undefined) {
		const next = queued;
		queued = undefined;
		show(next);
	}
}

// a link to the same page with the other compiler build
function showBuildSwitch() {
	const url = new URL(location.href);
	if (debugBuild) url.searchParams.delete(DEBUG_PARAMETER);
	else url.searchParams.set(DEBUG_PARAMETER, "");
	Object.assign($("build"), { href: url.href, textContent: debugBuild ? "debug build ⇄ optimized" : "optimized build ⇄ debug",
		title: debugBuild ? "warp.debug.wasm: Rust function names and lines in traces and the browser's debugger" : "warp.wasm, the small one" });
}

function downloadModule() {
	if (!lastModule) return setStatus("no module yet: a constant result needs none");
	const link = element("a", { href: URL.createObjectURL(new Blob([lastModule], { type: "application/wasm" })), download: "program.wasm" });
	link.click();
	URL.revokeObjectURL(link.href);
}

// ---- the editor ---------------------------------------------------------------------------------------------

let editor;

function runNow() {
	return show(editor.getValue());
}

const exampleSource = name => EXAMPLES[name]?.code ?? SAMPLES[name];

// shows the example or sample; resolves once its report is shown
function chooseExample(name) {
	const source = exampleSource(name);
	if (source === undefined) return;
	$("examples").value = name;
	editor.setValue(source);
	clearTimeout(typingTimer); // the change event's run would run it twice
	return runNow();
}

function fillExamples() {
	const group = (label, names) => element("optgroup", { label }, ...names.map(name => element("option", { value: name }, name)));
	$("examples").replaceChildren(group("tour", Object.keys(EXAMPLES)), group("samples/", Object.keys(SAMPLES).sort()));
	$("examples").onchange = event => chooseExample(event.target.value);
}

function initialize() {
	editor = CodeMirror.fromTextArea($("code"), {
		lineNumbers: true, mode: "wasp", indentWithTabs: true, tabSize: 4,
		extraKeys: { "Ctrl-Enter": runNow, "Cmd-Enter": runNow },
	});
	editor.on("change", () => {
		if (!$("auto").checked) return;
		clearTimeout(typingTimer);
		typingTimer = setTimeout(runNow, TYPING_DELAY_MS);
	});
	$("run").onclick = runNow;
	$("rendered").onclick = click => sendElementEvent("click", click, clickDetail(click));
	$("rendered").oninput = input => sendElementEvent("input", input, inputDetail(input.composedPath()[0]));
	$("output").onclick = click => sendPageEvent("click", clickDetail(click));
	$("output").onkeydown = key => sendPageEvent("key", { key: key.key });
	for (const event of ["pointermove", "pointerdown", "pointerup"]) $("output").addEventListener(event, trackPointer);
	document.addEventListener("pointerup", () => pointer && Atomics.store(pointer, POINTER_NAMES.indexOf("mouse_down"), 0));
	$("download").onclick = downloadModule;
	showBuildSwitch();
	fillExamples();
	startWorker();
	const requested = new URLSearchParams(location.search).get("example");
	chooseExample(requested && exampleSource(requested) !== undefined ? requested : DEFAULT_EXAMPLE);
}

// for the headless probes (probes/web_playground.py, test_in_browser.py --examples): evaluate code as the page does and return the report
window.playground = { evaluate, applyFix, chooseExample, code: () => editor.getValue(), setCode: source => editor.setValue(source), lastModule: () => lastModule, acknowledge: topic => saveAcknowledged([...acknowledged, topic]), forgetAll: () => saveAcknowledged([]) };

initialize();
