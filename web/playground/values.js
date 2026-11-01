// The values of a module the self-hosted compiler made (core/compiler.warp), read and built by a host word in the page
// as core/native/driver.warp reads and builds them natively: through the module's exports, never its GC layout.
// Reading: shown_at (a value's text, packed as address · 2^32 + length in the module's memory), is_textual, item_count,
// item_at and int_item_at (1-based). Building: the constructors of core/emitter/host.warp host_builders (new_empty,
// new_int, new_bool, new_float, new_text, new_list, error_of, new_bytes …). reader.js reads the Rust-era modules
// (reflect_* exports), which these have not.

const TEXT_SHIFT = 2n ** 32n; // driver.warp TEXT_SHIFT
const SQUARE_LIST = 1n; // the bracket info of a [list] (core/node.warp Bracket: Curly 0, Square 1, Round 2)
const WASM_PAGE_BYTES = 65536;
const valueDecoder = new TextDecoder();
const valueEncoder = new TextEncoder();

// the value's text: a text's own, any other value as warp shows it
function guestText(program, value) {
	const packed = program.shown_at(value);
	if (packed < 0n) throw new Error("a host word's argument is too long to show");
	return valueDecoder.decode(new Uint8Array(program.memory.buffer, Number(packed / TEXT_SHIFT), Number(packed % TEXT_SHIFT)));
}
const isGuestText = (program, value) => program.is_textual(value) !== 0;
const guestCount = (program, list) => Number(program.item_count(list));
const guestItem = (program, list, index) => program.item_at(list, BigInt(index + 1));
const guestItems = (program, list) => Array.from({ length: guestCount(program, list) }, (_, index) => guestItem(program, list, index));
// the first `count` items of a list of Ints, as numbers
const guestInts = (program, list, count = guestCount(program, list)) =>
	Array.from({ length: count }, (_, index) => Number(program.int_item_at(list, BigInt(index + 1))));

// the bytes placed in the module's memory, where its next text goes: above its text heap when it exports one, else at
// the end of its memory, grown for them (driver.warp guest_bytes_built); their address
function placedBytes(program, bytes) {
	const heap = program.text_heap;
	let address = heap ? Number(heap.value) : 0;
	if (address === 0 || address + bytes.length > program.memory.buffer.byteLength) {
		address = program.memory.buffer.byteLength;
		program.memory.grow(Math.floor(bytes.length / WASM_PAGE_BYTES) + 1);
	}
	new Uint8Array(program.memory.buffer, address, bytes.length).set(bytes);
	if (heap) heap.value = address + bytes.length;
	return address;
}
function newGuestText(program, text) {
	const bytes = valueEncoder.encode(text);
	return program.new_text(placedBytes(program, bytes), bytes.length);
}
const newGuestError = (program, message) => program.error_of(newGuestText(program, message));
// a [list] of the module's values, built from its last item on, a cell at a time (driver.warp code_list_built)
function newGuestList(program, items) {
	if (items.length === 0) return program.new_bytes(0, 0, SQUARE_LIST);
	return items.reduceRight((rest, item) => program.new_list(item, rest, SQUARE_LIST), null);
}
// a plain JavaScript value as the module's: ø, yes/no, Int, Float, text, list
function newGuestValue(program, plain) {
	if (plain === null || plain === undefined) return program.new_empty();
	if (typeof plain === "boolean") return program.new_bool(plain ? 1n : 0n);
	if (typeof plain === "bigint") return program.new_int(plain);
	if (typeof plain === "number") return Number.isInteger(plain) ? program.new_int(BigInt(plain)) : program.new_float(plain);
	if (Array.isArray(plain) || ArrayBuffer.isView(plain)) return newGuestList(program, Array.from(plain, item => newGuestValue(program, item)));
	return newGuestText(program, String(plain));
}
