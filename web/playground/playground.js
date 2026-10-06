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
// the gray levels of paint: a nonzero pixel, a zero pixel
const PAINT_INK = 29;
const PAINT_PAPER = 250;

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
			if (data.type === "failed") return reject(new Error(data.message));
			if (!pending) return;
			if (data.type === "print") pending.printed.push(data);
			if (data.type === "paint") pending.paintings.push(data);
			if (data.type === "module") lastModule = data.bytes;
			if (data.type === "report" && data.id === pending.id) finish(pending, { ...data.report, printed: pending.printed, paintings: pending.paintings, milliseconds: data.milliseconds });
		};
	});
	workerReady.then(() => setStatus("ready"), failure => setStatus(failure.message, true));
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
		run.timer = setTimeout(() => {
			worker.terminate(); // a program that does not stop blocks the worker: replace it
			startWorker();
			finish(run, { value: `stopped after ${RUN_TIMEOUT_MS / 1000} s: the program may not terminate`, error: true, printed: run.printed,
				warnings: [], runtime_warnings: [], hints: [], notes: [] });
		}, RUN_TIMEOUT_MS);
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
	showPaintings(report.paintings ?? []);
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
	showAcknowledged();
	setStatus(report.crashed ? "the compiler crashed; reloaded" : `${Math.round(report.milliseconds ?? 0)} ms`, report.crashed);
}

// paint(pixels, width, height): one canvas per call, a pixel dark where its value is nonzero (true), light where 0
function showPaintings(paintings) {
	$("paintings").replaceChildren(...paintings.map(({ pixels, width, height }) => {
		const canvas = element("canvas", { width, height, className: "painting" });
		const image = canvas.getContext("2d").createImageData(width, height);
		for (let index = 0; index < width * height; index++) {
			const shade = pixels[index] ? PAINT_INK : PAINT_PAPER;
			image.data.set([shade, shade, shade, 255], index * 4);
		}
		canvas.getContext("2d").putImageData(image, 0, 0);
		return canvas;
	}));
}

async function show(code) {
	if (showing) {
		queued = code; // only the newest code is worth running next
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
	show(editor.getValue());
}

function chooseExample(name) {
	const source = EXAMPLES[name] ?? SAMPLES[name];
	if (source === undefined) return;
	$("examples").value = name;
	editor.setValue(source);
	runNow();
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
	$("download").onclick = downloadModule;
	showBuildSwitch();
	fillExamples();
	startWorker();
	const requested = new URLSearchParams(location.search).get("example");
	chooseExample(requested && (EXAMPLES[requested] ?? SAMPLES[requested]) !== undefined ? requested : DEFAULT_EXAMPLE);
}

// for the headless probe (probes/web_playground.sh): evaluate code as the page does and return the report
window.playground = { evaluate, applyFix, code: () => editor.getValue(), setCode: source => editor.setValue(source), lastModule: () => lastModule, acknowledge: topic => saveAcknowledged([...acknowledged, topic]), forgetAll: () => saveAcknowledged([]) };

initialize();
