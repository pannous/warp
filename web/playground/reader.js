// What every script of a run reads first: a text of a module's memory, and the kinds of the trees host.js buildValue
// builds. The values of a module are read through its exports (values.js); the Rust-era reader of reflect_* exports
// (readNode) went with the last module that had them (card drop-readnode-branch).

const KIND_INT = "1";
const KIND_FLOAT = "2";
const KIND_MASK = 0xFFn;
const utf8Decoder = new TextDecoder("utf-8", { fatal: false });

function readText(module, pointer, length) {
	return utf8Decoder.decode(new Uint8Array(module.memory.buffer, pointer, length));
}
