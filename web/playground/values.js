// The values of a module the self-hosted compiler made (core/compiler.warp), read and built by a host word in the page
// as core/native/driver.warp reads and builds them natively: through the module's exports, never its GC layout.
// Reading: shown_at (a value's text, packed as address · 2^32 + length in the module's memory), is_textual, item_count,
// item_at and int_item_at (1-based). Building: the constructors of core/emitter/host.warp host_builders (new_empty,
// new_int, new_bool, new_float, new_text, new_list, error_of, new_bytes …). reader.js reads the Rust-era modules
// (reflect_* exports), which these have not.

// a module of the Rust-era compiler, which reader.js reads, has reflect_* exports: plainValue and builtValue serve both
// until the page compiles with the self-hosted compiler only (card wasm-playground-build), then they are these
const selfHosted = program => !program.reflect_value;
const plainValue = (program, value) => selfHosted(program) ? guestPlain(program, value) : plainOfTree(readNode(program, value));
const errorValue = (program, reason) => selfHosted(program) ? newGuestError(program, reason) : errorInProgram(program, reason);
const builtValue = (program, plain) => selfHosted(program) ? newGuestValue(program, plain) : buildValue(program, treeOfPlain(plain));

const SHOWN_TEXT_SHIFT = 2n ** 32n; // driver.warp TEXT_SHIFT
const SQUARE_BRACKET = 1n; // the bracket info of a [list] (core/node.warp Bracket: Curly 0, Square 1, Round 2)
const WASM_PAGE_BYTES = 65536;
const valueDecoder = new TextDecoder();
const valueEncoder = new TextEncoder();

// the value's text: a text's own, any other value as warp shows it
function guestText(program, value) {
	const packed = program.shown_at(value);
	if (packed < 0n) throw new Error("a host word's argument is too long to show");
	return valueDecoder.decode(new Uint8Array(program.memory.buffer, Number(packed / SHOWN_TEXT_SHIFT), Number(packed % SHOWN_TEXT_SHIFT)));
}
const isGuestText = (program, value) => program.is_textual(value) !== 0;
const guestCount = (program, list) => Number(program.item_count(list));
const guestItem = (program, list, index) => program.item_at(list, BigInt(index + 1));
const guestItems = (program, list) => Array.from({ length: guestCount(program, list) }, (_, index) => guestItem(program, list, index));
// the first `count` items of a list of Ints, as numbers
const guestInts = (program, list, count = guestCount(program, list)) =>
	Array.from({ length: count }, (_, index) => Number(program.int_item_at(list, BigInt(index + 1))));

// the value as plain JavaScript (a text a string, a list an array, a map an object, yes/no booleans, ø null, numbers,
// a ratio its quotient), read from the text warp shows it as: the one reading of any value, as the native warp hands
// shown values to its C libraries (lib/native/foreign.c)
const guestPlain = (program, value) => isGuestText(program, value) ? guestText(program, value) : plainOfShown(guestText(program, value));
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
// a [list] of the module's values, built from its last item on, a cell at a time (driver.warp code_list_built)
function newGuestList(program, items) {
	if (items.length === 0) return program.new_bytes(0, 0, SQUARE_BRACKET);
	return items.reduceRight((rest, item) => program.new_list(item, rest, SQUARE_BRACKET), null);
}
const newGuestFloats = (program, numbers) => newGuestList(program, Array.from(numbers, number => program.new_float(number)));
// a map of the module's: its entries `name: value`, in curly brackets
function newGuestMap(program, entries) {
	const keys = entries.map(([name, value]) => program.new_spelled_key(newGuestBytes(program, name, "new_symbol"), newGuestText(program, ":"), value));
	return program.new_spelled_list(newGuestList(program, keys), newGuestText(program, "{"), newGuestText(program, ","));
}
// a plain JavaScript value as the module's: ø, yes/no, Int (a whole number), Float, text, list, map (an object)
function newGuestValue(program, plain) {
	if (plain === null || plain === undefined) return program.new_empty();
	if (typeof plain === "boolean") return program.new_bool(plain ? 1n : 0n);
	if (typeof plain === "bigint") return program.new_int(plain);
	if (typeof plain === "number") return Number.isInteger(plain) ? program.new_int(BigInt(plain)) : program.new_float(plain);
	if (Array.isArray(plain) || ArrayBuffer.isView(plain)) return newGuestList(program, Array.from(plain, item => newGuestValue(program, item)));
	if (typeof plain === "object") return newGuestMap(program, Object.entries(plain).map(([name, value]) => [name, newGuestValue(program, value)]));
	return newGuestText(program, String(plain));
}
