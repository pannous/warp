// Reads a WASM GC result node into the JSON tree src/web.rs node_from_tree turns into a Node: JavaScript cannot read
// GC struct fields, so it calls the module's reflection exports (src/wasm_emitter/reflection.rs).
// A node is {kind, data, chain}: chain holds the nodes reached through value fields (a list's cons cells), kept flat.

// the order of PAYLOAD_TAGS in reflection.rs
const PAYLOAD_TAGS = ["null", "node", "int", "float", "string", "i31", "big_int", "ratio", "other"];
const KIND_INT = "1";
const KIND_FLOAT = "2";
const utf8Decoder = new TextDecoder("utf-8", { fatal: false });

function readText(module, pointer, length) {
	return utf8Decoder.decode(new Uint8Array(module.memory.buffer, pointer, length));
}

// JSON has no NaN, Infinity or -0: those travel as text
function floatPayload(value) {
	return Number.isFinite(value) && !Object.is(value, -0) ? value : String(Object.is(value, -0) ? "-0" : value);
}

function readPayload(module, payload) {
	switch (PAYLOAD_TAGS[module.reflect_tag(payload)]) {
		case "node": return { node: readNode(module, payload) };
		case "int": return { int: String(module.reflect_i64(payload)) };
		case "float": return { float: floatPayload(module.reflect_f64(payload)) };
		case "string": return { text: readText(module, module.reflect_text_ptr(payload), module.reflect_text_len(payload)) };
		case "i31": return { i31: module.reflect_i31(payload) };
		case "big_int": {
			const limbs = Array.from({ length: module.reflect_limb_count(payload) }, (_, index) => module.reflect_limb(payload, index) >>> 0);
			return { big: { negative: module.reflect_negative(payload) !== 0, limbs } };
		}
		case "ratio": return { ratio: [readPayload(module, module.reflect_numerator(payload)), readPayload(module, module.reflect_denominator(payload))] };
		default: return null;
	}
}

function readCell(module, node) {
	return { kind: String(module.get_kind(node)), data: readPayload(module, module.reflect_data(node)) };
}

function readNode(module, node) {
	const tree = readCell(module, node);
	tree.chain = [];
	for (let next = module.reflect_value(node); next !== null; next = module.reflect_value(next)) tree.chain.push(readCell(module, next));
	return tree;
}

// what main returned: a Node, or a plain number of a module that returns one
function readResult(module, result) {
	if (typeof result === "bigint") return { kind: KIND_INT, data: { int: String(result) } };
	if (typeof result === "number") return { kind: KIND_FLOAT, data: { float: floatPayload(result) } };
	if (result === null || result === undefined) return { kind: "0", data: null };
	return readNode(module, result);
}

// The {module, name, kind} of each import a module's bytes declare, read from its import section. Not
// WebAssembly.Module.imports: Safari 27 throws "unable to produce import descriptors" for any module importing a
// function with a GC reference (anyref, eqref) in its signature, as the task words do (card task-sample).
// Shared by the playground's host (host.js) and the uniscript page (web/uniscript/build.sh copies it)
const WASM_HEADER_BYTES = 8;
const IMPORT_SECTION = 2;
const IMPORT_KINDS = ["function", "table", "memory", "global", "tag"]; // the kinds of WebAssembly.Module.imports, by their byte
const REFERENCE_WITH_HEAP_TYPE = [0x63, 0x64]; // (ref null ht), (ref ht): a heap type follows
const LIMITS_HAVE_MAXIMUM = 1;
function importDescriptors(bytes) {
	const view = new Uint8Array(bytes);
	let at = WASM_HEADER_BYTES;
	const number = () => {
		let value = 0, shift = 0, byte;
		do { byte = view[at++]; value += (byte & 0x7F) * 2 ** shift; shift += 7; } while (byte & 0x80);
		return value;
	};
	const name = () => { const length = number(); return utf8Decoder.decode(view.subarray(at, at += length)); };
	const valueType = () => { if (REFERENCE_WITH_HEAP_TYPE.includes(view[at++])) number(); };
	const limits = () => { const flags = view[at++]; number(); if (flags & LIMITS_HAVE_MAXIMUM) number(); };
	const skipDescription = {
		function: number,
		table: () => { valueType(); limits(); },
		memory: limits,
		global: () => { valueType(); at++; },
		tag: () => { at++; number(); },
	};
	while (at < view.length) {
		const section = view[at++], size = number(), end = at + size;
		if (section !== IMPORT_SECTION) { at = end; continue; }
		return Array.from({ length: number() }, () => {
			const entry = { module: name(), name: name(), kind: IMPORT_KINDS[view[at++]] };
			skipDescription[entry.kind]();
			return entry;
		});
	}
	return [];
}
