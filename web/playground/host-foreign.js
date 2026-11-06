// Foreign code in the page (a part of host.js, which says how parts work; needs host-files.js): foreign_call to the
// runtimes registered here (js, python), libm and libc.wasm for `use c`, and the WebAssembly modules a program imports
// by path

/// C names of libm functions (ffi "m") that Math spells differently
const LIBM = { fabs: Math.abs, fmin: Math.min, fmax: Math.max, fmod: (a, b) => a % b, ceil: Math.ceil, floor: Math.floor };
const LIBC_URL = new URL("lib/libc.wasm", self.location.href).href; // libc for `use c` (lib/build_libc.sh, P147)
const LIBC_LIST = "web/playground/lib/build_libc.sh lists libc.wasm's functions";
const C_FAILED = -1; // a C file call's answer when it failed (unlink, rename, rmdir)
const MODULE_PATH = /\.(wasm|wat)$/; // src/modules/wasm_modules.rs MODULE_EXTENSIONS
const SETTER_PREFIX = "set "; // src/modules/wasm_modules.rs SETTER_PREFIX: the import that sets a mutable global
// the C functions a module calls, one line each: import module, library, name, parameter kinds (comma-separated),
// result register, what the result crosses as, an out-pointer's pointee, -, how a failure shows
// (core/emitter/c_functions.warp, read as core/native/driver.warp foreign_of reads it)
const C_FUNCTIONS_SECTION = "warp.c_functions";
const IMPORTED_MEMORY_PAGES = 1; // src/wasm_emitter/mod.rs MEMORY: one page at least
const PROMISE_REFUSAL = "gives a promise, which a program in the page cannot wait for; natively (the warp CLI) node awaits it";

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
const NULL_POINTER = 0;
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
		if (kind === "out") return out === undefined ? out = malloc(new Uint8Array(4)) : NULL_POINTER; // a later out-pointer NULL
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
// FILE * is its descriptor, what it puts goes to the run's output; dlopen of warp's own libraries; getenv of the run's
// environment (driver.js programEnvironment), NULL for a name it lacks
const pageStdio = (holder, hooks) => ({
	getenv: name => environmentValue(holder.environment ?? ENVIRONMENT, name),
	fdopen: descriptor => descriptor,
	fputs: (text, descriptor) => (hooks.print(text, descriptor), 0),
	fflush: () => 0,
	dlopen: path => self.PAGE_LIBRARIES?.has(path.match(OWN_LIBRARY)?.[1]) ? PAGE_LIBRARY_HANDLE : 0,
	// lib/file.warp's file words over the page's written files (host-files.js), as C answers: 0 done, -1 failed; folders
	// exist by the files in them
	unlink: path => writtenFiles.has(filePath(path)) ? (forgetWritten(path), 0) : C_FAILED,
	rename: (from, to) => writtenFiles.has(filePath(from)) ? (keepWritten(to, writtenFiles.get(filePath(from))), forgetWritten(from), 0) : C_FAILED,
	mkdir: () => 0,
	rmdir: folder => isWrittenFolder(folder) ? C_FAILED : 0,
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
	const stdio = libraryInJs(holder, "c", name => pageStdio(holder, hooks)[name]);
	const libc = wasmLibraryImports(holder, "c", libcExports, LIBC_LIST);
	return new Proxy({}, { get: (_, name) => name in pageStdio(holder, hooks) ? stdio[name] : libc[name] });
}
const environmentValue = (environment, name) => environment.find(entry => entry.startsWith(`${name}=`))?.slice(name.length + 1) ?? null;

// libm (ffi "m"): Math, under C's names where they differ, else libc.wasm's musl (erf, ldexp, sqrtf …)
function libmImports(holder) {
	const musl = wasmLibraryImports(holder, "m", libcExports, LIBC_LIST);
	return new Proxy(LIBM, { get: (libm, name) => libm[name] ?? Math[name] ?? musl[name] });
}

// a C library the page has as a WebAssembly module, natively a shared library: exportsOf() its exports, memory and
// malloc among them; each program call goes through callC
function wasmLibraryImports(holder, library, exportsOf, missing = "") {
	return new Proxy({}, {
		get: (_, name) => (...values) => {
			const exports = exportsOf();
			const member = exports[name];
			if (typeof member !== "function") throw new Error(`${library}.${name} is not available in the browser${missing && ` (${missing})`}`);
			return callC(holder, exports, member, cCalls(holder.run).get(`${library}\t${name}`), values, `${library}.${name}`);
		},
	});
}

// a package's C library the page has as lib/<name>.wasm (build.sh pack_package_libraries; natively lib<name> beside
// warp, scripts/link-packages.sh): instantiated once per worker at its first call, else undefined
const packedLibraries = new Map();
function packedLibraryImports(holder, name) {
	const file = `${PAGE_PREFIX}lib/${name}.wasm`;
	if (!/^\w+$/.test(name) || !servedFileExists(file)) return undefined;
	const exportsOf = () => {
		if (!packedLibraries.has(name)) {
			const bytes = new Uint8Array(getSync(fileUrl(file), 0, true));
			packedLibraries.set(name, typedExports(bytes, new WebAssembly.Instance(new WebAssembly.Module(bytes), {}).exports));
		}
		return packedLibraries.get(name);
	};
	return wasmLibraryImports(holder, name, exportsOf);
}

// the exported functions of a module's bytes, each taking its arguments as its parameters' types are: a C library's
// pointer is an i32 in wasm32 where the program declares it an i64 (sqlite3_bind_text's destructor), an i64 a BigInt
const WASM_ARGUMENT = { 0x7f: value => Number(BigInt.asIntN(32, BigInt(value))), 0x7e: value => BigInt(value), 0x7d: Number, 0x7c: Number };
function typedExports(bytes, exports) {
	const parameters = exportedParameters(bytes);
	return Object.fromEntries(Object.entries(exports).map(([name, member]) => {
		const types = parameters.get(name);
		return [name, types ? (...values) => member(...values.map((value, index) => WASM_ARGUMENT[types[index]]?.(value) ?? value)) : member];
	}));
}

// export name → its function's parameter types (the binary's type, import, function and export sections)
function exportedParameters(bytes) {
	let at = 8; // after the magic and the version
	const unsigned = () => {
		let value = 0, shift = 0, byte;
		do { byte = bytes[at++]; value += (byte & 0x7f) * 2 ** shift; shift += 7; } while (byte & 0x80);
		return value;
	};
	const vector = item => Array.from({ length: unsigned() }, item);
	const name = () => { const length = unsigned(); at += length; return decode(bytes.subarray(at - length, at)); };
	let signatures = [], functions = [];
	const exported = new Map();
	const IMPORT_SKIPS = { 1: () => { at++; limits(); }, 2: () => limits(), 3: () => { at += 2; } }; // table, memory, global
	const limits = () => { const flags = unsigned(); unsigned(); if (flags & 1) unsigned(); };
	const SECTIONS = {
		1: () => { signatures = vector(() => { at++; const parameters = vector(() => bytes[at++]); vector(() => bytes[at++]); return parameters; }); },
		2: () => vector(() => { name(); name(); const kind = bytes[at++]; if (kind === 0) functions.push(unsigned()); else IMPORT_SKIPS[kind](); }),
		3: () => { functions.push(...vector(unsigned)); },
		7: () => vector(() => { const exportName = name(); const kind = bytes[at++]; const index = unsigned(); if (kind === 0) exported.set(exportName, signatures[functions[index]]); }),
	};
	while (at < bytes.length) {
		const id = bytes[at++];
		const size = unsigned();
		const end = at + size;
		SECTIONS[id]?.();
		at = end;
	}
	return exported;
}

// the exports of a WebAssembly module the program imports by path (src/modules/wasm_modules.rs link): instantiated once per
// run at the first call, with its own imports linked like the program's; a global reads through a getter and is set
// through `set g`
function moduleImports(holder, hooks, path) {
	const instance = () => {
		const modules = holder.run.wasmModules ??= new Map();
		// one instance however the path is spelled; a path in the compiler's repository folder (a `use cargo` crate in
		// /warp/web/playground/warp-lib/crates) is that file of the served repository
		const served = path.startsWith(REPOSITORY_DIRECTORY + "/") ? path.slice(REPOSITORY_DIRECTORY.length + 1) : path;
		const file = new URL(served, FILE_ROOT).href;
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
			return member(...values);
		},
	});
}

// a promise a foreign call gave: refused, as no run here waits (core/native/driver.warp's node awaits it)
function refusedPromise(called, promise) {
	promise.then(undefined, () => {}); // its failure is not the program's: no unhandled rejection
	throw new Error(`${called}: ${PROMISE_REFUSAL}`);
}

addHostPart({
	words: (holder, hooks, { program }) => {
		// a module of another runtime (src/modules/foreign.rs), run by the runtime registered under its name
		const foreignCall = (runtime, module, member, call, argumentList) => {
			const program_ = program();
			const [runtimeName, moduleName, memberName] = [runtime, module, member].map(node => guestPlain(program_, node));
			const foreign = foreignRuntimes.get(runtimeName);
			if (!foreign) throw new Error(`${runtimeName} ${moduleName}.${memberName}: ${runtimeName} runs only in the native host (the warp CLI)`);
			const given = guestPlain(program_, call) === 1 ? guestPlain(program_, argumentList) : undefined;
			const argumentValues = given === undefined ? null : given === null ? [] : Array.isArray(given) ? given : [given];
			const valueOf = result => newGuestValue(program_, result);
			const result = foreign.call(moduleName, memberName, argumentValues, hooks);
			const called = `${runtimeName} ${typeof moduleName === "string" ? moduleName : "value"}.${memberName}`;
			return typeof result?.then === "function" ? refusedPromise(called, result) : valueOf(result);
		};
		return {
			foreign_call: foreignCall,
		};
	},
	imports: (holder, hooks) => ({
		m: libmImports(holder),
		// the pure part of libc (ffi "c"): numbers, and C strings read up to their zero byte
		c: libcImports(holder, hooks),
		python: pythonLibrary(holder),
		// `use { memory, table } from "env"` (src/run/wasm_reader.rs define_imported_entities): a fresh memory and table
		env: {
			get memory() { return new WebAssembly.Memory({ initial: IMPORTED_MEMORY_PAGES }); },
			get table() { return new WebAssembly.Table({ initial: 0, element: "anyfunc" }); },
		},
	}),
	importModule: (holder, hooks, name) => MODULE_PATH.test(name) ? moduleImports(holder, hooks, name) : packedLibraryImports(holder, name),
});
