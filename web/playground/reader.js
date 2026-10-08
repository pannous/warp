// Reads a WASM GC result node into the JSON tree src/web.rs node_from_tree turns into a Node: JavaScript cannot read
// GC struct fields, so it calls the module's reflection exports (src/wasm_emitter/reflection.rs).
// A node is {kind, data, chain}: chain holds the nodes reached through value fields (a list's cons cells), kept flat.

// the order of PAYLOAD_TAGS in reflection.rs
const PAYLOAD_TAGS = ["null", "node", "int", "float", "string", "i31", "big_int", "ratio", "floats", "other"];
const KIND_INT = "1";
const KIND_FLOAT = "2";
const KIND_MASK = 0xFFn;
const KIND_FUNCTION = 16n; // src/type_kinds.rs Kind::Function: a closure
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
		case "floats": return { floats: Array.from({ length: module.reflect_float_count(payload) }, (_, index) => floatPayload(module.reflect_float(payload, index))) };
		default: return null;
	}
}

// a closure keeps its node, out of JSON, so foreign code can call it (host.js callableOf)
function readCell(module, node) {
	const cell = { kind: String(module.get_kind(node)), data: readPayload(module, module.reflect_data(node)) };
	if ((BigInt(cell.kind) & KIND_MASK) === KIND_FUNCTION) Object.defineProperty(cell, "warpFunction", { value: { module, node } });
	return cell;
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
