// Messages in the page (a part of host.js, which says how parts work): the pool of helper Workers, the channels between
// programs (BroadcastChannel, WebSocket), the clipboard and the voice a run's sounds keep

const SOCKET_ADDRESS = /^wss?:\/\//; // src/web/web_sockets.rs SOCKET_SCHEMES

const TASK_WORKER = "task-worker.js";
const TASK_WORKER_READY = "ready"; // a task Worker's first message: loaded
const TASK_RESULT_BYTES = 1 << 16; // the shared buffer a Worker writes its result into, grown as needed
const TASK_RESULT_LIMIT = 1 << 28;
const TASK_HEADER = 8; // [state, length] as Int32, then the result's JSON
const PAGE_ANSWER_MS = 60000; // how long a Worker waits for its page's answer: a clipboard read may wait for a permission
// Workers a task runs on: made while the program's worker is idle (prepareTaskPool), since a Worker only starts once
// its creator returns to its event loop and a running program never does (emscripten keeps a thread pool for this)
const taskPool = [];
const TASK_POOL_SIZE = Math.min(4, self.navigator?.hardwareConcurrency ?? 2);
// how long a run waits for the pool's Workers to load before it starts anyway (a task then runs inline)
const TASK_POOL_WAIT_MS = 10000;
const TASK_POOL_POLL_MS = 10;
const hasTaskWorkers = () => self.crossOriginIsolated && self.Worker;

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

function addTaskWorker(loaded) {
	const id = taskWorkersAsked++;
	madeTaskWorkers.set(id, worker => {
		worker.taskWorkerId = id; // the page's name for it (task-workers.js postToTaskWorker)
		worker.terminate = () => self.postMessage({ endTaskWorker: id });
		// loaded: it can take tasks
		worker.onmessage = ({ data }) => {
			taskPool.push(worker);
			loaded?.();
		};
	});
	self.postMessage({ taskWorker: { id, url: new URL(TASK_WORKER, self.location.href).href } });
}

// a voice (card playground-go, natively src/sound/mod.rs Voice): when this Worker's queued sounds end on the clock the
// page and every Worker share, the seconds of sound it made in all (its sounds' places for save sound) and the
// clock's time of place 0 (where `at 2 beats` counts from). A task's voice starts where its starter's stands
let voice = {};
let voicesBeforeAt = []; // the voices `at` statements left (sound_at), taken up again by sound_at_end
const soundClock = () => (performance.timeOrigin + performance.now()) / 1000;
// a moment this little (in periods) after a cycle's start still takes that start: float rounding of the clock
const LIVE_GRID_SLACK = 1e-6;
self.soundsQueued = () => Math.max(0, (voice.end ?? 0) - soundClock()); // sound_queued()
self.voiceStopped = () => { voice = { ...voice, end: undefined }; };
self.joinVoice = starters => { voice = { ...starters }; }; // a task Worker's: its starter's; none: a run's fresh one
self.currentVoice = () => voice;
// a sound of that many seconds behind the voice's sounds, never before now: its place and its start on the clock
self.voiceSounded = seconds => {
	const at = Math.max(voice.end ?? 0, soundClock());
	const place = voice.seconds ?? 0;
	voice = { ...voice, end: at + seconds, seconds: place + seconds, origin: voice.origin ?? at - place };
	return { place, at };
};
// `at time …`: the voice placed that many seconds after its place 0; its end puts it back, keeping that origin
self.voiceAt = seconds => {
	voicesBeforeAt.push(voice);
	const origin = voice.origin ?? soundClock();
	voice = { loop: voice.loop, end: origin + seconds, seconds, origin };
};
self.insideAt = () => voicesBeforeAt.length > 0;
// where this voice's next sound would start: its queued sounds' end, never before now
self.voiceMoment = () => Math.max(voice.end ?? 0, soundClock());
// card playground-live: a named live loop's cycle starts (lib live_cycle_started). The voice is that loop's from now on,
// its sounds tagged with its name for the page; a loop the page handed over from the run before (liveHandover: name →
// that run's {start, period} of its cycles) starts its first cycle on that run's grid, where the old loop's next cycle
// would have, and the page stops the old one's sounds from there
self.liveHandover = {};
self.liveCycleStarted = name => {
	const handedOver = self.liveHandover[name];
	if (handedOver && voice.loop !== name) voice = { ...voice, end: liveStart(handedOver, self.voiceMoment()) };
	voice = { ...voice, loop: name };
	self.liveCycled?.(name, self.voiceMoment()); // a run without a page (tests, node) has no live loops to tell
};
// the first start on the grid of the old run's cycles at or after the moment (a cycle of an unknown period: its start)
const liveStart = ({ start, period }, moment) =>
	period > 0 ? start + Math.max(0, Math.ceil((moment - start) / period - LIVE_GRID_SLACK)) * period : Math.max(start, moment);
self.voiceAtEnd = () => {
	if (voicesBeforeAt.length) voice = { ...voicesBeforeAt.pop(), origin: voice.origin };
};

// play's handles (lib/sound.warp, card sound-pro): the sounds and music files of a run counted from 1, as natively, in
// one counter the run's task Workers share
let soundHandles = new Int32Array(1);
self.newSoundHandles = () => soundHandles = new Int32Array(new (self.crossOriginIsolated ? SharedArrayBuffer : ArrayBuffer)(4));
const nextSoundHandle = () => Atomics.add(soundHandles, 0, 1) + 1;
self.lastSoundHandle = () => Atomics.load(soundHandles, 0);

// the words a program's sounds and MIDI reach the page by (playground.js), posted by the program's Worker and by a task's
// alike: a task posts to the page itself (task-workers.js, card playground-live), as the program's Worker waiting for it
// in Atomics.wait could pass nothing on in time and a live loop's sounds came bunched once it was done
function addPageSoundWords(post) {
	self.playSoundFile = (url, bytes) => post({ type: "sound file", url, bytes, handle: nextSoundHandle() }); // host-files.js STD_ADAPTERS.sound: a worker has no <audio>
	self.stopSoundFiles = () => { self.voiceStopped(); post({ type: "stop sound files" }); };
	self.sendMidi = (port, message, at) => post({ type: "midi", port, message, at }); // host-files.js STD_ADAPTERS.midi
	self.stopSound = handle => post({ type: "stop sound", handle }); // stop_sound(handle)
	self.fadeSound = (handle, gain, seconds) => post({ type: "fade sound", handle, gain, seconds }); // fade_sound
	// a worker has no AudioContext: the page plays it at its start on the shared clock (voiceSounded); a signal's blocks
	// share one handle
	self.playSamples = sound => post({ type: "sound", ...sound, handle: sound.handle || nextSoundHandle() });
	self.liveCycled = (name, at) => post({ type: "live cycle", name, at }); // liveCycleStarted
	self.liveLoopsNamed = names => post({ type: "live loops", names }); // each run's, as it starts (liveLoopNames)
}

const LIVE_LOOPS_SECTION = "warp.live_loops"; // src/host.rs
// the names of the run's live loops, from its module's section (none: no section), which the page compares with the
// loops of the run before (playground.js liveLoopsNamed, decision deleted-loop)
function liveLoopNames(module) {
	const [section] = WebAssembly.Module.customSections(module, LIVE_LOOPS_SECTION);
	return section ? decode(new Uint8Array(section)).split("\n") : [];
}

// the pool of task Workers of the workers that run programs (worker.js, site-worker.js), asked for when they start, one
// after the other
function prepareTaskPool(size = TASK_POOL_SIZE) {
	if (!hasTaskWorkers()) return;
	const addFrom = index => index < size && addTaskWorker(() => addFrom(index + 1));
	addFrom(0);
}

// resolves once every task Worker of the pool has loaded: a run that starts before would run its tasks inline, where
// `stop` cannot end one (samples/tasks/threads.warp's endless spin hung the browser suite, card flaky-browser)
function taskPoolReady() {
	if (!hasTaskWorkers()) return Promise.resolve();
	const deadline = performance.now() + TASK_POOL_WAIT_MS;
	return new Promise(resolve => {
		const check = () => (taskPool.length >= TASK_POOL_SIZE || performance.now() > deadline) ? resolve() : setTimeout(check, TASK_POOL_POLL_MS);
		check();
	});
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
// answerInto) while the Worker waits, up to `waiting` milliseconds; without shared memory (a page not cross-origin
// isolated) a loud error
function askPage(what, ask, waiting = PAGE_ANSWER_MS) {
	if (!ask) throw new Error(`${what}: this page has no way to answer its Worker`);
	if (!self.crossOriginIsolated) throw new Error(`${what}: the page is not cross-origin isolated, so its Worker cannot wait for it`);
	const shared = takeAnswerBuffer();
	ask(shared);
	if (Atomics.wait(new Int32Array(shared, 0, 2), 0, 0, waiting) === "timed-out") throw new Error(`${what}: the page did not answer in ${waiting / 1000} s`);
	const { error, ...answer } = readShared(shared);
	returnAnswerBuffer(shared);
	if (error) throw new Error(`${what}: ${error}`);
	return answer;
}

// The shared buffers Workers answer in (a task's record, the page's or a GPU job's answer), each
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
	started: run => self.liveLoopsNamed?.(liveLoopNames(run.module)),
	words: (holder, hooks, { program, cString }) => ({
		// `clipboard`: the page reads it (asynchronously) while the Worker waits (askPage; shipped with this part only,
		// host_parts.rs, so a page that reads no clipboard carries none of it)
		clipboard_text: () => buildValue(program(), treeOfPlain(self.askPage("clipboard", self.askPageClipboard).text)),
		// channels between programs (src/host/channels.rs): a BroadcastChannel of the name, which reaches the other tabs and
		// workers of this page's origin; the listener's timer (lowering/signals/system_signals.rs) takes what arrived
		channel_listen: (id, channel) => listenOnChannel(holder, hooks, Number(id), plainValue(program(), channel)),
		channel_pending: id => BigInt(holder.channels?.get(Number(id))?.messages.length ?? 0),
		channel_next: id => builtValue(program(), holder.channels?.get(Number(id))?.messages.shift() ?? null),
		channel_send: (channel, message) => {
			const [name, value] = [channel, message].map(node => plainValue(program(), node));
			if (SOCKET_ADDRESS.test(name)) return sendOnSocket(holder, hooks, name, value);
			new BroadcastChannel(name).postMessage(value);
		},
	}),
	stopped: holder => {
		holder.channels?.forEach(({ channel }) => channel.close());
		holder.sockets?.forEach(({ socket }) => socket.close());
	},
});
