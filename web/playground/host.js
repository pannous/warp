// What a host gives the warp compiler built without the native feature (src/web.rs `page`): `warp_host` runs the modules
// it compiles. A compiled program's imports mirror what the CLI's wasmtime runner links (src/host.rs, WASI fd_write,
// libm). Shared by the playground (worker.js), the browser test runner (test-worker.js) and built sites (site.js);
// needs reader.js. What only some programs reach is in the parts (HOST_PART_FILES, below).

const TEXT_HEAP_EXPORT = "text_heap";
const TRAP_DETAIL_EXPORT = "trap_detail";
const SHARED_CHECK_MILLISECONDS = 10; // how often a busy sleep looks at its check points
const PAGE_ROUTES_EXPORT = "page·routes";
const PAGE_BITS = 16;
const STDERR = 2;
const DARK_MODE_QUERY = "(prefers-color-scheme: dark)";
// random_seed (crates/warp-runtime host_words.rs): after a seed, xorshift64* as natively, so a seeded program gives the
// same numbers in both; unseeded, Math.random
const U64 = (1n << 64n) - 1n;
let seededRandom = null;
const seedRandom = seed => { seededRandom = (BigInt.asUintN(64, seed) * 0x9E3779B97F4A7C15n & U64) | 1n; };
function nextSeeded() {
	let x = seededRandom;
	x ^= x >> 12n;
	x = (x ^ (x << 25n)) & U64;
	x ^= x >> 27n;
	seededRandom = x;
	return x * 0x2545F4914F6CDD1Dn & U64;
}
const randomFloat = () => seededRandom === null ? Math.random() : Number(nextSeeded() >> 11n) / 2 ** 53;
const randomBelow = bound => bound <= 0n ? 0n : seededRandom === null ? BigInt(Math.floor(Math.random() * Number(bound))) : nextSeeded() % bound;

// the standard library's adapters (src/std_adapters.rs, notes/stdlib.md section 7): module → member → function of
// plain values (plainOfTree / treeOfPlain, as for foreign_call); the parts add theirs (host-files.js file and net)
const STD_ADAPTERS = {
	json: { parse: text => JSON.parse(text), to_json: (value, classes) => JSON.stringify(classes ? withoutClassTags(value, new Set(classes)) : value) },
	os: { env: () => null }, // a page has no environment
	// `stored theme = "dark"` (src/lowering/stored_values.rs): the page's values (markup.js keptValues), each save sent
	// back to it with its store (the dev store of a `warp dev` page, else the program's)
	store: {
		load: (name, fallback) => name in storedValues ? storedValues[name] : fallback,
		save: (name, value, file) => { storedValues[name] = value; self.keepStored?.(name, value, file); return null; },
	},
	regex: {
		matches: (subject, pattern) => regexOf(pattern).test(subject),
		first: (subject, pattern) => subject.match(regexOf(pattern))?.[0] ?? null,
		all: (subject, pattern) => [...subject.matchAll(regexOf(pattern, "g"))].map(found => found[0]),
		replace: (subject, pattern, replacement) => subject.replace(regexOf(pattern, "g"), replacement),
	},
};
// an instance of one of the program's classes is its fields: {Point: {x: 1}} is {x: 1} (src/std_adapters.rs)
function withoutClassTags(value, classes) {
	if (Array.isArray(value)) return value.map(item => withoutClassTags(item, classes));
	if (value === null || typeof value !== "object") return value;
	const keys = Object.keys(value);
	if (keys.length === 1 && classes.has(keys[0]) && value[keys[0]] !== null && typeof value[keys[0]] === "object" && !Array.isArray(value[keys[0]])) return withoutClassTags(value[keys[0]], classes);
	return Object.fromEntries(keys.map(key => [key, withoutClassTags(value[key], classes)]));
}
// what Rust's regex lacks is refused here too, so a pattern means the same in both hosts (src/std_adapters.rs regex_of)
function regexOf(pattern, flags = "") {
	const feature = /\(\?<?[=!]/.test(pattern) ? "look-around" : /\\[1-9]/.test(pattern) ? "a backreference" : null;
	if (feature) throw new Error(`${feature} is not in wasp's regex (one engine lacks it): ${pattern}`);
	return new RegExp(pattern, flags + "u");
}
// the stored values of `stored x = v`, by name: the page's localStorage as the worker started (playground.js)
const storedValues = {};
const contentText = content => typeof content === "string" ? content : JSON.stringify(content);
const utf8 = new TextEncoder();

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

// The parts of the host a program reaches only through some of its imports, each a file that adds itself here:
// host-files.js, host-hashes.js, host-tasks.js, host-foreign.js (needs host-files.js), host-compiler.js, host-routes.js. A built site
// ships a part only when its module imports one of the part's words (src/site.rs HOST_PARTS); the playground's workers
// load them all. A part gives any of: words(holder, hooks, access) the host words it adds, access being {program, text,
// cString} of programImports; imports(holder, hooks) import modules of their own (m, c); importModule(holder, hooks,
// name) an import module by its name (a .wasm path), else undefined; adapters, std modules for STD_ADAPTERS;
// started(run) as a run begins; poll(holder) at each check point (sleep, signal_poll); finished(holder, hooks) after a
// call into the run returned, a failure nobody read or nothing; ended(holder) after the call failed; stopped(holder)
// when the page drops the run (stopListening)
const HOST_PART_FILES = ["host-files.js", "host-hashes.js", "host-tasks.js", "host-foreign.js", "host-compiler.js", "host-routes.js"];
const hostParts = [];
function addHostPart(part) {
	hostParts.push(part);
	Object.assign(STD_ADAPTERS, part.adapters);
}
const eachHostPart = (step, ...values) => hostParts.map(part => part[step]?.(...values));

// hooks: print(text, fd), module(bytes) (each compiled module), panicked(message) (the compiler's)
function programImports(holder, hooks) {
	seededRandom = null; // each run starts unseeded
	const program = () => holder.exports;
	const text = (pointer, length) => readText(program(), pointer, length);
	const cString = pointer => {
		const bytes = new Uint8Array(program().memory.buffer);
		let end = pointer;
		while (bytes[end] !== 0 && end < bytes.length) end++;
		return bytes.slice(pointer, end);
	};
	const access = { program, text, cString };
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
					const [pointer, length] = writeBytes(module, utf8.encode("call stack exhausted"));
					return module.error_of(module.new_text(pointer, length));
				}
			},
			// std_pure / std_io(module, member, arguments): a word of std/<module>.wasp (src/std_adapters.rs)
			std_pure: (module, member, argumentList) => stdCall(program(), module, member, argumentList),
			std_io: (module, member, argumentList) => stdCall(program(), module, member, argumentList),
			// `serve 8080 {…}` (src/web_server.rs): a page cannot listen on a port
			serve_routes: port => { throw new Error(`serve ${port}: a server runs only in the native host (the warp CLI)`); },
			// the host words (src/host.rs): a page cannot block, so sleep busy-waits
			sleep: milliseconds => {
				hooks.sleeping?.(); // a paint after a sleep is the next frame of an animation (playground.js)
				const until = Date.now() + Number(milliseconds);
				let check = Date.now() + SHARED_CHECK_MILLISECONDS;
				while (Date.now() < until) {
					if (Date.now() >= check) {
						checkPoint();
						check = Date.now() + SHARED_CHECK_MILLISECONDS;
					}
				}
				checkPoint();
			},
			random: randomFloat,
			random_below: randomBelow,
			random_seed: seedRandom,
			clock: () => BigInt(Date.now()),
			// a page has no ctrl-c: `on interrupt {…}` never runs here (notes/system_signals.md); shared listeners do
			signal_poll: checkPoint,
			signal_every: (id, milliseconds) => addTimer(holder, hooks, id, { every: Number(milliseconds) }, "on every …"),
			signal_daily: (id, minute, weekdays) => addTimer(holder, hooks, id, { minute: Number(minute), weekdays: Number(weekdays) }, "on every day at …"),
			signal_at: (id, minute) => addTimer(holder, hooks, id, { minute: Number(minute), once: true }, "at 9:00 {…}"),
			signal_watch: () => { holder.warnings.push("on file … change: a page has no files to watch"); },
			// system values (crates/warp-runtime/src/system_values.rs): what the browser tells, a loud error for the rest
			system_value: name => {
				const value = decode(cString(name));
				if (value === "online") return BigInt(navigator.onLine);
				const pointerIndex = self.pagePointer?.names.indexOf(value) ?? -1; // playground.js trackPointer
				if (pointerIndex >= 0) return BigInt(Atomics.load(self.pagePointer.values, pointerIndex));
				// a Worker has no matchMedia: the page tells it (playground.js tellSystemValues)
				const known = value === "dark mode" && globalThis.matchMedia ? matchMedia(DARK_MODE_QUERY).matches : self.pageSystemValues?.[value];
				if (known !== undefined) return BigInt(known);
				throw new Error(`${value}: the playground cannot read it yet`);
			},
			// `notify "text"` (src/host.rs notify): the page shows it (playground.js notification)
			notify: text => {
				const shown = plainOfTree(readNode(program(), text));
				if (!hooks.notify) throw new Error(`notify ${JSON.stringify(shown)}: this page shows no notifications`);
				hooks.notify(typeof shown === "string" ? shown : JSON.stringify(shown));
			},
			clipboard_text: () => { throw new Error("clipboard: the playground cannot read it (the browser's clipboard is asynchronous)"); },
			// `exit(code)` ends the run, its value ø (P121): runProgram tells it from a failure by holder.exitCode
			exit: code => {
				holder.exitCode = Number(code);
				throw new Error(`exit(${code})`);
			},
			// paint(pixels, width, height) (src/host.rs): the page draws them on a canvas (playground.js showPaintings)
			paint: (pixels, width, height) => {
				if (!hooks.paint) throw new Error("paint: no canvas here; it draws in the playground page");
				hooks.paint(plainOfTree(readNode(program(), pixels)), Number(width), Number(height));
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

const KIND_MASK = 0xFFn;
const decode = bytes => utf8Decoder.decode(bytes);

const textTree = text => ({ kind: "3", data: { text }, chain: [] });
const KIND_LIST = 8n;
// the inverse of reader.js readNode for the values a task carries (src/tasks.rs TaskValue): built through the module's
// constructors; a list from its cons cells, the first item in data, the next cells in the chain
function buildValue(module, tree) {
	if (tree.closure !== undefined) {
		const name = buildValue(module, { kind: "5", data: { text: tree.closure }, chain: [] });
		return module.closure_rebuild(name, tree.captured === null ? null : buildValue(module, tree.captured));
	}
	if (tree.key) return module.new_key(buildValue(module, tree.key[0]), buildValue(module, tree.key[1]), BigInt(tree.kind) >> 8n);
	if (tree.items) return tree.items.reduceRight((rest, item) => module.new_list(buildValue(module, item), rest, BigInt(tree.kind) >> 8n), null) ?? module.new_empty();
	const kind = BigInt(tree.kind);
	const payload = tree.data ?? {};
	switch (Number(kind & KIND_MASK)) {
		case 0: return module.new_empty();
		case 1:
			if (payload.exact) return module.new_int(exactHandle(module, ...payload.exact.map(BigInt)));
			if (payload.int === undefined) throw new Error("an exact number beyond the fixnum range cannot cross to another task yet");
			return module.new_int(BigInt(payload.int));
		case 2: return module.new_float(Number(payload.float));
		case 3: case 5: {
			const [pointer, length] = writeBytes(module, utf8.encode(payload.text));
			return Number(kind & KIND_MASK) === 3 ? module.new_text(pointer, length) : module.new_symbol(pointer, length);
		}
		case 4: return module.new_codepoint(payload.i31);
		case 7: case 8: {
			const items = [payload.node, ...tree.chain.map(cell => cell.data?.node)].filter(Boolean);
			return items.reduceRight((rest, item) => module.new_list(buildValue(module, item), rest, kind >> 8n), null) ?? module.new_empty();
		}
		default: throw new Error(`a value of kind ${kind & KIND_MASK} cannot cross to another task yet`);
	}
}

function stdCall(program_, module, member, argumentList) {
	const [moduleName, memberName, given] = [module, member, argumentList].map(node => plainOfTree(readNode(program_, node)));
	const adapter = STD_ADAPTERS[moduleName]?.[memberName];
	if (!adapter) throw new Error(`${moduleName}.${memberName}: no such word in the browser`);
	try {
		return buildValue(program_, treeOfPlain(adapter(...(Array.isArray(given) ? given : [given]))));
	} catch (error) {
		throw new Error(`${moduleName}.${memberName}: ${error.message}`);
	}
}

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
			if (payload.ratio) return Number(payload.ratio[0].int) / Number(payload.ratio[1].int);
			return payload.int === undefined ? null : Number(payload.int);
		case 2: return Number(payload.float);
		case 3: case 5: return payload.text;
		case 4: return String.fromCodePoint(payload.i31);
		// the value's own cells follow it in the key's flat chain (reader.js): `{a:[1, 2]}` keeps its 2
		case 6: {
			const [value, ...rest] = tree.chain ?? [];
			return { [plainOfTree(payload.node)]: value ? plainOfTree({ ...value, chain: rest }) : null };
		}
		case 7: case 8: {
			const values = items();
			const isObject = (kind >> 8n) === 0n && values.length > 0 && values.every(item => item !== null && (BigInt(item.kind) & KIND_MASK) === KIND_KEY);
			return isObject ? Object.assign({}, ...values.map(plainOfTree)) : values.map(plainOfItem);
		}
		default: return null;
	}
}

// objects of the page without a plain form (a Date, a Map, an instance, a function), kept for foreign_call behind ids:
// they cross as `{$handle: id, type, text}` and are the object again when they come back
const foreignHandles = [];
const handleOf = value => ({ $handle: foreignHandles.push(value), type: value?.constructor?.name ?? typeof value, text: String(value).slice(0, 200) });
const unhandled = value => value !== null && typeof value === "object" && "$handle" in value ? foreignHandles[value.$handle - 1] : value;
const isPlainObject = value => [Object.prototype, null].includes(Object.getPrototypeOf(value));

// a plain JavaScript value as a tree buildValue builds: arrays square lists, objects `{key:value …}`, booleans 1/0
const SQUARE_LIST = String((1n << 8n) | KIND_LIST);
const CURLY_LIST = String(KIND_LIST);
const COLON_KEY = String((1n << 8n) | KIND_KEY); // src/operators.rs OP_CODES: Colon is 1
// a codepoint (a WIT char, P94) as src/foreign.rs sends it, since JSON would make it a text: `{$char: "b"}`
const KIND_CODEPOINT = "4";
const isCodepoint = value => value !== null && typeof value === "object" && Object.keys(value).length === 1 && typeof value.$char === "string" && [...value.$char].length === 1;
function treeOfPlain(value) {
	if (value === null || value === undefined) return { kind: "0", data: null, chain: [] };
	if (typeof value === "boolean") return { kind: KIND_INT, data: { int: value ? "1" : "0" }, chain: [] };
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

// the Int handle of an exact number beyond the fixnums, composed from fixnum pieces with the module's exported Int
// operations (src/tasks.rs EXACT_BUILDERS, integer_handle)
const FIXNUM_MIN = -(2n ** 62n - 1n);
const FIXNUM_MAX = 2n ** 62n;
function exactHandle(module, numerator, denominator) {
	const integer = n => {
		if (n >= FIXNUM_MIN && n <= FIXNUM_MAX) return n;
		const limbs = [];
		for (let rest = n < 0n ? -n : n; rest > 0n; rest >>= 32n) limbs.push(rest & 0xffffffffn);
		const magnitude = limbs.reduceRight((handle, limb) => module.exact_add(module.int_shift_left(handle, 32n), limb), 0n);
		return n < 0n ? module.exact_sub(0n, magnitude) : magnitude;
	};
	return denominator === 1n ? integer(numerator) : module.exact_div(integer(numerator), integer(denominator));
}

// Timers (crates/warp-runtime/src/system_signals.rs): main records them, the page starts them after it (startTimers,
// in the playground's worker.js and a built site's site.js); a host without a page that stays (test-worker.js) only warns
const TIMER_HANDLER_PREFIX = "on·every·";
function addTimer(holder, hooks, id, timer, written) {
	if (!hooks.listen) return holder.warnings.push(`${written}: timers do not run here`);
	(holder.timers ??= []).push({ id: Number(id), handler: TIMER_HANDLER_PREFIX + id, ...timer });
}

// milliseconds from now to the next local minute_of_day on a weekday of the mask (bit 0 Sunday), as seconds_until_on
const EVERY_DAY = 0b1111111;
const DAY_MILLISECONDS = 24 * 3600 * 1000;
function millisecondsUntil(minuteOfDay, weekdays = EVERY_DAY, now = new Date()) {
	const midnight = new Date(now.getFullYear(), now.getMonth(), now.getDate());
	for (let daysAhead = 0; daysAhead <= 7; daysAhead++) {
		const due = new Date(midnight.getFullYear(), midnight.getMonth(), midnight.getDate() + daysAhead, 0, minuteOfDay);
		if (due > now && (weekdays || EVERY_DAY) & (1 << due.getDay())) return due - now;
	}
	return 7 * DAY_MILLISECONDS;
}

// a run's timers started: `fire(handler)` runs the handler on·every·<id> of each when it is due; stopTimers ends them
function startTimers(holder, fire) {
	const handles = [];
	const atClock = timer => handles.push(setTimeout(() => {
		fire(timer.handler);
		if (!timer.once) atClock(timer);
	}, millisecondsUntil(timer.minute, timer.weekdays)));
	for (const timer of holder.timers ?? []) {
		if (timer.every !== undefined) handles.push(setInterval(() => fire(timer.handler), timer.every));
		else atClock(timer);
	}
	holder.stopTimers = () => handles.forEach(handle => { clearTimeout(handle); clearInterval(handle); });
}

// what a timer says in the page: "every 500 ms", "at 09:00", "every day at 09:00", or the channel its listener reads
function timerLabel(holder, { id, every, minute, once }) {
	const channel = holder.channels?.get(id);
	if (channel) return `message from "${channel.name}"`;
	if (every !== undefined) return every % 1000 ? `every ${every} ms` : `every ${every / 1000} s`;
	const time = `${String(Math.floor(minute / 60)).padStart(2, "0")}:${String(minute % 60).padStart(2, "0")}`;
	return once ? `at ${time}` : `every day at ${time}`;
}

// a run's timers and channels end with it (the next run, worker.js)
function stopListening(holder) {
	holder.stopped = true;
	eachHostPart("stopped", holder);
	holder.stopTimers?.();
}

// run a compiled program: the outcome src/web.rs run_outcome reads
function runProgram(bytes, hooks) {
	const holder = instantiateProgram(bytes, hooks);
	return holder.failure ? holder : runMain(holder, hooks);
}

// the program's instance, its main not run yet (a built site first loads the module of the route it shows, site.js);
// { failure } when it cannot be instantiated
function instantiateProgram(bytes, hooks) {
	// the runtime warnings go back to the compiler, which reports them (src/web.rs); the page's path is the page's own
	const holder = { warnings: [], pagePath: hooks.pagePath?.() };
	try {
		const module = new WebAssembly.Module(bytes);
		holder.run = { module };
		eachHostPart("started", holder.run);
		holder.exports = new WebAssembly.Instance(holder.run.module, programImports(holder, hooks)).exports;
		hooks.instantiated?.(holder);
	} catch (failure) {
		return { failure: String(failure.message ?? failure) };
	}
	return holder;
}

// main of an instantiated program, and what it left
function runMain(holder, hooks) {
	const { exports } = holder;
	const outcome = outcomeOf(holder, hooks, () => withExitHandler(holder, exports, () => exports.main()));
	const events = pageEvents(exports);
	// a program with routes stays for its links (lowering/routes.rs page·routes)
	if ((events.length > 0 || holder.timers || holder.fetches || exports[PAGE_ROUTES_EXPORT]) && outcome.result) hooks.listen?.(holder, events);
	// a run without page events (std/markup.wasp rendering the page's HTML, src/markup.rs) keeps the page's run
	if (events.length > 0 && outcome.result) listeningRun = holder;
	return outcome;
}

// the last run that handles page events, for the compiler's warp_host.page_event (src/headless.rs in the browser tests)
let listeningRun;
const PAGE_VALUE_EXPORT = "page·value";
const PAGE_RENDER_EXPORT = "page·render"; // src/lowering/page_html.rs PAGE_RENDER

// the HTML of a value by the program's own renderer (std/markup.wasp's to_html, exported as page·render by a program
// holding markup): undefined for a number, a program without one, or a rendering that failed (the compiler then
// renders the value itself, src/web.rs html_of)
function renderedHtml(exports, value) {
	const render = exports[PAGE_RENDER_EXPORT];
	if (!render || value === null || typeof value !== "object") return undefined;
	try {
		return plainOfTree(readNode(exports, render(value)));
	} catch {
		return undefined;
	}
}

// `on exit {…}` (src/lowering/event_signals.rs, natively system_signals.rs with_exit_handler): on·exit runs once after
// main returns or `exit(code)` ends it, never after a failure
function withExitHandler(holder, exports, call) {
	const handler = exports["on·exit"];
	if (!handler) return call();
	// `event` is the exit code (P134), 0 when main returned: an i64 parameter takes a BigInt, a Node one a new_int
	const withCode = () => {
		const code = BigInt(holder.exitCode ?? 0);
		try {
			return handler(code);
		} catch (failure) {
			if (!(failure instanceof TypeError)) throw failure; // the parameter is a Node: the call never started
			return handler(exports.new_int(code));
		}
	};
	const runHandler = () => (handler.length ? withCode() : handler());
	let result;
	try {
		result = call();
	} catch (trap) {
		if (holder.exitCode !== undefined) runHandler();
		throw trap;
	}
	runHandler();
	return result;
}

// the outcome of a call into a run's instance (main, or a page event's handler), as src/web.rs run_outcome reads it
function outcomeOf(holder, hooks, call) {
	const { warnings, exports } = holder;
	try {
		const result = call();
		const unread = eachHostPart("finished", holder, hooks).find(Boolean);
		if (unread) return { failure: unread, warnings };
		return { result: readResult(exports, result), html: hooks.renders ? renderedHtml(exports, result) : undefined, warnings };
	} catch (trap) {
		eachHostPart("ended", holder);
		if (holder.exitCode !== undefined) return { result: { kind: "0", data: null, chain: [] }, warnings };
		if (holder.blockError !== undefined) return { error: holder.blockError, warnings };
		if (!(trap instanceof WebAssembly.RuntimeError || trap instanceof RangeError)) return { failure: String(trap.message ?? trap), warnings };
		const detail = exports[TRAP_DETAIL_EXPORT]?.value;
		return { trap: trap.message, trace: trap.stack ?? "", detail: detail ? readNode(exports, detail) : null, warnings };
	}
}

// the page events a program handles (src/lowering/event_signals.rs PAGE_EVENTS): `on click {…}` exports on·click·node
const PAGE_EVENT_HANDLER = /^on·((?:click|key|input)(?:·\d+)?)·node$/;
function pageEvents(exports) {
	return Object.keys(exports).map(name => name.match(PAGE_EVENT_HANDLER)?.[1]).filter(Boolean);
}

// a page event of a program that handles it, after its main ran: the handler with the event's data, its outcome
function runPageEvent(holder, hooks, event, detail) {
	holder.warnings = [];
	const handler = holder.exports[`on·${event}·node`];
	return outcomeOf(holder, hooks, () => handler(buildValue(holder.exports, treeOfPlain([detail]))));
}

// a timer's handler (on·every·<id>), given ø when it reads `event`, as natively (system_signals.rs call_handler)
function runTimer(holder, hooks, name) {
	holder.warnings = [];
	const handler = holder.exports[name];
	return outcomeOf(holder, hooks, () => handler.length ? handler(holder.exports.new_empty()) : handler());
}
