// Tasks and messages in the page (a part of host.js, which says how parts work): tasks on Workers (`go f(x)`), the
// channels inside a run and between programs (BroadcastChannel, WebSocket), shared arrays and their listeners, and the
// fetches that run without waiting (fetch_start)

// the checks of the listeners on shared values (src/lowering/signals/signal_values.rs), run at every check point
const SHARED_HANDLER = "on·shared";
const SOCKET_ADDRESS = /^wss?:\/\//; // src/web/web_sockets.rs SOCKET_SCHEMES
const TEXT_REPLY_TYPE = "text/plain"; // src/web/web_server.rs TEXT_REPLY_TYPE: a text reply of a server function
const SERVER_REPLIES_ID = "warp-replies"; // src/web/site.rs REPLIES_ID
const ROUTE_DATA_URL = "/rpc/route·data·"; // src/lowering/web/serve.rs RPC_PREFIX + ROUTE_DATA_PREFIX

const TASK_FINISHED = 1n; // src/host.rs TASK_FINISHED, TASK_FAILED, TASK_STOPPED, TASK_STOP
const TASK_FAILED = 2n;
const TASK_STOPPED_CODE = 3n;
const TASK_STOP = 1n;
const TASK_PAUSE = 2n; // src/host.rs TASK_PAUSE, TASK_RESUME, TASK_PAUSED
const TASK_RESUME = 3n;
const TASK_PAUSED = 4n;
const CONTROL_RUNNING = 0; // a task Worker's control word
const CONTROL_PAUSED = 1;
// the control buffer's second word: set by the task Worker once it took the task (task-worker.js runTaskInto)
const TASK_TAKEN_SLOT = 1;
// how long a run waits for a task Worker to take its task before it calls the Worker lost (card task-hang)
const TASK_TAKE_MS = 15000;
const TASK_CHECK_MS = 100; // how often a run waiting for a task looks after its other tasks

const TASK_WORKER = "task-worker.js";
const TASK_WORKER_READY = "ready"; // a task Worker's first message: loaded
const FETCH_DONE = "fetched"; // a task Worker's message after a fetch it made (startFetch)
const INDEX_OUT_OF_RANGE = "index out of range"; // src/wasm_emitter list_ops runtime_error_message("index_out_of_range")
const EMPTY_RANGE = "an empty list"; // src/wasm_emitter EMPTY_RANGE
const POSITION_COLUMN_BITS = 12; // src/wasm_emitter COLUMN_BITS: trap_position is line << 12 | column
const TASK_RESULT_BYTES = 1 << 16; // the shared buffer a Worker writes its result into, grown as needed
const TASK_RESULT_LIMIT = 1 << 28;
const TASK_HEADER = 8; // [state, length] as Int32, then the result's JSON
const TASK_STOPPED = "task stopped";
const PAGE_ANSWER_MS = 60000; // how long a Worker waits for its page's answer: a clipboard read may wait for a permission
// Workers a task runs on: made while the program's worker is idle (prepareTaskPool), since a Worker only starts once
// its creator returns to its event loop and a running program never does (emscripten keeps a thread pool for this)
const taskPool = [];
const TASK_POOL_SIZE = Math.min(4, self.navigator?.hardwareConcurrency ?? 2);
// how long a run waits for the pool's Workers to load before it starts anyway (a task then runs inline)
const TASK_POOL_WAIT_MS = 10000;
const TASK_POOL_POLL_MS = 10;
const hasTaskWorkers = () => self.crossOriginIsolated && self.Worker;
// why a run's tasks take turns: said once per run, as a warning and at once to the page (hooks.tasksInline), whose
// timeout then names it instead of "the program may not terminate" (card coi-headless)
const TASKS_INLINE = "tasks take turns here, one runs to its end before the program goes on: the page is not cross-origin isolated (its service worker cannot run, as in a private window), so it has no shared memory for task Workers. A task that waits for another, or runs until stopped, never ends";

// the page makes a task Worker for this Worker and gives it a MessagePort to it (task-workers.js serveTaskWorkers): in
// the pool the port stands for the Worker, `terminate` asks the page to stop it
const madeTaskWorkers = new Map(); // id → what to do with the port of the task Worker the page made
let taskWorkersAsked = 0;
self.addEventListener("message", event => {
	if (event.data?.taskWorker === undefined || !event.data.port) return;
	event.stopImmediatePropagation(); // registered before the Worker's own onmessage, which never sees it
	madeTaskWorkers.get(event.data.taskWorker)(event.data.port);
	madeTaskWorkers.delete(event.data.taskWorker);
});

// a built site's task Worker loads the site's scripts (site-worker.js siteScripts), the playground's all of them
function addTaskWorker(loaded) {
	const id = taskWorkersAsked++;
	madeTaskWorkers.set(id, worker => {
		worker.taskWorkerId = id; // the page's name for it (task-workers.js postToTaskWorker)
		worker.terminate = () => self.postMessage({ endTaskWorker: id });
		// loaded: it can take tasks; later a fetch it made is done (startFetch)
		worker.onmessage = ({ data }) => {
			if (data === FETCH_DONE) return worker.fetched?.();
			if (data.relay) return relayed(data.relay);
			taskPool.push(worker);
			loaded?.();
		};
	});
	const url = self.siteScripts ? `${TASK_WORKER}?scripts=${self.siteScripts.join(",")}` : TASK_WORKER;
	self.postMessage({ taskWorker: { id, url: new URL(url, self.location.href).href } });
}

// a voice (card playground-go, natively src/sound/mod.rs Voice): when this Worker's queued sounds end on the clock the
// page and every Worker share, the seconds of sound it made in all (its sounds' places for render_sound) and the
// clock's time of place 0 (where `at 2 beats` counts from). A task's voice starts where its starter's stands
let voice = {};
let voicesBeforeAt = []; // the voices `at` statements left (sound_at), taken up again by sound_at_end
const soundClock = () => (performance.timeOrigin + performance.now()) / 1000;
self.soundsQueued = () => Math.max(0, (voice.end ?? 0) - soundClock()); // sound_queued()
self.voiceStopped = () => { voice = { ...voice, end: undefined }; };
self.joinVoice = starters => { voice = { ...starters }; }; // a task Worker's: its starter's; none: a run's fresh one
self.currentVoice = () => voice;
// a sound of that many seconds behind the voice's sounds, never before now: its place and its start on the clock
self.voiceSounded = seconds => {
	const at = Math.max(voice.end ?? 0, soundClock());
	const place = voice.seconds ?? 0;
	voice = { end: at + seconds, seconds: place + seconds, origin: voice.origin ?? at - place };
	return { place, at };
};
// `at time …`: the voice placed that many seconds after its place 0; its end puts it back, keeping that origin
self.voiceAt = seconds => {
	voicesBeforeAt.push(voice);
	const origin = voice.origin ?? soundClock();
	voice = { end: origin + seconds, seconds, origin };
};
self.insideAt = () => voicesBeforeAt.length > 0;
// where this voice's next sound would start: its queued sounds' end, never before now (sound_moment)
self.voiceMoment = () => Math.max(voice.end ?? 0, soundClock());
self.voiceAtEnd = () => {
	if (voicesBeforeAt.length) voice = { ...voicesBeforeAt.pop(), origin: voice.origin };
};
// a task run inline sounds as a voice of its own too: the starter's voice stands where it stood
function asVoice(run) {
	const starters = voice;
	try { return run(); } finally { voice = starters; }
}
// the sounds a task made go to its starter's run, for its render_sound (card playground-drops)
function withTaskSounds(run, record) {
	(run.unrenderedSounds ??= []).push(...(record.sounds ?? []));
	delete record.sounds;
	return record;
}

// a sound or MIDI message a task sends, for the page, which only the program's Worker talks to (task-worker.js relay);
// it reaches the page once that Worker is back in its event loop, not while it waits for the task
const RELAYED = ["playSamples", "playSoundFile", "stopSound", "stopSoundFiles", "fadeSound", "sendMidi"];
function relayed([method, ...values]) {
	if (!RELAYED.includes(method)) throw new Error(`a task Worker relayed ${method}, which is no word of the page`);
	self[method]?.(...values);
}

// the pool of task Workers of the workers that run programs (worker.js, test-worker.js), asked for when they start, one
// after the other
function prepareTaskPool(size = TASK_POOL_SIZE) {
	if (!hasTaskWorkers()) return;
	const addFrom = index => index < size && addTaskWorker(() => addFrom(index + 1));
	addFrom(0);
}

// resolves once every task Worker of the pool has loaded: a run that starts before would run its tasks inline, where
// `stop` cannot end one (samples/threads.warp's endless spin hung the browser suite, card flaky-browser)
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

// The local channels of a run (P155, src/host/tasks.rs Channels) in one SharedArrayBuffer every task Worker gets: a lock,
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
	return ![...holder.run.tasks.values()].some(task => isPending(task) && !isDone(task));
}

// wait on channel id until `ready` answers ({value} or {error}), under the lock; it may change the channel. The run's
// stalled tasks run here meanwhile, since the answer may have to come from one (card task-hang-channels)
function channelWait(holder, hooks, id, waiting, ready) {
	const table = channelsOf(holder.run);
	for (;;) {
		runStalledTasks(holder.run, hooks);
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
function putOnChannel(holder, hooks, id, bytes) {
	if (bytes.length > CHANNEL_VALUE_BYTES) throw new Error(`ch.send: a value of ${bytes.length} bytes is more than the playground's ${CHANNEL_VALUE_BYTES} per channel`);
	let mine;
	channelWait(holder, hooks, id, "ch.send", slot => {
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
function takeFromChannel(holder, hooks, id) {
	return channelWait(holder, hooks, id, "ch.receive()", slot => {
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
function moreOnChannel(holder, hooks, id) {
	return channelWait(holder, hooks, id, "for … in ch", slot => slot.get(SLOT_FULL) ? { value: 1n } : slot.get(SLOT_CLOSED) ? { value: 0n } : null);
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

function sayTasksInline(run, holder, hooks) {
	if (run.saidTasksInline) return;
	run.saidTasksInline = true;
	holder.warnings.push(TASKS_INLINE);
	hooks.tasksInline?.(TASKS_INLINE);
}

// a task of the run: f(arguments) in a fresh instance of the program (src/host/tasks.rs TaskTable::run), the Int arguments as
// they are or the argument list (`values`, a tree of reader.js) rebuilt for a wrapper f·node. On a Worker of the pool
// (shared memory needs cross-origin isolation), which writes the result into a SharedArrayBuffer. With every Worker
// busy the task is queued until one frees, as natively it waits for a free thread (src/host/tasks.rs); run inline here
// instead, a task the program waits for (a channel's sender) could never answer it (card primer-channel-flake). Inline
// only without Workers, or in a task (whose Worker has no pool) or a run holding none (no Worker would ever free)
function startTask(holder, hooks, name, values) {
	const run = holder.run;
	const id = BigInt(run.tasks.size + 1);
	const captured = capturedValues(holder.exports);
	const inline = () => withTaskSounds(run, asVoice(() => runTask(run.module, hooks, holder.warnings, name, values, run.shared, captured, null, run.channels)));
	const shared = takeAnswerBuffer();
	const control = new Int32Array(new SharedArrayBuffer(2 * Int32Array.BYTES_PER_ELEMENT)); // a queued task's too: `pause job` holds before it starts
	const message = { module: run.module, name, values, shared, arrays: run.shared, captured, control, channels: run.channels, voice: { ...voice } };
	const task = { name, shared, control, inline, message };
	if (taskPool.length > 0) startOnWorker(task, run);
	else if (!holder.inTask && holdsAWorker(run)) {
		task.queued = true;
		run.queued = (run.queued ?? 0) + 1;
	} else {
		if (!hasTaskWorkers()) sayTasksInline(run, holder, hooks);
		returnAnswerBuffer(shared);
		run.tasks.set(id, inline());
		return id;
	}
	run.tasks.set(id, task);
	return id;
}

function startOnWorker(task, run) {
	if (task.queued) run.queued--;
	task.queued = false;
	task.worker = taskPool.pop();
	task.started = performance.now();
	task.worker.postMessage({ ...task.message, files: self.writtenFilesForTask?.() });
}

const holdsAWorker = run => [...run.tasks.values()].some(task => isRunningOnWorker(task) && !task.released);

// a finished task's Worker back to the pool, once, whether or not its record was read yet
function releaseWorker(task) {
	if (task.released) return;
	task.released = true;
	taskPool.push(task.worker);
}

// the queued tasks onto the Workers that finished theirs, in the order they were started; at every check point and
// loop start of the program, as natively a free thread takes the next task by itself
function startQueuedTasks(run) {
	if (!run.queued) return;
	for (const task of run.tasks.values()) if (isRunningOnWorker(task) && isDone(task)) releaseWorker(task);
	for (const task of run.tasks.values()) if (isQueued(task) && taskPool.length > 0) startOnWorker(task, run);
}

// the task's function as the program wrote it (src/host/tasks.rs task_name): `f`, not `f·node`; `go block 1`, not `go·block·1`
function taskName(name) {
	return name.replace(/·node$/, "").replace(/^go·block·/, "go block ");
}

// the task here: {value} (a number or a tree) or {failure}
function runTask(module, hooks, warnings, name, values, arrays, captured = [], control = null, channels = null) {
	const taskHolder = { warnings, run: { module, tasks: new Map(), shared: arrays, channels }, control, inTask: true };
	let instance;
	try {
		instance = new WebAssembly.Instance(module, programImports(taskHolder, hooks));
		taskHolder.exports = instance.exports;
		// what the spawning instance's closures captured, as it had it
		for (const [global, value] of captured) instance.exports[global].value = value.tree ? buildValue(instance.exports, value.tree)
			: value.exact ? exactHandle(instance.exports, ...value.exact) : value.raw;
		const callee = instance.exports[name];
		const result = callee(buildValue(instance.exports, values));
		const unread = joinTasks(taskHolder.run, hooks);
		const { signals, unrenderedSounds: sounds } = taskHolder.run;
		if (unread) return { failure: `task ${taskName(name)}: ${unread}`, signals, sounds };
		return { value: typeof result === "object" && result !== null ? readNode(instance.exports, result) : portableInt(instance.exports, result), signals, sounds };
	} catch (trap) {
		// no instance: the task never ran (`Out of memory: Cannot allocate Wasm memory`, see finishedTask)
		return { failure: `task ${taskName(name)}: ${taskFailure(instance?.exports, trap, name)}`, unstarted: !instance, signals: taskHolder.run.signals };
	}
}

// the task's record once it is done: a Worker's is waited for (the program runs in a worker, which may block) and
// read from its shared buffer, its output printed then. Meanwhile the run's stalled tasks run here, since the awaited
// one may wait for them (`after done` waits for the go block that sets done)
function finishedTask(run, hooks, id) {
	for (;;) {
		const waited = run.tasks.get(id);
		if (isQueued(waited)) isWritten(waited, TASK_CHECK_MS); // never written before it starts: a pause
		else if (!isRunningOnWorker(waited) || isWritten(waited, TASK_CHECK_MS)) break;
		runStalledTasks(run, hooks);
	}
	const task = run.tasks.get(id);
	if (!task.worker) return queuedSignals(run, task);
	let record = withTaskSounds(run, readShared(task.shared));
	returnAnswerBuffer(task.shared);
	if (record.output) hooks.print(record.output, 1);
	record.files?.forEach(([path, content]) => self.keepWritten?.(path, content.bytes ? Uint8Array.from(content.bytes) : content));
	releaseWorker(task); // free for the next task
	// a Worker that could not even instantiate the task runs it here instead: a page holds ~124 Wasm memories in all its
	// Workers, and only the isolate that dropped an instance collects it, so the dead instances of this one (the
	// starting program's) block the Worker until this isolate collects them, which it does when its own allocation
	// fails (card browser-test, probes/wasm_memory_limit.html)
	if (record.unstarted) record = task.inline();
	run.tasks.set(id, record);
	return queuedSignals(run, record);
}

const isRunningOnWorker = task => Boolean(task.worker);
const isQueued = task => Boolean(task.queued); // waits for a free Worker (startQueuedTasks)
const isPending = task => isQueued(task) || isRunningOnWorker(task);
const isDone = task => Atomics.load(new Int32Array(task.shared, 0, 1), 0) !== 0; // its Worker wrote the record
const isWritten = (task, milliseconds = 0) => Atomics.wait(new Int32Array(task.shared, 0, 1), 0, 0, milliseconds) !== "timed-out";
const isTaken = task => Atomics.load(task.control, TASK_TAKEN_SLOT) !== 0;

// the run's tasks that cannot go on by themselves, here: one whose Worker could not instantiate it (out of Wasm memory,
// see finishedTask) or did not take it in time (card task-hang)
function runStalledTasks(run, hooks) {
	startQueuedTasks(run);
	for (const [id, task] of run.tasks) {
		if (!isRunningOnWorker(task) || task.checked) continue;
		if (isWritten(task)) {
			task.checked = true; // its record is read once here
			if (readShared(task.shared).unstarted) finishedTask(run, hooks, id);
		} else if (!isTaken(task) && performance.now() - task.started > TASK_TAKE_MS) runLostTask(run, hooks, id, task);
	}
}

// a task whose Worker never took it: the Worker is replaced and the task runs here, loudly
function runLostTask(run, hooks, id, task) {
	const lost = `task ${taskName(task.name)}: its Worker did not take it within ${TASK_TAKE_MS / 1000} s, so it runs here (card task-hang)\n`;
	console.error(lost);
	hooks.print(lost, STDERR);
	task.worker.terminate();
	addTaskWorker();
	run.tasks.set(id, task.inline());
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

// every task done; the first failure nobody read, but a stopped task's (src/host/tasks.rs join_all)
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
	if (isPending(task) && !isDone(task)) return Atomics.load(task.control, 0) === CONTROL_PAUSED ? TASK_PAUSED : 0n;
	const record = isRunningOnWorker(task) ? readShared(task.shared) : task;
	if (!record.failure) return TASK_FINISHED;
	return record.failure.endsWith(TASK_STOPPED) ? TASK_STOPPED_CODE : TASK_FAILED;
}

// `stop job` ends a Worker; `pause job` / `resume job` set its control word, which the task reads where a loop starts
function controlTask(run, id, operation) {
	const task = run.tasks.get(id);
	if (!isPending(task) || isDone(task)) return 0n;
	if (operation === TASK_PAUSE || operation === TASK_RESUME) {
		Atomics.store(task.control, 0, operation === TASK_PAUSE ? CONTROL_PAUSED : CONTROL_RUNNING);
		Atomics.notify(task.control, 0);
		return 1n;
	}
	if (isQueued(task)) run.queued--;
	if (isRunningOnWorker(task)) {
		task.worker.terminate();
		addTaskWorker(); // the pool's replacement, loaded once this run returns to its event loop
	}
	run.tasks.set(id, { failure: `task ${task.name}: ${TASK_STOPPED}` });
	return 1n;
}

// the message a raised error left in trap_detail before trapping (src/run/wasm_reader.rs with_trap_detail), if any
function raisedText(exports) {
	const detail = exports?.[TRAP_DETAIL_EXPORT]?.value;
	if (!detail) return null;
	const tree = readNode(exports, detail);
	return tree.data?.text ?? tree.data?.node?.data?.text ?? null;
}

// what a task's trap means, as the compiler words a run's (src/wasm_emitter trap_error): the raised text, else the
// runtime error with an index error's range, then where it happened; a task stopped at the program's end says only that
function taskFailure(exports, trap, name) {
	const raised = raisedText(exports);
	const runtime = raised ?? trapMessage(trap, name);
	if (runtime === TASK_STOPPED) return runtime;
	const range = !raised && runtime === INDEX_OUT_OF_RANGE ? indexRange(exports) : "";
	return `${runtime}${range}${trapPosition(exports)}`;
}

// ": 7 not in 1…3" of index_out_of_range_of's detail 7:3, the index asked for and the list's count (src/wasm_emitter
// index_range_message)
function indexRange(exports) {
	const detail = exports?.[TRAP_DETAIL_EXPORT]?.value;
	if (!detail) return "";
	const [index, count] = [exports.reflect_data(detail), exports.reflect_value(detail)].map(side => side && readNode(exports, side).data?.int);
	if (index === undefined || count === undefined) return "";
	return `: ${index} not in ${count === "0" ? EMPTY_RANGE : `1…${count}`}`;
}

// " at line:column" of the statement the task trapped in (src/diagnostics/diagnostic.rs located_at), from its trap_position
function trapPosition(exports) {
	const position = exports?.[TRAP_POSITION_EXPORT]?.value;
	return position ? ` at ${position >> POSITION_COLUMN_BITS}:${position & ((1 << POSITION_COLUMN_BITS) - 1)}` : "";
}

// the runtime error a trap means: the first runtime error function on its stack (`index_out_of_range` reads "index out
// of range", as src/wasm_emitter list_ops runtime_error_message has it), else the engine's words
function trapMessage(trap, callee) {
	// V8 names a wasm frame `module.function (wasm://…)`
	const names = [...String(trap.stack ?? "").matchAll(/at (?:\w+\.)?([a-z]+(?:_[a-z]+)+) \(wasm/g)].map(match => match[1]);
	const runtime = names.find(name => name !== callee);
	return runtime ? runtime.replaceAll("_", " ") : String(trap.message ?? trap);
}

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

// the capture globals of the program's closures (src/host/tasks.rs captured): their values now, for the task's instance
function capturedValues(exports) {
	const names = Object.keys(exports).filter(name => name.startsWith(CAPTURE_PREFIX));
	return names.map(name => {
		const value = exports[name].value;
		if (typeof value === "object" && value !== null) return [name, { tree: readTaskValue(exports, value) }];
		return [name, isExactHandle(value) ? { exact: exactParts(exports, value) } : { raw: value }];
	});
}

// an Int handle beyond the fixnums as [numerator, denominator]: the handle means nothing in another instance (card
// error-undefined, src/host/tasks.rs Captured::Int)
function exactParts(exports, handle) {
	return exactOfPayload(readNode(exports, exports.new_int(handle)).data);
}

// a task's whole-number result as it can leave its instance: a handle beyond the fixnums as the tree of its parts
function portableInt(exports, result) {
	return isExactHandle(result) ? { kind: KIND_INT, data: { exact: exactParts(exports, result).map(String) }, chain: [] } : result;
}

// a task's result as a tree: the number a wrapper of a function of numbers gives (f·node gives f's i64 or f64), else
// the tree already read
function taskTree(value) {
	if (typeof value === "bigint") return { kind: KIND_INT, data: { int: String(value) }, chain: [] };
	if (typeof value === "number") return { kind: KIND_FLOAT, data: { float: value }, chain: [] };
	return value ?? { kind: "0", data: null, chain: [] };
}

// `users := fetch url` (src/web/fetches.rs, src/lowering/signals/fetch_signals.rs): fetch_start fetches without waiting. A Worker
// of the task pool fetches into shared memory, so a running main sees the reply at its check points (sleep, loop
// starts: deliverFetches runs on·fetch·<id> there, as natively); else, and for a reply that comes after main returned,
// the page runs the handler (worker.js hooks.arrived). A fetch started anew drops the reply of the one before, going
// to another page (navigate) the pending ones, a run that ended all.
const FETCH_HANDLER_PREFIX = "on·fetch·";
// the replies of the server calls of a page a server rendered for its path (src/web/site.rs replies_script): each answers
// the first fetch of its request, so the page starts showing what the server rendered (P221)
const serverReplies = JSON.parse(globalThis.document?.getElementById(SERVER_REPLIES_ID)?.textContent ?? "[]");
function serverReply(url, body) {
	const asked = JSON.stringify(JSON.parse(body));
	const index = serverReplies.findIndex(reply => reply.url === url && JSON.stringify(reply.arguments) === asked);
	return index < 0 ? undefined : serverReplies.splice(index, 1)[0];
}

// the replies of route data in so far, by request (card route-data-cache): what a route shows for a path is asked once,
// so going back to a page, or to a path of another route (ø), asks the server nothing
const routeDataReplies = new Map();
const routeDataKey = (url, body) => body !== undefined && decodeURI(url).startsWith(ROUTE_DATA_URL) ? url + " " + body : undefined;

// A request [url, body] POSTs the body as JSON: a server function the page calls (src/lowering/web/serve.rs)
function startFetch(holder, hooks, id, request) {
	const [url, body] = Array.isArray(request) ? [request[0], JSON.stringify(request[1])] : [request];
	if (!hooks.arrived && taskPool.length === 0) return holder.warnings.push(`fetch ${url}: replies arriving later are not handled here`);
	const started = { url, posted: body !== undefined };
	(holder.fetches ??= new Map()).set(id, started);
	const current = () => holder.fetches.get(id) === started && !holder.stopped;
	const routeData = routeDataKey(url, body);
	const arrive = reply => {
		if (routeData && !reply.error) routeDataReplies.set(routeData, reply);
		if (!current() || started.reply) return;
		started.reply = reply;
		hooks.arrived?.(holder, FETCH_HANDLER_PREFIX + id);
	};
	const failed = reason => ({ error: `fetch ${url} failed: ${reason}` });
	const rendered = routeDataReplies.get(routeData) ?? (body !== undefined && serverReply(url, body));
	if (rendered) return void queueMicrotask(() => arrive({ body: rendered.body, text: rendered.text }));
	if (taskPool.length === 0) return void fetchReplyOf(url, body).then(reply => arrive(reply.error ? failed(reply.error) : reply));
	const worker = taskPool.pop();
	started.shared = takeAnswerBuffer();
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
	worker.postMessage({ fetch: new URL(url, self.location.href).href, body, shared: started.shared });
}

// the page went to another path (card fetch-cancel): a reply still pending runs no handler; the ones in stay the values
function dropPendingFetches(holder) {
	for (const [id, started] of holder.fetches ?? []) {
		if (!started.reply) holder.fetches.delete(id);
	}
}

// {body} or {error} of a URL: the HTTP status of a failed request, or why it failed
function fetchReplyOf(url, body) {
	const request = body === undefined ? undefined : { method: "POST", body };
	const replied = async response => ({ body: await response.text(), text: response.headers.get("content-type")?.startsWith(TEXT_REPLY_TYPE) });
	return fetch(url, request).then(response => response.ok ? replied(response) : { error: `HTTP status ${response.status}` }, failure => ({ error: failure.message }));
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

// the page's answer to a Worker's question its asynchronous browser APIs answer (the clipboard's text, card
// web-apis-rest): `ask(shared)` hands the page a shared buffer, the page writes its record into it (markup.js
// answerInto) while the Worker waits; without shared memory (a page not cross-origin isolated) a loud error
function askPage(what, ask) {
	if (!ask) throw new Error(`${what}: this page has no way to answer its Worker`);
	if (!self.crossOriginIsolated) throw new Error(`${what}: the page is not cross-origin isolated, so its Worker cannot wait for it`);
	const shared = takeAnswerBuffer();
	ask(shared);
	if (Atomics.wait(new Int32Array(shared, 0, 2), 0, 0, PAGE_ANSWER_MS) === "timed-out") throw new Error(`${what}: the page did not answer in ${PAGE_ANSWER_MS / 1000} s`);
	const { error, ...answer } = readShared(shared);
	returnAnswerBuffer(shared);
	if (error) throw new Error(`${what}: ${error}`);
	return answer;
}

// The shared buffers Workers answer in (a task's record, a fetch's reply, the page's or a GPU job's answer), each
// handed back once its answer is read and emptied for the next: a growable one reserves TASK_RESULT_LIMIT, and Safari
// ran out of memory with ~200 alive (cards webgpu-safari, safari-shared-buffers; notes/web_playground.md). One whose
// writer may still come (a timed-out question, a stopped run's task) is never handed back
const freeAnswerBuffers = [];
function takeAnswerBuffer() {
	const shared = freeAnswerBuffers.pop() ?? new SharedArrayBuffer(TASK_HEADER + TASK_RESULT_BYTES, { maxByteLength: TASK_RESULT_LIMIT });
	Atomics.store(new Int32Array(shared, 0, 2), 0, 0);
	return shared;
}
const returnAnswerBuffer = shared => void freeAnswerBuffers.push(shared);

// the record a task Worker wrote into a shared buffer (writeShared), undefined while none is there yet
function readShared(shared) {
	const header = new Int32Array(shared, 0, 2);
	if (Atomics.load(header, 0) === 0) return undefined;
	return JSON.parse(decode(new Uint8Array(shared, TASK_HEADER, header[1]).slice()));
}

// the reply a task Worker wrote into the fetch's shared buffer, once: undefined while none is there or it was taken
function sharedFetchReply(started) {
	const answer = started.reply || !started.shared ? undefined : readShared(started.shared);
	if (!answer) return undefined;
	returnAnswerBuffer(started.shared);
	started.shared = undefined;
	return answer.error ? started.failed(answer.error) : answer;
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

// [value, error] as src/web/fetches.rs reply gives it: a JSON object or array parsed, any other body the text
function fetchReply(holder, id) {
	const { reply, posted } = holder.fetches?.get(id) ?? {};
	if (!reply) return [null, `fetch ${id} has no reply yet`];
	if (reply.error) return [null, reply.error];
	// a server function's reply (src/lowering/web/serve.rs) is a text as text/plain, JSON of any other value (src/web/web_server.rs)
	if (posted && reply.text) return [reply.body, null];
	if (posted) try { return [JSON.parse(reply.body), null]; } catch (failure) { return [null, `the server's reply is no JSON: ${failure.message}`]; }
	const structure = structureOf(reply.body);
	if (structure !== undefined) return [structure, null];
	return [reply.body.endsWith("\n") ? reply.body : reply.body + "\n", null]; // warp convention (src/host.rs fetch)
}

// a JSON object or array of the text, undefined for any other text (src/web/web_server.rs value_of_body)
function structureOf(text) {
	try {
		const value = JSON.parse(text);
		if (value !== null && typeof value === "object") return value;
	} catch {} // not JSON
	return undefined;
}

// a channel named by a ws:// or wss:// address is a WebSocket (src/web/web_sockets.rs): one connection per address in a
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
	(holder.channels ??= new Map()).set(id, SOCKET_ADDRESS.test(name) ? socketListener(holder, hooks, name) : broadcastListener(name));
}

function socketListener(holder, hooks, name) {
	const { socket, messages } = webSocket(holder, hooks, name);
	return { name, channel: socket, messages };
}

function broadcastListener(name) {
	const listener = { name, channel: new BroadcastChannel(name), messages: [] };
	listener.channel.onmessage = ({ data }) => listener.messages.push(data);
	return listener;
}

addHostPart({
	words: (holder, hooks, { program, cString }) => ({
		// `clipboard`: the page reads it (asynchronously) while the Worker waits (askPage; shipped with this part only,
		// host_parts.rs, so a page that reads no clipboard carries none of it)
		clipboard_text: () => buildValue(program(), treeOfPlain(self.askPage("clipboard", self.askPageClipboard).text)),
		// channels between programs (src/host/channels.rs): a BroadcastChannel of the name, which reaches the other tabs and
		// workers of this page's origin; the listener's timer (lowering/signals/system_signals.rs) takes what arrived
		channel_listen: (id, channel) => listenOnChannel(holder, hooks, Number(id), plainOfTree(readNode(program(), channel))),
		channel_pending: id => BigInt(holder.channels?.get(Number(id))?.messages.length ?? 0),
		channel_next: id => buildValue(program(), treeOfPlain(holder.channels?.get(Number(id))?.messages.shift() ?? null)),
		channel_send: (channel, message) => {
			const program_ = program();
			const [name, value] = [channel, message].map(node => plainOfTree(readNode(program_, node)));
			if (SOCKET_ADDRESS.test(name)) return sendOnSocket(holder, hooks, name, value);
			new BroadcastChannel(name).postMessage(value);
		},
		// channels inside one run (P155, src/host/tasks.rs Channels): Go's unbuffered channel between the task Workers
		channel_new: () => openChannel(holder.run),
		channel_put: (id, value) => putOnChannel(holder, hooks, id, utf8.encode(JSON.stringify(readTaskValue(program(), value)))),
		channel_take: id => {
			const bytes = takeFromChannel(holder, hooks, id);
			return bytes ? buildValue(program(), JSON.parse(decode(bytes))) : null;
		},
		channel_more: id => moreOnChannel(holder, hooks, id),
		channel_close: id => closeChannel(holder.run, id),
		fetch_start: (id, url) => startFetch(holder, hooks, Number(id), plainOfTree(readNode(program(), url))),
		fetch_reply: id => buildValue(program(), treeOfPlain(fetchReply(holder, Number(id)))),
		// tasks (src/host/tasks.rs): `go f(x)` runs f in a fresh instance of the program, values copied in and out; a page
		// without cross-origin isolation has no shared memory to wait on, so the task runs at once where it starts
		task_spawn_values: (name, values) => startTask(holder, hooks, decode(cString(name)), readTaskValue(program(), values)),
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
		// a raise inside a task goes to the starting program's handlers (src/host/tasks.rs signal_send, deliver)
		task_inside: () => holder.inTask ? 1n : 0n,
		// a parallel let forks from the program only: a task's Worker does not see how many others run (src/host/tasks.rs counts them)
		task_forks: () => holder.inTask ? 0n : 1n,
		// a split float map runs on one thread in the browser: 0 makes the program call its kernel itself (src/host/tasks.rs split_map)
		task_split_mapf: () => 0n,
		signal_send: (handler, values) => {
			(holder.run.signals ??= []).push({ handler: decode(cString(handler)), values: readTaskValue(program(), values) });
			return 0n;
		},
		task_control: (id, operation) => controlTask(holder.run, id, operation),
		// where a loop starts: a task paused from its starting program waits (its control word, set by controlTask)
		task_poll: () => {
			deliverSignals(holder);
			startQueuedTasks(holder.run);
			const control = holder.control;
			if (!control) return;
			while (Atomics.load(control, 0) === CONTROL_PAUSED) Atomics.wait(control, 0, CONTROL_PAUSED);
		},
		// shared arrays (src/host/shared.rs): Ints every task of the run reaches, in shared memory when the page is isolated;
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
		startQueuedTasks(holder.run);
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
