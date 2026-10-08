// Tasks and messages in the page (a part of host.js, which says how parts work): tasks on Workers (`go f(x)`), the
// channels inside a run and between programs (BroadcastChannel, WebSocket), shared arrays and their listeners, and the
// fetches that run without waiting (fetch_start)

// the checks of the listeners on shared values (src/lowering/signal_values.rs), run at every check point
const SHARED_HANDLER = "on·shared";
const SOCKET_ADDRESS = /^wss?:\/\//; // src/web_sockets.rs SOCKET_SCHEMES

const TASK_FINISHED = 1n; // src/host.rs TASK_FINISHED, TASK_FAILED, TASK_STOPPED, TASK_STOP
const TASK_FAILED = 2n;
const TASK_STOPPED_CODE = 3n;
const TASK_STOP = 1n;
const TASK_PAUSE = 2n; // src/host.rs TASK_PAUSE, TASK_RESUME, TASK_PAUSED
const TASK_RESUME = 3n;
const TASK_PAUSED = 4n;
const CONTROL_RUNNING = 0; // a task Worker's control word
const CONTROL_PAUSED = 1;

const TASK_WORKER = "task-worker.js";
const TASK_WORKER_READY = "ready"; // a task Worker's first message: loaded
const FETCH_DONE = "fetched"; // a task Worker's message after a fetch it made (startFetch)
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

// a built site's task Worker loads the site's scripts (site-worker.js siteScripts), the playground's all of them
function addTaskWorker() {
	const worker = new Worker(self.siteScripts ? `${TASK_WORKER}?scripts=${self.siteScripts.join(",")}` : TASK_WORKER);
	// loaded: it can take tasks; later a fetch it made is done (startFetch)
	worker.onmessage = ({ data }) => data === FETCH_DONE ? worker.fetched?.() : taskPool.push(worker);
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
	if (!(cell >= 0 && cell < array.length - 1)) throw new WebAssembly.RuntimeError("index out of range");
	return [array, cell];
}

// the cell a set or add writes, counted in the array's last cell
function writtenCell(run, id, index) {
	const [array, cell] = sharedCell(run, id, index);
	Atomics.add(array, array.length - 1, 1n);
	return [array, cell];
}

// The local channels of a run (P155, src/tasks.rs Channels) in one SharedArrayBuffer every task Worker gets: a lock,
// a change counter the waiting sides sleep on (a condition variable), the next channel id and whether the program
// ended, then per channel [full, closed, sent, taken, length] and the bytes of the value offered (JSON of a task tree).
// Go's unbuffered channel: a send waits until a receiver took its value
const CHANNEL_SLOTS = 64;
const CHANNEL_VALUE_BYTES = 1 << 16;
const [CHANNEL_LOCK, CHANNEL_CHANGED, CHANNEL_NEXT_ID, CHANNEL_ENDED, CHANNEL_HEADER] = [0, 1, 2, 3, 4];
const [SLOT_FULL, SLOT_CLOSED, SLOT_SENT, SLOT_TAKEN, SLOT_LENGTH, SLOT_FIELDS] = [0, 1, 2, 3, 4, 5];
const CHANNEL_CHECK_MS = 20; // how often a waiting side looks whether it waits forever, as natively
const CHANNEL_INTS = CHANNEL_HEADER + CHANNEL_SLOTS * SLOT_FIELDS;

// a run's channel table, when its program makes channels and the page has shared memory
function channelTable(bytes) {
	if (!importDescriptors(bytes).some(entry => entry.name === "channel_new")) return null;
	if (!hasTaskWorkers()) return null;
	return new Int32Array(new SharedArrayBuffer(4 * CHANNEL_INTS + CHANNEL_SLOTS * CHANNEL_VALUE_BYTES));
}

function channelsOf(run) {
	if (!run.channels) throw new Error("ch = channel(): channels need shared memory between Workers, which this page lacks (it is not cross-origin isolated); `warp run` has them");
	return run.channels;
}

function openChannel(run) {
	const table = channelsOf(run);
	const id = Atomics.add(table, CHANNEL_NEXT_ID, 1) + 1;
	if (id > CHANNEL_SLOTS) throw new Error(`channel(): the playground holds ${CHANNEL_SLOTS} channels per run`);
	return BigInt(id);
}

// channel id's fields, and the bytes of its value
function channelSlot(table, id) {
	const index = Number(id);
	if (!(index >= 1 && index <= Atomics.load(table, CHANNEL_NEXT_ID))) throw new Error(`no channel ${id}`);
	const base = CHANNEL_HEADER + (index - 1) * SLOT_FIELDS;
	const field = name => base + name;
	const bytes = new Uint8Array(table.buffer, 4 * CHANNEL_INTS + (index - 1) * CHANNEL_VALUE_BYTES, CHANNEL_VALUE_BYTES);
	return { get: name => table[field(name)], set: (name, value) => { table[field(name)] = value; }, bytes };
}

function lockChannels(table) {
	while (Atomics.compareExchange(table, CHANNEL_LOCK, 0, 1) !== 0) Atomics.wait(table, CHANNEL_LOCK, 1);
}

function unlockChannels(table, changed) {
	if (changed) Atomics.add(table, CHANNEL_CHANGED, 1);
	Atomics.store(table, CHANNEL_LOCK, 0);
	Atomics.notify(table, CHANNEL_LOCK, 1);
	if (changed) Atomics.notify(table, CHANNEL_CHANGED);
}

// the program alone: no task of it runs any more, so nothing could answer. A task run inline (no free Worker) blocks
// its starting program, which could never answer either
function waitsAlone(holder) {
	if (holder.inTask) return !holder.control;
	return ![...holder.run.tasks.values()].some(task => task.worker && Atomics.load(new Int32Array(task.shared, 0, 1), 0) === 0);
}

// wait on channel id until `ready` answers ({value} or {error}), under the lock; it may change the channel
function channelWait(holder, id, waiting, ready) {
	const table = channelsOf(holder.run);
	for (;;) {
		lockChannels(table);
		const seen = Atomics.load(table, CHANNEL_CHANGED);
		let answer;
		try {
			answer = ready(channelSlot(table, id));
			if (!answer && Atomics.load(table, CHANNEL_ENDED)) answer = { error: TASK_STOPPED };
			if (!answer && waitsAlone(holder)) answer = { error: `${waiting} waits forever: no task is left to answer it` };
		} catch (failure) {
			unlockChannels(table, false);
			throw failure;
		}
		unlockChannels(table, Boolean(answer));
		if (answer?.error) throw new Error(answer.error);
		if (answer) return answer.value;
		Atomics.wait(table, CHANNEL_CHANGED, seen, CHANNEL_CHECK_MS);
	}
}

// `ch.send(v)`: offers v once the channel is free, then waits until a receiver took it
function putOnChannel(holder, id, bytes) {
	if (bytes.length > CHANNEL_VALUE_BYTES) throw new Error(`ch.send: a value of ${bytes.length} bytes is more than the playground's ${CHANNEL_VALUE_BYTES} per channel`);
	let mine;
	channelWait(holder, id, "ch.send", slot => {
		if (mine === undefined) {
			if (slot.get(SLOT_CLOSED)) return { error: "send on a closed channel" };
			if (slot.get(SLOT_FULL)) return null;
			slot.bytes.set(bytes);
			slot.set(SLOT_LENGTH, bytes.length);
			slot.set(SLOT_FULL, 1);
			mine = slot.get(SLOT_SENT);
			slot.set(SLOT_SENT, mine + 1);
			Atomics.add(holder.run.channels, CHANNEL_CHANGED, 1); // the offer wakes a receiver
			Atomics.notify(holder.run.channels, CHANNEL_CHANGED);
		}
		return slot.get(SLOT_TAKEN) > mine ? { value: undefined } : null;
	});
}

// `ch.receive()`: the value offered, waiting for one; null (ø) once the channel is closed and empty
function takeFromChannel(holder, id) {
	return channelWait(holder, id, "ch.receive()", slot => {
		if (slot.get(SLOT_FULL)) {
			const bytes = slot.bytes.slice(0, slot.get(SLOT_LENGTH));
			slot.set(SLOT_FULL, 0);
			slot.set(SLOT_TAKEN, slot.get(SLOT_TAKEN) + 1);
			return { value: bytes };
		}
		return slot.get(SLOT_CLOSED) ? { value: null } : null;
	});
}

// `for v in ch`: 1 when a value is offered, 0 once the channel is closed and empty
function moreOnChannel(holder, id) {
	return channelWait(holder, id, "for … in ch", slot => slot.get(SLOT_FULL) ? { value: 1n } : slot.get(SLOT_CLOSED) ? { value: 0n } : null);
}

function closeChannel(run, id) {
	const table = channelsOf(run);
	lockChannels(table);
	const slot = channelSlot(table, id);
	const closed = slot.get(SLOT_CLOSED);
	slot.set(SLOT_CLOSED, 1);
	unlockChannels(table, true);
	if (closed) throw new Error("close of a closed channel");
}

// the program ended: tasks still waiting on a channel stop
function endChannels(run) {
	const table = run.channels;
	if (!table || run.channelsEnded) return;
	run.channelsEnded = true;
	lockChannels(table);
	Atomics.store(table, CHANNEL_ENDED, 1);
	unlockChannels(table, true);
}

// a task of the run: f(arguments) in a fresh instance of the program (src/tasks.rs TaskTable::run), the Int arguments as
// they are or the argument list (`values`, a tree of reader.js) rebuilt for a wrapper f·node. On a Worker of the pool
// (shared memory needs cross-origin isolation), which writes the result into a SharedArrayBuffer; else at once, here
function startTask(holder, hooks, name, ints, values) {
	const run = holder.run;
	const id = BigInt(run.tasks.size + 1);
	const captured = capturedValues(holder.exports);
	if (taskPool.length > 0) {
		const shared = new SharedArrayBuffer(TASK_HEADER + TASK_RESULT_BYTES, { maxByteLength: TASK_RESULT_LIMIT });
		const worker = taskPool.pop();
		const control = new Int32Array(new SharedArrayBuffer(4));
		worker.postMessage({ module: run.module, name, ints, values, shared, arrays: run.shared, captured, control, channels: run.channels });
		run.tasks.set(id, { name, worker, shared, control, inline: () => runTask(run.module, hooks, holder.warnings, name, ints, values, run.shared, captured, null, run.channels) });
	} else {
		run.tasks.set(id, runTask(run.module, hooks, holder.warnings, name, ints, values, run.shared, captured, null, run.channels));
	}
	return id;
}

// the task's function as the program wrote it (src/tasks.rs task_name): `f`, not `f·node`; `go block 1`, not `go·block·1`
function taskName(name) {
	return name.replace(/·node$/, "").replace(/^go·block·/, "go block ");
}

// the task here: {value} (a number or a tree) or {failure}
function runTask(module, hooks, warnings, name, ints, values, arrays, captured = [], control = null, channels = null) {
	const taskHolder = { warnings, run: { module, tasks: new Map(), shared: arrays, channels }, control, inTask: true };
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
		// no instance: the task never ran (`Out of memory: Cannot allocate Wasm memory`, see finishedTask)
		return { failure: `task ${taskName(name)}: ${raisedText(instance?.exports) ?? trapMessage(trap, name)}`, unstarted: !instance, signals: taskHolder.run.signals };
	}
}

// the task's record once it is done: a Worker's is waited for (the program runs in a worker, which may block) and
// read from its shared buffer, its output printed then
function finishedTask(run, hooks, id) {
	const task = run.tasks.get(id);
	if (!task.worker) return queuedSignals(run, task);
	let record = readShared(task.shared, true);
	if (record.output) hooks.print(record.output, 1);
	taskPool.push(task.worker); // free for the next task
	if (record.value?.kind === KIND_INT && record.ints) record.value = BigInt(record.value.data.int);
	// a Worker that could not even instantiate the task runs it here instead: a page holds ~124 Wasm memories in all its
	// Workers, and only the isolate that dropped an instance collects it, so the dead instances of this one (the
	// starting program's) block the Worker until this isolate collects them, which it does when its own allocation
	// fails (card browser-test, probes/wasm_memory_limit.html)
	if (record.unstarted) record = task.inline();
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

const KIND_FUNCTION = 16n; // src/type_kinds.rs Kind::Function: a closure

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
function capturedValues(exports) {
	const names = Object.keys(exports).filter(name => name.startsWith(CAPTURE_PREFIX));
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

// `users := fetch url` (src/fetches.rs, src/lowering/fetch_signals.rs): fetch_start fetches without waiting. A Worker
// of the task pool fetches into shared memory, so a running main sees the reply at its check points (sleep, loop
// starts: deliverFetches runs on·fetch·<id> there, as natively); else, and for a reply that comes after main returned,
// the page runs the handler (worker.js hooks.arrived). A fetch started anew drops the reply of the one before, going
// to another page (navigate) the pending ones, a run that ended all.
const FETCH_HANDLER_PREFIX = "on·fetch·";
function startFetch(holder, hooks, id, url) {
	if (!hooks.arrived && taskPool.length === 0) return holder.warnings.push(`fetch ${url}: replies arriving later are not handled here`);
	const started = { url };
	(holder.fetches ??= new Map()).set(id, started);
	const current = () => holder.fetches.get(id) === started && !holder.stopped;
	const arrive = reply => {
		if (!current() || started.reply) return;
		started.reply = reply;
		hooks.arrived?.(holder, FETCH_HANDLER_PREFIX + id);
	};
	const failed = reason => ({ error: `fetch ${url} failed: ${reason}` });
	if (taskPool.length === 0) return void fetchReplyOf(url).then(({ body, error }) => arrive(error ? failed(error) : { body }));
	const worker = taskPool.pop();
	started.shared = new SharedArrayBuffer(TASK_HEADER + TASK_RESULT_BYTES, { maxByteLength: TASK_RESULT_LIMIT });
	started.failed = failed;
	// the Worker is free once it wrote the reply: taken at a check point or here, whichever comes first
	started.release = () => {
		if (!taskPool.includes(worker)) taskPool.push(worker);
		started.release = () => {};
	};
	worker.fetched = () => {
		started.release();
		const reply = sharedFetchReply(started);
		if (reply) arrive(reply);
	};
	worker.postMessage({ fetch: new URL(url, self.location.href).href, shared: started.shared });
}

// the page went to another path (card fetch-cancel): a reply still pending runs no handler; the ones in stay the values
function dropPendingFetches(holder) {
	for (const [id, started] of holder.fetches ?? []) {
		if (!started.reply) holder.fetches.delete(id);
	}
}

// {body} or {error} of a URL: the HTTP status of a failed request, or why it failed
function fetchReplyOf(url) {
	return fetch(url).then(async response => response.ok ? { body: await response.text() } : { error: `HTTP status ${response.status}` }, failure => ({ error: failure.message }));
}

// a task Worker's answer: the JSON of `record` in the shared buffer, then its state set to done for the waiting side
function writeShared(shared, record) {
	const reply = utf8.encode(JSON.stringify(record));
	if (TASK_HEADER + reply.length > shared.byteLength) shared.grow(TASK_HEADER + reply.length);
	new Uint8Array(shared, TASK_HEADER, reply.length).set(reply);
	const header = new Int32Array(shared, 0, 2);
	header[1] = reply.length;
	Atomics.store(header, 0, 1);
	Atomics.notify(header, 0);
}

// the record a task Worker wrote into a shared buffer (writeShared): waited for when `wait`, else undefined while none is
// there yet
function readShared(shared, wait = false) {
	const header = new Int32Array(shared, 0, 2);
	if (wait) Atomics.wait(header, 0, 0);
	if (Atomics.load(header, 0) === 0) return undefined;
	return JSON.parse(decode(new Uint8Array(shared, TASK_HEADER, header[1]).slice()));
}

// the reply a task Worker wrote into the fetch's shared buffer, once: undefined while none is there or it was taken
function sharedFetchReply(started) {
	const answer = started.reply ? undefined : readShared(started.shared);
	if (!answer) return undefined;
	const { body, error } = answer;
	return error ? started.failed(error) : { body };
}

// a check point of a running main (sleep, loop starts): the handlers of the fetches whose reply is in shared memory
function deliverFetches(holder) {
	for (const [id, started] of holder.fetches ?? []) {
		const reply = started.shared && !holder.stopped ? sharedFetchReply(started) : undefined;
		if (!reply) continue;
		started.reply = reply;
		started.release();
		holder.exports[FETCH_HANDLER_PREFIX + id]();
	}
}

// [value, error] as src/fetches.rs reply gives it: a JSON object or array parsed, any other body the text
function fetchReply(holder, id) {
	const { reply } = holder.fetches?.get(id) ?? {};
	if (!reply) return [null, `fetch ${id} has no reply yet`];
	if (reply.error) return [null, reply.error];
	const structure = structureOf(reply.body);
	if (structure !== undefined) return [structure, null];
	return [reply.body.endsWith("\n") ? reply.body : reply.body + "\n", null]; // wasp convention (src/host.rs fetch)
}

// a JSON object or array of the text, undefined for any other text (src/web_server.rs value_of_body)
function structureOf(text) {
	try {
		const value = JSON.parse(text);
		if (value !== null && typeof value === "object") return value;
	} catch {} // not JSON
	return undefined;
}

// a channel named by a ws:// or wss:// address is a WebSocket (src/web_sockets.rs): one connection per address in a
// run, its listeners share what arrives; a message sent before it is open waits for it
function webSocket(holder, hooks, address) {
	const sockets = holder.sockets ??= new Map();
	if (sockets.has(address)) return sockets.get(address);
	const connection = { socket: new WebSocket(address), messages: [], waiting: [] };
	connection.socket.onopen = () => connection.waiting.splice(0).forEach(text => connection.socket.send(text));
	connection.socket.onmessage = ({ data }) => connection.messages.push(structureOf(data) ?? data);
	connection.socket.onerror = () => hooks.print?.(`on message from "${address}": the connection failed\n`, STDERR);
	sockets.set(address, connection);
	return connection;
}

// a text as it is, any other value as JSON
function sendOnSocket(holder, hooks, address, value) {
	const { socket, waiting } = webSocket(holder, hooks, address);
	const text = typeof value === "string" ? value : JSON.stringify(value);
	if (socket.readyState === WebSocket.OPEN) socket.send(text);
	else waiting.push(text);
}

// `on message from "chat" {…}`: what arrives waits in the listener's queue until its timer asks
function listenOnChannel(holder, hooks, id, name) {
	if (!hooks.listen) return holder.warnings.push(`on message from "${name}": channels are not received here`);
	if (SOCKET_ADDRESS.test(name)) {
		const { socket, messages } = webSocket(holder, hooks, name);
		return void (holder.channels ??= new Map()).set(id, { name, channel: socket, messages });
	}
	const channel = new BroadcastChannel(name);
	const listener = { name, channel, messages: [] };
	channel.onmessage = ({ data }) => listener.messages.push(data);
	(holder.channels ??= new Map()).set(id, listener);
}

addHostPart({
	words: (holder, hooks, { program, cString }) => ({
		// channels between programs (src/channels.rs): a BroadcastChannel of the name, which reaches the other tabs and
		// workers of this page's origin; the listener's timer (lowering/system_signals.rs) takes what arrived
		channel_listen: (id, channel) => listenOnChannel(holder, hooks, Number(id), plainOfTree(readNode(program(), channel))),
		channel_pending: id => BigInt(holder.channels?.get(Number(id))?.messages.length ?? 0),
		channel_next: id => buildValue(program(), treeOfPlain(holder.channels?.get(Number(id))?.messages.shift() ?? null)),
		channel_send: (channel, message) => {
			const program_ = program();
			const [name, value] = [channel, message].map(node => plainOfTree(readNode(program_, node)));
			if (SOCKET_ADDRESS.test(name)) return sendOnSocket(holder, hooks, name, value);
			new BroadcastChannel(name).postMessage(value);
		},
		// channels inside one run (P155, src/tasks.rs Channels): Go's unbuffered channel between the task Workers
		channel_new: () => openChannel(holder.run),
		channel_put: (id, value) => putOnChannel(holder, id, utf8.encode(JSON.stringify(readTaskValue(program(), value)))),
		channel_take: id => {
			const bytes = takeFromChannel(holder, id);
			return bytes ? buildValue(program(), JSON.parse(decode(bytes))) : null;
		},
		channel_more: id => moreOnChannel(holder, id),
		channel_close: id => closeChannel(holder.run, id),
		fetch_start: (id, url) => startFetch(holder, hooks, Number(id), plainOfTree(readNode(program(), url))),
		fetch_reply: id => buildValue(program(), treeOfPlain(fetchReply(holder, Number(id)))),
		// tasks (src/tasks.rs): `go f(x)` runs f in a fresh instance of the program, values copied in and out; a page
		// without cross-origin isolation has no shared memory to wait on, so the task runs at once where it starts
		task_spawn: (name, a0, a1, a2, a3) => startTask(holder, hooks, decode(cString(name)), [a0, a1, a2, a3]),
		task_spawn_values: (name, values) => startTask(holder, hooks, decode(cString(name)), null, readTaskValue(program(), values)),
		task_await: id => {
			const task = takenTask(holder.run, hooks, id);
			if (task.failure) throw new Error(task.failure);
			// a task whose function returns a Node (`go { n + 1 }` of a Node parameter n) still gives its number here
			if (typeof task.value === "object" && task.value !== null) {
				const number = task.value.data?.int;
				if (number === undefined) throw new Error(`task ${taskName(task.name ?? "")}: its result is no whole number`);
				return BigInt(number);
			}
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
		// shared arrays (src/shared.rs): Ints every task of the run reaches, in shared memory when the page is isolated;
		// one cell more, the last, counts the writes (shared_writes)
		shared_new: length => {
			const Buffer = self.crossOriginIsolated ? SharedArrayBuffer : ArrayBuffer;
			holder.run.shared.push(new BigInt64Array(new Buffer(8 * (Math.max(0, Number(length)) + 1))));
			return BigInt(holder.run.shared.length);
		},
		shared_get: (id, index) => Atomics.load(...sharedCell(holder.run, id, index)),
		shared_set: (id, index, value) => (Atomics.store(...writtenCell(holder.run, id, index), value), value),
		shared_add: (id, index, value) => Atomics.add(...writtenCell(holder.run, id, index), value) + value,
		shared_count: id => BigInt(sharedArray(holder.run, id).length - 1),
		shared_writes: id => {
			const array = sharedArray(holder.run, id);
			return Atomics.load(array, array.length - 1);
		},
		// an array of floats: the cells hold the bits; an add swaps until no other task came between
		shared_getf: (id, index) => floatOfBits(Atomics.load(...sharedCell(holder.run, id, index))),
		shared_setf: (id, index, value) => (Atomics.store(...writtenCell(holder.run, id, index), bitsOfFloat(value)), value),
		shared_addf: (id, index, value) => {
			const [array, cell] = writtenCell(holder.run, id, index);
			for (;;) {
				const old = Atomics.load(array, cell);
				const sum = floatOfBits(old) + value;
				if (Atomics.compareExchange(array, cell, old, bitsOfFloat(sum)) === old) return sum;
			}
		},
	}),
	started: run => Object.assign(run, { tasks: new Map(), shared: [], channels: channelTable(run.bytes) }),
	poll: holder => {
		checkShared(holder);
		deliverFetches(holder);
	},
	finished: (holder, hooks) => {
		endChannels(holder.run); // a task still waiting on a channel stops, as natively
		// the tasks nobody awaited finish before the result, as natively; a failure nobody read ends the run
		const unread = joinTasks(holder.run, hooks);
		deliverSignals(holder); // the raises of tasks nobody awaited run their handlers before the run ends
		checkShared(holder); // and the listeners on shared values see what the tasks left
		return unread;
	},
	ended: holder => endChannels(holder.run),
	navigated: dropPendingFetches,
	stopped: holder => {
		holder.channels?.forEach(({ channel }) => channel.close());
		holder.sockets?.forEach(({ socket }) => socket.close());
	},
});
