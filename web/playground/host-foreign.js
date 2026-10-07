// Foreign code in the page (a part of host.js, which says how parts work; needs host-files.js): foreign_call to the
// runtimes registered here (js, python), libm and libc.wasm for `use c`, and the WebAssembly modules a program imports
// by path

/// C names of libm functions (ffi "m") that Math spells differently
const LIBM = { fabs: Math.abs, fmin: Math.min, fmax: Math.max, fmod: (a, b) => a % b, ceil: Math.ceil, floor: Math.floor };
const LIBC_URL = new URL("lib/libc.wasm", self.location.href).href; // libc for `use c` (lib/build_libc.sh, P147)
const MODULE_PATH = /\.(wasm|wat)$/; // src/wasm_modules.rs MODULE_EXTENSIONS
const SETTER_PREFIX = "set "; // src/wasm_modules.rs SETTER_PREFIX: the import that sets a mutable global
const C_CALLS_SECTION = "warp.c_calls"; // src/wasm_modules.rs C_CALLS_SECTION
const IMPORTED_MEMORY_PAGES = 1; // src/wasm_emitter/mod.rs MEMORY: one page at least

// the runtimes foreign_call reaches in the page, by their name in `use <runtime> …`: call(module, member, arguments,
// hooks) gives the member's plain value (arguments null: a read, no call); prepare(code), when given, readies the runtime
// for a program before it runs (an asynchronous load), since a call itself is synchronous; a later registration of a
// name adds to the earlier one (worker.js gives python its prepare)
const foreignRuntimes = new Map();
function registerForeignRuntime(name, runtime) {
	foreignRuntimes.set(name, { ...foreignRuntimes.get(name), ...runtime });
}
const prepareForeignRuntimes = code => Promise.all([...foreignRuntimes.values()].map(runtime => runtime.prepare?.(code)));

// what wasp's operators on a value of the page forward to (src/lowering/foreign_modules.rs), as in src/foreign.rs's loop
const FOREIGN_OPERATORS = { add: (a, b) => a + b, sub: (a, b) => a - b, mul: (a, b) => a * b, truediv: (a, b) => a / b, mod: (a, b) => a % b, pow: (a, b) => a ** b,
	lt: (a, b) => a < b, gt: (a, b) => a > b, le: (a, b) => a <= b, ge: (a, b) => a >= b, eq: (a, b) => a === b, ne: (a, b) => a !== b, neg: a => -a,
	getitem: (a, i) => typeof a.get === "function" ? a.get(i) : a[i], len: a => a.length ?? a.size, list: a => Array.from(a) };

// `use python` in the page: the bridge of src/foreign.rs (foreign_python.py) in Pyodide, which worker.js loads before
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
		// a module's name, or a handle: an object of the page kept behind an id (src/foreign.rs)
		let owner = null, value = typeof moduleName === "object" ? unhandled(moduleName) : moduleName === "operator" ? FOREIGN_OPERATORS : globalThis[moduleName];
		if (value === undefined) throw new Error(`js ${moduleName}.${memberName}: the page has no global ${moduleName} (modules need the native host)`);
		for (const part of memberName.split(".")) {
			if (value?.[part] === undefined) throw new Error(`js ${moduleName}.${memberName}: ReferenceError: ${moduleName} has no ${memberName}`);
			[owner, value] = [value, value[part]];
		}
		return argumentValues === null ? value : value.apply(owner, argumentValues.map(unhandled));
	},
});

// the NUL-terminated bytes at `address` of a linear memory
function cText(memory, address) {
	const bytes = new Uint8Array(memory.buffer, address);
	const end = bytes.indexOf(0);
	return bytes.slice(0, end < 0 ? bytes.length : end);
}

// the program's C calls (src/wasm_modules.rs c_calls, which lists the letters) by "module\tname"
function cCalls(run) {
	if (!run.cCalls) {
		const lines = WebAssembly.Module.customSections(run.module, C_CALLS_SECTION).flatMap(section => decode(new Uint8Array(section)).split("\n"));
		run.cCalls = new Map(lines.filter(Boolean).map(line => line.split("\t")).map(([module, name, parameters, result]) =>
			[`${module}\t${name}`, { parameters: [...parameters], result }]));
	}
	return run.cCalls;
}

// a C function of a module, called with the program's values: each text (a NUL-terminated copy in the program's memory,
// or a pointer and length) is copied into a block of the module's malloc, each out-pointer gets a NULL slot there, each
// buffer a block of its capacity and a length slot, all freed after the call; a char * result (or char ** out-pointer,
// or the bytes a buffer received) is read back as a text of the program (NULL is ø)
function callC(holder, exports, member, call, values, what) {
	const program = holder.exports;
	const blocks = [];
	const outSlots = [];
	const buffers = [];
	const moduleArguments = [];
	const malloc = bytes => {
		if (!exports.malloc) throw new Error(`${what} takes a text, buffer or out-pointer, but its module exports no malloc(size)`);
		const block = exports.malloc(bytes.length);
		new Uint8Array(exports.memory.buffer).set(bytes, block);
		blocks.push(block);
		return block;
	};
	const u32 = count => new Uint8Array(new Uint32Array([count]).buffer);
	const remaining = [...values];
	(call?.parameters ?? values.map(() => "n")).forEach((letter, index) => {
		if (letter === "o") return moduleArguments.push(outSlots[outSlots.push(malloc(u32(0))) - 1]);
		if (letter === "k") return moduleArguments.push((buffers.at(-1).slot = malloc(u32(buffers.at(-1).capacity))));
		const value = remaining.shift();
		if (letter === "l") return; // the length of the text before it, libc.wasm reads up to the NUL
		if (letter === "b") return moduleArguments.push(buffers[buffers.push({ capacity: Number(value) }) - 1].block = malloc(new Uint8Array(Number(value))));
		if (letter !== "t") return moduleArguments.push(value);
		const length = ["l", "s"].includes(call.parameters[index + 1]) ? Number(remaining[0]) : undefined;
		const text = length === undefined ? cText(program.memory, value) : new Uint8Array(program.memory.buffer, value, length).slice();
		moduleArguments.push(malloc([...text, 0]));
	});
	const returned = member(...moduleArguments);
	const textOf = bytes => bytes === null ? program.new_empty() : program.new_text(...writeBytes(program, bytes));
	const memory = () => new DataView(exports.memory.buffer);
	const received = () => {
		const address = memory().getUint32(outSlots[0], true);
		if (address === 0) throw new Error(`${what} gave nothing through its out-pointer (C status ${returned ?? 0})`);
		return address;
	};
	const bufferBytes = () => {
		if (returned !== 0) throw new Error(`${what} failed (C status ${returned})`);
		const { block, slot } = buffers[0];
		return new Uint8Array(exports.memory.buffer, block, memory().getUint32(slot, true)).slice();
	};
	const value = {
		t: () => textOf(returned === 0 ? null : cText(exports.memory, returned)),
		u: () => BigInt(returned >>> 0),
		ot: () => textOf(cText(exports.memory, received())),
		on: () => received() | 0,
		b: () => textOf(bufferBytes()),
	}[call?.result]?.() ?? returned;
	if (exports.free) blocks.forEach(block => exports.free(block));
	return value;
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
function libcImports(holder) {
	return new Proxy({}, {
		get: (_, name) => (...values) => {
			const exports = libcExports();
			const member = exports[name];
			if (typeof member !== "function") throw new Error(`c.${name} is not available in the browser (web/playground/lib/build_libc.sh lists libc.wasm's functions)`);
			return callC(holder, exports, member, cCalls(holder.run).get(`c\t${name}`), values, `c.${name}`);
		},
	});
}

// the exports of a WebAssembly module the program imports by path (src/wasm_modules.rs link): instantiated once per
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
			return call ? callC(holder, exports, member, call, values, `${path} ${name}`) : member(...values);
		},
	});
}

addHostPart({
	words: (holder, hooks, { program }) => ({
		// a module of another runtime (src/foreign.rs), run by the runtime registered under its name
		foreign_call: (runtime, module, member, call, argumentList) => {
			const program_ = program();
			const [runtimeName, moduleName, memberName] = [runtime, module, member].map(node => plainOfTree(readNode(program_, node)));
			const foreign = foreignRuntimes.get(runtimeName);
			if (!foreign) throw new Error(`${runtimeName} ${moduleName}.${memberName}: ${runtimeName} runs only in the native host (the warp CLI)`);
			const given = plainOfTree(readNode(program_, call)) === 1 ? plainOfTree(readNode(program_, argumentList)) : undefined;
			const argumentValues = given === undefined ? null : given === null ? [] : Array.isArray(given) ? given : [given];
			return buildValue(program_, treeOfPlain(foreign.call(moduleName, memberName, argumentValues, hooks)));
		},
	}),
	imports: holder => ({
		m: new Proxy(LIBM, { get: (libm, name) => libm[name] ?? Math[name] }),
		// the pure part of libc (ffi "c"): numbers, and C strings read up to their zero byte
		c: libcImports(holder),
		// `use { memory, table } from "env"` (src/wasm_reader.rs define_imported_entities): a fresh memory and table
		env: {
			get memory() { return new WebAssembly.Memory({ initial: IMPORTED_MEMORY_PAGES }); },
			get table() { return new WebAssembly.Table({ initial: 0, element: "anyfunc" }); },
		},
	}),
	importModule: (holder, hooks, name) => MODULE_PATH.test(name) ? moduleImports(holder, hooks, name) : undefined,
});
