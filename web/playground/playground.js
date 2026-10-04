// The page: the editor, the worker that compiles and runs (worker.js), and the report shown like the CLI prints it:
// the value, printed output, errors, warnings, hints. An ambiguity is a warning naming its explicit form; "got it"
// silences a warning's topic in this browser (it never changes the value) until "show again".

const ACKNOWLEDGED_KEY = "warp-playground-acknowledged";
const OLD_ANSWERS_KEY = "warp-playground-answers"; // the Ask era kept {topic: form, "ack:<topic>": "acknowledged"}
const RUN_TIMEOUT_MS = 10000;
const TYPING_DELAY_MS = 300;
const DEFAULT_EXAMPLE = "welcome";
const ACKNOWLEDGED = "acknowledged";
const ACKNOWLEDGED_PREFIX = "ack:";
const STDERR = 2;

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
let queued; // code waiting for the running evaluation to finish
let typingTimer;
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
	worker = new Worker("worker.js");
	workerReady = new Promise((resolve, reject) => {
		worker.onmessage = ({ data }) => {
			if (data.type === "ready") return resolve();
			if (data.type === "failed") return reject(new Error(data.message));
			if (!pending) return;
			if (data.type === "print") pending.printed.push(data);
			if (data.type === "module") lastModule = data.bytes;
			if (data.type === "report" && data.id === pending.id) finish({ ...data.report, printed: pending.printed, milliseconds: data.milliseconds });
		};
	});
	workerReady.then(() => setStatus("ready"), failure => setStatus(failure.message, true));
}

function finish(report) {
	clearTimeout(pending.timer);
	const { resolve } = pending;
	pending = undefined;
	resolve(report);
	if (queued !== undefined) {
		const code = queued;
		queued = undefined;
		show(code);
	}
}

// evaluate code in the worker: the report of src/web.rs evaluate plus `printed` [{text, stream}]
async function evaluate(code) {
	await workerReady;
	return new Promise(resolve => {
		const id = ++nextRunId;
		const timer = setTimeout(() => {
			worker.terminate(); // a program that does not stop blocks the worker: replace it
			const printed = pending.printed;
			startWorker();
			finish({ value: `stopped after ${RUN_TIMEOUT_MS / 1000} s: the program may not terminate`, error: true, printed, warnings: [], runtime_warnings: [], hints: [], notes: [] });
		}, RUN_TIMEOUT_MS);
		pending = { id, resolve, printed: [], timer };
		worker.postMessage({ id, code, acknowledged: acknowledgements() });
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
	const items = [
		...report.warnings.map(warning => diagnostic("warning", at(warning.line, warning.column), warning.message,
			warning.fix ? element("span", { className: "fix" }, "fix: ", code(warning.fix)) : "")),
		...report.runtime_warnings.map(message => diagnostic("warning", "runtime", message)),
		...report.hints.map(hint => diagnostic("hint", hint.position, "prefer ", code(hint.canonical), " over ", code(hint.original),
			element("span", { className: "reason" }, hint.reason))),
		...report.notes.map(topic => diagnostic("note", "", `the ${topic} warning above shows until you `,
			element("button", { onclick: () => acknowledge(topic) }, "got it"))),
	];
	$("diagnostics").replaceChildren(...items);
	showAcknowledged();
	setStatus(report.crashed ? "the compiler crashed; reloaded" : `${Math.round(report.milliseconds ?? 0)} ms`, report.crashed);
}

async function show(code) {
	if (pending) {
		queued = code;
		return;
	}
	setStatus("running…");
	showReport(await evaluate(code));
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
	fillExamples();
	startWorker();
	const requested = new URLSearchParams(location.search).get("example");
	chooseExample(requested && (EXAMPLES[requested] ?? SAMPLES[requested]) !== undefined ? requested : DEFAULT_EXAMPLE);
}

// for the headless probe (probes/web_playground.sh): evaluate code as the page does and return the report
window.playground = { evaluate, lastModule: () => lastModule, acknowledge: topic => saveAcknowledged([...acknowledged, topic]), forgetAll: () => saveAcknowledged([]) };

initialize();
