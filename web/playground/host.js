// What a host gives the warp compiler built without the native feature (src/web.rs `page`): `warp_host` runs the modules
// it compiles. A compiled program's imports mirror what the CLI's wasmtime runner links (src/host.rs, WASI fd_write,
// libm). Shared by the playground (worker.js) and the browser test runner (test-worker.js); needs reader.js.

const TEXT_HEAP_EXPORT = "text_heap";
const TRAP_DETAIL_EXPORT = "trap_detail";
// the checks of the listeners on shared values (src/lowering/signal_values.rs), run at every check point
const SHARED_HANDLER = "on·shared";
const SHARED_CHECK_MILLISECONDS = 10;
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
			// a block known only at run time (src/host.rs run_block): a compiler instance of its own compiles and runs it
			run_block: (block, names, values, definitions) => {
				const module = program();
				const request = { block: readNode(module, block), names: readNode(module, names), values: readNode(module, values), definitions: readNode(module, definitions) };
				const report = evalBlock(hooks, JSON.stringify(request));
				if (report.error !== undefined) {
					holder.blockError = report.error;
					throw new Error(report.error);
				}
				return buildValue(module, report.result);
			},
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
			// the host words (src/host.rs): a page cannot block, so sleep busy-waits
			sleep: milliseconds => {
				const until = Date.now() + Number(milliseconds);
				let check = Date.now() + SHARED_CHECK_MILLISECONDS;
				while (Date.now() < until) {
					if (Date.now() >= check) {
						checkShared(holder);
						check = Date.now() + SHARED_CHECK_MILLISECONDS;
					}
				}
				checkShared(holder);
			},
			random: () => Math.random(),
			random_below: bound => bound > 0n ? BigInt(Math.floor(Math.random() * Number(bound))) : 0n,
			clock: () => BigInt(Date.now()),
			// a page has no ctrl-c: `on interrupt {…}` never runs here (notes/system_signals.md); shared listeners do
			signal_poll: () => checkShared(holder),
			signal_every: () => { holder.warnings.push("on every …: timers do not run in the playground yet"); },
			signal_watch: () => { holder.warnings.push("on file … change: a page has no files to watch"); },
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
			// tasks (src/tasks.rs): `go f(x)` runs f in a fresh instance of the program, values copied in and out; a page
			// without cross-origin isolation has no shared memory to wait on, so the task runs at once where it starts
			task_spawn: (name, a0, a1, a2, a3) => startTask(holder, hooks, decode(cString(name)), [a0, a1, a2, a3]),
			task_spawn_values: (name, values) => startTask(holder, hooks, decode(cString(name)), null, readTaskValue(program(), values)),
			task_await: id => {
				const task = takenTask(holder.run, hooks, id);
				if (task.failure) throw new Error(task.failure);
				return task.value;
			},
			task_await_value: id => buildValue(program(), taskTree(takenTask(holder.run, hooks, id).value)),
			// every await joins first: the raises the task forwarded run here, before its value is read
			task_join: id => {
				const failed = takenTask(holder.run, hooks, id).failure ? 1n : 0n;
				deliverSignals(holder);
				return failed;
			},
			task_failure: id => buildValue(program(), textTree(takenTask(holder.run, hooks, id).failure ?? "")),
			task_status: id => {
				const status = taskStatus(holder.run, id);
				deliverSignals(holder);
				return status;
			},
			// a raise inside a task goes to the starting program's handlers (src/tasks.rs signal_send, deliver)
			task_inside: () => holder.inTask ? 1n : 0n,
			signal_send: (handler, values) => {
				(holder.run.signals ??= []).push({ handler: decode(cString(handler)), values: readTaskValue(program(), values) });
				return 0n;
			},
			task_control: (id, operation) => controlTask(holder.run, id, operation),
			// where a loop starts: a task paused from its starting program waits (its control word, set by controlTask)
			task_poll: () => {
				deliverSignals(holder);
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
const TASK_POOL_SIZE = Math.min(4, self.navigator?.hardwareConcurrency ?? 2);
// how long a run waits for the pool's Workers to load before it starts anyway (a task then runs inline)
const TASK_POOL_WAIT_MS = 10000;
const TASK_POOL_POLL_MS = 10;
const hasTaskWorkers = () => self.crossOriginIsolated && self.Worker;

function addTaskWorker() {
	const worker = new Worker(TASK_WORKER);
	worker.onmessage = () => taskPool.push(worker); // loaded: it can take tasks
}

// the pool of task Workers, made by the workers that run programs (worker.js, test-worker.js) when they start
function prepareTaskPool(size = TASK_POOL_SIZE) {
	if (!hasTaskWorkers()) return;
	for (let index = 0; index < size; index++) addTaskWorker();
}

// resolves once every task Worker of the pool has loaded: a run that starts before would run its tasks inline, where
// `stop` cannot end one (samples/threads.wasp's endless spin hung the browser suite, card flaky-browser)
function taskPoolReady() {
	if (!hasTaskWorkers()) return Promise.resolve();
	const deadline = performance.now() + TASK_POOL_WAIT_MS;
	return new Promise(resolve => {
		const check = () => (taskPool.length >= TASK_POOL_SIZE || performance.now() > deadline) ? resolve() : setTimeout(check, TASK_POOL_POLL_MS);
		check();
	});
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

// the task's function as the program wrote it (src/tasks.rs task_name): `f`, not `f·node`; `go block 1`, not `go·block·1`
function taskName(name) {
	return name.replace(/·node$/, "").replace(/^go·block·/, "go block ");
}

// the task here: {value} (a number or a tree) or {failure}
function runTask(module, hooks, warnings, name, ints, values, arrays, captured = [], control = null) {
	const taskHolder = { warnings, run: { module, tasks: new Map(), shared: arrays }, control, inTask: true };
	let instance;
	try {
		instance = new WebAssembly.Instance(module, programImports(taskHolder, hooks));
		taskHolder.exports = instance.exports;
		// what the spawning instance's closures captured, as it had it
		for (const [global, value] of captured) instance.exports[global].value = value.tree ? buildValue(instance.exports, value.tree) : value.raw;
		const callee = instance.exports[name];
		const result = values ? callee(buildValue(instance.exports, values)) : callee(...ints.slice(0, callee.length));
		const unread = joinTasks(taskHolder.run, hooks);
		if (unread) return { failure: `task ${taskName(name)}: ${unread}`, signals: taskHolder.run.signals };
		return { value: typeof result === "object" && result !== null ? readNode(instance.exports, result) : result, signals: taskHolder.run.signals };
	} catch (trap) {
		return { failure: `task ${taskName(name)}: ${raisedText(instance?.exports) ?? trapMessage(trap, name)}`, signals: taskHolder.run.signals };
	}
}

// the task's record once it is done: a Worker's is waited for (the program runs in a worker, which may block) and
// read from its shared buffer, its output printed then
function finishedTask(run, hooks, id) {
	const task = run.tasks.get(id);
	if (!task.worker) return queuedSignals(run, task);
	const header = new Int32Array(task.shared, 0, 2);
	Atomics.wait(header, 0, 0);
	const record = JSON.parse(decode(new Uint8Array(task.shared, TASK_HEADER, header[1]).slice()));
	if (record.output) hooks.print(record.output, 1);
	taskPool.push(task.worker); // free for the next task
	if (record.value?.kind === KIND_INT && record.ints) record.value = BigInt(record.value.data.int);
	run.tasks.set(id, record);
	return queuedSignals(run, record);
}

// the raises a finished task forwarded, queued once for the program (deliverSignals)
function queuedSignals(run, record) {
	if (record.signals?.length && !record.signalsQueued) {
		(run.signals ??= []).push(...record.signals);
		record.signalsQueued = true;
	}
	return record;
}

// the raises tasks forwarded, run in the program's instance by their handlers' node wrappers; a task leaves them to
// the program (its record carries them)
function deliverSignals(holder) {
	if (holder.inTask) return;
	const queue = holder.run.signals ?? [];
	while (queue.length > 0) {
		const { handler, values } = queue.shift();
		holder.exports[handler](buildValue(holder.exports, values));
	}
}

// the listeners on shared values look whether a task changed them; a task leaves that to the program
function checkShared(holder) {
	if (!holder.inTask) holder.exports?.[SHARED_HANDLER]?.();
}

// the task's record, read by the program (await, join): a failure nobody read ends the run (joinTasks)
function takenTask(run, hooks, id) {
	(run.taken ??= new Set()).add(id);
	return finishedTask(run, hooks, id);
}

// every task done; the first failure nobody read, but a stopped task's (src/tasks.rs join_all)
function joinTasks(run, hooks) {
	let unread;
	for (const id of run.tasks.keys()) {
		const { failure } = finishedTask(run, hooks, id);
		if (failure && unread === undefined && !failure.endsWith(TASK_STOPPED) && !run.taken?.has(id)) unread = failure;
	}
	return unread;
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
	addTaskWorker(); // the pool's replacement, loaded once this run returns to its event loop
	run.tasks.set(id, { failure: `task ${task.name}: ${TASK_STOPPED}` });
	return 1n;
}

// the message a raised error left in trap_detail before trapping (src/wasm_reader.rs with_trap_detail), if any
function raisedText(exports) {
	const detail = exports?.[TRAP_DETAIL_EXPORT]?.value;
	if (!detail) return null;
	const tree = readNode(exports, detail);
	return tree.data?.text ?? tree.data?.node?.data?.text ?? null;
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
	// a key (`value: 5`, a field of a raised event): left and right, as buildValue rebuilds it
	if ((kind & KIND_MASK) === KIND_KEY) return { kind: String(kind), key: [readTaskValue(module, module.reflect_data(node)), readTaskValue(module, module.reflect_value(node))] };
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

// a value of the program (a reader.js tree) as a plain JavaScript value, for foreign_call: lists arrays, `{a:1}` objects
const KIND_KEY = 6n;
function plainOfTree(tree) {
	const kind = BigInt(tree.kind);
	const payload = tree.data ?? {};
	const items = () => [payload.node, ...(tree.chain ?? []).map(cell => cell.data?.node)].filter(Boolean);
	switch (Number(kind & KIND_MASK)) {
		case 0: return null;
		case 1:
			if (payload.ratio) return Number(payload.ratio[0].int) / Number(payload.ratio[1].int);
			return payload.int === undefined ? null : Number(payload.int);
		case 2: return Number(payload.float);
		case 3: case 5: return payload.text;
		case 4: return String.fromCodePoint(payload.i31);
		case 6: return { [plainOfTree(payload.node)]: plainOfTree(tree.chain?.[0] ?? { kind: "0" }) };
		case 7: case 8: {
			const values = items();
			const isObject = (kind >> 8n) === 0n && values.length > 0 && values.every(item => (BigInt(item.kind) & KIND_MASK) === KIND_KEY);
			return isObject ? Object.assign({}, ...values.map(plainOfTree)) : values.map(plainOfTree);
		}
		default: return null;
	}
}

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

// objects of the page without a plain form (a Date, a Map, an instance, a function), kept for foreign_call behind ids:
// they cross as `{$handle: id, type, text}` and are the object again when they come back
const foreignHandles = [];
const handleOf = value => ({ $handle: foreignHandles.push(value), type: value?.constructor?.name ?? typeof value, text: String(value).slice(0, 200) });
const unhandled = value => value !== null && typeof value === "object" && "$handle" in value ? foreignHandles[value.$handle - 1] : value;
const isPlainObject = value => [Object.prototype, null].includes(Object.getPrototypeOf(value));

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
	const outcome = outcomeOf(holder, hooks, () => instance.exports.main());
	const events = pageEvents(instance.exports);
	if (events.length > 0 && outcome.result) hooks.listen?.(holder, events);
	return outcome;
}

// the outcome of a call into a run's instance (main, or a page event's handler), as src/web.rs run_outcome reads it
function outcomeOf(holder, hooks, call) {
	const { warnings, exports } = holder;
	try {
		const result = call();
		// the tasks nobody awaited finish before the result, as natively; a failure nobody read ends the run
		const unread = joinTasks(holder.run, hooks);
		deliverSignals(holder); // the raises of tasks nobody awaited run their handlers before the run ends
		checkShared(holder); // and the listeners on shared values see what the tasks left
		if (unread) return { failure: unread, warnings };
		return { result: readResult(exports, result), warnings };
	} catch (trap) {
		if (holder.exitCode !== undefined) return { result: { kind: "0", data: null, chain: [] }, warnings };
		if (holder.blockError !== undefined) return { error: holder.blockError, warnings };
		if (!(trap instanceof WebAssembly.RuntimeError || trap instanceof RangeError)) return { failure: String(trap.message ?? trap), warnings };
		const detail = exports[TRAP_DETAIL_EXPORT]?.value;
		return { trap: trap.message, trace: trap.stack ?? "", detail: detail ? readNode(exports, detail) : null, warnings };
	}
}

// the page events a program handles (src/lowering/event_signals.rs PAGE_EVENTS): `on click {…}` exports on·click·node
const PAGE_EVENT_HANDLER = /^on·(click|key)·node$/;
function pageEvents(exports) {
	return Object.keys(exports).map(name => name.match(PAGE_EVENT_HANDLER)?.[1]).filter(Boolean);
}

// a page event of a program that handles it, after its main ran: the handler with the event's data, its outcome
function runPageEvent(holder, hooks, event, detail) {
	holder.warnings = [];
	const handler = holder.exports[`on·${event}·node`];
	return outcomeOf(holder, hooks, () => handler(buildValue(holder.exports, treeOfPlain([detail]))));
}

// The compiler that runs the blocks a program builds at run time (run_block): loaded on first use, an instance of its own,
// so it never re-enters the compiler whose program is running. A page sets BLOCK_COMPILER_URL to its compiler's URL, or
// BLOCK_COMPILER to a function giving the exports of a new compiler instance (test-worker.js: the test binary itself).
let blockCompilerExports;
function blockCompiler(hooks) {
	blockCompilerExports ??= self.BLOCK_COMPILER ? self.BLOCK_COMPILER(hooks) : compilerFromUrl(hooks);
	return blockCompilerExports;
}

// the next block gets a new compiler instance: after a trap the old one is unusable
function forgetBlockCompiler() {
	blockCompilerExports = undefined;
}

function compilerFromUrl(hooks) {
	const url = self.BLOCK_COMPILER_URL ?? "warp.wasm";
	let bytes;
	try {
		bytes = getSync(url, undefined, true);
	} catch (failure) {
		throw new Error(`a block known only at run time needs the warp compiler ${url} (${failure.message}): build it with web/playground/build.sh`);
	}
	let exports;
	exports = new WebAssembly.Instance(new WebAssembly.Module(bytes), { warp_host: warpHost(() => exports.memory, hooks) }).exports;
	return exports;
}

// the report of src/web.rs eval_block_report: {result: tree} or {error: message}
function evalBlock(hooks, request) {
	const compiler = blockCompiler(hooks);
	const bytes = utf8.encode(request);
	const pointer = compiler.web_alloc(bytes.length);
	new Uint8Array(compiler.memory.buffer, pointer, bytes.length).set(bytes);
	let length;
	try {
		length = compiler.web_eval_block(pointer, bytes.length);
	} catch (trap) {
		forgetBlockCompiler();
		throw trap;
	}
	const report = JSON.parse(readText(compiler, compiler.web_report(), length));
	compiler.web_free(pointer, bytes.length);
	return report;
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
