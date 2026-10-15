// The page's side of a program Worker's task pool (host-tasks.js addTaskWorker): the page makes the task Workers, since in
// Firefox a Worker's own `new Worker` waits for the page's thread and, right after the browser started, that wait
// sometimes never ends (card tour-firefox-stall: 4 fresh starts in 80 stalled, probes/firefox_start/starts.py). The
// program's Worker asks {taskWorker: {id, url}} and gets {taskWorker: id, port}, a MessagePort to the new Worker;
// {endTaskWorker: id} stops one. The program's Worker, terminated, takes its task Workers with it as before. What a task
// Worker posts itself (its sounds) reaches the program Worker's listeners, as if that Worker had said it.

// call before setting programWorker.onmessage: these messages are answered here and reach no other listener
function serveTaskWorkers(programWorker) {
	const taskWorkers = new Map();
	programWorker.addEventListener("message", event => {
		const { taskWorker, endTaskWorker, releasePageObject } = event.data ?? {};
		if (taskWorker === undefined && endTaskWorker === undefined && releasePageObject === undefined) return;
		event.stopImmediatePropagation();
		if (releasePageObject) return releaseFrom(programWorker, releasePageObject);
		if (endTaskWorker !== undefined) {
			taskWorkers.get(endTaskWorker)?.terminate();
			return taskWorkers.delete(endTaskWorker);
		}
		const made = new Worker(taskWorker.url);
		const channel = new MessageChannel();
		made.postMessage({ port: channel.port1 }, [channel.port1]);
		programWorker.postMessage({ taskWorker: taskWorker.id, port: channel.port2 }, [channel.port2]);
		reportBareErrors(made, `task Worker ${taskWorker.id}`);
		// its sounds and MIDI (host-tasks.js addPageSoundWords) are heard as the program Worker's own messages
		made.onmessage = ({ data }) => programWorker.dispatchEvent(new MessageEvent("message", { data }));
		taskWorkers.set(taskWorker.id, made);
	});
	// a message of the page's own to one of them (playground.js sendPaintCanvas)
	// an unknown id throws: a dropped canvas would leave its program waiting for it forever (host-gpu.js arrivingCanvas)
	programWorker.postToTaskWorker = (id, message, transfer) => {
		const taskWorker = taskWorkers.get(id);
		if (!taskWorker) throw new Error(`no task worker ${id} to post ${Object.keys(message)} to`);
		taskWorker.postMessage(message, transfer);
	};
	reportBareErrors(programWorker, "the program's Worker");
	const terminate = programWorker.terminate.bind(programWorker);
	programWorker.terminate = () => {
		taskWorkers.forEach(made => made.terminate());
		forgetHolder(programWorker);
		terminate();
	};
	return programWorker;
}

// a failure without an Error (Firefox's bare "uncaught exception: undefined") at the page or one of its Workers is said
// with where it happened; a Worker's error event never carries the Error (errors do not cross threads), only its message
const reportBareErrors = (target, label) => target.addEventListener("error", event => {
	if (event.error instanceof Error) return;
	const what = target instanceof Worker ? "a Worker error" : `an error without an Error (${event.error})`;
	console.error(`warp ${label}: ${what} at ${event.filename}:${event.lineno}:${event.colno}: ${event.message}`);
});

// `clipboard` in a Worker (host-tasks.js askPage): the page's text into the Worker's shared buffer. A browser that reads
// the clipboard only right after the user acted (Firefox; Chrome when the page has no focus) refuses a run's read: a
// button then asks for it, and its click reads (card dir-clipboard)
const pasteInto = shared => answerInto(shared, async () => ({ text: await withPermission(CLIPBOARD_ASK, () => navigator.clipboard.readText()) }));
// a Worker's call of a global only the page has (host-foreign.js callOnPage, card browser-interaction): its value when
// plain, else the object kept here as {$page: id, type, text}, which the Worker's later calls name; the same object keeps
// its id, so reading `$who.value` again and again keeps one entry. Each Worker holding it counts how often it got the
// id; the object goes when every holder released it (its PageObjects collected, {releasePageObject: {id, received}})
// or was terminated (card page-pageobjects)
const pageObjects = new Map(); // id → {object, holders: Map(Worker → times given)}
const pageObjectIds = new Map(); // object → id
let lastPageObjectId = 0;
function onPage(value) {
	if (!value?.$page) return value;
	const kept = pageObjects.get(value.$page);
	if (!kept) throw new Error(`the page no longer keeps ${value.type} ${value.$page}, which its Worker released`);
	return kept.object;
}
function pageObjectId(value, holder) {
	const id = pageObjectIds.get(value) ?? ++lastPageObjectId;
	if (!pageObjects.has(id)) {
		pageObjectIds.set(value, id);
		pageObjects.set(id, { object: value, holders: new Map() });
	}
	const { holders } = pageObjects.get(id);
	holders.set(holder, (holders.get(holder) ?? 0) + 1);
	return id;
}
function dropHolder(id, holder, received = Infinity) {
	const kept = pageObjects.get(id);
	if (!kept) return;
	const left = (kept.holders.get(holder) ?? 0) - received;
	if (left > 0) return kept.holders.set(holder, left);
	kept.holders.delete(holder);
	if (kept.holders.size > 0) return;
	pageObjects.delete(id);
	pageObjectIds.delete(kept.object);
}
const releaseFrom = (holder, { id, received }) => dropHolder(id, holder, received);
const forgetHolder = holder => [...pageObjects.keys()].forEach(id => dropHolder(id, holder));
function forWorker(value, holder) {
	if (value === null || value === undefined || typeof value !== "object" && typeof value !== "function") return value ?? null;
	if (Array.isArray(value)) return value.map(item => forWorker(item, holder));
	return { $page: pageObjectId(value, holder), type: value.constructor?.name ?? typeof value, text: String(value).slice(0, 200) };
}
// `$who` is document.getElementById("who") (src/lowering/foreign/dollar_names.rs): the playground shows the program's
// markup in a shadow root, which it names as self.programElements, so its elements are found there
const ownerOnPage = (receiver, member) => receiver === "document" && member === "getElementById" ? self.programElements?.() ?? document
	: typeof receiver === "string" ? globalThis[receiver] : onPage(receiver);
// `holder`: the Worker asking, which keeps the objects it gets
const answerPageCall = (shared, { receiver, member, arguments: values }, holder) => answerInto(shared, async () => {
	const owner = ownerOnPage(receiver, member);
	if (owner === undefined) throw new Error(`the page has no global ${receiver}`);
	return { value: forWorker(jsMember(owner, receiver.$page ? `page ${receiver.type}` : receiver, member, values && values.map(onPage)), holder) }; // js-members.js
});

const CLIPBOARD_ASK = "📋 the program reads the clipboard: click to allow";
const PERMISSION_WAIT_MS = 60000; // host-tasks.js PAGE_ANSWER_MS: the Worker waits no longer
const ASKING_CLASS = "permission-ask";
const ASKING_STYLE = "position:fixed;bottom:1em;left:50%;transform:translateX(-50%);z-index:9;padding:.6em 1em;font:inherit;cursor:pointer";

// `use`'s answer; when the browser refuses it (NotAllowedError), a button the user clicks to let it run again
async function withPermission(ask, use) {
	try {
		return await use();
	} catch (failure) {
		if (failure?.name !== "NotAllowedError") throw failure;
		return afterClick(ask, use);
	}
}

function afterClick(ask, use) {
	document.querySelectorAll(`.${ASKING_CLASS}`).forEach(asking => asking.remove()); // an earlier run's question
	const button = Object.assign(document.createElement("button"), { className: ASKING_CLASS, textContent: ask });
	button.style.cssText = ASKING_STYLE;
	document.body.append(button);
	return new Promise((resolve, reject) => {
		const timeout = setTimeout(() => { button.remove(); reject(new Error(`not allowed: nobody clicked "${ask}"`)); }, PERMISSION_WAIT_MS);
		button.onclick = () => {
			clearTimeout(timeout);
			button.remove();
			use().then(resolve, reject);
		};
	});
}

// the page's answer to a waiting Worker, or {error}, as JSON in its shared buffer: host-tasks.js writeShared's layout
// ([state, length] as Int32, then the JSON), the state set last for the Worker waiting on it
async function answerInto(shared, answer) {
	const record = await answer().catch(failure => ({ error: failure.message ?? String(failure) }));
	const json = new TextEncoder().encode(JSON.stringify(record));
	const header = Int32Array.BYTES_PER_ELEMENT * 2;
	if (header + json.length > shared.byteLength) shared.grow(header + json.length);
	new Uint8Array(shared, header, json.length).set(json);
	const state = new Int32Array(shared, 0, 2);
	state[1] = json.length;
	Atomics.store(state, 0, 1);
	Atomics.notify(state, 0);
}
