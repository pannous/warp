// Runs uniscript.wasp (the uniscript package) compiled to WASM GC by warp (build.sh): the conversion happens in uniscript.wasm, this file is only
// the host: the "host" imports (src/host.rs), texts into and out of wasm memory, and the page.

const MODULE_URL = "uniscript.wasm";
const PRELOADED_FILES = ["packages/uniscript/data/entities.idx"]; // read by the module's init, fetched before it runs
const TEXT_HEAP_EXPORT = "text_heap";
const PAGE_BITS = 16;
const KIND_MASK = 0xffn;
const KIND_TEXT = 3n;
const KIND_ERROR = 11n;
const INPUT_DELAY_MS = 60;

const utf8 = new TextEncoder();
const decoder = new TextDecoder("utf-8", { fatal: false });
const preloaded = new Map();
let wasm; // the instance's exports
let heapMark = 0; // text_heap after the module's init: every conversion's texts are dropped by resetting to it
let warningList; // where host.warn reports: the list under the output being converted

const $ = id => document.getElementById(id);

function warn(message) {
	console.warn(message);
	if (!warningList) return $("status").append(` · ${message}`);
	const item = document.createElement("li");
	item.textContent = message;
	warningList.append(item);
}

// ---- texts in wasm memory ------------------------------------------------------------------------------------

// bytes into the module's memory from its text heap, the same rule as src/host.rs write_bytes_to_caller
function writeBytes(bytes) {
	const heap = wasm[TEXT_HEAP_EXPORT];
	const memoryEnd = wasm.memory.buffer.byteLength;
	let pointer = heap.value;
	if (pointer === 0 || pointer + bytes.length > memoryEnd) {
		wasm.memory.grow((bytes.length >> PAGE_BITS) + 1);
		pointer = memoryEnd;
	}
	new Uint8Array(wasm.memory.buffer, pointer, bytes.length).set(bytes);
	heap.value = pointer + bytes.length;
	return [pointer, bytes.length];
}

const readString = (pointer, length) => decoder.decode(new Uint8Array(wasm.memory.buffer, pointer, length));

const newText = text => wasm.new_text(...writeBytes(utf8.encode(text)));

// a result node as {kind, text}: texts and errors carry a $String, anything else is shown by its kind
function nodeResult(node) {
	const kind = wasm.get_kind(node) & KIND_MASK;
	const text = readString(wasm.get_text_ptr(node), wasm.get_text_len(node));
	return { kind, text };
}

// ---- host imports --------------------------------------------------------------------------------------------

// a synchronous GET (host calls are synchronous); bytes kept as they are via the x-user-defined charset
function getSync(url) {
	const request = new XMLHttpRequest();
	request.open("GET", url, false);
	request.overrideMimeType("text/plain; charset=x-user-defined");
	request.send();
	if (request.status >= 400) throw new Error(`HTTP ${request.status}`);
	return Uint8Array.from(request.responseText, character => character.charCodeAt(0) & 0xff);
}

// the body of a host call written back as (pointer, length), or (pointer, -length) of the failure reason
function hostResult(action, what) {
	try {
		return writeBytes(action());
	} catch (reason) {
		const message = `${what} failed: ${reason.message ?? reason}`;
		warn(message);
		const [pointer, length] = writeBytes(utf8.encode(message));
		return [pointer, -length];
	}
}

function readFile(pointer, length) {
	const path = readString(pointer, length);
	return hostResult(() => preloaded.get(path) ?? getSync(path), `read ${path}`);
}

function fetchUrl(pointer, length) {
	const url = readString(pointer, length);
	return hostResult(() => getSync(url), `fetch ${url}`);
}

const hostFunctions = {
	read: readFile,
	fetch: fetchUrl,
	fetch_within: (pointer, length, _timeout) => fetchUrl(pointer, length),
	run: () => {
		warn("host.run is not available in the browser");
		return -1n;
	},
	warn: (pointer, length) => warn(readString(pointer, length)),
};

// every import the module declares (reader.js importDescriptors): the host functions above, anything else a stub that warns when called
function importsOf(bytes) {
	const imports = {};
	for (const { module: space, name, kind } of importDescriptors(bytes)) {
		if (kind !== "function") throw new Error(`unsupported import ${space}.${name} (${kind})`);
		const known = space === "host" && hostFunctions[name];
		(imports[space] ??= {})[name] = known || ((...args) => {
			warn(`${space}.${name} is not available in the browser`);
			return 0;
		});
	}
	return imports;
}

async function preload(path) {
	const response = await fetch(path);
	if (!response.ok) throw new Error(`${path}: HTTP ${response.status}`);
	preloaded.set(path, new Uint8Array(await response.arrayBuffer()));
}

async function load() {
	const [bytes] = await Promise.all([fetch(MODULE_URL).then(response => response.arrayBuffer()), ...PRELOADED_FILES.map(preload)]);
	const { instance } = await WebAssembly.instantiate(bytes, importsOf(bytes));
	wasm = instance.exports;
	const init = nodeResult(wasm.main());
	if (init.kind === KIND_ERROR) throw new Error(init.text);
	heapMark = wasm[TEXT_HEAP_EXPORT].value;
	preloaded.clear();
}

// ---- conversion ----------------------------------------------------------------------------------------------

function convert(functionName, text) {
	try {
		return nodeResult(wasm[functionName](newText(text)));
	} catch (trap) {
		return { kind: KIND_ERROR, text: `${functionName} trapped: ${trap.message}` };
	} finally {
		wasm[TEXT_HEAP_EXPORT].value = heapMark;
	}
}

function show(output, { kind, text }) {
	output.textContent = text;
	output.classList.toggle("error", kind === KIND_ERROR);
	if (kind !== KIND_ERROR && kind !== KIND_TEXT) output.textContent = `(node of kind ${kind}) ${text}`;
}

const codepoints = text => [...text].map(c => "U+" + c.codePointAt(0).toString(16).toUpperCase().padStart(4, "0")).join(" ");

function update(inputId, functionName, outputId) {
	const output = $(outputId);
	warningList = $(outputId + "-warnings");
	warningList.replaceChildren();
	const result = convert(functionName, $(inputId).value);
	show(output, result);
	$(outputId + "-codes").textContent = result.kind === KIND_ERROR ? "" : codepoints(result.text);
	return result;
}

function debounced(action) {
	let timer;
	return () => {
		clearTimeout(timer);
		timer = setTimeout(action, INPUT_DELAY_MS);
	};
}

const forward = () => update("source", "uniscript", "unicode");
const reverse = () => update("reverse-source", "unicode_to_uniscript", "reverse-uniscript");

function setSource(text) {
	$("source").value = text;
	forward();
}

async function start() {
	try {
		await load();
	} catch (reason) {
		$("status").textContent = `could not load the module: ${reason.message}`;
		$("status").classList.add("error");
		return;
	}
	$("status").textContent = "ready";
	$("source").addEventListener("input", debounced(forward));
	$("reverse-source").addEventListener("input", debounced(reverse));
	for (const example of document.querySelectorAll("[data-example]")) {
		example.addEventListener("click", () => setSource(example.dataset.example));
	}
	$("to-reverse").addEventListener("click", () => {
		$("reverse-source").value = $("unicode").textContent;
		reverse();
	});
	forward();
	reverse();
	document.body.dataset.ready = "true";
}

start();
