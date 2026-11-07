// The host words of a program warp compiled, run in the page (driver.js programRan): its imports mirror what the native
// warp answers (core/native/driver.warp, WASI, libm). Shared by the playground (worker.js) and built sites
// (site-worker.js); needs values.js. What only some programs reach is in the parts (HOST_PART_FILES, below).

const TEXT_HEAP_EXPORT = "text_heap";
const TRAP_DETAIL_EXPORT = "trap_detail";
// a program with a `try` exports it: the runtime error that raises the Error left in trap_detail (runtime_errors.rs)
const RETURNED_ERROR_EXPORT = "returned_error";
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
// plain values (values.js guestPlain / newGuestValue, as for foreign_call); the parts add them (host-hashes.js, host-files.js)
const STD_ADAPTERS = {};

// the stored values of `stored x = v` and `local[k]`, by name: the page's localStorage as the worker started
// (playground.js); those of `session[k]`, its sessionStorage
const storedValues = {};
const sessionValues = {};
const contentText = content => typeof content === "string" ? content : JSON.stringify(content);
const utf8 = new TextEncoder();
const utf8Decoder = new TextDecoder("utf-8", { fatal: false });
const readText = (module, pointer, length) => utf8Decoder.decode(new Uint8Array(module.memory.buffer, pointer, length));

// the system values of the pointer the page shares; without shared memory a running program cannot see it move (the
// run blocks its Worker's messages), so it would wait for a finger forever (card abysmally-slow: the in-app browser
// view of an iPad app painted nothing). A key held reads none there, as in the test suite's page
const POINTER_VALUES = ["mouse_x", "mouse_y", "mouse_down", "mouse_down_x", "mouse_down_y", "mouse_wheel"];
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
					return newGuestError(module, "call stack exhausted");
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
				const shown = guestPlain(program(), text);
				if (!hooks.notify) throw new Error(`notify ${JSON.stringify(shown)}: this page shows no notifications`);
				hooks.notify(typeof shown === "string" ? shown : JSON.stringify(shown));
			},
			// an emit no handler of a page's program receives (card page-custom-event): a Worker posts it to its page
			// (hooks.dispatch), a page's own thread dispatches it (markup.js dispatchToPage)
			dispatch_event: (name, detail) => {
				const dispatch = hooks.dispatch ?? globalThis.dispatchToPage;
				const event = { name: guestPlain(program(), name), detail: guestPlain(program(), detail) };
				if (!dispatch) throw new Error(`emit ${event.name}: this host has no page to dispatch it to`);
				dispatch(event);
			},
			// `exit(code)` ends the run, its value ø (P121): runProgram tells it from a failure by holder.exitCode
			exit: code => {
				holder.exitCode = Number(code);
				throw new Error(`exit(${code})`);
			},
			// a text's NFC for text == (decision nfc-slot, driver.warp canonical_text_answered): lib/unicode, its module
			// linked on this first call, never at the run's start
			canonical_text: text => linkedInstance(UNICODE_MODULE, linkedUrl(UNICODE_MODULE), holder.imports, holder.linked).canonical_text(text),
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

// objects of the page without a plain form (a Date, a Map, an instance, a function), kept for foreign_call behind ids:
// they cross as `{$handle: id, type, text}` and are the object again when they come back
const foreignHandles = [];
const handleOf = value => ({ $handle: foreignHandles.push(value), type: value?.constructor?.name ?? typeof value, text: String(value).slice(0, 200) });
const unhandled = value => value !== null && typeof value === "object" && "$handle" in value ? foreignHandles[value.$handle - 1] : value;
const isPlainObject = value => [Object.prototype, null].includes(Object.getPrototypeOf(value));

// a codepoint (a WIT char, P94) as a foreign runtime sends it, since JSON would make it a text: `{$char: "b"}`
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
	detail.value = newGuestValue(exports, failure.message);
	returnedError();
	throw failure;
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
const UNICODE_MODULE = "warp/unicode";
const compiledLibraries = new Map(); // by url: {module, descriptors}, compiled once per worker
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
		const url = linkedUrl(name);
		if (url) linked[name] = linkedInstance(name, url, imports, instances);
	}
	return new Proxy(imports, { get: (modules, module) => linked[module] ?? modules[module] });
}
// a module linked by its import's name: a standard one's file, or a program's own module warp built (`import shapes`
// beside shapes.wasm, imported by its path), which imports runtime.wasm's words as a standard module does
function linkedUrl(name) {
	const file = libraryFile(name);
	if (file) return libraryUrl(file);
	if (!MODULE_PATH.test(name)) return null;
	const url = servedModuleUrl(name);
	return compiledLibrary(url).descriptors.some(entry => entry.module === RUNTIME_MODULE) ? url : null;
}
function compiledLibrary(url) {
	if (!compiledLibraries.has(url)) {
		const bytes = getSync(url, 0, true);
		compiledLibraries.set(url, { module: new WebAssembly.Module(bytes), descriptors: importDescriptors(bytes) });
	}
	return compiledLibraries.get(url);
}
function linkedInstance(name, url, imports, instances) {
	if (instances.has(name)) return instances.get(name);
	const { module, descriptors } = compiledLibrary(url);
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
// the one a fetch's answer comes by, fetch_answered(id: int, answer) (lib/net.warp fetch_later, host-timers.js)
const FETCH_ANSWERED_EXPORT = "fetch_answered";
