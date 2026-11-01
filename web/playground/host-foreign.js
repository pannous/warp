// Foreign code in the page (a part of host.js, which says how parts work; needs host-files.js): foreign_call to the
// runtimes registered here (js, python), libm and libc.wasm for `use c`, and the WebAssembly modules a program imports
// by path

/// C names of libm functions (ffi "m") that Math spells differently
const LIBM = { fabs: Math.abs, fmin: Math.min, fmax: Math.max, fmod: (a, b) => a % b, ceil: Math.ceil, floor: Math.floor };
const LIBC_URL = new URL("lib/libc.wasm", self.location.href).href; // libc for `use c` (lib/build_libc.sh, P147)
const MODULE_PATH = /\.(wasm|wat)$/; // src/modules/wasm_modules.rs MODULE_EXTENSIONS
const SETTER_PREFIX = "set "; // src/modules/wasm_modules.rs SETTER_PREFIX: the import that sets a mutable global
// the C functions a module calls, one line each: import module, library, name, parameter kinds (comma-separated),
// result register, what the result crosses as, an out-pointer's pointee, -, how a failure shows
// (core/emitter/c_functions.warp, read as core/native/driver.warp foreign_of reads it)
const C_FUNCTIONS_SECTION = "warp.c_functions";
const IMPORTED_MEMORY_PAGES = 1; // src/wasm_emitter/mod.rs MEMORY: one page at least
// a promise is awaited only by an awaited run's main (host.js JSPI), not where the browser has no JSPI (Safari), nor in a
// page event's handler, nor in a warp function JavaScript calls back (a JavaScript frame between: the engine refuses
// to suspend it, SUSPENSION_FAILURE)
const PROMISE_REFUSAL = "gives a promise, which a program here cannot wait for: only its main waits, in a browser with JSPI (not Safari); natively (the warp CLI) node awaits it";
const NESTED_PROMISE_REFUSAL = "gives a promise, which a warp function called back by JavaScript cannot wait for";
const SUSPENSION_FAILURE = /suspend/i;

// the runtimes foreign_call reaches in the page, by their name in `use <runtime> …`: call(module, member, arguments,
// hooks) gives the member's plain value (arguments null: a read, no call); prepare(code), when given, readies the runtime
// for a program before it runs (an asynchronous load), since a call itself is synchronous; a later registration of a
// name adds to the earlier one (worker.js gives python its prepare)
const foreignRuntimes = new Map();
function registerForeignRuntime(name, runtime) {
	foreignRuntimes.set(name, { ...foreignRuntimes.get(name), ...runtime });
}
const prepareForeignRuntimes = code => Promise.all([...foreignRuntimes.values()].map(runtime => runtime.prepare?.(code)));

// what warp's operators on a value of the page forward to (src/lowering/foreign/foreign_modules.rs), as in src/modules/foreign.rs's loop
const FOREIGN_OPERATORS = { add: (a, b) => a + b, sub: (a, b) => a - b, mul: (a, b) => a * b, truediv: (a, b) => a / b, mod: (a, b) => a % b, pow: (a, b) => a ** b,
	lt: (a, b) => a < b, gt: (a, b) => a > b, le: (a, b) => a <= b, ge: (a, b) => a >= b, eq: (a, b) => a === b, ne: (a, b) => a !== b, neg: a => -a,
	getitem: (a, i) => typeof a.get === "function" ? a.get(i) : a[i], len: a => a.length ?? a.size, list: a => Array.from(a) };

// `use python` in the page: the bridge of src/modules/foreign.rs (foreign_python.py) in Pyodide, which worker.js loads before
// a program that says `use python` runs; requests and answers are the same JSON, handles stay in Pyodide
// `use python`: Pyodide's bridge (web/playground/foreign_python.py), which the playground's worker.js loads before a run
registerForeignRuntime("python", {
	call(moduleName, memberName, argumentValues) {
		if (!self.pythonAnswer) throw new Error(`python ${moduleName}.${memberName}: Python (Pyodide) is not loaded here`);
		const reply = JSON.parse(self.pythonAnswer(JSON.stringify({ module: moduleName, member: memberName, arguments: argumentValues })));
		if (reply.error !== undefined) throw new Error(`python ${typeof moduleName === "string" ? moduleName : "value"}.${memberName}: ${reply.error}`);
		return reply.value;
	},
});

// `use js Math`: the page's own globalThis.Math; npm modules need the native host
registerForeignRuntime("js", {
	call(moduleName, memberName, argumentValues) {
		// a module's name, or a handle: an object of the page kept behind an id (src/modules/foreign.rs)
		const value = typeof moduleName === "object" ? unhandled(moduleName) : moduleName === "operator" ? FOREIGN_OPERATORS : globalThis[moduleName];
		if (value instanceof PageObject || (value === undefined && self.askPageCall)) return callOnPage(value ?? moduleName, memberName, argumentValues);
		if (value === undefined) throw new Error(`js ${moduleName}.${memberName}: the page has no global ${moduleName} (modules need the native host)`);
		return jsMember(value, moduleName, memberName, argumentValues?.map(unhandled) ?? null);
	},
});

// a global only the page has (`use js document`, `document`, `$who`) in the program's Worker: the page calls it while the
// Worker waits (host-tasks.js askPage; task-workers.js answerPageCall), and an object it gives stays on the page, here
// a PageObject naming its id there (card browser-interaction)
class PageObject {
	constructor({ $page, type, text }) {
		Object.assign(this, { $page, type, text });
	}
	toString() {
		return this.text;
	}
}
// one PageObject per id while one lives, counting how often the page gave the id; once it is collected the page hears how
// often, and lets the object go when no Worker holds it (task-workers.js releaseFrom, card page-pageobjects)
const pageObjectsHere = new Map(); // id → {ref: WeakRef(PageObject), received}
const releasedOnPage = new FinalizationRegistry(({ id, entry }) => {
	if (pageObjectsHere.get(id) !== entry) return; // a newer PageObject of the id took over the count
	pageObjectsHere.delete(id);
	self.postMessage({ releasePageObject: { id, received: entry.received } });
});
function pageObjectOf(value) {
	const entry = pageObjectsHere.get(value.$page);
	const living = entry?.ref.deref();
	if (living) {
		entry.received++;
		return Object.assign(living, { type: value.type, text: value.text });
	}
	const made = new PageObject(value);
	// a collected one not yet finalized: its count goes on in the new entry
	const fresh = { ref: new WeakRef(made), received: (entry?.received ?? 0) + 1 };
	pageObjectsHere.set(value.$page, fresh);
	releasedOnPage.register(made, { id: value.$page, entry: fresh });
	return made;
}
const fromPage = value => Array.isArray(value) ? value.map(fromPage) : value?.$page ? pageObjectOf(value) : value;
const toPage = value => value instanceof PageObject ? { $page: value.$page, type: value.type } : value;
function callOnPage(receiver, memberName, argumentValues) {
	const label = `js ${receiver.type ?? receiver}.${memberName}`;
	const request = { receiver: toPage(receiver), member: memberName, arguments: argumentValues?.map(value => toPage(unhandled(value))) ?? null };
	if (!self.askPage) throw new Error(`${label}: only the page has it, and this Worker carries no way to ask it (host-tasks.js)`);
	const { value } = self.askPage(label, shared => self.askPageCall(shared, request));
	return fromPage(value);
}

// the NUL-terminated bytes at `address` of a linear memory
function cText(memory, address) {
	const bytes = new Uint8Array(memory.buffer, address);
	const end = bytes.indexOf(0);
	return bytes.slice(0, end < 0 ? bytes.length : end);
}

// the program's C functions by "module\tname" (C_FUNCTIONS_SECTION)
function cCalls(run) {
	if (!run.cCalls) {
		const lines = WebAssembly.Module.customSections(run.module, C_FUNCTIONS_SECTION).flatMap(section => decode(new Uint8Array(section)).split("\n"));
		run.cCalls = new Map(lines.filter(Boolean).map(line => line.split("\t")).map(([module, , name, parameters, register, crossing, pointee, , failure]) =>
			[`${module}\t${name}`, { parameters: parameters ? parameters.split(",") : [], register, crossing, pointee, failure }]));
	}
	return run.cCalls;
}

// a C function of a module, called with the program's values as driver.warp c_called calls one: each text (ptr) a
// NUL-terminated copy in a block of the module's malloc, a list of texts (texts) a NULL-terminated array of them, a
// text or list of bytes (bytes) its bytes and their count for a following length, the first out-pointer a NULL slot
// there, all freed after the call; a text result (text, owned_text) is read back as a text of the program (NULL is ø)
function callC(holder, exports, member, call, values, what) {
	const program = holder.exports;
	const blocks = [];
	const malloc = bytes => {
		if (!exports.malloc) throw new Error(`${what} takes a text or out-pointer, but its module exports no malloc(size)`);
		const block = exports.malloc(bytes.length);
		new Uint8Array(exports.memory.buffer).set(bytes, block);
		blocks.push(block);
		return block;
	};
	const textBlock = value => malloc([...utf8.encode(guestText(program, value)), 0]);
	const bytesOf = value => isGuestText(program, value) ? utf8.encode(guestText(program, value)) : guestInts(program, value).map(byte => byte & 0xff);
	const memory = () => new DataView(exports.memory.buffer);
	let out, bytesLength = 0;
	const remaining = [...values];
	const moduleArguments = (call?.parameters ?? values.map(() => "i64")).map(kind => {
		if (kind === "out") return out ??= malloc(new Uint8Array(4));
		if (kind === "length") return BigInt(bytesLength);
		const value = remaining.shift();
		if (kind === "ptr") return textBlock(value);
		if (kind === "texts") {
			const pointers = new Uint32Array([...guestItems(program, value).map(textBlock), 0]);
			return malloc(new Uint8Array(pointers.buffer));
		}
		if (kind === "bytes") {
			const bytes = bytesOf(value);
			bytesLength = bytes.length;
			return malloc(bytes);
		}
		return value;
	});
	let returned = member(...moduleArguments);
	const asNumber = Number(returned ?? 0);
	if ((call?.failure === "not_null" && asNumber !== 0) || (call?.failure === "false" && (asNumber & 0xff) === 0)) throw new Error(`${what} failed, C result ${returned}`);
	if (call?.pointee) {
		returned = out === undefined ? 0 : memory().getUint32(out, true);
		if (returned === 0) throw new Error(`${what} gave no ${call.pointee}, C status ${asNumber}`);
	}
	const result = crossedResult(program, call, returned, address => utf8Decoder.decode(cText(exports.memory, Number(address))));
	if (exports.free) blocks.forEach(block => exports.free(block));
	return result;
}

// a C function's result as the program takes it (driver.warp c_result): a text (NULL is ø, textOf reads it), a
// handle's id, a yes/no, an i64 or an i32
function crossedResult(program, call, returned, textOf) {
	if (call?.crossing === "text" || call?.crossing === "owned_text") return !returned ? program.new_empty() : newGuestText(program, textOf(returned));
	if (call?.crossing === "handle" || call?.register === "i32") return Number(returned ?? 0) | 0;
	if (call?.register === "bool") return Number(returned) & 0xff ? 1 : 0;
	if (call?.register === "i64") return BigInt(returned ?? 0);
	return returned;
}

// a C library the page answers in JavaScript (natively a shared library): `functions(name)` gives a C function as a
// JavaScript one, which takes a text as a string and gives one so, its numbers and handles' ids as they cross
function libraryInJs(holder, library, functions) {
	return new Proxy({}, {
		get: (_, name) => (...values) => {
			const member = functions(name);
			if (!member) throw new Error(`${library}.${name} is not available in the browser`);
			const call = cCalls(holder.run).get(`${library}\t${name}`);
			const program = holder.exports;
			const remaining = [...values];
			const given = (call?.parameters ?? []).filter(kind => kind !== "out" && kind !== "length")
				.map(kind => (value => kind === "ptr" ? guestText(program, value) : value)(remaining.shift()));
			return crossedResult(program, call, member(...given), text => text);
		},
	});
}

// dlopen of one of warp's own libraries beside the executable (lib/sound.warp: @executable_path/libwarp_sound.dylib):
// a handle when the page has it (host-sound.js PAGE_LIBRARIES), else 0, as for a library that is not there
const OWN_LIBRARY = /\blib(warp_\w+)\.(dylib|so)$/;
const PAGE_LIBRARY_HANDLE = 1;

// the C functions of stdio a program reaches without a file (lib/sound.warp said: fdopen(2, "w"), fputs, fflush): a
// FILE * is its descriptor, what it puts goes to the run's output; and dlopen of warp's own libraries
const pageStdio = hooks => ({
	fdopen: descriptor => descriptor,
	fputs: (text, descriptor) => (hooks.print(text, descriptor), 0),
	fflush: () => 0,
	dlopen: path => self.PAGE_LIBRARIES?.has(path.match(OWN_LIBRARY)?.[1]) ? PAGE_LIBRARY_HANDLE : 0,
});

// `use python` (lib/native/python.warp: the Python C API, natively libpython's) answered by Pyodide's Python
// (python_capi.py), loaded before the run (worker.js prepareForeignRuntimes)
function pythonLibrary(holder) {
	return libraryInJs(holder, "python", name => {
		if (!self.pythonCapi) throw new Error(`python.${name}: Python is not loaded in this page (Pyodide)`);
		const member = self.pythonCapi.get(name);
		return member && ((...values) => member(...values));
	});
}

// libc.wasm, instantiated once per worker: its WASI imports serve only stdio, which nothing here reaches
let libc;
function libcExports() {
	const refused = name => () => { throw new Error(`libc.wasm called WASI ${name}`); };
	libc ??= new WebAssembly.Instance(new WebAssembly.Module(getSync(LIBC_URL, 0, true)),
		{ wasi_snapshot_preview1: new Proxy({}, { get: (_, name) => refused(String(name)) }) }).exports;
	return libc;
}

// `use c` in the browser: the program's C calls go to libc.wasm (natively warp opens the system libc)
function libcImports(holder, hooks) {
	const stdio = libraryInJs(holder, "c", name => pageStdio(hooks)[name]);
	return new Proxy({}, {
		get: (_, name) => name in pageStdio(hooks) ? stdio[name] : (...values) => {
			const exports = libcExports();
			const member = exports[name];
			if (typeof member !== "function") throw new Error(`c.${name} is not available in the browser (web/playground/lib/build_libc.sh lists libc.wasm's functions)`);
			return callC(holder, exports, member, cCalls(holder.run).get(`c\t${name}`), values, `c.${name}`);
		},
	});
}

// the exports of a WebAssembly module the program imports by path (src/modules/wasm_modules.rs link): instantiated once per
// run at the first call, with its own imports linked like the program's; a global reads through a getter and is set
// through `set g`
function moduleImports(holder, hooks, path) {
	const instance = () => {
		const modules = holder.run.wasmModules ??= new Map();
		const file = new URL(path, FILE_ROOT).href; // one instance however the path is spelled
		if (!modules.has(file)) {
			if (file.endsWith(".wat")) throw new Error(`${path}: a module in WAT text needs the native build`);
			const moduleHolder = { warnings: holder.warnings, run: holder.run };
			const module = new WebAssembly.Module(readBytes(file));
			moduleHolder.exports = new WebAssembly.Instance(module, programImports(moduleHolder, hooks)).exports;
			modules.set(file, moduleHolder.exports);
		}
		return modules.get(file);
	};
	return new Proxy({}, {
		get: (_, name) => (...values) => {
			const exports = instance();
			if (name.startsWith(SETTER_PREFIX)) return (exports[name.slice(SETTER_PREFIX.length)].value = values[0]);
			const member = exports[name];
			if (member instanceof WebAssembly.Global) return member.value;
			if (!member) throw new Error(`${path} exports no ${name}`);
			const call = cCalls(holder.run).get(`${path}\t${name}`);
			if (call) return callC(holder, exports, member, call, values, `${path} ${name}`);
			const program = holder.exports;
			const copies = values.map(value => copied(program, exports, value));
			const result = copied(exports, program, member(...copies));
			values.forEach((value, index) => writtenBack(program, value, exports, copies[index]));
			return result;
		},
	});
}

// what the module changed in its copy of an argument, made to the program's own Node (src/modules/wasm_modules.rs write_back)
function writtenBack(program, value, module, copy) {
	if (copy === value || !isNode(value) || !isNode(copy)) return;
	const changed = readNode(module, copy);
	if (JSON.stringify(changed) !== JSON.stringify(readNode(program, value))) program.reflect_assign(value, buildValue(program, changed));
}

// a module warp compiled reads its Nodes (reader.js) and builds them (host.js buildValue)
const isWarpModule = exports => typeof exports.reflect_data === "function" && typeof exports.new_text === "function";
// a Node of one warp instance as a copy in the other (src/modules/wasm_modules.rs call_with_values): its texts live in the
// memory of the instance that made it
const isNode = value => value !== null && typeof value === "object";
const copied = (from, to, value) => isNode(value) && isWarpModule(from) && isWarpModule(to) ? buildValue(to, readNode(from, value)) : value;

// a promise a foreign call gave, awaited (the call suspends main, host.js JSPI); a refusal where main cannot wait
function awaitedPromise(holder, called, promise, valueOf) {
	if (!holder.waiting) {
		promise.then(undefined, () => {}); // its failure is not the program's: no unhandled rejection
		throw new Error(`${called}: ${PROMISE_REFUSAL}`);
	}
	holder.lastPromised = called; // the engine may refuse to suspend (host.js outcomeOf)
	const awaited = Promise.resolve(promise).then(valueOf, reason => { throw new Error(`${called}: the promise was rejected: ${reason?.message ?? reason}`); });
	awaited.catch(() => {}); // a refused suspension leaves it: no unhandled rejection
	return awaited;
}

// a program's foreign_call where its main waits (host.js JSPI) goes through this module: host.call gives the value, and
// when that is a promise (host.pending), host.await, a Suspending import, waits for it. The engine suspends wasm frames
// only, and a Suspending import always suspends, so the module calls it only for a promise.
// Assembled by wasm-tools from:
// (func (export "foreign_call") (param anyref anyref anyref anyref anyref) (result anyref) (local $given anyref)
//   (local.set $given (call $call (local.get 0) (local.get 1) (local.get 2) (local.get 3) (local.get 4)))
//   (if (result anyref) (call $pending) (then (call $await)) (else (local.get $given))))
const AWAITING_CALL_WASM = "AGFzbQEAAAABEgNgBW5ubm5uAW5gAAF/YAABbgIpAwRob3N0BGNhbGwAAARob3N0B3BlbmRpbmcAAQRob3N0BWF3YWl0AAIDAgEABxABDGZvcmVpZ25fY2FsbAADCh4BHAEBbiAAIAEgAiADIAQQACEFEAEEbhACBSAFCwsAKgRuYW1lARcDAARjYWxsAQdwZW5kaW5nAgVhd2FpdAIKAQMBBQVnaXZlbg==";
let awaitingCallModule;
function awaitingCall(call) {
	awaitingCallModule ??= new WebAssembly.Module(Uint8Array.from(atob(AWAITING_CALL_WASM), letter => letter.charCodeAt(0)));
	let promised;
	const host = {
		call: (...parts) => {
			const given = call(...parts);
			if (!(given instanceof Promise)) return given;
			promised = given;
			return null;
		},
		pending: () => Number(promised !== undefined),
		await: new WebAssembly.Suspending(() => {
			const promise = promised;
			promised = undefined;
			return promise;
		}),
	};
	return new WebAssembly.Instance(awaitingCallModule, { host }).exports.foreign_call;
}

addHostPart({
	words: (holder, hooks, { program }) => {
		// a module of another runtime (src/modules/foreign.rs), run by the runtime registered under its name
		const foreignCall = (runtime, module, member, call, argumentList) => {
			const program_ = program();
			const [runtimeName, moduleName, memberName] = [runtime, module, member].map(node => plainOfTree(readNode(program_, node)));
			const foreign = foreignRuntimes.get(runtimeName);
			if (!foreign) throw new Error(`${runtimeName} ${moduleName}.${memberName}: ${runtimeName} runs only in the native host (the warp CLI)`);
			const given = plainOfTree(readNode(program_, call)) === 1 ? plainOfTree(readNode(program_, argumentList)) : undefined;
			const argumentValues = given === undefined ? null : given === null ? [] : Array.isArray(given) ? given : [given];
			const valueOf = result => buildValue(program_, treeOfPlain(result));
			const result = foreign.call(moduleName, memberName, argumentValues, hooks);
			const called = `${runtimeName} ${typeof moduleName === "string" ? moduleName : "value"}.${memberName}`;
			return typeof result?.then === "function" ? awaitedPromise(holder, called, result, valueOf) : valueOf(result);
		};
		return {
			foreign_call: holder.awaited ? awaitingCall(foreignCall) : foreignCall,
		};
	},
	imports: (holder, hooks) => ({
		m: new Proxy(LIBM, { get: (libm, name) => libm[name] ?? Math[name] }),
		// the pure part of libc (ffi "c"): numbers, and C strings read up to their zero byte
		c: libcImports(holder, hooks),
		python: pythonLibrary(holder),
		// `use { memory, table } from "env"` (src/run/wasm_reader.rs define_imported_entities): a fresh memory and table
		env: {
			get memory() { return new WebAssembly.Memory({ initial: IMPORTED_MEMORY_PAGES }); },
			get table() { return new WebAssembly.Table({ initial: 0, element: "anyfunc" }); },
		},
	}),
	importModule: (holder, hooks, name) => MODULE_PATH.test(name) ? moduleImports(holder, hooks, name) : undefined,
});
