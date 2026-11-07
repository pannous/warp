// The values of a module the self-hosted compiler made (core/compiler.warp), read and built by a host word in the page
// as core/native/driver.warp reads and builds them natively: through the module's exports, never its GC layout.
// Reading: shown_at (a value's text, packed as address · 2^32 + length in the module's memory), is_textual, item_count,
// item_at and int_item_at (1-based). Building: the constructors of core/emitter/host.warp host_builders (new_empty,
// new_int, new_bool, new_float, new_text, new_list, error_of, new_bytes …).

const SHOWN_TEXT_SHIFT = 2n ** 32n; // driver.warp TEXT_SHIFT
const SQUARE_BRACKET = 1n; // the bracket info of a [list] (core/node.warp Bracket: Curly 0, Square 1, Round 2)
const WASM_PAGE_BYTES = 65536;
const valueDecoder = new TextDecoder();
const valueEncoder = new TextEncoder();

// the value's text: a text's own, any other value as warp shows it; undefined for one past the module's room to show
// it in (runtime/show.warp shown_at's -1)
function shownText(program, value) {
	const packed = program.shown_at(value);
	return packed < 0n ? undefined : valueDecoder.decode(packedBytes(program, packed));
}
const packedBytes = (program, packed) => new Uint8Array(program.memory.buffer, Number(packed / SHOWN_TEXT_SHIFT), Number(packed % SHOWN_TEXT_SHIFT));
// a host word's text argument of any length: at once, else part by part through the output area (runtime/show.warp
// shown_from), each part copied before the next overwrites it, as the native driver's shown reads one
function guestText(program, value) {
	const text = shownText(program, value);
	if (text !== undefined) return text;
	const parts = [];
	for (let at = 0n, part; (part = packedBytes(program, program.shown_from(value, at))).length; at += BigInt(part.length)) parts.push(part.slice());
	const whole = new Uint8Array(parts.reduce((length, part) => length + part.length, 0));
	parts.reduce((at, part) => (whole.set(part, at), at + part.length), 0);
	return valueDecoder.decode(whole);
}
const isGuestText = (program, value) => program.is_textual(value) !== 0;
const guestCount = (program, list) => Number(program.item_count(list));
const guestItem = (program, list, index) => program.item_at(list, BigInt(index + 1));
const guestItems = (program, list) => Array.from({ length: guestCount(program, list) }, (_, index) => guestItem(program, list, index));
// the first `count` items of a list of Ints, as numbers
const guestInts = (program, list, count = guestCount(program, list)) =>
	Array.from({ length: count }, (_, index) => Number(program.int_item_at(list, BigInt(index + 1))));

// a map's entries [name, value], read item by item and never shown (a Key is two items, its name and its value:
// decision pair-items), so a map of a million numbers is read without its text; undefined for a value of no such
// entries
function guestEntries(program, value) {
	try {
		if (isGuestText(program, value)) return undefined;
		// the name a Symbol, shown as itself
		const named = entry => guestCount(program, entry) === 2 ? shownText(program, guestItem(program, entry, 0)) : undefined;
		const entries = guestItems(program, value).map(entry => named(entry) === undefined ? undefined : [named(entry), guestItem(program, entry, 1)]);
		return entries.length && entries.every(Boolean) ? entries : undefined;
	} catch {
		return undefined; // a number, or an item of no items
	}
}

// the value as plain JavaScript (a text a string, a list an array, a map an object, yes/no booleans, ø null, numbers,
// a ratio its quotient), read from the text warp shows it as: the one reading of any value, as the native warp hands
// shown values to its C libraries (lib/native/foreign.c); a list too long to show (a million numbers for the GPU) is
// read item by item
function guestPlain(program, value) {
	if (isGuestText(program, value)) return guestText(program, value);
	const shown = shownText(program, value);
	return shown === undefined ? guestItems(program, value).map(item => guestPlain(program, item)) : plainOfShown(shown);
}
const SHOWN_WORDS = { yes: true, no: false, "ø": null };
const SHOWN_NUMBER = /^-?(\d+(\.\d*)?|\.\d+)([eE][-+]?\d+)?(\/\d+)?/;
const SHOWN_NAME = /^[^\s,:{}\[\]()"']+/u;
function plainOfShown(shown) {
	let at = 0;
	const fail = () => { throw new Error(`a host word cannot read the value ${shown} (at ${at})`); };
	const skip = () => { while (/[\s,]/.test(shown[at] ?? "")) at++; };
	const quoted = quote => {
		let text = "";
		for (at++; shown[at] !== quote; at++) {
			if (at >= shown.length) fail();
			text += shown[at] === "\\" ? JSON.parse(`"\\${shown[++at]}"`) : shown[at];
		}
		at++;
		return text;
	};
	const items = (close, keyed) => {
		const entries = [];
		for (at++, skip(); shown[at] !== close; skip()) {
			if (at >= shown.length) fail();
			entries.push(item());
		}
		at++;
		const keys = entries.filter(entry => entry.key !== undefined);
		if (keyed || (keys.length && keys.length === entries.length)) return Object.fromEntries(entries.map(entry => [entry.key, entry.value]));
		return entries.map(entry => entry.key === undefined ? entry.value : { [entry.key]: entry.value });
	};
	const value = () => {
		const next = shown[at];
		if (next === "[" || next === "(") return items(next === "[" ? "]" : ")", false);
		if (next === "{") return items("}", true);
		if (next === '"' || next === "'") return quoted(next);
		const number = shown.slice(at).match(SHOWN_NUMBER);
		if (number) {
			at += number[0].length;
			const [numerator, denominator] = number[0].split("/");
			return denominator ? Number(numerator) / Number(denominator) : Number(numerator);
		}
		const name = shown.slice(at).match(SHOWN_NAME) ?? fail();
		at += name[0].length;
		return name[0] in SHOWN_WORDS ? SHOWN_WORDS[name[0]] : name[0];
	};
	// a list's item: a value, or `name: value`
	const item = () => {
		const start = at, first = value();
		skip();
		if (shown[at] !== ":" || typeof first !== "string" || shown[start] === '"') return { value: first };
		at++;
		skip();
		return { key: first, value: value() };
	};
	skip();
	const plain = value();
	skip();
	return at === shown.length ? plain : fail();
}

// a bytes value as a Uint8Array: a text's UTF-8, else its Int items, each taken modulo 256 (driver.warp c_bytes)
const guestBytes = (program, value) => isGuestText(program, value) ? valueEncoder.encode(guestText(program, value))
	: Uint8Array.from(guestInts(program, value), byte => byte & 0xFF);

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
// a text of the module's by its builder of an (address, length): new_text, new_symbol
function newGuestBytes(program, text, builder) {
	const bytes = valueEncoder.encode(text);
	return program[builder](placedBytes(program, bytes), bytes.length);
}
const newGuestText = (program, text) => newGuestBytes(program, text, "new_text");
const newGuestError = (program, message) => program.error_of(newGuestText(program, message));
// a text's NFC (decision nfc-slot, for text ==): JavaScript's own normalize, where the module builds texts (warp.wasm)
const canonicalText = (program, text) => newGuestText(program, guestText(program, text).normalize("NFC"));
// a [list] of the module's values, built from its last item on, a cell at a time (driver.warp code_list_built)
function newGuestList(program, items) {
	if (items.length === 0) return program.new_bytes(0, 0, SQUARE_BRACKET);
	return items.reduceRight((rest, item) => program.new_list(item, rest, SQUARE_BRACKET), null);
}
// a [list] of numbers, its cells laid in the module's memory and built in one call (new_floats, new_ints of
// core/emitter/host.warp): new_list a cell at a time copies the list built so far, a million items stalled the page
// (card list-cell-build)
const newGuestCells = (program, cells, builder) =>
	program[builder](placedBytes(program, new Uint8Array(cells.buffer, cells.byteOffset, cells.byteLength)), cells.length, SQUARE_BRACKET);
const newGuestFloats = (program, numbers) => newGuestCells(program, Float64Array.from(numbers), "new_floats");
const newGuestInts = (program, numbers) => newGuestCells(program, BigInt64Array.from(numbers, BigInt), "new_ints");
// a map of the module's: its entries `name: value`, in curly brackets
function newGuestMap(program, entries) {
	const keys = entries.map(([name, value]) => program.new_spelled_key(newGuestBytes(program, name, "new_symbol"), newGuestText(program, ":"), value));
	return program.new_spelled_list(newGuestList(program, keys), newGuestText(program, "{"), newGuestText(program, ","));
}
// a plain JavaScript value as the module's: ø, yes/no, Int (a whole number, also a foreign runtime's `{$int: digits}`),
// Float, a codepoint `{$char}`, text, list, map (an object); an object of the page without a plain form (a Date, a
// function) as its handle (host.js handleOf)
function newGuestValue(program, plain) {
	if (plain === null || plain === undefined) return program.new_empty();
	if (isCodepoint(plain)) return program.new_codepoint(plain.$char.codePointAt(0));
	if (typeof plain === "function" || (typeof plain === "object" && !ArrayBuffer.isView(plain) && !Array.isArray(plain) && !isPlainObject(plain))) return newGuestValue(program, handleOf(plain));
	if (typeof plain === "object" && Object.keys(plain).length === 1 && typeof plain.$int === "string") return program.new_int(BigInt(plain.$int));
	if (typeof plain === "boolean") return program.new_bool(plain ? 1n : 0n);
	if (typeof plain === "bigint") return program.new_int(plain);
	if (typeof plain === "number") return Number.isInteger(plain) ? program.new_int(BigInt(plain)) : program.new_float(plain);
	if (Array.isArray(plain) || ArrayBuffer.isView(plain)) return newGuestList(program, Array.from(plain, item => newGuestValue(program, item)));
	if (typeof plain === "object") return newGuestMap(program, Object.entries(plain).map(([name, value]) => [name, newGuestValue(program, value)]));
	return newGuestText(program, String(plain));
}
