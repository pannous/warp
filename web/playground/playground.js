// The page: the editor, the worker that compiles and runs (worker.js), and the report shown like the CLI prints it:
// the value, printed output, errors, warnings, hints. An ambiguity is a warning naming its explicit form; "got it"
// silences a warning's topic in this browser (it never changes the value) until "show again". Each reading the user
// might have meant is an "I meant: …" button that rewrites the code at the warning and runs it again (notes/fixits.md).

const ACKNOWLEDGED_KEY = "warp-playground-acknowledged";
const HINTS_KEY = "warp-playground-hints"; // the hints checkbox, on by default (card hints-toggle)
const SIZES_KEY = "warp-playground-sizes"; // the guide's width and the editor's height as the resizers left them
const GUIDE_WIDTH_RANGE = [200, 0.6]; // pixels, then the share of the page's width
const EDITOR_HEIGHT_RANGE = [120, 0.85]; // pixels, then the share of the window's height
const RESIZE_STEP = 20; // pixels per arrow key on a focused resizer
const OLD_ANSWERS_KEY = "warp-playground-answers"; // the Ask era kept {topic: form, "ack:<topic>": "acknowledged"}
const RUN_TIMEOUT_MS = 10000;
const LOADING_STALL_MS = 30000; // the compiler's download shows a note when it made no progress this long
// the origin a link of the shown program resolves against: "/about" is a page of the program, "https://…" is not
const PROGRAM_ORIGIN = "http://program.invalid";
const TYPING_DELAY_MS = 300;
const PENDING_VALUE = "…"; // the value shown from Run until the new one arrives
const DEFAULT_EXAMPLE = "hello";
const COMMIT_URL = "https://github.com/pannous/warp/commit/";
const SHORT_COMMIT = 9;
// terminal codes in printed text (`print "\e[H"`, samples/game_of_life.warp): clearing the screen or moving the cursor
// home starts the text anew, the other codes (colors, cursor moves) are dropped
const TERMINAL_RESTART = /\x1b\[(?:2J|H|1;1H)/g;
const TERMINAL_CODE = /\x1b\[[0-9;?]*[A-Za-z]/g;
const EXAMPLE_PARAMETERS = ["example", "sample"]; // ?example=fizzbuzz (or #fizzbuzz) picks a tour example or sample; the address shows the chosen one as #fizzbuzz
const DEBUG_PARAMETER = "debug"; // ?debug runs warp.debug.wasm: Rust names and lines in traces and the debugger
const DEBUG_COMPILER = "warp.debug.wasm";
const SLOW_START_PARAMETER = "slow_start"; // ?slow_start=<ms>: the worker reports ready that much later (a slow machine)
// a starting worker silent this long is stuck (card firefox-hello-hang, three of six deploys): started again once, loudly
const STALLED_START_MS = 60_000;
const ACKNOWLEDGED = "acknowledged";
const ACKNOWLEDGED_PREFIX = "ack:";
const STDERR = 2;
const STDOUT = 1;
const NOTIFICATION_TITLE = "warp";
// a page event, or one element's (`on click·1`, src/lowering/element_events.rs)
const PAGE_EVENT = /^on ((?:click|key|input)(?:·\d+)?)$/;
const DARK_MODE_QUERY = "(prefers-color-scheme: dark)";
// the gray levels of paint: a nonzero pixel, a zero pixel; from PAINT_COLOR_FROM on a value is a color 0xAARRGGBB
// (src/paint.rs shade, lib/draw.warp)
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
let workerStage = "not started"; // the worker's last stage (worker.js stage), for playground.state
let workerSettled = "starting"; // workerReady's outcome
let workerWarmed; // the worker was asked to compile the markup renderer ahead of the first markup run (worker.js warmUp)
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

// ---- the resizers: the guide's width and the editor's height, remembered in this browser -------------------

let sizes = (() => { try { return JSON.parse(localStorage.getItem(SIZES_KEY)) ?? {}; } catch { return {}; } })();
const clamp = (value, [least, share], whole) => Math.round(Math.min(Math.max(value, least), whole * share));

function applySizes() {
	const main = document.querySelector("main");
	if (sizes.guide) main.style.setProperty("--guide-width", `${clamp(sizes.guide, GUIDE_WIDTH_RANGE, main.clientWidth)}px`);
	else main.style.removeProperty("--guide-width");
	editor.getWrapperElement().style.height = sizes.editor ? `${clamp(sizes.editor, EDITOR_HEIGHT_RANGE, innerHeight)}px` : "";
	editor.refresh();
}

function setSize(name, value) {
	if (value === undefined) delete sizes[name];
	else sizes[name] = Math.round(value);
	applySizes();
	try { localStorage.setItem(SIZES_KEY, JSON.stringify(sizes)); } catch { /* private window: lasts for this page */ }
}

// drag the handle (or press its arrow keys) to set a size; sizeAt(pointer event) is the size there, current() the size now
function dragToResize(handle, name, sizeAt, current, [lessKey, moreKey]) {
	handle.addEventListener("pointerdown", down => {
		down.preventDefault();
		handle.setPointerCapture(down.pointerId);
		handle.classList.add("dragging");
		const move = event => setSize(name, sizeAt(event));
		handle.addEventListener("pointermove", move);
		handle.addEventListener("pointerup", () => { handle.removeEventListener("pointermove", move); handle.classList.remove("dragging"); }, { once: true });
	});
	handle.addEventListener("keydown", key => {
		const step = { [lessKey]: -RESIZE_STEP, [moreKey]: RESIZE_STEP }[key.key];
		if (step) { key.preventDefault(); setSize(name, current() + step); }
	});
	handle.ondblclick = () => setSize(name, undefined);
}

function startResizers() {
	const main = document.querySelector("main"), editorPane = document.querySelector(".editor-pane");
	dragToResize($("guide-resizer"), "guide", event => event.clientX - main.getBoundingClientRect().left, () => $("guide").offsetWidth, ["ArrowLeft", "ArrowRight"]);
	dragToResize($("editor-resizer"), "editor", event => event.clientY - editorPane.getBoundingClientRect().top, () => editorPane.offsetHeight, ["ArrowUp", "ArrowDown"]);
	addEventListener("resize", applySizes);
	applySizes();
}

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

// a starting worker silent for STALLED_START_MS: started again once (a warning), then its runs fail naming its stage
function restartStalledWorker(stalledWorker, restarts, resolve, reject) {
	const stall = `the compiler's worker gave no sign for ${STALLED_START_MS / 1000} s at stage "${workerStage}"`;
	if (restarts > 0) return reject(new Error(stall));
	console.warn(`${stall}: starting it again`);
	stalledWorker.terminate();
	startWorker(restarts + 1);
	resolve(workerReady);
}

function startWorker(restarts = 0) {
	workerWarmed = false;
	const options = new URLSearchParams();
	if (debugBuild) options.set("compiler", DEBUG_COMPILER);
	// test_in_browser.py --examples: a worker that starts as late as on a slow CI runner
	const slowStart = new URLSearchParams(location.search).get(SLOW_START_PARAMETER);
	if (slowStart) options.set(SLOW_START_PARAMETER, slowStart);
	worker = new Worker(options.size ? `worker.js?${options}` : "worker.js");
	showLoading("starting the compiler's worker", false);
	workerReady = new Promise((resolve, reject) => {
		const starting = worker;
		let stalled;
		// each message of the starting worker shows it alive: the stall timer starts over
		const watchStart = () => {
			clearTimeout(stalled);
			stalled = setTimeout(() => restartStalledWorker(starting, restarts, resolve, reject), STALLED_START_MS);
		};
		const settled = outcome => value => {
			clearTimeout(stalled);
			stalled = undefined;
			outcome(value);
		};
		const [ready, failed] = [settled(resolve), settled(reject)];
		watchStart();
		// worker.js or a script it imports failed to load: without this the first run waits for "ready" forever (card firefox-hello-hang)
		worker.onerror = event => failed(new Error(`the compiler's worker failed to start: ${event.message || "worker.js did not load"}`));
		worker.onmessage = ({ data }) => {
			if (data.type === "notify") data = notification(data.text);
			if (!data) return;
			if (stalled) watchStart();
			if (data.type === "stage") return workerStage = data.stage;
			if (data.type === "loading") return showLoading(`loading the compiler: ${megabytes(data.loaded)}${data.total ? ` of ${megabytes(data.total)}` : ""} MB`);
			if (data.type === "compiling") return stopLoading();
			if (data.type === "ready") return ready();
			if (data.type === "stored") return keepValue(data.name, data.value, data.file);
			if (data.type === "clipboard") return copyText(data.text);
			if (data.type === "failed") return failed(new Error(data.message));
			if (data.type === "bundle") return bundled(data.bundle); // deploy.js
			if (!pending) return showEventOutput(data);
			if (data.type === "listening") Object.assign(pending, { listening: data.events, address: data.address });
			if (data.type === "print") printedChunk(pending, data);
			if (data.type === "sleep") pending.frame++;
			if (data.type === "tasks inline") pending.tasksInline = data.reason;
			if (data.type === "paint") painted(pending, data);
			if (data.type === "module") lastModule = data.bytes;
			if (data.type === "report" && data.id === pending.id) finish(pending, { ...data.report, printed: pending.printed, paintings: pending.paintings, listening: pending.listening ?? [], address: pending.address, milliseconds: data.milliseconds });
		};
	});
	// a slow start (the CI runner) finishes after the first run began: "ready" then must not hide its "running…"
	workerReady.then(() => stopLoading() || showing || setStatus("ready"), failure => stopLoading() || setStatus(failure.message, true));
	workerSettled = "starting";
	workerReady.then(() => workerSettled = "ready", failure => workerSettled = `failed: ${failure.message}`);
	tellSystemValues();
	sharePointer();
	worker.postMessage({ stored: keptValues(), session: keptValues([SESSION_STORE]) });
	tellEnvironment();
}

// the compiler's download (worker.js downloaded), beside the status; without progress for LOADING_STALL_MS it says so,
// so a stalled download is seen instead of a page that waits (card firefox-hello-hang)
let loadingStall;
function showLoading(progress, visible = true) {
	$("loading").textContent = progress;
	$("loading").hidden = !visible;
	clearTimeout(loadingStall);
	loadingStall = setTimeout(() => {
		$("loading").textContent = `${progress}; no progress for ${LOADING_STALL_MS / 1000} s: a slow or stalled connection`;
		$("loading").hidden = false;
	}, LOADING_STALL_MS);
}

function stopLoading() {
	clearTimeout(loadingStall);
	$("loading").hidden = true;
}

const megabytes = bytes => (bytes / 1e6).toFixed(1);

// the program's environment (assistant.js: the stand-in of the API key), sent again when the key changes
const tellEnvironment = () => worker.postMessage(programEnvironment());

// `notify "text"`: the browser's notification once the page may show them; until then (or when refused) a printed
// line, so the text never goes missing; the first one asks for the permission
function notification(text) {
	if (globalThis.Notification?.permission === "granted") return void new Notification(NOTIFICATION_TITLE, { body: text });
	globalThis.Notification?.requestPermission?.();
	return { type: "print", text: `notification: ${text}\n`, stream: STDOUT };
}

// the system values a Worker cannot read itself (host.js system_value), sent again when they change
const darkMode = matchMedia(DARK_MODE_QUERY);
const tellSystemValues = () => worker.postMessage({ system: { "dark mode": darkMode.matches } });
darkMode.addEventListener("change", tellSystemValues);

// `loop { …; show(); sleep(16) }`: each sleep starts an animation's next frame, what the run paints or prints in it
// replaces the last frame's; frames keep the run alive past RUN_TIMEOUT_MS, the next run stops it (cards
// drawing-frames, snake-frames)
function startsFrame(run, output) {
	const starts = run.frame > 0 && run.framesShown[output] !== run.frame;
	run.framesShown[output] = run.frame;
	if (starts) {
		run.animating = true;
		stopAfterTimeout(run);
	}
	return starts;
}

// each painting shows at once, while the program still runs: one the size of the last replaces it, as a frame (card
// g_oldM: `if frame % 6 == 0 { render() }` animates without a sleep, as the native window does)
function painted(run, painting) {
	const last = run.paintings.at(-1);
	if (startsFrame(run, "paintings")) run.paintings = [];
	else if (last && sameSize(last, painting)) run.paintings.pop();
	run.paintings.push(painting);
	showFrame(run.paintings);
}

// a text frame (`render(); sleep(.1s)`) arrives line by line: the last one is shown whole once the next one begins
function printedChunk(run, chunk) {
	if (startsFrame(run, "printed")) {
		showPrinted(run.printed);
		run.printed = [];
	}
	run.printed.push(chunk);
}

function stopAfterTimeout(run) {
	clearTimeout(run.timer);
	run.timer = setTimeout(() => stopRun(run, `stopped after ${RUN_TIMEOUT_MS / 1000} s: ${run.tasksInline ?? "the program may not terminate"}`), RUN_TIMEOUT_MS);
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
		const run = { id: ++nextRunId, resolve, printed: [], paintings: [], frame: 0, framesShown: {} };
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

const POSITION = /^(\d+):(\d+)$/;

// the editor's cursor at the 1-based line and column, in view
function jumpTo(line, column) {
	editor.focus();
	editor.setCursor({ line: line - 1, ch: column - 1 });
	editor.scrollIntoView(null, 40);
}

// a line:column goes there when clicked
function positionLink(position) {
	const [, line, column] = position.match(POSITION) ?? [];
	if (!line) return element("span", { className: "position" }, position);
	return element("button", { className: "position", title: "go there", onclick: () => jumpTo(Number(line), Number(column)) }, position);
}

function diagnostic(kind, position, ...content) {
	return element("li", { className: kind }, element("span", { className: "label" }, kind), position ? positionLink(position) : "", ...content);
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

// the » value line; markup the page renders (#rendered) is shown there only, its text would repeat it (card hide-html)
function showValue(value, error, html) {
	$("value").textContent = value;
	$("value").classList.toggle("error", Boolean(error));
	$("value").parentElement.hidden = Boolean(html) && !error;
}

function showReport(report) {
	showValue(report.value, report.error, report.html);
	const errorAt = report.error ? report.error_at : null; // the failing place: clicking the error goes there
	$("value").classList.toggle("located", Boolean(errorAt));
	$("value").title = errorAt ? `go to ${at(errorAt.line, errorAt.column)}` : "";
	$("value").onclick = errorAt ? () => jumpTo(errorAt.line, errorAt.column) : null;
	showPrinted(report.printed);
	showRendered(report.html);
	showPaintings(report.paintings ?? []);
	listenTo(report.listening ?? []);
	showAddress(report.address);
	const hints = $("hints").checked ? report.hints : [];
	const notes = report.notes ?? [];
	const inline = new Set((report.warnings ?? []).map(warning => warning.topic).filter(topic => notes.includes(topic)));
	const expressionOf = topic => (report.got_it ?? []).find(offer => offer.topic === topic)?.expression;
	const shown = (kind, problem, extra = "") => diagnostic(kind, at(problem.line, problem.column), problem.message,
		problem.fix ? element("span", { className: "fix" }, "fix: ", code(problem.fix)) : "", fixButtons(problem.fixes), extra);
	const items = [
		...(report.errors ?? []).map(error => shown("error", error)),
		...report.warnings.map(warning => shown("warning", warning, inline.has(warning.topic) ? gotIt(warning.topic, warning.expression_key, warning.line) : "")),
		...report.runtime_warnings.map(message => diagnostic("warning", "runtime", message)),
		...hints.map(hint => diagnostic("hint", hint.position, "prefer ", code(hint.canonical), " over ", code(hint.original),
			element("span", { className: "reason" }, hint.reason), fixButtons(hint.fixes))),
		...notes.filter(topic => !inline.has(topic) && $("hints").checked).map(topic => diagnostic("note", "", `the ${topic} note above shows until you say `, gotIt(topic, expressionOf(topic)))),
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

// a markup value as DOM (lib/markup.warp, card web-dom), in a shadow root so its own style cannot restyle the page.
// Markup shown anew after a handler changes only the text nodes and attributes that differ (card web-fine): the
// elements stay, with their focus, input and scroll state.
function showRendered(html) {
	const host = $("rendered");
	host.hidden = !html;
	const template = document.createElement("template");
	template.innerHTML = html ?? "";
	morphChildren(host.shadowRoot ?? renderedRoot(host), template.content);
}

// a form's submit stays inside the shadow root (it is not composed): listened to there
function renderedRoot(host) {
	const root = host.attachShadow({ mode: "open" });
	root.addEventListener("submit", submitForm);
	return root;
}

// a form of the shown markup asks the program's own route (lowering/serve.rs page·submitted), as `warp serve` would:
// its fields are the body of a POST, the query of a GET (src/web_server.rs request_node), not a request leaving here
function submitForm(submitted) {
	submitted.preventDefault();
	const form = submitted.target;
	const method = (form.getAttribute("method") ?? "get").toUpperCase();
	const fields = Object.fromEntries(new FormData(form, submitted.submitter));
	const path = new URL(form.getAttribute("action") ?? "", PROGRAM_ORIGIN + ($("address").value || "/")).pathname;
	const posted = method !== "GET";
	worker.postMessage({ submit: { method, path, query: posted ? {} : fields, body: posted ? fields : null } });
}

function showPaintings(paintings) {
	$("paintings").replaceChildren(...paintings.map(painting => {
		const canvas = element("canvas", { width: painting.width, height: painting.height, className: "painting" });
		canvas.style.width = `${painting.width * Math.max(1, Math.floor(PAINT_SHOWN_SIDE / Math.max(painting.width, painting.height, 1)))}px`;
		return drawn(canvas, painting);
	}));
}

const sameSize = (one, other) => one.width === other.width && one.height === other.height;

// the run's paintings, its last one drawn into the canvas shown for it, which keeps the pointer over it
function showFrame(paintings) {
	const shown = $("paintings").querySelectorAll("canvas");
	const canvas = shown[shown.length - 1];
	const painting = paintings.at(-1);
	if (shown.length === paintings.length && sameSize(canvas, painting)) drawn(canvas, painting);
	else showPaintings(paintings);
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

// a click on a link inside the shown markup goes to that page of the program (lowering/routes.rs), not away from here
function followLink(click) {
	const link = click.composedPath().find(element => element.matches?.("a[href]"));
	if (!link || click.metaKey || click.ctrlKey || click.shiftKey) return false;
	const url = new URL(link.getAttribute("href"), PROGRAM_ORIGIN);
	if (url.origin !== PROGRAM_ORIGIN) return false;
	click.preventDefault();
	worker.postMessage({ navigate: url.pathname });
	return true;
}

// the address bar shows the path of a program with routes (worker.js addressOf); a path typed there goes to that page
function showAddress(path) {
	$("address").hidden = path === undefined;
	$("address").value = path ?? "";
}

function goToAddress(key) {
	if (key.key !== "Enter") return;
	worker.postMessage({ navigate: new URL($("address").value, PROGRAM_ORIGIN).pathname });
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

function terminalText(text) {
	const restarts = [...text.matchAll(TERMINAL_RESTART)];
	const last = restarts.at(-1);
	return (last ? text.slice(last.index + last[0].length) : text).replace(TERMINAL_CODE, "");
}

function showPrinted(chunks) {
	const printed = terminalText(chunks.map(chunk => chunk.stream === STDERR ? "" : chunk.text).join(""));
	$("printed").textContent = printed;
	$("printed").hidden = printed === "";
}

// what a handler printed, painted and gave, while no run is pending
function showEventOutput(data) {
	if (data.type === "print" && data.stream !== STDERR) {
		$("printed").textContent = terminalText($("printed").textContent + data.text);
		$("printed").hidden = false;
	}
	if (data.type === "paint") showPaintings([data]);
	if (data.type === "address") showAddress(data.path);
	if (data.type !== "handled") return;
	if (data.value !== undefined) showValue(data.value, data.error, data.html);
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
		return;
	}
	warmWorker();
}

// once per worker, after a run is shown: the page is idle, the worker free
function warmWorker() {
	if (workerWarmed) return;
	workerWarmed = true;
	worker.postMessage({ warm: true });
}

// a link to the same page with the other compiler build
function showBuildSwitch() {
	const url = new URL(location.href);
	if (debugBuild) url.searchParams.delete(DEBUG_PARAMETER);
	else url.searchParams.set(DEBUG_PARAMETER, "");
	Object.assign($("build"), { href: url.href, textContent: debugBuild ? "debug build ⇄ optimized" : "optimized build ⇄ debug",
		title: debugBuild ? "warp.debug.wasm: Rust function names and lines in traces and the browser's debugger" : "warp.wasm, the small one" });
}

// the commit the page was built from and the build time in the viewer's time zone (build.sh version.js), linked to
// the commit on GitHub
function showVersion() {
	if (!PLAYGROUND_VERSION) return;
	const { commit, date, built } = PLAYGROUND_VERSION;
	const when = built ? new Date(built).toLocaleString([], { dateStyle: "short", timeStyle: "short" }) : date;
	Object.assign($("version"), { href: `${COMMIT_URL}${commit}`, textContent: `version ${commit.slice(0, SHORT_COMMIT)} · built ${when}`,
		title: `committed ${date}`, hidden: false });
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

// Run pressed (the button, Ctrl/Cmd-Enter): the old value gives way to "…" at once, and the old errors, warnings,
// underlines and printed text go, so none is taken for the new run's (card g_oc54)
function runPressed() {
	showValue(PENDING_VALUE);
	$("value").classList.remove("located");
	$("value").onclick = null;
	$("diagnostics").replaceChildren();
	markPositions({});
	showPrinted([]);
	return runNow();
}

const exampleSource = name => EXAMPLES[name]?.code ?? SAMPLES[name];

// puts the code in the editor and runs it once; resolves once its report is shown
function runCode(source) {
	editor.setValue(source);
	clearTimeout(typingTimer); // the change event's run would run it twice
	return runNow();
}

// shows the example or sample; resolves once its report is shown
function chooseExample(name) {
	const source = exampleSource(name);
	if (source === undefined) return;
	$("examples").value = name;
	showExampleInAddress(name);
	return runCode(source);
}

// the address names the chosen example as its hash (#circle), so it can be shared or reloaded; the default one leaves
// it plain, and a guide chapter's hash (#guide-lists) stays (card sample-hash)
function showExampleInAddress(name) {
	const address = new URL(location.href);
	EXAMPLE_PARAMETERS.forEach(parameter => address.searchParams.delete(parameter));
	if (name !== DEFAULT_EXAMPLE) address.hash = name;
	else if (exampleSource(hashName()) !== undefined) address.hash = "";
	if (address.href !== location.href) history.replaceState(history.state, "", address);
}

const hashName = () => decodeURIComponent(location.hash.slice(1));

// the example the address names: ?example=circle (?sample=circle), else #circle
function requestedExample() {
	const parameters = new URLSearchParams(location.search);
	return [...EXAMPLE_PARAMETERS.map(parameter => parameters.get(parameter)), hashName()].find(name => name && exampleSource(name) !== undefined);
}

// #circle typed into the address shows that example
function chooseExampleOfHash() {
	const name = hashName();
	if (exampleSource(name) !== undefined && name !== $("examples").value) chooseExample(name);
}

function fillExamples() {
	const group = (label, names) => element("optgroup", { label }, ...names.map(name => element("option", { value: name }, name)));
	$("examples").replaceChildren(group("tour", Object.keys(EXAMPLES)), group("samples/", Object.keys(SAMPLES).sort()));
	$("examples").onchange = event => chooseExample(event.target.value);
}

function initialize() {
	editor = CodeMirror.fromTextArea($("code"), {
		// fixedGutter moves the gutter on every scroll, which Firefox warns about; wrapped lines never scroll sideways
		lineNumbers: true, lineWrapping: true, fixedGutter: false, mode: "warp", indentWithTabs: true, tabSize: 4,
		extraKeys: SHORTCUTS, // shortcuts.js
	});
	editor.on("change", () => {
		if (!$("auto").checked) return;
		clearTimeout(typingTimer);
		typingTimer = setTimeout(runNow, TYPING_DELAY_MS);
	});
	$("run").onclick = runPressed;
	try { $("hints").checked = localStorage.getItem(HINTS_KEY) !== "off"; } catch { /* private window: on */ }
	$("hints").onchange = () => {
		try { localStorage.setItem(HINTS_KEY, $("hints").checked ? "on" : "off"); } catch { /* private window: lasts for this page */ }
		runNow();
	};
	$("address").onkeydown = goToAddress;
	$("rendered").onclick = click => followLink(click) || sendElementEvent("click", click, clickDetail(click));
	$("rendered").oninput = input => sendElementEvent("input", input, inputDetail(input.composedPath()[0]));
	$("output").onclick = click => sendPageEvent("click", clickDetail(click));
	$("output").onkeydown = key => sendPageEvent("key", { key: key.key });
	for (const event of ["pointermove", "pointerdown", "pointerup"]) $("output").addEventListener(event, trackPointer);
	document.addEventListener("pointerup", () => pointer && Atomics.store(pointer, POINTER_NAMES.indexOf("mouse_down"), 0));
	$("download").onclick = downloadModule;
	startResizers();
	showBuildSwitch();
	showVersion();
	fillExamples();
	startWorker();
	startAssistant(editor, tellEnvironment);
	startCompletion(editor);
	chooseExample(requestedExample() ?? DEFAULT_EXAMPLE);
	addEventListener("hashchange", chooseExampleOfHash);
}

// for the headless probes (probes/web_playground.py, test_in_browser.py --examples): evaluate code as the page does and return the report
window.playground = { evaluate, applyFix, chooseExample, runCode, code: () => editor.getValue(), setCode: source => editor.setValue(source), lastModule: () => lastModule, acknowledge: topic => saveAcknowledged([...acknowledged, topic]), forgetAll: () => saveAcknowledged([]),
	// what a run waits for, for the Firefox driver's timeout report (card firefox-hello-again)
	state: () => ({ worker: workerStage, ready: workerSettled, pending: pending?.id, queued: queued !== undefined, showing }) };

(window.pageStarts ?? Promise.resolve()).then(initialize); // index.html: not before its reload for isolation
