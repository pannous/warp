// What a host gives the warp compiler built without the native feature (src/web.rs `page`): `warp_host` runs the modules
// it compiles. A compiled program's imports mirror what the CLI's wasmtime runner links (src/host.rs, WASI fd_write,
// libm). Shared by the playground (worker.js) and the browser test runner (test-worker.js); needs reader.js.

const TEXT_HEAP_EXPORT = "text_heap";
const TRAP_DETAIL_EXPORT = "trap_detail";
const PAGE_BITS = 16;
const STDERR = 2;
/// where host.read and the test runner's file system find files: the repository root the static server serves
const FILE_ROOT = new URL("../../", self.location.href).href;
/// C names of libm functions (ffi "m") that Math spells differently
/// libc's RAND_MAX on glibc and macOS: rand() is 0…RAND_MAX
const RAND_MAX = 2147483647;
const LIBM = { fabs: Math.abs, fmin: Math.min, fmax: Math.max, fmod: (a, b) => a % b, ceil: Math.ceil, floor: Math.floor };

const HTTP_NOT_FOUND = "HTTP status 404";
const FILE_NOT_FOUND = "No such file or directory (os error 2)";

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

// a synchronous GET (host calls are synchronous, so this runs in a worker); `timeout` in ms
function getSync(url, timeout, binary = false) {
	const request = new XMLHttpRequest();
	request.open("GET", url, false);
	if (timeout) request.timeout = timeout;
	if (binary) request.overrideMimeType("text/plain; charset=x-user-defined"); // bytes as they are
	request.send();
	if (request.status >= 400 || request.status === 0) throw new Error(request.status ? `HTTP status ${request.status}` : "network error (blocked by CORS?)");
	return binary ? Uint8Array.from(request.responseText, character => character.charCodeAt(0) & 0xff) : request.responseText;
}

// a file of the served repository, failing in the words of the native read (src/host.rs)
function readFile(path) {
	try {
		return getSync(FILE_ROOT + path);
	} catch (failure) {
		throw failure.message === HTTP_NOT_FOUND ? new Error(FILE_NOT_FOUND) : failure;
	}
}

// the bytes of a file of the served repository or of a URL, failing in the words of the native read
function readBytes(path) {
	try {
		return getSync(/^https?:/.test(path) ? path : FILE_ROOT + path, 0, true);
	} catch (failure) {
		throw failure.message === HTTP_NOT_FOUND ? new Error(FILE_NOT_FOUND) : failure;
	}
}

// the body of a host call as (pointer, length), or (pointer, -length) of the failure reason, like src/host.rs
function hostResult(program, action, what) {
	try {
		let text = action();
		if (!text.endsWith("\n")) text += "\n"; // wasp convention (src/host.rs fetch)
		return writeBytes(program, utf8.encode(text));
	} catch (reason) {
		const [pointer, length] = writeBytes(program, utf8.encode(`${what} failed: ${reason.message ?? reason}`));
		return [pointer, -length];
	}
}

// hooks: print(text, fd), module(bytes) (each compiled module), panicked(message) (the compiler's)
function programImports(holder, hooks) {
	const program = () => holder.exports;
	const text = (pointer, length) => readText(program(), pointer, length);
	const cString = pointer => {
		const bytes = new Uint8Array(program().memory.buffer);
		let end = pointer;
		while (bytes[end] !== 0 && end < bytes.length) end++;
		return bytes.slice(pointer, end);
	};
	const bytes = (pointer, length) => new Uint8Array(program().memory.buffer).slice(pointer, pointer + length);
	// C's comparison: the difference of the first differing bytes, a text's end counting as byte 0
	const compareBytes = (a, b) => {
		for (let index = 0; index < Math.max(a.length, b.length); index++) {
			if (a[index] !== b[index]) return (a[index] ?? 0) - (b[index] ?? 0);
		}
		return 0;
	};
	const cNumber = pointer => parseFloat(new TextDecoder().decode(cString(pointer)));
	const fetchUrl = (pointer, length, timeout) => {
		const url = text(pointer, length);
		return hostResult(program(), () => getSync(url, timeout), `fetch ${url}`);
	};
	const known = {
		host: {
			fetch: (pointer, length) => fetchUrl(pointer, length),
			fetch_within: (pointer, length, milliseconds) => fetchUrl(pointer, length, Number(milliseconds)),
			// the file's bytes as they are, like src/host.rs read; a URL (a package's own files) is fetched
			read: (pointer, length) => {
				const path = text(pointer, length);
				try {
					return writeBytes(program(), readBytes(path));
				} catch (reason) {
					const [failed, failedLength] = writeBytes(program(), utf8.encode(`read ${path} failed: ${reason.message ?? reason}`));
					return [failed, -failedLength];
				}
			},
			warn: (pointer, length) => holder.warnings.push(text(pointer, length)),
			run: () => -1n,
			// the host words (src/host.rs): a page cannot block, so sleep busy-waits
			sleep: milliseconds => {
				const until = Date.now() + Number(milliseconds);
				while (Date.now() < until);
			},
			random: () => Math.random(),
			random_below: bound => bound > 0n ? BigInt(Math.floor(Math.random() * Number(bound))) : 0n,
			clock: () => BigInt(Date.now()),
			// tasks (src/tasks.rs): `go f(x)` runs f in a fresh instance of the program, values copied in and out; a page
			// without cross-origin isolation has no shared memory to wait on, so the task runs at once where it starts
			task_spawn: (name, a0, a1, a2, a3) => startTask(holder, hooks, decode(cString(name)), [a0, a1, a2, a3]),
			task_spawn_values: (name, values) => startTask(holder, hooks, decode(cString(name)), null, readTaskValue(program(), values)),
			task_await: id => {
				const task = finishedTask(holder.run, hooks, id);
				if (task.failure) throw new Error(task.failure);
				return task.value;
			},
			task_await_value: id => buildValue(program(), taskTree(finishedTask(holder.run, hooks, id).value)),
			task_join: id => finishedTask(holder.run, hooks, id).failure ? 1n : 0n,
			task_failure: id => buildValue(program(), textTree(finishedTask(holder.run, hooks, id).failure ?? "")),
			task_status: id => taskStatus(holder.run, id),
			task_control: (id, operation) => controlTask(holder.run, id, operation),
			// where a loop starts: a task paused from its starting program waits (its control word, set by controlTask)
			task_poll: () => {
				const control = holder.control;
				if (!control) return;
				while (Atomics.load(control, 0) === CONTROL_PAUSED) Atomics.wait(control, 0, CONTROL_PAUSED);
			},
			// shared arrays (src/shared.rs): Ints every task of the run reaches, in shared memory when the page is isolated
			shared_new: length => {
				const Buffer = self.crossOriginIsolated ? SharedArrayBuffer : ArrayBuffer;
				holder.run.shared.push(new BigInt64Array(new Buffer(8 * Math.max(0, Number(length)))));
				return BigInt(holder.run.shared.length);
			},
			shared_get: (id, index) => Atomics.load(...sharedCell(holder.run, id, index)),
			shared_set: (id, index, value) => (Atomics.store(...sharedCell(holder.run, id, index), value), value),
			shared_add: (id, index, value) => Atomics.add(...sharedCell(holder.run, id, index), value) + value,
			shared_count: id => BigInt(sharedArray(holder.run, id).length),
			// an array of floats: the cells hold the bits; an add swaps until no other task came between
			shared_getf: (id, index) => floatOfBits(Atomics.load(...sharedCell(holder.run, id, index))),
			shared_setf: (id, index, value) => (Atomics.store(...sharedCell(holder.run, id, index), bitsOfFloat(value)), value),
			shared_addf: (id, index, value) => {
				const [array, cell] = sharedCell(holder.run, id, index);
				for (;;) {
					const old = Atomics.load(array, cell);
					const sum = floatOfBits(old) + value;
					if (Atomics.compareExchange(array, cell, old, bitsOfFloat(sum)) === old) return sum;
				}
			},
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
		m: new Proxy(LIBM, { get: (libm, name) => libm[name] ?? Math[name] }),
		// the pure part of libc (ffi "c"): numbers, and C strings read up to their zero byte
		c: {
			rand: () => Math.floor(Math.random() * RAND_MAX),
			srand: () => {},
			abs: Math.abs,
			labs: value => value < 0n ? -value : value,
			// size_t and long are i64: BigInt; texts come as (pointer, length) pairs (src/ffi.rs signatures)
			strlen: pointer => BigInt(cString(pointer).length),
			strcmp: (a, aLength, b, bLength) => compareBytes(bytes(a, aLength), bytes(b, bLength)),
			strncmp: (a, aLength, b, bLength, count) => {
				const prefix = Number(count);
				return compareBytes(bytes(a, Math.min(aLength, prefix)), bytes(b, Math.min(bLength, prefix)));
			},
			atoi: pointer => Math.trunc(cNumber(pointer)) | 0,
			atol: pointer => BigInt(Math.trunc(cNumber(pointer)) || 0),
			atof: pointer => cNumber(pointer) || 0,
		},
	};
	// anything else (native FFI libraries) is missing in the browser: say which, when the program calls it
	const missing = (module, name) => () => { throw new Error(`${module}.${name} is not available in the browser`); };
	return new Proxy(known, {
		get: (modules, module) => new Proxy(modules[module] ?? {}, {
			get: (functions, name) => functions[name] ?? missing(module, String(name)),
		}),
	});
}

const TASK_FINISHED = 1n; // src/host.rs TASK_FINISHED, TASK_FAILED, TASK_STOPPED, TASK_STOP
const TASK_FAILED = 2n;
const TASK_STOPPED_CODE = 3n;
const TASK_STOP = 1n;
const TASK_PAUSE = 2n; // src/host.rs TASK_PAUSE, TASK_RESUME, TASK_PAUSED
const TASK_RESUME = 3n;
const TASK_PAUSED = 4n;
const CONTROL_RUNNING = 0; // a task Worker's control word
const CONTROL_PAUSED = 1;
const KIND_MASK = 0xFFn;
const decode = bytes => utf8Decoder.decode(bytes);

const TASK_WORKER = "task-worker.js";
const TASK_RESULT_BYTES = 1 << 16; // the shared buffer a Worker writes its result into, grown as needed
const TASK_RESULT_LIMIT = 1 << 28;
const TASK_HEADER = 8; // [state, length] as Int32, then the result's JSON
const TASK_STOPPED = "task stopped";
// Workers a task runs on: made while the program's worker is idle (prepareTaskPool), since a Worker only starts once
// its creator returns to its event loop and a running program never does (emscripten keeps a thread pool for this)
const taskPool = [];

// the pool of task Workers, made by the workers that run programs (worker.js, test-worker.js) when they start
function prepareTaskPool(size = Math.min(4, self.navigator?.hardwareConcurrency ?? 2)) {
	if (!self.crossOriginIsolated || !self.Worker) return;
	for (let index = 0; index < size; index++) {
		const worker = new Worker(TASK_WORKER);
		worker.onmessage = () => taskPool.push(worker); // loaded: it can take tasks
	}
}

const floatBits = new DataView(new ArrayBuffer(8));
const floatOfBits = bits => (floatBits.setBigInt64(0, bits), floatBits.getFloat64(0));
const bitsOfFloat = number => (floatBits.setFloat64(0, number), floatBits.getBigInt64(0));

function sharedArray(run, id) {
	const array = run.shared[Number(id) - 1];
	if (!array) throw new WebAssembly.RuntimeError(`no shared array ${id}`);
	return array;
}

// the array and the cell of a 1-based index; out of range is the runtime error natively too
function sharedCell(run, id, index) {
	const array = sharedArray(run, id);
	const cell = Number(index) - 1;
	if (!(cell >= 0 && cell < array.length)) throw new WebAssembly.RuntimeError("index out of range");
	return [array, cell];
}

// a task of the run: f(arguments) in a fresh instance of the program (src/tasks.rs TaskTable::run), the Int arguments as
// they are or the argument list (`values`, a tree of reader.js) rebuilt for a wrapper f·node. On a Worker of the pool
// (shared memory needs cross-origin isolation), which writes the result into a SharedArrayBuffer; else at once, here
function startTask(holder, hooks, name, ints, values) {
	const run = holder.run;
	const id = BigInt(run.tasks.size + 1);
	const captured = capturedValues(holder.exports, run.module);
	if (taskPool.length > 0) {
		const shared = new SharedArrayBuffer(TASK_HEADER + TASK_RESULT_BYTES, { maxByteLength: TASK_RESULT_LIMIT });
		const worker = taskPool.pop();
		const control = new Int32Array(new SharedArrayBuffer(4));
		worker.postMessage({ module: run.module, name, ints, values, shared, arrays: run.shared, captured, control });
		run.tasks.set(id, { name, worker, shared, control });
	} else {
		run.tasks.set(id, runTask(run.module, hooks, holder.warnings, name, ints, values, run.shared, captured));
	}
	return id;
}

// the task here: {value} (a number or a tree) or {failure}
function runTask(module, hooks, warnings, name, ints, values, arrays, captured = [], control = null) {
	const taskHolder = { warnings, run: { module, tasks: new Map(), shared: arrays }, control };
	try {
		const instance = new WebAssembly.Instance(module, programImports(taskHolder, hooks));
		taskHolder.exports = instance.exports;
		// what the spawning instance's closures captured, as it had it
		for (const [global, value] of captured) instance.exports[global].value = value.tree ? buildValue(instance.exports, value.tree) : value.raw;
		const callee = instance.exports[name];
		const result = values ? callee(buildValue(instance.exports, values)) : callee(...ints.slice(0, callee.length));
		joinTasks(taskHolder.run, hooks);
		return { value: typeof result === "object" && result !== null ? readNode(instance.exports, result) : result };
	} catch (trap) {
		return { failure: `task ${name}: ${trapMessage(trap, name)}` };
	}
}

// the task's record once it is done: a Worker's is waited for (the program runs in a worker, which may block) and
// read from its shared buffer, its output printed then
function finishedTask(run, hooks, id) {
	const task = run.tasks.get(id);
	if (!task.worker) return task;
	const header = new Int32Array(task.shared, 0, 2);
	Atomics.wait(header, 0, 0);
	const record = JSON.parse(decode(new Uint8Array(task.shared, TASK_HEADER, header[1]).slice()));
	if (record.output) hooks.print(record.output, 1);
	taskPool.push(task.worker); // free for the next task
	if (record.value?.kind === KIND_INT && record.ints) record.value = BigInt(record.value.data.int);
	run.tasks.set(id, record);
	return record;
}

function joinTasks(run, hooks) {
	for (const id of run.tasks.keys()) finishedTask(run, hooks, id);
}

// running, finished or failed, without waiting (src/host.rs task status codes)
function taskStatus(run, id) {
	const task = run.tasks.get(id);
	if (task.worker && Atomics.load(new Int32Array(task.shared, 0, 1), 0) === 0) return Atomics.load(task.control, 0) === CONTROL_PAUSED ? TASK_PAUSED : 0n;
	const record = task.worker ? JSON.parse(decode(new Uint8Array(task.shared, TASK_HEADER, new Int32Array(task.shared, 0, 2)[1]).slice())) : task;
	if (!record.failure) return TASK_FINISHED;
	return record.failure.endsWith(TASK_STOPPED) ? TASK_STOPPED_CODE : TASK_FAILED;
}

// `stop job` ends a Worker; `pause job` / `resume job` set its control word, which the task reads where a loop starts
function controlTask(run, id, operation) {
	const task = run.tasks.get(id);
	if (!task.worker || Atomics.load(new Int32Array(task.shared, 0, 1), 0) !== 0) return 0n;
	if (operation === TASK_PAUSE || operation === TASK_RESUME) {
		Atomics.store(task.control, 0, operation === TASK_PAUSE ? CONTROL_PAUSED : CONTROL_RUNNING);
		Atomics.notify(task.control, 0);
		return 1n;
	}
	task.worker.terminate();
	run.tasks.set(id, { failure: `task ${task.name}: ${TASK_STOPPED}` });
	return 1n;
}

// the runtime error a trap means: the first runtime error function on its stack (`index_out_of_range` reads "index out
// of range", as src/wasm_emitter list_ops runtime_error_message has it), else the engine's words
function trapMessage(trap, callee) {
	// V8 names a wasm frame `module.function (wasm://…)`
	const names = [...String(trap.stack ?? "").matchAll(/at (?:\w+\.)?([a-z]+(?:_[a-z]+)+) \(wasm/g)].map(match => match[1]);
	const runtime = names.find(name => name !== callee);
	return runtime ? runtime.replaceAll("_", " ") : String(trap.message ?? trap);
}

const textTree = text => ({ kind: "3", data: { text }, chain: [] });
const KIND_FUNCTION = 16n; // src/type_kinds.rs Kind::Function: a closure
const KIND_LIST = 8n;
const CAPTURE_PREFIX = "capture·"; // src/wasm_emitter CAPTURE_EXPORT_PREFIX

// a value of the program for a task: a closure as its target's name and captured values ({closure, captured}, rebuilt
// by the module's closure_rebuild), a list item by item, anything else as reader.js reads it
function readTaskValue(module, node) {
	if (node === null) return { kind: "0", data: null, chain: [] };
	const kind = BigInt(module.get_kind(node));
	if ((kind & KIND_MASK) === KIND_FUNCTION) {
		const captured = module.closure_captured(node);
		return { closure: readNode(module, module.reflect_value(node)).data.text, captured: captured === null ? null : readTaskValue(module, captured) };
	}
	if ((kind & KIND_MASK) !== KIND_LIST) return readNode(module, node);
	const items = [];
	for (let cell = node; cell !== null; cell = module.reflect_value(cell)) {
		const item = module.reflect_data(cell);
		if (item !== null) items.push(readTaskValue(module, item));
	}
	return { kind: String(kind), data: null, chain: [], items };
}

// the capture globals of the program's closures (src/tasks.rs captured): their values now, for the task's instance
function capturedValues(exports, module) {
	const names = WebAssembly.Module.exports(module).map(entry => entry.name).filter(name => name.startsWith(CAPTURE_PREFIX));
	return names.map(name => {
		const value = exports[name].value;
		return [name, typeof value === "object" && value !== null ? { tree: readTaskValue(exports, value) } : { raw: value }];
	});
}

// a task's result as a tree: a number of a function of numbers, else the tree already read
function taskTree(value) {
	if (typeof value === "bigint") return { kind: KIND_INT, data: { int: String(value) }, chain: [] };
	if (typeof value === "number") return { kind: KIND_FLOAT, data: { float: value }, chain: [] };
	return value ?? { kind: "0", data: null, chain: [] };
}

// the inverse of reader.js readNode for the values a task carries (src/tasks.rs TaskValue): built through the module's
// constructors; a list from its cons cells, the first item in data, the next cells in the chain
function buildValue(module, tree) {
	if (tree.closure !== undefined) {
		const name = buildValue(module, { kind: "5", data: { text: tree.closure }, chain: [] });
		return module.closure_rebuild(name, tree.captured === null ? null : buildValue(module, tree.captured));
	}
	if (tree.items) return tree.items.reduceRight((rest, item) => module.new_list(buildValue(module, item), rest, BigInt(tree.kind) >> 8n), null) ?? module.new_empty();
	const kind = BigInt(tree.kind);
	const payload = tree.data ?? {};
	switch (Number(kind & KIND_MASK)) {
		case 0: return module.new_empty();
		case 1:
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

// run a compiled program: the outcome src/web.rs run_outcome reads
function runProgram(bytes, hooks) {
	let instance;
	const holder = { warnings: [] }; // the runtime warnings go back to the compiler, which reports them (src/web.rs)
	try {
		holder.run = { module: new WebAssembly.Module(bytes), tasks: new Map(), shared: [] };
		instance = new WebAssembly.Instance(holder.run.module, programImports(holder, hooks));
		holder.exports = instance.exports;
	} catch (failure) {
		return { failure: String(failure.message ?? failure) };
	}
	const { warnings } = holder;
	try {
		const result = instance.exports.main();
		joinTasks(holder.run, hooks); // the tasks nobody awaited finish before the result, as natively
		return { result: readResult(instance.exports, result), warnings };
	} catch (trap) {
		if (!(trap instanceof WebAssembly.RuntimeError || trap instanceof RangeError)) return { failure: String(trap.message ?? trap), warnings };
		const detail = instance.exports[TRAP_DETAIL_EXPORT]?.value;
		return { trap: trap.message, trace: trap.stack ?? "", detail: detail ? readNode(instance.exports, detail) : null, warnings };
	}
}

// the `warp_host` imports of a compiler instance; `memory()` is its memory (known only after instantiation)
function warpHost(memory, hooks) {
	let pendingOutcome; // the JSON a run left for warp_host.take
	let pendingFetched; // the text a fetch left for warp_host.take_fetched
	return {
		run: (pointer, length) => {
			const bytes = new Uint8Array(memory().buffer, pointer, length).slice();
			hooks.module?.(bytes);
			pendingOutcome = utf8.encode(JSON.stringify(runProgram(bytes, hooks)));
			return pendingOutcome.length;
		},
		take: into => new Uint8Array(memory().buffer, into, pendingOutcome.length).set(pendingOutcome),
		now_ms: () => Date.now(),
		panicked: (pointer, length) => hooks.panicked(utf8Decoder.decode(new Uint8Array(memory().buffer, pointer, length))),
		// a module file or package source for the compiler: a path of the served repository, or a URL
		fetch: (pointer, length) => {
			const address = utf8Decoder.decode(new Uint8Array(memory().buffer, pointer, length));
			try {
				pendingFetched = utf8.encode(/^https?:/.test(address) ? getSync(address) : readFile(address.replace(/^\.\//, "")));
				return pendingFetched.length;
			} catch {
				return -1;
			}
		},
		take_fetched: into => new Uint8Array(memory().buffer, into, pendingFetched.length).set(pendingFetched),
	};
}
