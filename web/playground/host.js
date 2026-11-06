// The host words of a program warp compiled, run in the page (driver.js programRan): its imports mirror what the native
// warp answers (core/native/driver.warp, WASI, libm). Shared by the playground (worker.js) and built sites
// (site-worker.js); needs reader.js. What only some programs reach is in the parts (HOST_PART_FILES, below).

const TEXT_HEAP_EXPORT = "text_heap";
const TRAP_DETAIL_EXPORT = "trap_detail";
// a program with a `try` exports it: the runtime error that raises the Error left in trap_detail (runtime_errors.rs)
const RETURNED_ERROR_EXPORT = "returned_error";
// the position of the statement a trapped run was in, line << 12 | column (src/wasm_emitter/globals.rs emit_trap_position)
const TRAP_POSITION_EXPORT = "trap_position";
const SHARED_CHECK_MILLISECONDS = 10; // how often a busy sleep looks at its check points
const PAGE_BITS = 16;
const STDERR = 2;
// display.dpi where the page told none (a Worker without its page, the browser tests): CSS fixes 96 px to the inch, as
// warp-runtime system_values.rs DEFAULT_DPI does where the screen tells none
const DEFAULT_DPI = 96;
const DARK_MODE_QUERY = "(prefers-color-scheme: dark)";
// view_width, view_height on a page that tells no output pane (playground.js tellSystemValues), as natively
// (crates/warp-runtime/src/system_values.rs NATIVE_VIEW)
// window_open: no page told it shows the pictures (playground.js tellSystemValues), so `while window_open` never starts
const VIEW_DEFAULTS = { view_width: 640, view_height: 480, window_open: 0, gpu_drawing: 0 };
// window.width, window.height (card make-width): the frame the paintings show in, in CSS pixels; the view's size until the
// run sets it (set_system_value), which the page then gives its paintings (hooks.resizeWindow, canvas.js)
const WINDOW_SIDES = { window_width: "view_width", window_height: "view_height" };
const DISPLAY_READ_ONLY = "a program cannot resize the display: set window.width or canvas.width instead";
const toldValue = name => self.pageSystemValues?.[name] ?? VIEW_DEFAULTS[name];
// the standard library's adapters (src/modules/std_adapters.rs, notes/stdlib.md section 7): module → member → function of
// plain values (plainOfTree / treeOfPlain, as for foreign_call); the parts add them (host-hashes.js, host-files.js)
const STD_ADAPTERS = {};

// the stored values of `stored x = v` and `local[k]`, by name: the page's localStorage as the worker started
// (playground.js); those of `session[k]`, its sessionStorage
const storedValues = {};
const sessionValues = {};
const contentText = content => typeof content === "string" ? content : JSON.stringify(content);
const utf8 = new TextEncoder();

// the system values of the pointer the page shares; without shared memory a running program cannot see it move (the
// run blocks its Worker's messages), so it would wait for a finger forever (card abysmally-slow: the in-app browser
// view of an iPad app painted nothing). A key held reads none there, as in the test suite's page
const POINTER_VALUES = ["mouse_x", "mouse_y", "mouse_down", "mouse_wheel"];
const NO_SHARED_POINTER = "this page has no shared memory (it is not cross-origin isolated, as in an app's built-in browser view), so a running program cannot follow the pointer: open the page in Safari, Chrome or Firefox";
// the pointer the page shares (canvas.js pointerMessage) as the worker keeps it: self.pagePointer, read by system_value
const sharedPointer = ({ buffer, names }) => ({ values: new Int32Array(buffer), names });

// copy bytes into the program's memory from its text heap, the rule of src/host.rs write_bytes_to_caller
function writeBytes(program, bytes) {
	const heap = program[TEXT_HEAP_EXPORT];
	const memoryEnd = program.memory.buffer.byteLength;
	let pointer = heap ? heap.value : 0;
	if (pointer === 0 || pointer + bytes.length > memoryEnd) {
		program.memory.grow((bytes.length >> PAGE_BITS) + 1);
		pointer = memoryEnd;
	}
	new Uint8Array(program.memory.buffer, pointer, bytes.length).set(bytes);
	if (heap) heap.value = pointer + bytes.length;
	return [pointer, bytes.length];
}

// the Error of `reason`, built with the program's own error_of: a failure its `try` catches (src/host.rs error_in_program)
function errorInProgram(program, reason) {
	return program.error_of(program.new_text(...writeBytes(program, utf8.encode(reason))));
}

// The parts of the host a program reaches only through some of its imports, each a file that adds itself here:
// host-files.js, host-sound.js (needs host-files.js), host-hashes.js, host-tasks.js, host-foreign.js (needs host-files.js and js-members.js), host-sqlite.js (needs host-foreign.js and wasi.js), host-compiler.js, host-routes.js,
// host-gpu.js (needs host-media.js), host-timers.js, host-random.js, host-media.js. A built site
// ships a part only when its module imports one of the part's words or calls one of its std modules (src/host/host_parts.rs HOST_PARTS); the playground's workers
// load them all. A part gives any of: words(holder, hooks, access) the host words it adds, access being {program, text,
// cString} of programImports; imports(holder, hooks) import modules of their own (m, c); importModule(holder, hooks,
// name) an import module by its name (a .wasm path), else undefined; adapters, std modules for STD_ADAPTERS;
// systemValue(name, holder) a system value it knows, else undefined; started(run) as a run begins; poll(holder) at each check point (sleep, signal_poll); finished(holder, hooks) after a
// call into the run returned, a failure nobody read or nothing; ended(holder) after the call failed; stopped(holder)
// when the page drops the run (stopListening); navigated(holder) when the page goes to another path (navigate)
// values.js first: not a part, the reader and builders of self-hosted modules' values the parts' words use
const HOST_PART_FILES = ["values.js", "host-files.js", "host-sound.js", "host-hashes.js", "host-tasks.js", "host-foreign.js", "host-sqlite.js", "host-compiler.js", "host-routes.js", "host-gpu.js", "host-timers.js", "host-random.js", "host-media.js"];
const hostParts = [];
function addHostPart(part) {
	hostParts.push(part);
	Object.assign(STD_ADAPTERS, part.adapters);
}
const eachHostPart = (step, ...values) => hostParts.map(part => part[step]?.(...values));

// hooks: print(text, fd), module(bytes) (each compiled module), panicked(message) (the compiler's)
function programImports(holder, hooks) {
	const program = () => holder.exports;
	// the host word sleep_ms (src/host.rs): a page cannot block, so it busy-waits
	holder.sleep = milliseconds => {
		hooks.sleeping?.(Number(milliseconds)); // the next frame of an animation, and no runaway time (playground.js)
		const until = Date.now() + Number(milliseconds);
		let check = Date.now() + SHARED_CHECK_MILLISECONDS;
		while (Date.now() < until) {
			if (Date.now() >= check) {
				checkPoint();
				check = Date.now() + SHARED_CHECK_MILLISECONDS;
			}
		}
		checkPoint();
	};
	const text = (pointer, length) => readText(program(), pointer, length);
	const cString = pointer => {
		const bytes = new Uint8Array(program().memory.buffer);
		let end = pointer;
		while (bytes[end] !== 0 && end < bytes.length) end++;
		return bytes.slice(pointer, end);
	};
	const access = { program, text, cString };
	const windowSide = side => holder.windowSize?.[side] ?? toldValue(WINDOW_SIDES[side]);
	const checkPoint = () => eachHostPart("poll", holder);
	const known = {
		host: {
			warn: (pointer, length) => holder.warnings.push(text(pointer, length)),
			run: () => -1n,
			// `try f(args) else Y` (src/host.rs guarded_call): f's node wrapper called in this instance; the engine's stack
			// overflow (a RangeError) is the Error "call stack exhausted", any other failure (a raised error) goes on up
			guarded_call: (name, values) => {
				const module = program();
				try {
					const result = module[decode(cString(name))](values);
					if (typeof result === "bigint") return module.new_int(result);
					return typeof result === "number" ? module.new_float(result) : result;
				} catch (failure) {
					if (!(failure instanceof RangeError)) throw failure;
					return errorInProgram(module, "call stack exhausted");
				}
			},
			// `serve 8080 {…}` (src/web/web_server.rs): a page cannot listen on a port
			serve_routes: port => { throw new Error(`serve ${port}: a server runs only in the native host (the warp CLI)`); },
			// the host words (src/host.rs)
			sleep_ms: holder.sleep,
			clock: () => BigInt(Date.now()),
			// seconds east of UTC in the browser's time zone at that instant: an instant's wall clock (wasm_emitter/times.rs)
			local_offset: milliseconds => BigInt(-new Date(Number(milliseconds)).getTimezoneOffset() * 60),
			// a page has no ctrl-c: `on interrupt {…}` never runs here (notes/system_signals.md); shared listeners do
			signal_poll: checkPoint,
			signal_watch: () => { holder.warnings.push("on file … change: a page has no files to watch"); },
			// system values (crates/warp-runtime/src/system_values.rs): what the browser tells, a loud error for the rest
			system_value: name => {
				const value = decode(cString(name));
				if (value === "online") return BigInt(navigator.onLine);
				if (value === "time of day") { const now = new Date(); return BigInt(now - new Date(now).setHours(0, 0, 0, 0)); }
				if (POINTER_VALUES.includes(value) && !self.pagePointer) throw new Error(`${value}: ${NO_SHARED_POINTER}`);
				// key_down: whether the page's held key (canvas.js trackKey) is any; "key code held" which (held_key names it)
				const heldKey = self.pagePointer ? Atomics.load(self.pagePointer.values, self.pagePointer.names.indexOf("key")) : 0;
				if (value === "key_down") return BigInt(heldKey !== 0);
				const partValue = eachHostPart("systemValue", value, holder).find(known => known !== undefined); // "gpu_paints": host-gpu.js
				if (partValue !== undefined) return BigInt(partValue);
				if (value === "key code held") return BigInt(heldKey);
				if (value in WINDOW_SIDES) return BigInt(windowSide(value));
				if (value === "display_dpi") return BigInt(holder.displayDpi ?? toldValue(value) ?? Math.round((globalThis.devicePixelRatio ?? 1) * DEFAULT_DPI));
				const pointerIndex = self.pagePointer?.names.indexOf(value) ?? -1; // canvas.js trackPointer
				if (pointerIndex >= 0) return BigInt(Atomics.load(self.pagePointer.values, pointerIndex));
				// a Worker has no matchMedia: the page tells it (playground.js tellSystemValues)
				const known = value === "dark mode" && globalThis.matchMedia ? matchMedia(DARK_MODE_QUERY).matches : toldValue(value);
				if (known !== undefined) return BigInt(known);
				throw new Error(`${value}: the playground cannot read it yet`);
			},
			// key_held(code) (crates/warp-runtime host_words.rs): whether that key is among those held (canvas.js trackKey),
			// code 0 whether none is
			key_held: code => {
				const names = self.pagePointer?.names ?? [];
				const read = name => Atomics.load(self.pagePointer.values, names.indexOf(name));
				const held = names.filter(name => name === "key" || name.startsWith("held ")).map(read);
				return BigInt(code === 0n ? held.every(heldCode => heldCode === 0) : held.includes(Number(code)));
			},
			// window.width = 800: the paintings' frame (crates/warp-runtime/src/system_values.rs write); display.dpi = 300 dpi
			// the dots per inch px count in; the display's sides read only
			set_system_value: (name, value) => {
				const side = decode(cString(name));
				// lib/frames.warp asks for a native window; the page's canvas is the window already
				if (side === "window_open") return;
				// window.fullscreen = yes: the paintings' frame fills the screen (playground.js fullScreen), read back as the
				// page tells it; until it does, as asked
				if (side === "window_fullscreen") {
					(self.pageSystemValues ??= {}).window_fullscreen = Number(value !== 0n);
					return hooks.fullScreen?.(value !== 0n);
				}
				if (side === "display_dpi") {
					if (value <= 0n) throw new Error(`display.dpi = ${value} dpi: a display has a positive number of dots per inch`);
					holder.displayDpi = Number(value);
					return;
				}
				if (!(side in WINDOW_SIDES)) throw new Error(side.startsWith("display") ? DISPLAY_READ_ONLY : `${side} cannot be set`);
				if (value <= 0n) throw new Error(`a window side of ${value} cannot show: its sides must be positive`);
				holder.windowSize = { window_width: windowSide("window_width"), window_height: windowSide("window_height"), [side]: Number(value) };
				hooks.resizeWindow?.(holder.windowSize.window_width, holder.windowSize.window_height);
			},
			// `notify "text"` (src/host.rs notify): the page shows it (playground.js notification)
			notify: text => {
				const shown = plainOfTree(readNode(program(), text));
				if (!hooks.notify) throw new Error(`notify ${JSON.stringify(shown)}: this page shows no notifications`);
				hooks.notify(typeof shown === "string" ? shown : JSON.stringify(shown));
			},
			// an emit no handler of a page's program receives (card page-custom-event): a Worker posts it to its page
			// (hooks.dispatch), a page's own thread dispatches it (markup.js dispatchToPage)
			dispatch_event: (name, detail) => {
				const dispatch = hooks.dispatch ?? globalThis.dispatchToPage;
				const event = { name: plainOfTree(readNode(program(), name)), detail: plainOfTree(readNode(program(), detail)) };
				if (!dispatch) throw new Error(`emit ${event.name}: this host has no page to dispatch it to`);
				dispatch(event);
			},
			// `exit(code)` ends the run, its value ø (P121): runProgram tells it from a failure by holder.exitCode
			exit: code => {
				holder.exitCode = Number(code);
				throw new Error(`exit(${code})`);
			},
			...Object.assign({}, ...eachHostPart("words", holder, hooks, access)),
		},
		wasi_snapshot_preview1: {
			fd_write: (fd, vectors, count, written) => {
				const memory = new DataView(program().memory.buffer);
				let total = 0;
				for (let index = 0; index < count; index++) {
					const pointer = memory.getUint32(vectors + 8 * index, true);
					const length = memory.getUint32(vectors + 8 * index + 4, true);
					hooks.print(text(pointer, length), fd);
					total += length;
				}
				memory.setUint32(written, total, true);
				return 0;
			},
		},
		...Object.assign({}, ...eachHostPart("imports", holder, hooks)),
	};
	// anything else (native FFI libraries) is missing in the browser: say which, when the program calls it
	const missing = (module, name) => () => { throw new Error(`${module}.${name} is not available in the browser`); };
	return new Proxy(known, {
		get: (modules, module) => new Proxy(modules[module] ?? eachHostPart("importModule", holder, hooks, module).find(Boolean) ?? {}, {
			get: (functions, name) => functions[name] ?? missing(module, String(name)),
		}),
	});
}

const decode = bytes => utf8Decoder.decode(bytes);

const textTree = text => ({ kind: "3", data: { text }, chain: [] });
const KIND_LIST = 8n;
// the inverse of reader.js readNode for the values a task carries (src/host/tasks.rs TaskValue): built through the module's
// constructors; a list from its cons cells, the first item in data, the next cells in the chain
function buildValue(module, tree) {
	// where it was written (src/web/mod.rs tree_of): kept by node_at, which only a module quoting or parsing code has
	if (tree.at !== undefined) {
		const { at: [line, column, comment], ...bare } = tree;
		const node = buildValue(module, bare);
		if (!module.node_at) return node;
		const text = comment === null ? null : buildValue(module, { kind: "3", data: { text: comment }, chain: [] });
		return module.node_at(node, BigInt(line), BigInt(column), text);
	}
	if (tree.closure !== undefined) {
		const name = buildValue(module, { kind: "5", data: { text: tree.closure }, chain: [] });
		return module.closure_rebuild(name, tree.captured === null ? null : buildValue(module, tree.captured));
	}
	if (tree.key) return module.new_key(buildValue(module, tree.key[0]), buildValue(module, tree.key[1]), BigInt(tree.kind) >> 8n);
	if (tree.type) return module.new_type(buildValue(module, tree.type[0]), buildValue(module, tree.type[1]));
	if (tree.items) return tree.items.reduceRight((rest, item) => module.new_list(buildValue(module, item), rest, BigInt(tree.kind) >> 8n), null) ?? module.new_empty();
	const kind = BigInt(tree.kind);
	const payload = tree.data ?? {};
	switch (Number(kind & KIND_MASK)) {
		case 0: return module.new_empty();
		case 1: {
			// a module without bools (no new_bool) takes a truth as its Int 1/0, as src/host/tasks.rs does
			if (payload.int !== undefined && String(kind) === BOOL_KIND && module.new_bool) return module.new_bool(BigInt(payload.int));
			if (payload.int !== undefined) return module.new_int(BigInt(payload.int));
			// beyond the fixnums: exact.js starts the parts that carry such numbers (tasks, foreign calls, run blocks)
			const exact = typeof exactOfPayload === "function" ? exactOfPayload(payload) : undefined;
			if (exact) return module.new_int(exactHandle(module, ...exact));
			throw new Error(`an Int payload of no known form (or exact.js not loaded): ${JSON.stringify(payload)}`);
		}
		case 2: return module.new_float(Number(payload.float));
		case 3: case 5: {
			const [pointer, length] = writeBytes(module, utf8.encode(payload.text));
			return Number(kind & KIND_MASK) === 3 ? module.new_text(pointer, length) : module.new_symbol(pointer, length);
		}
		case 4: return module.new_codepoint(payload.i31);
		case 6: { // a key (an instance among them): its right side the first cell of the chain, which holds that side's own chain
			const [right, ...rest] = tree.chain;
			return module.new_key(buildValue(module, payload.node), right ? buildValue(module, { ...right, chain: rest }) : module.new_empty(), kind >> 8n);
		}
		case 7: case 8: {
			const items = [payload.node, ...tree.chain.map(cell => cell.data?.node)].filter(Boolean);
			return items.reduceRight((rest, item) => module.new_list(buildValue(module, item), rest, kind >> 8n), null) ?? module.new_empty();
		}
		default: throw new Error(`a value of kind ${kind & KIND_MASK} cannot cross to another task yet`);
	}
}

// src/operators/codes.rs OP_CODES as written (Op::as_str), by the code a key carries above the kind bits
const OPERATOR_SPELLINGS = ["", ":", "=", ":=", ".", "?.", "::", "->", "+", "-", "*", "/", "%", "rem", "^", "<<", ">>", "+=", "-=", "*=", "/=", "%=", "^=", "&&=", "||=", "^^=", "<", ">", "<=", ">=", "==", "!=", "≈", "and", "or", "xor", "not", "-", "√", "∛", "‖", "++", "--", "?", "if", "then", "else", "while", "do", "#", "..", "to", "as", "±", "??", "===", "!==", "~", "⌞", "⌟", "&", "|", "↑", "↓", "/!", "/!=", "is"];
// the keys that are entries of an object; the others are operations
const ENTRY_OPERATORS = ["", ":", "=", ":="];
const ROUND_BRACKET = 2n; // src/wasm_emitter bracket info: Curly 0, Square 1, Round 2
// a value of the program (a reader.js tree) as a plain JavaScript value, for foreign_call: lists arrays, `{a:1}` objects
const KIND_KEY = 6n;
function plainOfTree(tree) {
	const kind = BigInt(tree.kind);
	const payload = tree.data ?? {};
	// a ø item reads as a null node: it stays null ([1, ø] is [1, null]); only an empty list has no first node
	const items = () => payload.node === undefined ? [] : [payload.node, ...(tree.chain ?? []).map(cell => cell.data?.node ?? null)];
	const plainOfItem = item => item === null ? null : plainOfTree(item);
	switch (Number(kind & KIND_MASK)) {
		case 0: return null;
		case 1:
			if (String(kind) === BOOL_KIND) return Number(payload.int) !== 0;
			if (payload.ratio) return Number(payload.ratio[0].int) / Number(payload.ratio[1].int);
			// a big integer keeps its digits as a BigInt (exact.js, which the parts carrying big numbers start)
			if (payload.big) return typeof integerOfPayload === "function" ? integerOfPayload(payload) : null;
			return payload.int === undefined ? null : Number(payload.int);
		case 2: return Number(payload.float);
		case 3: case 5: return payload.text;
		case 4: return String.fromCodePoint(payload.i31);
		// the value's own cells follow it in the key's flat chain (reader.js): `{a:[1, 2]}` keeps its 2; code, `1+1`, is
		// `{"+": [1, 1]}` (the JsonLogic shape of src/node/json_xml.rs), a prefix operation without its empty left side
		case 6: {
			const [value, ...rest] = tree.chain ?? [];
			const right = value ? plainOfTree({ ...value, chain: rest }) : null;
			const operator = OPERATOR_SPELLINGS[Number((kind >> 8n) & KIND_MASK)] ?? ":"; // the operator code is the byte above the kind (src/run/wasm_reader.rs)
			if (ENTRY_OPERATORS.includes(operator)) return { [keyOfTree(payload.node)]: right };
			const left = plainOfTree(payload.node);
			return { [operator]: [left, right].filter(side => side !== null) };
		}
		case 7: case 8: {
			const values = items();
			const bracket = (kind >> 8n) & KIND_MASK;
			// a group of one, `(b + 1)`, is its item (src/node/json_xml.rs)
			if (bracket === ROUND_BRACKET && values.length === 1) return plainOfItem(values[0]);
			const isObject = bracket === 0n && values.length > 0 && values.every(item => item !== null && (BigInt(item.kind) & KIND_MASK) === KIND_KEY);
			return isObject ? Object.assign({}, ...values.map(plainOfTree)) : values.map(plainOfItem);
		}
		// its value the function's name (src/wasm_emitter/functions/closures.rs), kept for a host word calling the export itself
		case Number(KIND_FUNCTION): return tree.warpFunction ? Object.assign(callableOf(tree.warpFunction), { warpName: tree.chain?.[0]?.data?.text }) : null;
		default: return null;
	}
}

// the key of an entry: a name or text as itself, any other key as written, the head of `f(x) := …` "(f x)" (as
// src/node/json_xml.rs json_key)
const BRACKET_PAIRS = ["{}", "[]", "()"];
function keyOfTree(tree) {
	const kind = Number(BigInt(tree.kind) & KIND_MASK);
	if (kind !== 7 && kind !== 8) return String(plainOfTree(tree));
	const payload = tree.data ?? {};
	const items = payload.node === undefined ? [] : [payload.node, ...(tree.chain ?? []).map(cell => cell.data?.node)].filter(Boolean);
	const [open = "", close = ""] = BRACKET_PAIRS[Number((BigInt(tree.kind) >> 8n) & KIND_MASK)] ?? "";
	return open + items.map(keyOfTree).join(" ") + close;
}

// a warp function given to foreign code: a JavaScript function calling it back through the module's closure_apply
// (src/wasm_emitter/functions/closures.rs), synchronously, with as many of its arguments as it takes
function callableOf({ module, node }) {
	return (...values) => plainOfTree(readResult(module, module.closure_apply(node, buildValue(module, treeOfPlain(values)))));
}

// objects of the page without a plain form (a Date, a Map, an instance, a function), kept for foreign_call behind ids:
// they cross as `{$handle: id, type, text}` and are the object again when they come back
const foreignHandles = [];
const handleOf = value => ({ $handle: foreignHandles.push(value), type: value?.constructor?.name ?? typeof value, text: String(value).slice(0, 200) });
const unhandled = value => value !== null && typeof value === "object" && "$handle" in value ? foreignHandles[value.$handle - 1] : value;
const isPlainObject = value => [Object.prototype, null].includes(Object.getPrototypeOf(value));

// a plain JavaScript value as a tree buildValue builds: arrays square lists, objects `{key:value …}`, booleans warp's
// bools, an Int marked BOOL_INFO above the kind bits (src/type_kinds.rs BOOL_KIND, card booleans-foreign)
const BOOL_KIND = String((1n << 8n) | 1n); // BOOL_INFO above Kind::Int
const SQUARE_LIST = String((1n << 8n) | KIND_LIST);
const CURLY_LIST = String(KIND_LIST);
const COLON_KEY = String((1n << 8n) | KIND_KEY); // src/operators.rs OP_CODES: Colon is 1
// a codepoint (a WIT char, P94) as src/modules/foreign.rs sends it, since JSON would make it a text: `{$char: "b"}`
const KIND_CODEPOINT = "4";
const isCodepoint = value => value !== null && typeof value === "object" && Object.keys(value).length === 1 && typeof value.$char === "string" && [...value.$char].length === 1;
// a host word failing for the program's reason (a shader that does not compile), as native TaskFailure
class HostWordFailure extends Error {}

// a host word's failure as the program's own error (card wgsl-error, as src/host/native.rs raised): in a program with a
// `try` its message goes to trap_detail and the program's returned_error raises it, which a running `try` catches
// (`catch e` binds the message) and which otherwise traps as `raise` does; without a `try`, or any other exception (a
// bug of the host), the failure as it was
function raisedInProgram(exports, failure) {
	const returnedError = exports[RETURNED_ERROR_EXPORT], detail = exports[TRAP_DETAIL_EXPORT];
	if (!(failure instanceof HostWordFailure) || !returnedError || !detail) throw failure;
	detail.value = buildValue(exports, treeOfPlain(failure.message));
	returnedError();
	throw failure;
}

function treeOfPlain(value) {
	if (value === null || value === undefined) return { kind: "0", data: null, chain: [] };
	if (typeof value === "boolean") return { kind: BOOL_KIND, data: { int: value ? "1" : "0" }, chain: [] };
	if (typeof value === "bigint") return { kind: KIND_INT, data: { int: String(value) }, chain: [] };
	if (typeof value === "number") return Number.isSafeInteger(value) ? { kind: KIND_INT, data: { int: String(value) }, chain: [] } : { kind: KIND_FLOAT, data: { float: value }, chain: [] };
	if (typeof value === "string") return textTree(value);
	if (isCodepoint(value)) return { kind: KIND_CODEPOINT, data: { i31: value.$char.codePointAt(0) }, chain: [] };
	if (Array.isArray(value)) return { kind: SQUARE_LIST, items: value.map(treeOfPlain) };
	if (typeof value === "function" || (typeof value === "object" && !isPlainObject(value))) return treeOfPlain(handleOf(value));
	// an integer beyond 64 bits from Python: its digits (foreign_python.py plain)
	if (typeof value === "object" && Object.keys(value).length === 1 && typeof value.$int === "string") return { kind: KIND_INT, data: { exact: [value.$int, "1"] }, chain: [] };
	if (typeof value === "object") return { kind: CURLY_LIST, items: Object.entries(value).map(([key, item]) => ({ kind: COLON_KEY, key: [{ kind: "5", data: { text: key }, chain: [] }, treeOfPlain(item)] })) };
	return textTree(String(value));
}

// a run's timers and channels end with it (the next run, worker.js)
function stopListening(holder) {
	holder.stopped = true;
	eachHostPart("stopped", holder);
	holder.stopTimers?.();
}

// a program using standard modules imports its runtime words, memory and error tag from runtime.wasm and their words
// from modules/<name>.wasm (beside warp.wasm, build.sh), which import theirs the same way: one instance of each for a
// run, with the run's host words, each module's `warp/start` run before what imports it, as the native warp links
// them (core/native/driver.warp instances_linked; notes/runtime_linking.md "Load-time linking")
const RUNTIME_MODULE = "warp_runtime";
const LINKED_MODULE_PREFIX = "warp/";
const MODULE_START = "warp/start";
const compiledLibraries = new Map(); // by file: {module, descriptors}, compiled once per worker
// (a deployed site has no compiler folder: its runtime and modules beside the page)
function libraryUrl(file) { return typeof COMPILER_FOLDER === "undefined" ? file : `../../${COMPILER_FOLDER}/${file}`; }
function libraryFile(name) {
	if (name === RUNTIME_MODULE) return "runtime.wasm";
	return name.startsWith(LINKED_MODULE_PREFIX) ? `modules/${name.slice(LINKED_MODULE_PREFIX.length)}.wasm` : null;
}
// imports read from the bytes (imports.js importDescriptors): Safari's WebAssembly.Module.imports throws for GC signatures
// (instances: the run's linked instances by module name, filled while they start)
function withRuntime(bytes, imports, instances) { return withLinkedInstances(importDescriptors(bytes), imports, instances); }
// a run's holder of the fields given: its `exports` are the program's and those of the modules linked to it, as one
// (markup's page_render, runtime.wasm's memory and runtime words), so they answer while the linked modules start,
// before the program's own instance exists (a module's start reading os args or making a text); `linked` holds the
// run's linked instances by module name
function runHolder(fields) {
	const linked = new Map();
	let program;
	const instances = () => [program, ...linked.values()].filter(Boolean);
	const exportOf = name => instances().find(exports => name in exports)?.[name];
	const exports = new Proxy({}, {
		get: (_, name) => exportOf(name),
		has: (_, name) => exportOf(name) !== undefined,
		ownKeys: () => [...new Set(instances().flatMap(Object.keys))],
		getOwnPropertyDescriptor: (_, name) => { const value = exportOf(name); return value === undefined ? undefined : { value, enumerable: true, configurable: true }; },
	});
	return Object.defineProperty({ ...fields, linked }, "exports", { get: () => instances().length ? exports : undefined, set: given => { program = given; } });
}
// (proxies, never copies: an import object may answer only through get, as withWasi's does)
function withLinkedInstances(descriptors, imports, instances) {
	const linked = {};
	for (const name of new Set(descriptors.map(entry => entry.module))) {
		const file = libraryFile(name);
		if (file) linked[name] = linkedInstance(name, file, imports, instances);
	}
	return new Proxy(imports, { get: (modules, module) => linked[module] ?? modules[module] });
}
function linkedInstance(name, file, imports, instances) {
	if (instances.has(name)) return instances.get(name);
	if (!compiledLibraries.has(file)) {
		const bytes = getSync(libraryUrl(file), 0, true);
		compiledLibraries.set(file, { module: new WebAssembly.Module(bytes), descriptors: importDescriptors(bytes) });
	}
	const { module, descriptors } = compiledLibraries.get(file);
	const exports = new WebAssembly.Instance(module, withUnlinkedTrapping(descriptors, withLinkedInstances(descriptors, imports, instances))).exports;
	instances.set(name, exports);
	exports[MODULE_START]?.();
	return exports;
}
// runtime.wasm declares every host word: one this page does not give fails when reached, not at instantiation
function withUnlinkedTrapping(descriptors, imports) {
	const stubs = {};
	for (const { module: library, name, kind } of descriptors) {
		if (kind !== "function" || imports[library]?.[name]) continue;
		(stubs[library] ??= {})[name] = () => { throw new Error(`${library}.${name} is not linked in the browser`); };
	}
	return new Proxy(imports, { get: (modules, module) => module in stubs ? { ...modules[module], ...stubs[module] } : modules[module] });
}

// the one export a program with timers fires them by, timer_fired(id: int) (host-timers.js, driver.js timerFired)
const TIMER_FIRED_EXPORT = "timer_fired";
