// The page: the editor, the worker that compiles and runs (worker.js), and the report shown like the CLI prints it:
// the value, printed output, errors, warnings, hints. An ambiguity is a warning naming its explicit form; "got it"
// silences a warning's topic in this browser (it never changes the value) until "show again". Each reading the user
// might have meant is an "I meant: …" button that rewrites the code at the warning and runs it again (notes/fixits.md).

const ACKNOWLEDGED_KEY = "warp-playground-acknowledged";
const SHOWN_FROM_KEY = "warp-playground-shown-from"; // the least diagnostic level shown, hint by default (card warning-levels)
const HINTS_KEY = "warp-playground-hints"; // the former hints checkbox: "off" now means showing from weak
const LEVELS = ["hint", "weak", "normal", "severe"]; // src/diagnostics/diagnostic.rs Level, least first
const SIZES_KEY = "warp-playground-sizes"; // the guide's width and the editor's height as the resizers left them
const GUIDE_WIDTH_RANGE = [200, 0.6]; // pixels, then the share of the page's width
const EDITOR_HEIGHT_RANGE = [120, 0.85]; // pixels, then the share of the window's height
// the most sounds sounding at once and how fast a stolen one fades out (src/sound/sound_engine.rs MAX_VOICES, STEAL_SECONDS)
const MAX_VOICES = 32;
const STEAL_SECONDS = 0.005;
const RESIZE_STEP = 20; // pixels per arrow key on a focused resizer
const OLD_ANSWERS_KEY = "warp-playground-answers"; // the Ask era kept {topic: form, "ack:<topic>": "acknowledged"}
const RUN_TIMEOUT_MS = 10000;
// a run going on longer than this, or a program left running (timers, an animation), turns Run into Stop (card
// playground-turn); a quick run never flickers
const STOP_GRACE_MS = 500;
const RUN_LABEL = "Run ▶";
const STOP_LABEL = "Stop ■";
const STOPPED = "stopped";
// card playground-live: a rerun while live loops play hands them over. The old run's Worker retires instead of stopping:
// its loops sound on until the new run's loop of the same name starts its first cycle, which they give way to; a loop the
// new run's names lack, deleted from the source, fades out in LIVE_FADE_SECONDS as soon as the new run starts (decision
// deleted-loop: the sound matches the visible code). A new run that names none in LIVE_RETIRE_SECONDS fades them all
const LIVE_HANDED_OVER = "its live loops play on into the next run";
const LIVE_FADE_SECONDS = 1;
const LIVE_RETIRE_SECONDS = 4;
// what a retired Worker still says: its loops' sounds and cycles, and the report of its run's end, which ends it
const RETIRED_MESSAGES = new Set(["sound", "live cycle", "midi", "report"]);
const LOADING_STALL_MS = 30000; // the compiler's download shows a note when it made no progress this long
// the origin a link of the shown program resolves against: "/about" is a page of the program, "https://…" is not
const PROGRAM_ORIGIN = "http://program.invalid";
const TYPING_DELAY_MS = 300;
const PENDING_VALUE = "…"; // the value shown from Run until the new one arrives
const DEFAULT_EXAMPLE = "hello";
const COMMIT_URL = "https://github.com/pannous/warp/commit/";
const SHORT_COMMIT = 9;
// terminal codes in printed text (`print "\e[H"`, samples/game_of_life.warp): clearing the screen or moving the cursor
// home starts the text anew, the other codes (colors, cursor moves) are dropped
const TERMINAL_RESTART = /\x1b\[(?:2J|H|1;1H)/g;
const TERMINAL_CODE = /\x1b\[[0-9;?]*[A-Za-z]/g;
const EXAMPLE_PARAMETERS = ["example", "sample"]; // ?example=fizzbuzz (or #fizzbuzz) picks a tour example or sample; the address shows the chosen one as #fizzbuzz
const DEBUG_PARAMETER = "debug"; // ?debug runs warp.debug.wasm: Rust names and lines in traces and the debugger
const DEBUG_COMPILER = "warp.debug.wasm";
const WAV_TYPE = "audio/wav"; // render_sound's files (host-files.js wavOf)
const SLOW_START_PARAMETER = "slow_start"; // ?slow_start=<ms>: the worker reports ready that much later (a slow machine)
// a starting worker silent this long is stuck (card firefox-hello-hang, three of six deploys): started again once, loudly
const STALLED_START_MS = 60_000;
const ACKNOWLEDGED = "acknowledged";
const ACKNOWLEDGED_PREFIX = "ack:";
const STDERR = 2;
const PAINTING_MARGIN = 10, PAINTING_BORDERS = 2; // playground.css .painting: its margin above, its border on both sides
const MIN_VIEW_HEIGHT = 160; // the view's height when the pane's text reaches the window's bottom
const STDOUT = 1;
const NOTIFICATION_TITLE = "warp";
// card report-button: ?report turns the report button on in this browser, ?report=off off; only for who knows it
const REPORT_PARAMETER = "report";
const REPORT_KEY = "warp-playground-report";
const REPORT_OFF = "off";
const ISSUE_URL = "https://github.com/pannous/warp/issues/new";
const REPORT_LIMITS = { code: 6000, output: 1500 }; // characters: an issue address stays under GitHub's 8 KB
// a page event, or one element's (`on click·1`, src/lowering/web/element_events.rs)
const PAGE_EVENT = /^on ((?:click|key|input)(?:·\d+)?|(?:change|focus|blur)·\d+)$/;
const DARK_MODE_QUERY = "(prefers-color-scheme: dark)";
const SOUND_OFFSET = 32768; // a sound's samples cross as amplitude + 32768 (lib/sound.warp, src/sound/mod.rs SAMPLE_OFFSET)

const $ = id => document.getElementById(id);

// this browser's localStorage (the other page scripts use it too), which a private window may refuse: then nothing is
// remembered beyond the page
function stored(key) {
	try { return localStorage.getItem(key); } catch { return null; }
}
function store(key, value) {
	try { value ? localStorage.setItem(key, value) : localStorage.removeItem(key); } catch { /* not remembered */ }
}
// a stored JSON value, null when there is none or it is no JSON
function storedJson(key) {
	try { return JSON.parse(stored(key)); } catch { return null; }
}
function element(tag, properties = {}, ...children) {
	const node = Object.assign(document.createElement(tag), properties);
	node.append(...children);
	return node;
}

let worker;
let workerReady;
let workerStage = "not started"; // the worker's last stage (worker.js stage), for playground.state
let workerSettled = "starting"; // workerReady's outcome
let workerWarmed; // the worker was asked to compile the markup renderer ahead of the first markup run (worker.js warmUp)
let nextRunId = 0;
let pending; // {id, resolve, printed, timer} of the run in the worker
let liveHandover = {}; // the next run's live loops of the run before it: name → {start, period} (host-tasks.js liveCycleStarted)
const retiredWorkers = new Set(); // the Workers whose live loops sound on into the next run (handOver)
let runs = Promise.resolve(); // the evaluations, one after another
let showing = false; // the page is evaluating its editor's code
let queued; // the editor's code to show once the current run is shown
let typingTimer;
const debugBuild = new URLSearchParams(location.search).has(DEBUG_PARAMETER);
let lastModule; // the bytes of the last module the compiler emitted, for the download button

// ---- acknowledged topics, remembered in this browser ---------------------------------------------------------

function loadAcknowledged() {
	const saved = storedJson(ACKNOWLEDGED_KEY);
	if (Array.isArray(saved)) return saved;
	const old = storedJson(OLD_ANSWERS_KEY) ?? {};
	return Object.keys(old).filter(key => key.startsWith(ACKNOWLEDGED_PREFIX)).map(key => key.slice(ACKNOWLEDGED_PREFIX.length));
}

function saveAcknowledged(topics) {
	acknowledged = [...new Set(topics)].sort();
	store(ACKNOWLEDGED_KEY, JSON.stringify(acknowledged));
}

let acknowledged = loadAcknowledged();

// ---- the resizers: the guide's width and the editor's height, remembered in this browser -------------------

let sizes = storedJson(SIZES_KEY) ?? {};
const clamp = (value, [least, share], whole) => Math.round(Math.min(Math.max(value, least), whole * share));

function applySizes() {
	const main = document.querySelector("main");
	if (sizes.guide) main.style.setProperty("--guide-width", `${clamp(sizes.guide, GUIDE_WIDTH_RANGE, main.clientWidth)}px`);
	else main.style.removeProperty("--guide-width");
	editor.getWrapperElement().style.height = sizes.editor ? `${clamp(sizes.editor, EDITOR_HEIGHT_RANGE, innerHeight)}px` : "";
	editor.refresh();
}

function setSize(name, value) {
	if (value === undefined) delete sizes[name];
	else sizes[name] = Math.round(value);
	applySizes();
	store(SIZES_KEY, JSON.stringify(sizes));
}

// drag the handle (or press its arrow keys) to set a size; sizeAt(pointer event) is the size there, current() the size now
function dragToResize(handle, name, sizeAt, current, [lessKey, moreKey]) {
	handle.addEventListener("pointerdown", down => {
		down.preventDefault();
		handle.setPointerCapture(down.pointerId);
		handle.classList.add("dragging");
		const move = event => setSize(name, sizeAt(event));
		handle.addEventListener("pointermove", move);
		handle.addEventListener("pointerup", () => { handle.removeEventListener("pointermove", move); handle.classList.remove("dragging"); }, { once: true });
	});
	handle.addEventListener("keydown", key => {
		const step = { [lessKey]: -RESIZE_STEP, [moreKey]: RESIZE_STEP }[key.key];
		if (step) { key.preventDefault(); setSize(name, current() + step); }
	});
	handle.ondblclick = () => setSize(name, undefined);
}

function startResizers() {
	const main = document.querySelector("main"), editorPane = document.querySelector(".editor-pane");
	dragToResize($("guide-resizer"), "guide", event => event.clientX - main.getBoundingClientRect().left, () => $("guide").offsetWidth, ["ArrowLeft", "ArrowRight"]);
	dragToResize($("editor-resizer"), "editor", event => event.clientY - editorPane.getBoundingClientRect().top, () => editorPane.offsetHeight, ["ArrowUp", "ArrowDown"]);
	addEventListener("resize", applySizes);
	applySizes();
}

// what web_evaluate takes: `ack:<topic>` keys (the newer compiler also takes the plain list of topics)
const acknowledgements = () => Object.fromEntries(acknowledged.map(topic => [ACKNOWLEDGED_PREFIX + topic, ACKNOWLEDGED]));

function acknowledge(topic) {
	saveAcknowledged([...acknowledged, topic]);
	runNow();
}

function showAgain(topic) {
	saveAcknowledged(acknowledged.filter(known => known !== topic));
	runNow();
}

// ---- the worker ---------------------------------------------------------------------------------------------

// a starting worker silent for STALLED_START_MS: started again once (a warning), then its runs fail naming its stage
function restartStalledWorker(stalledWorker, restarts, resolve, reject) {
	const stall = `the compiler's worker gave no sign for ${STALLED_START_MS / 1000} s at stage "${workerStage}"`;
	if (restarts > 0) return reject(new Error(stall));
	console.warn(`${stall}: starting it again`);
	stalledWorker.terminate();
	startWorker(restarts + 1);
	resolve(workerReady);
}

function startWorker(restarts = 0) {
	workerWarmed = false;
	const options = new URLSearchParams();
	if (debugBuild) options.set("compiler", DEBUG_COMPILER);
	// test_in_browser.py --examples: a worker that starts as late as on a slow CI runner
	const slowStart = new URLSearchParams(location.search).get(SLOW_START_PARAMETER);
	if (slowStart) options.set(SLOW_START_PARAMETER, slowStart);
	worker = serveTaskWorkers(new Worker(options.size ? `worker.js?${options}` : "worker.js")); // task-workers.js
	worker.liveLoops = new Map(); // its run's live loops: name → {start, period} of their last cycle (liveCycled)
	showLoading("starting the compiler's worker", false);
	workerReady = new Promise((resolve, reject) => {
		const starting = worker;
		let stalled;
		// each message of the starting worker shows it alive: the stall timer starts over
		const watchStart = () => {
			clearTimeout(stalled);
			stalled = setTimeout(() => restartStalledWorker(starting, restarts, resolve, reject), STALLED_START_MS);
		};
		const settled = outcome => value => {
			clearTimeout(stalled);
			stalled = undefined;
			outcome(value);
		};
		const [ready, failed] = [settled(resolve), settled(reject)];
		watchStart();
		// worker.js or a script it imports failed to load: without this the first run waits for "ready" forever (card firefox-hello-hang)
		worker.onerror = event => failed(new Error(`the compiler's worker failed to start: ${event.message || "worker.js did not load"}`));
		worker.onmessage = ({ data }) => {
			if (starting.retired && !RETIRED_MESSAGES.has(data?.type)) return;
			if (starting.retired && data.type === "report") return retire(starting);
			if (data.type === "notify") data = notification(data.text);
			if (data.type === "window") return resizeWindow(data, $("paintings")); // canvas.js
			if (data.type === "dispatch") return dispatchToPage(data.event); // markup.js, card page-custom-event
			if (!data) return;
			if (stalled) watchStart();
			if (data.type === "stage") return workerStage = data.stage;
			if (data.type === "loading") {
				workerStage = `downloading the compiler: ${megabytes(data.loaded)} MB so far`; // a stalled download says how far
				return showLoading(`loading the compiler: ${megabytes(data.loaded)}${data.total ? ` of ${megabytes(data.total)}` : ""} MB`);
			}
			if (data.type === "compiling") return stopLoading();
			if (data.type === "ready") return ready();
			if (data.type === "stored") return keepValue(data.name, data.value, data.file);
			if (data.type === "clipboard") return copyText(data.text);
			if (data.type === "clipboard read") return pasteInto(data.shared);
			if (data.type === "page call") return answerPageCall(data.shared, data.request, starting); // task-workers.js
			if (data.type === "failed") return failed(new Error(data.message));
			if (data.type === "bundle") return bundled(data.bundle); // deploy.js
			if (data.type === "members") return answerMembers(data);
			if (data.type === "sound") return playSound(data, starting);
			if (data.type === "live cycle") return liveCycled(starting, data.name, data.at);
			if (data.type === "live loops") return liveLoopsNamed(data.names);
			if (data.type === "sound file") return playSoundFile(data.url, data.handle, data.bytes);
			if (data.type === "file") return offerFile(data.path, data.bytes);
			if (data.type === "midi") return sendMidi(data.port, data.message, data.at);
			if (data.type === "stop sound files") return silence(); // stop_sound: the queued sounds and the music files
			if (data.type === "stop sound") return silenced(data.handle); // stop_sound(handle): that one alone
			if (data.type === "fade sound") return faded(data.handle, data.gain, data.seconds); // fade_sound
			if (data.type === "gpu canvas") return sendPaintCanvas(data);
			if (!pending) return showEventOutput(data);
			if (data.type === "listening") Object.assign(pending, { listening: data.events, address: data.address });
			if (data.type === "print") printedChunk(pending, data);
			if (data.type === "sleep") {
				pending.frame++;
				slept(pending, data.milliseconds);
			}
			if (data.type === "tasks inline") pending.tasksInline = data.reason;
			if (data.type === "paint") painted(pending, paintingOf(data));
			if (data.type === "module") lastModule = data.bytes;
			if (data.type === "report" && data.id === pending.id) finish(pending, { ...data.report, printed: pending.printed, paintings: pending.paintings, listening: pending.listening ?? [], address: pending.address, milliseconds: data.milliseconds });
		};
	});
	// a slow start (the CI runner) finishes after the first run began: "ready" then must not hide its "running…"
	workerReady.then(() => stopLoading() || showing || setStatus("ready"), failure => stopLoading() || setStatus(failure.message, true));
	workerSettled = "starting";
	workerReady.then(() => workerSettled = "ready", failure => workerSettled = `failed: ${failure.message}`);
	tellSystemValues();
	sharePointer();
	worker.postMessage({ stored: keptValues(), session: keptValues([SESSION_STORE]) });
	tellEnvironment();
}

// the members to offer after the code and a dot (completion.js), from the compiler (src/lowering/data/members.rs)
const memberRequests = new Map(); // request id → its promise's resolve
let memberRequestCount = 0;
function askMembers(code) {
	const id = ++memberRequestCount;
	return new Promise(resolve => {
		memberRequests.set(id, resolve);
		worker.postMessage({ members: code, id });
	});
}

function answerMembers({ id, names }) {
	memberRequests.get(id)?.(names);
	memberRequests.delete(id);
}

// the compiler's download (worker.js downloaded), beside the status; without progress for LOADING_STALL_MS it says so,
// so a stalled download is seen instead of a page that waits (card firefox-hello-hang)
let loadingStall;
function showLoading(progress, visible = true) {
	$("loading").textContent = progress;
	$("loading").hidden = !visible;
	clearTimeout(loadingStall);
	loadingStall = setTimeout(() => {
		$("loading").textContent = `${progress}; no progress for ${LOADING_STALL_MS / 1000} s: a slow or stalled connection`;
		$("loading").hidden = false;
	}, LOADING_STALL_MS);
}

function stopLoading() {
	clearTimeout(loadingStall);
	$("loading").hidden = true;
}

const megabytes = bytes => (bytes / 1e6).toFixed(1);

// the program's environment (assistant.js: the stand-in of the API key), sent again when the key changes
const tellEnvironment = () => worker.postMessage(programEnvironment());

// `notify "text"`: the browser's notification once the page may show them; until then (or when refused) a printed
// line, so the text never goes missing; the first one asks for the permission
function notification(text) {
	if (globalThis.Notification?.permission === "granted") return void new Notification(NOTIFICATION_TITLE, { body: text });
	globalThis.Notification?.requestPermission?.();
	return { type: "print", text: `notification: ${text}\n`, stream: STDOUT };
}

// the system values a Worker cannot read itself (host.js system_value), sent again when they change. window_open: the
// canvas shows the pictures to someone once the page was clicked or typed in; before (the CI tour, as for sounds) a
// `while window_open` drawing loop ends at once instead of waiting for input forever
const darkMode = matchMedia(DARK_MODE_QUERY);
const tellSystemValues = () => worker.postMessage({ system: { "dark mode": darkMode.matches, window_open: Number(!beforeUserActivation()), display_width: screen.width, display_height: screen.height, ...viewSize() } });
darkMode.addEventListener("change", tellSystemValues);
["pointerdown", "keydown"].forEach(type => addEventListener(type, () => worker && tellSystemValues(), { once: true, capture: true }));
new ResizeObserver(() => worker && tellSystemValues()).observe($("output"));
document.addEventListener("fullscreenchange", () => worker && tellSystemValues());

// view_width, view_height (use draw's default canvas, paint(shader) without a size): the room below the output pane's
// text, in CSS pixels: the pane's width, down to the window's bottom; none while the pane is folded away, so host.js's
// defaults stand. Never measured from the paintings: the pane grows with them, so a view-sized painting made the next
// view taller each frame, until Safari ran out of memory (card webgpu-safari). What follows the paintings down to the
// page's end is left free: a view-sized painting that overflowed the window brought a scrollbar, which narrowed the
// next view (card tour-firefox). Measured again as each run starts: the text above the paintings moves without
// resizing the pane, which is all the ResizeObserver sees. In full screen (⛶) the view is the screen, so a shader
// painted at the view's size renders at the screen's (card full-screen)
function viewSize() {
	if (document.fullscreenElement === $("painted")) return { view_width: innerWidth, view_height: innerHeight };
	const pane = $("output"), style = getComputedStyle(pane), painted = $("painted").getBoundingClientRect();
	const width = Math.floor(pane.clientWidth - parseFloat(style.paddingLeft) - parseFloat(style.paddingRight) - PAINTING_BORDERS);
	const room = innerHeight - Math.max(painted.top, 0) - PAINTING_MARGIN - PAINTING_BORDERS - roomBelowPaintings(style);
	return width > 0 ? { view_width: width, view_height: Math.floor(Math.max(room, MIN_VIEW_HEIGHT)) } : {};
}

// the diagnostics, the pane's padding and border and the page's padding below the paintings, in CSS pixels
function roomBelowPaintings(paneStyle) {
	const diagnostics = $("diagnostics"), listStyle = getComputedStyle(diagnostics), pixels = parseFloat;
	return diagnostics.offsetHeight + pixels(listStyle.marginTop) + pixels(listStyle.marginBottom)
		+ pixels(paneStyle.paddingBottom) + pixels(paneStyle.borderBottomWidth) + pixels(getComputedStyle(document.body).paddingBottom);
}

// `loop { …; show(); sleep(16ms) }`: each sleep starts an animation's next frame, what the run paints or prints in it
// replaces the last frame's; frames keep the run alive past RUN_TIMEOUT_MS, the next run stops it (cards
// drawing-frames, snake-frames)
function startsFrame(run, output) {
	const starts = run.frame > 0 && run.framesShown[output] !== run.frame;
	run.framesShown[output] = run.frame;
	if (starts) {
		run.animating = true;
		stopAfterTimeout(run);
	}
	return starts;
}

// ---- sound ----------------------------------------------------------------------------------------------------

let audio; // the page's AudioContext, made at the first sound after a click or key press
// the clock the page and the Workers share for sounds (host-tasks.js soundClock), in seconds
const soundClock = () => (performance.timeOrigin + performance.now()) / 1000;
// the AudioContext's time minus soundClock, set anew when nothing sounds and kept while sounds follow, so a voice's
// sounds join without a gap
let clockOffset = 0;
let sounding = []; // the playing and queued sounds, silenced when the next run starts (typing reruns the code)

// before the page's first click or key press (the run on load, the CI tour) a sound stays silent: an AudioContext made
// then cannot start and the browser warns
const beforeUserActivation = () => navigator.userActivation && !navigator.userActivation.hasBeenActive;
function playSound({ samples, rate, channels, at, handle, loop }, from) {
	if (beforeUserActivation()) return;
	audio ??= new AudioContext();
	audio.resume();
	const frames = Math.max(Math.ceil(samples.length / channels), 1);
	const buffer = audio.createBuffer(channels, frames, rate);
	for (let channel = 0; channel < channels; channel++)
		buffer.getChannelData(channel).set(Float32Array.from({ length: frames }, (_, frame) => ((samples[frame * channels + channel] ?? SOUND_OFFSET) - SOUND_OFFSET) / SOUND_OFFSET));
	const source = audio.createBufferSource();
	source.buffer = buffer;
	source.gainNode = audio.createGain(); // fade_sound ramps it
	source.connect(source.gainNode).connect(audio.destination);
	if (sounding.length === 0) clockOffset = audio.currentTime - soundClock();
	source.startsAt = Math.max(audio.currentTime, at + clockOffset);
	source.endsAt = source.startsAt + buffer.duration;
	stealFor(source.startsAt);
	source.start(source.startsAt);
	Object.assign(source, { handle, loop, from });
	source.onended = () => sounding = sounding.filter(other => other !== source);
	sounding.push(source);
}

// midi_send / midi_on (card midi-out): Web MIDI, asked for once and kept; a browser without it, a refusal or a port that
// is not there reaches the console, as sound failures do. The first output when the port is "". Before the page's first
// click or key press (the run on load, the CI tour) a message goes nowhere, as a sound stays silent: asking for Web MIDI
// then is refused, and the refusal would land as a console error during whatever runs next. Only a granted access is
// kept: after a refusal the next message asks again (card webmidi-requires)
let midiAccess;
// Firefox refuses without asking ("WebMIDI requires a site permission add-on to activate") when it saw no MIDI device
// at its start; only then does it offer the add-on that grants the site access
const MIDI_REFUSED = "the browser refused Web MIDI. Firefox does so without asking when no MIDI device was connected as it started: connect the device, restart Firefox, then accept its site permission add-on. Otherwise allow MIDI in the site settings";
// `at`: when on the clock the page and the Workers share (host-tasks.js soundClock, seconds); Web MIDI's own timestamp
// sends it then, a past one at once (card midi-clock)
async function sendMidi(port, message, at) {
	if (beforeUserActivation()) return;
	try {
		if (!navigator.requestMIDIAccess) throw new Error("this browser has no Web MIDI");
		midiAccess ??= navigator.requestMIDIAccess().catch(error => {
			midiAccess = undefined;
			throw error.name === "SecurityError" ? new Error(`${MIDI_REFUSED} (${error.message})`) : error;
		});
		const outputs = [...(await midiAccess).outputs.values()];
		const output = port ? outputs.find(candidate => candidate.name === port) : outputs[0];
		if (!output) throw new Error(port ? `no MIDI output named ${port} (there are: ${outputs.map(candidate => candidate.name).join(", ") || "none"})` : "no MIDI output device");
		output.send(message, at ? at * 1000 - performance.timeOrigin : undefined);
	} catch (error) {
		console.error(`midi: ${error.message}`);
	}
}

// `play "song.mp3"` (card sound-library): a music file in the background, by its URL beside the page or anywhere
let playingFiles = [];
function playSoundFile(url, handle, bytes) {
	if (beforeUserActivation()) return;
	const player = new Audio(bytes ? URL.createObjectURL(new Blob([bytes], { type: WAV_TYPE })) : url);
	player.handle = handle;
	player.onended = () => playingFiles = playingFiles.filter(other => other !== player);
	player.play().catch(error => console.error(`play "${url}": ${error.message}`));
	playingFiles.push(player);
}

function stopSoundFiles() {
	playingFiles.forEach(player => player.pause());
	playingFiles = [];
}

// the sound or music file of that handle stopped; a queued one never starts
function silenced(handle) {
	sounding.filter(source => source.handle === handle).forEach(source => source.stop());
	playingFiles.filter(player => player.handle === handle).forEach(player => player.pause());
	sounding = sounding.filter(source => source.handle !== handle);
	playingFiles = playingFiles.filter(player => player.handle !== handle);
}

// room for one more sound sounding at that time (src/sound/sound_engine.rs MAX_VOICES): the oldest of MAX_VOICES sounding then
// fades out in STEAL_SECONDS and stops
function stealFor(time) {
	const playing = sounding.filter(source => !source.stolen && source.startsAt <= time && time < source.endsAt);
	if (playing.length < MAX_VOICES) return;
	const oldest = playing.reduce((first, source) => source.startsAt < first.startsAt ? source : first);
	oldest.stolen = true;
	fadeOut(oldest, Math.max(audio.currentTime, time), STEAL_SECONDS);
}

// the sound ramps down to silence from `from` (audio clock) and stops there
function fadeOut(source, from, seconds) {
	const level = source.gainNode.gain;
	level.setValueAtTime(level.value, from);
	level.linearRampToValueAtTime(0, from + seconds);
	source.stop(from + seconds);
}

// fade_sound(handle, gain, seconds): the sounds of that handle (0 all) ramp from their gain now to `gain`
function faded(handle, gain, seconds) {
	for (const source of sounding.filter(source => handle === 0 || source.handle === handle)) {
		const level = source.gainNode.gain;
		level.cancelScheduledValues(audio.currentTime);
		level.setValueAtTime(level.value, audio.currentTime);
		level.linearRampToValueAtTime(gain, audio.currentTime + seconds);
	}
}

// the sounds stopped and the music files, but those of live loops handed over to the next run (stopPressed: all)
function silence(all = true) {
	const handedOver = source => !all && source.from?.retired?.waiting.has(source.loop);
	sounding.filter(source => !handedOver(source)).forEach(source => source.stop());
	sounding = sounding.filter(handedOver);
	stopSoundFiles();
}

// ---- live loops handed over (card playground-live) -------------------------------------------------------------

// a named loop's cycle starts at `at` (shared clock) in that Worker's run: the run's own loop gives the retired run's loop
// of its name way there; a retired loop past its time ends at a cycle's start
function liveCycled(from, name, at) {
	const known = from.liveLoops.get(name);
	from.liveLoops.set(name, { start: at, period: known ? at - known.start : undefined });
	if (from.retired) {
		if (soundClock() - from.retired.at > LIVE_RETIRE_SECONDS) endHandedOver(from, name, at);
	} else if (!known) retiredWorkers.forEach(retired => endHandedOver(retired, name, at));
}

const playsLive = run => run && worker.liveLoops?.size > 0 && worker.runId === run.id;

// the retired Worker's loop of that name sounds until `at`, the Worker goes once none of its loops is left
function endHandedOver(retired, name, at) {
	if (!retired.retired.waiting.delete(name)) return;
	const end = at + clockOffset;
	sounding.filter(source => source.from === retired && source.loop === name && source.endsAt > end).forEach(source => source.stop(Math.max(end, audio.currentTime)));
	if (retired.retired.waiting.size === 0) retire(retired);
}

// the new run's live loops, named as it starts (host-tasks.js liveLoopNames): a retired loop it lacks is deleted from the
// source and fades out at once, the others give way at their cycle
function liveLoopsNamed(names) {
	retiredWorkers.forEach(retired => {
		clearTimeout(retired.retired.unnamed);
		fadeDeleted(retired, [...retired.retired.waiting].filter(name => !names.includes(name)));
	});
}

// the retired Worker's loops of those names fade out, the Worker goes once none of its loops is left
function fadeDeleted(retired, names) {
	const deleted = source => source.from === retired && names.includes(source.loop);
	sounding.filter(deleted).forEach(source => fadeOut(source, audio.currentTime, LIVE_FADE_SECONDS));
	names.forEach(name => retired.retired.waiting.delete(name));
	if (retired.retired.waiting.size === 0) retire(retired);
}

// the safety net: a new run that never named its loops (it failed before it started) leaves none of the old ones playing
function unnamedLoopsFade(retired) {
	console.warn(`the rerun named no live loops in ${LIVE_RETIRE_SECONDS} s: the loops of the run before fade out`);
	fadeDeleted(retired, [...retired.retired.waiting]);
}

function retire(retired) {
	clearTimeout(retired.retired?.unnamed);
	retired.terminate();
	retiredWorkers.delete(retired);
}

// the run's loops go on while the next run starts in a fresh Worker: their last cycle's start and period (a loop of one
// cycle so far: the end of its queued sounds) for the next run, whose loops of their names start on that grid
function handOver(run) {
	const retiring = worker;
	const queuedEnd = name => Math.max(...sounding.filter(source => source.from === retiring && source.loop === name).map(source => source.endsAt - clockOffset));
	liveHandover = Object.fromEntries([...retiring.liveLoops].map(([name, { start, period }]) =>
		[name, period > 0 ? { start, period } : { start: queuedEnd(name), period: 0 }]));
	const unnamed = setTimeout(() => unnamedLoopsFade(retiring), LIVE_RETIRE_SECONDS * 1000);
	retiring.retired = { at: soundClock(), waiting: new Set(retiring.liveLoops.keys()), unnamed };
	retiredWorkers.add(retiring);
	sounding.filter(source => source.from === retiring && !source.loop).forEach(source => source.stop());
	startWorker();
	finish(run, stoppedReport(run, LIVE_HANDED_OVER, false));
}

// each painting shows at once, while the program still runs: one the size of the last replaces it, as a frame (card
// g_oldM: `if frame % 6 == 0 { render() }` animates without a sleep, as the native window does)
function painted(run, painting) {
	const last = run.paintings.at(-1);
	if (startsFrame(run, "paintings")) run.paintings = [];
	else if (last && sameSize(last, painting)) run.paintings.pop();
	run.paintings.push(painting);
	showFrame(run.paintings);
}

// a text frame (`render(); sleep(.1s)`) arrives line by line: the last one is shown whole once the next one begins.
// Before any frame each print shows at once, so a print before a slow paint is read while it renders (card print-watch)
function printedChunk(run, chunk) {
	if (startsFrame(run, "printed")) {
		showPrinted(run.printed);
		run.printed = [];
	}
	run.printed.push(chunk);
	if (!run.animating) showPrinted(run.printed);
}

// the runaway limit counts running time only: a sleep moves the deadline on by its milliseconds (card sleep-run-limit)
function stopAfterTimeout(run, milliseconds = RUN_TIMEOUT_MS) {
	clearTimeout(run.timer);
	run.deadline = Date.now() + milliseconds;
	run.timer = setTimeout(() => stopRun(run, `stopped after ${RUN_TIMEOUT_MS / 1000} s: ${run.tasksInline ?? "the program may not terminate"}`), milliseconds);
}
const slept = (run, milliseconds) => stopAfterTimeout(run, run.deadline - Date.now() + milliseconds);

// a program that does not stop blocks the worker: replace it
function stopRun(run, message, failed = !run.animating) {
	worker.terminate();
	startWorker();
	finish(run, stoppedReport(run, message, failed));
}

const stoppedReport = (run, message, failed) => ({ value: message, error: failed, printed: run.printed, paintings: run.paintings,
	warnings: [], runtime_warnings: [], hints: [], notes: [] });

// the Run button while a program runs: Stop (card playground-turn)
function showStop(stoppable) {
	$("run").textContent = stoppable ? STOP_LABEL : RUN_LABEL;
	$("run").title = stoppable ? "stop the program" : "Ctrl/Cmd-Enter";
	$("run").classList.toggle("stop", stoppable);
}
const showsStop = () => $("run").classList.contains("stop");

// Stop pressed: a run going on ends as stopped; a program left listening to its timers stops listening
function stopPressed() {
	retiredWorkers.forEach(retire);
	silence();
	if (pending) return stopRun(pending, STOPPED, false);
	worker.terminate();
	startWorker();
	listenTo([]);
	setStatus(STOPPED);
}

function finish(run, report) {
	clearTimeout(run.timer);
	clearTimeout(run.stopTimer);
	if (pending === run) pending = undefined;
	showStop(false); // the shown report's timers turn it back to Stop (listenTo)
	run.resolve(report);
}

// evaluate code in the worker: the report of src/web/mod.rs evaluate plus `printed` [{text, stream}]. The worker runs one
// program at a time, so each call waits for the calls before it (typing while the compiler still loads makes several)
function evaluate(code) {
	const run = runs.then(() => runInWorker(code));
	runs = run.catch(() => {});
	return run;
}

async function runInWorker(code) {
	await workerReady;
	return new Promise(resolve => {
		const run = { id: ++nextRunId, resolve, printed: [], paintings: [], frame: 0, framesShown: {} };
		stopAfterTimeout(run);
		run.stopTimer = setTimeout(() => pending === run && showStop(true), STOP_GRACE_MS);
		pending = run;
		silence(false);
		Object.assign(worker, { runId: run.id, liveLoops: new Map() });
		paintingWindow = undefined; // canvas.js: each run's paintings at their own size until it sets window.width
		tellSystemValues();
		worker.postMessage({ id: run.id, code, acknowledged: acknowledgements(), file: programFile(), liveHandover });
		liveHandover = {};
	});
}

// ---- showing a report ---------------------------------------------------------------------------------------

function setStatus(text, failed = false) {
	$("status").textContent = text;
	$("status").classList.toggle("failed", failed);
}

const at = (line, column) => line ? `${line}:${column}` : "";

const POSITION = /^(\d+):(\d+)$/;

// the editor's cursor at the 1-based line and column, in view
function jumpTo(line, column) {
	editor.focus();
	editor.setCursor({ line: line - 1, ch: column - 1 });
	editor.scrollIntoView(null, 40);
}

// a line:column goes there when clicked
function positionLink(position) {
	const [, line, column] = position.match(POSITION) ?? [];
	if (!line) return element("span", { className: "position" }, position);
	return element("button", { className: "position", title: "go there", onclick: () => jumpTo(Number(line), Number(column)) }, position);
}

// a warning's label as the CLI prints it: "warning", or "warning (weak)" and "warning (severe)"
const warningKind = level => !level || level === "normal" ? "warning" : `warning (${level})`;

function diagnostic(kind, position, ...content) {
	return element("li", { className: kind.split(" ")[0] }, element("span", { className: "label" }, kind), position ? positionLink(position) : "", ...content);
}

const code = text => element("code", {}, text);

// apply a fix's edits to the editor's text (UTF-16 ranges, src/web/mod.rs fixes_json) and run the changed code
function applyFix(fix) {
	// every edit of the fix, the last in the text first, so the earlier offsets stay valid
	for (const edit of fix.edits) editor.replaceRange(edit.replacement, editor.posFromIndex(edit.start), editor.posFromIndex(edit.end));
	runChanged();
}

// the "I meant: …" buttons of a warning, error or hint; a fix whose text the code does not show cannot be applied
function fixButtons(fixes = []) {
	if (fixes.length === 0) return "";
	return element("span", { className: "fixes" }, ...fixes.map(fix => fix.start === null
		? element("button", { disabled: true, title: `${fix.meaning} (could not find \`${fix.written}\` in the code)` }, fix.label)
		: element("button", { className: "apply-fix", title: fix.meaning, onclick: () => applyFix(fix) }, fix.label)));
}

const GOT_IT_COMMENT = " // got it";

// a `// got it` comment at the end of the warning's line silences that line's warnings in the code itself
function commentGotIt(line) {
	const index = line - 1;
	editor.replaceRange(GOT_IT_COMMENT, { line: index, ch: editor.getLine(index).length });
	runChanged();
}

// "got it" (user, 2026-10-05): for this expression (its `topic@expression` key), for all of the kind (the topic), or
// as a `// got it` comment on its line; each keeps the code and never changes the value
function gotIt(topic, expression, line) {
	const button = (label, title, onclick) => element("button", { className: "got-it", title, onclick }, label);
	return element("span", { className: "got-it-choices" },
		expression ? button("got it", "stop warning about this expression", () => acknowledge(expression)) : "",
		button(`got it: all ${topic}`, `stop warning about every ${topic}`, () => acknowledge(topic)),
		line ? button("// got it", "say it in the code: a // got it comment on this line", () => commentGotIt(line)) : "");
}

function showAcknowledged() {
	$("acknowledged").replaceChildren(...acknowledged.map(topic => element("li", {}, `${topic} `,
		element("button", { className: "forget", title: "warn about it again", onclick: () => showAgain(topic) }, "show again"))));
	$("silenced").hidden = acknowledged.length === 0;
}

// the » value line; markup the page renders (#rendered) is shown there only, its text would repeat it (card hide-html)
function showValue(value, error, html) {
	$("value").textContent = value;
	$("value").classList.toggle("error", Boolean(error));
	$("value").parentElement.hidden = Boolean(html) && !error;
}

function showReport(report) {
	showValue(report.value, report.error, report.html);
	const errorAt = report.error ? report.error_at : null; // the failing place: clicking the error goes there
	$("value").classList.toggle("located", Boolean(errorAt));
	$("value").title = errorAt ? `go to ${at(errorAt.line, errorAt.column)}` : "";
	$("value").onclick = errorAt ? () => jumpTo(errorAt.line, errorAt.column) : null;
	showPrinted(report.printed);
	showRendered(report.html);
	showPaintings(report.paintings ?? []);
	listenTo(report.listening ?? []);
	showAddress(report.address);
	const isShown = level => LEVELS.indexOf(level ?? "normal") >= LEVELS.indexOf($("shown-from").value);
	const hints = isShown("hint") ? report.hints : [];
	const notes = report.notes ?? [];
	const inline = new Set((report.warnings ?? []).map(warning => warning.topic).filter(topic => notes.includes(topic)));
	const expressionOf = topic => (report.got_it ?? []).find(offer => offer.topic === topic)?.expression;
	const shown = (kind, problem, extra = "") => diagnostic(kind, at(problem.line, problem.column), problem.message,
		problem.fix ? element("span", { className: "fix" }, "fix: ", code(problem.fix)) : "", fixButtons(problem.fixes), extra);
	const items = [
		...(report.errors ?? []).map(error => shown("error", error)),
		...report.warnings.filter(warning => isShown(warning.level)).map(warning => shown(warningKind(warning.level), warning,
			inline.has(warning.topic) ? gotIt(warning.topic, warning.expression_key, warning.line) : "")),
		...(isShown("normal") ? report.runtime_warnings : []).map(message => diagnostic("warning", "runtime", message)),
		...hints.map(hint => diagnostic("hint", hint.position, "prefer ", code(hint.canonical), " over ", code(hint.original),
			element("span", { className: "reason" }, hint.reason), fixButtons(hint.fixes))),
		...notes.filter(topic => !inline.has(topic) && isShown("hint")).map(topic => diagnostic("note", "", `the ${topic} note above shows until you say `, gotIt(topic, expressionOf(topic)))),
	];
	$("diagnostics").replaceChildren(...items);
	markPositions(report);
	showAcknowledged();
	setStatus(report.crashed ? "the compiler crashed; reloaded" : `${Math.round(report.milliseconds ?? 0)} ms`, report.crashed);
}

let positionMarks = []; // the editor's underlines of the shown report

// the word at each error and warning position underlined in the editor, its message on hover (src/web/mod.rs error_at is
// the position of an error no diagnostic made); hints are not marked: their positions can lag behind (card hint-positions)
function markPositions(report) {
	positionMarks.forEach(mark => mark.clear());
	const errors = report.errors?.length ? report.errors : [report.error_at && { ...report.error_at, message: report.value }].filter(Boolean);
	const places = [...errors.map(error => ["error", error]), ...(report.warnings ?? []).map(warning => ["warning", warning])];
	positionMarks = places.filter(([, { line, column }]) => line > 0 && column > 0 && line <= editor.lineCount())
		.map(([kind, { line, column, message }]) => markWord(kind, line - 1, column - 1, message)).filter(Boolean);
}

// the word (or the one character) at a position in characters, as the compiler counts, not UTF-16 units
function markWord(kind, line, column, message) {
	const characters = Array.from(editor.getLine(line));
	const rest = characters.slice(column).join("");
	const marked = rest.match(/^[\p{L}\p{N}_]+/u)?.[0] ?? Array.from(rest)[0];
	if (!marked) return null;
	const ch = characters.slice(0, column).join("").length;
	return editor.markText({ line, ch }, { line, ch: ch + marked.length }, { className: `marked-${kind}`, title: message ?? "" });
}

// a markup value as DOM (lib/markup.warp, card web-dom), in a shadow root so its own style cannot restyle the page.
// Markup shown anew after a handler changes only the text nodes and attributes that differ (card web-fine): the
// elements stay, with their focus, input and scroll state.
function showRendered(html) {
	const host = $("rendered");
	host.hidden = !html;
	morphChildren(host.shadowRoot ?? renderedRoot(host), parsedHtml(html ?? ""));
}

self.programElements = () => $("rendered").shadowRoot ?? document; // task-workers.js ownerOnPage: `who`, `$who`

// a form's submit stays inside the shadow root (it is not composed): listened to there
function renderedRoot(host) {
	const root = host.attachShadow({ mode: "open" });
	root.addEventListener("submit", submitForm);
	// change is not composed either: it stays inside the shadow root
	for (const [event, type] of Object.entries(FIELD_EVENT_TYPES)) {
		root.addEventListener(type, happened => sendElementEvent(event, happened, inputDetail(happened.composedPath()[0])));
	}
	return root;
}

// a form of the shown markup with `on submit` runs that handler (card web-submit); any other asks the program's own
// route (lowering/web/serve.rs page·submitted), as `warp serve` would: its fields are the body of a POST, the query of a
// GET (src/web/web_server.rs request_node), not a request leaving here
function submitForm(submitted) {
	submitted.preventDefault();
	const form = submitted.target;
	const handled = elementEvent("submit", submitted, submitDetail(submitted));
	if (handled) return sendPageEvent(handled.event, handled.detail);
	const method = (form.getAttribute("method") ?? "get").toUpperCase();
	const fields = Object.fromEntries(new FormData(form, submitted.submitter));
	const path = new URL(form.getAttribute("action") ?? "", PROGRAM_ORIGIN + ($("address").value || "/")).pathname;
	const posted = method !== "GET";
	worker.postMessage({ submit: { method, path, query: posted ? {} : fields, body: posted ? fields : null } });
}

function showPaintings(paintings) {
	$("full-screen").hidden = paintings.length === 0;
	$("paintings").replaceChildren(...paintings.map(paintingCanvas));
}

// the run's paintings, its last one drawn into the canvas shown for it, which keeps the pointer over it; a canvas the GPU
// draws into (paint of a shader) is shown as it is
function showFrame(paintings) {
	const shown = $("paintings").querySelectorAll("canvas");
	const canvas = shown[shown.length - 1];
	const painting = paintings.at(-1);
	if (painting.canvas === canvas && shown.length === paintings.length) return;
	if (shown.length === paintings.length && sameSize(canvas, painting) && !painting.canvas && !canvas.gpu) drawn(canvas, painting);
	else showPaintings(paintings);
}

// the canvases paint of a shader draws into, by id (host-gpu.js gpuPainted): the page makes one and sends its
// OffscreenCanvas straight to the task Worker holding the GPU device (task-workers.js). Not through a port the program's
// Worker made: Firefox lost a canvas posted into a port still on its way to the task Worker (card shaders-firefox)
const paintCanvases = new Map();
// a paint message as shown: a GPU-drawn one names its canvas by id, the others change the painting before (canvas.js)
const paintingOf = data => data.canvas ? { ...data, canvas: paintCanvases.get(data.canvas) } : wholePainting(data);
function sendPaintCanvas({ id, width, height, taskWorkerId, replaces }) {
	const canvas = paintingElement({ width, height, shaded: true });
	canvas.gpu = true;
	paintCanvases.delete(replaces);
	paintCanvases.set(id, canvas);
	const offscreen = canvas.transferControlToOffscreen();
	worker.postToTaskWorker(taskWorkerId, { paintCanvas: { id, canvas: offscreen } }, [offscreen]);
}

// ⛶ (card little-full): the last painting fills the screen at its own aspect ratio, Esc or ⛶ again leaves; a click
// on ⛶ is no click of the program's `on click`
function toggleFullScreen(click) {
	click.stopPropagation();
	if (document.fullscreenElement) document.exitFullscreen();
	else $("painted").requestFullscreen();
}

// ---- page events (notes/signals.md phase 7): `on click {…}`, `on key {…}` of the program shown --------------------

let listening = new Set(); // the page events the shown program handles: "click", "key"

// what the shown program listens to: "on click", "on key" (page events, sent from here), "every 1 s", "at 09:00",
// "message from …" (timers and channels, run by the worker)
function listenTo(labels) {
	listening = new Set(labels.filter(label => PAGE_EVENT.test(label)).map(label => label.replace(PAGE_EVENT, "$1")));
	$("output").classList.toggle("listening", listening.size > 0);
	$("listening").hidden = labels.length === 0;
	const hint = listening.has("key") ? "; click here, then type" : listening.size ? "; click here" : "";
	$("listening").textContent = `listening: ${labels.join(", ")}${hint}`;
	// timers and channels go on by themselves: the program still runs (page events wait for the user)
	showStop(labels.some(label => !PAGE_EVENT.test(label)));
}

// a click on the output (on a canvas: its pixel), a key typed while the output has the focus
function sendPageEvent(event, detail) {
	if (listening.has(event)) worker.postMessage({ event, detail });
}

// an event inside the shown markup, for its element's handler (markup.js elementEvent)
function sendElementEvent(event, happened, detail) {
	const found = elementEvent(event, happened, detail);
	if (found) sendPageEvent(found.event, found.detail);
}

// a click on a link inside the shown markup goes to that page of the program (lowering/web/routes.rs), not away from here
function followLink(click) {
	const link = click.composedPath().find(element => element.matches?.("a[href]"));
	if (!link || click.metaKey || click.ctrlKey || click.shiftKey) return false;
	const url = new URL(link.getAttribute("href"), PROGRAM_ORIGIN);
	if (url.origin !== PROGRAM_ORIGIN) return false;
	click.preventDefault();
	worker.postMessage({ navigate: url.pathname });
	return true;
}

// the address bar shows the path of a program with routes (worker.js addressOf); a path typed there goes to that page
function showAddress(path) {
	$("address").hidden = path === undefined;
	$("address").value = path ?? "";
}

function goToAddress(key) {
	if (key.key !== "Enter") return;
	worker.postMessage({ navigate: new URL($("address").value, PROGRAM_ORIGIN).pathname });
}

// the pointer over a canvas (canvas.js) for the running program's worker
const sharePointer = () => pointer && worker.postMessage(pointerMessage());
const clickDetail = click => pointOn(click.target.closest("canvas") ?? $("output"), click);

function terminalText(text) {
	const restarts = [...text.matchAll(TERMINAL_RESTART)];
	const last = restarts.at(-1);
	return (last ? text.slice(last.index + last[0].length) : text).replace(TERMINAL_CODE, "");
}

function showPrinted(chunks) {
	const printed = terminalText(chunks.map(chunk => chunk.stream === STDERR ? "" : chunk.text).join(""));
	$("printed").textContent = printed;
	$("printed").hidden = printed === "";
}

// what a handler printed, painted and gave, while no run is pending
function showEventOutput(data) {
	if (data.type === "print" && data.stream !== STDERR) {
		$("printed").textContent = terminalText($("printed").textContent + data.text);
		$("printed").hidden = false;
	}
	// a frame of `on frame` (cards https-pannous, very-fast: its GPU canvas showed as its id, and a pixel frame made a
	// new canvas each time)
	if (data.type === "paint") showFrame([paintingOf(data)]);
	if (data.type === "address") showAddress(data.path);
	if (data.type !== "handled") return;
	if (data.value !== undefined) showValue(data.value, data.error, data.html);
	if (data.html !== undefined) showRendered(data.html);
	(data.patches ?? []).forEach(patch => showPatch($("rendered").shadowRoot, patch));
}

async function show(code) {
	if (showing) {
		queued = code; // only the newest code is worth running next
		if (pending?.animating) stopRun(pending, "animation stopped");
		else if (playsLive(pending)) handOver(pending);
		return;
	}
	showing = true;
	setStatus("running…");
	try {
		showReport(await evaluate(code));
	} catch (failure) {
		setStatus(failure.message, true);
	}
	showing = false;
	if (queued !== undefined) {
		const next = queued;
		queued = undefined;
		show(next);
		return;
	}
	warmWorker();
}

// once per worker, after a run is shown: the page is idle, the worker free
function warmWorker() {
	if (workerWarmed) return;
	workerWarmed = true;
	worker.postMessage({ warm: true });
}

// a link to the same page with the other compiler build
function showBuildSwitch() {
	const url = new URL(location.href);
	if (debugBuild) url.searchParams.delete(DEBUG_PARAMETER);
	else url.searchParams.set(DEBUG_PARAMETER, "");
	Object.assign($("build"), { href: url.href, textContent: debugBuild ? "debug build ⇄ optimized" : "optimized build ⇄ debug",
		title: debugBuild ? "warp.debug.wasm: Rust function names and lines in traces and the browser's debugger" : "warp.wasm, the small one" });
}

// the commit the page was built from and the build time in the viewer's time zone (build.sh version.js), linked to
// the commit on GitHub
function showReportButton() {
	const asked = new URLSearchParams(location.search).get(REPORT_PARAMETER);
	if (asked !== null) store(REPORT_KEY, asked === REPORT_OFF ? null : "on");
	$("report").hidden = stored(REPORT_KEY) !== "on";
	$("report").onclick = () => window.open(reportAddress(), "_blank", "noopener");
}

// a new GitHub issue holding the program, what it showed and the playground's version, for its reporter to submit
function reportAddress() {
	const code = editor.getValue();
	const fenced = (text, limit, language = "") => "```" + language + "\n" + text.slice(0, limit) + (text.length > limit ? "\n…" : "") + "\n```";
	const title = `playground: ${code.split("\n").find(line => line.trim()) ?? "empty program"}`;
	const body = [fenced(code, REPORT_LIMITS.code, "warp"), "Output:", fenced($("output").innerText.trim(), REPORT_LIMITS.output),
		$("version").textContent || "unversioned build"].join("\n\n");
	return `${ISSUE_URL}?${new URLSearchParams({ title, body })}`;
}

function showVersion() {
	if (!PLAYGROUND_VERSION) return;
	const { commit, date, built } = PLAYGROUND_VERSION;
	const when = built ? new Date(built).toLocaleString([], { dateStyle: "short", timeStyle: "short" }) : date;
	Object.assign($("version"), { href: `${COMMIT_URL}${commit}`, textContent: `version ${commit.slice(0, SHORT_COMMIT)} · built ${when}`,
		title: `committed ${date}`, hidden: false });
}

// a file the program rendered (render_sound's WAV): a download link beside the wasm's, one per path, the latest bytes
const offeredFiles = new Map();
function offerFile(path, bytes) {
	if (offeredFiles.has(path)) URL.revokeObjectURL(offeredFiles.get(path).href);
	const name = path.split("/").pop();
	const link = element("a", { href: URL.createObjectURL(new Blob([bytes], { type: WAV_TYPE })), download: name, textContent: `⤓ ${name}` });
	const offered = offeredFiles.get(path);
	if (offered) offered.replaceWith(link); else $("written-files").append(link, " ");
	offeredFiles.set(path, link);
	$("written-files").hidden = false;
}

function downloadModule() {
	if (!lastModule) return setStatus("no module yet: a constant result needs none");
	const link = element("a", { href: URL.createObjectURL(new Blob([lastModule], { type: "application/wasm" })), download: "program.wasm" });
	link.click();
	URL.revokeObjectURL(link.href);
}

// ---- the editor ---------------------------------------------------------------------------------------------

let editor;

function runNow() {
	return show(editor.getValue());
}

// the code changed by the page, not typed: it runs at once, and only once (the change event's run would run it again)
function runChanged() {
	clearTimeout(typingTimer);
	return runNow();
}

// Run pressed (the button, Ctrl/Cmd-Enter): the old value gives way to "…" at once, and the old errors, warnings,
// underlines and printed text go, so none is taken for the new run's (card g_oc54)
// Ctrl/Cmd-Enter while live loops play runs the code again, handing them over (card playground-live); the button stops
function runPressed(fromKeys = false) {
	if (showsStop() && !(fromKeys && playsLive(pending))) return stopPressed();
	showValue(PENDING_VALUE);
	$("value").classList.remove("located");
	$("value").onclick = null;
	$("diagnostics").replaceChildren();
	markPositions({});
	showPrinted([]);
	return runNow();
}

const exampleSource = name => EXAMPLES[name]?.code ?? SAMPLES[name];
// the file of the chosen sample, whose neighbours its runs name relatively (host-files.js besideProgram)
function programFile() {
	const name = $("examples").value;
	return EXAMPLES[name] === undefined && SAMPLES[name] !== undefined ? `samples/${name}.warp` : undefined;
}

// puts the code in the editor and runs it once; resolves once its report is shown
function runCode(source) {
	editor.setValue(source);
	return runChanged();
}

// shows the example or sample; resolves once its report is shown
function chooseExample(name) {
	const source = exampleSource(name);
	if (source === undefined) return;
	$("examples").value = name;
	showExampleInAddress(name);
	return runCode(source);
}

// the address names the chosen example as its hash (#circle), so it can be shared or reloaded; the default one leaves
// it plain, and a guide chapter's hash (#guide-lists) stays (card sample-hash)
function showExampleInAddress(name) {
	const address = new URL(location.href);
	EXAMPLE_PARAMETERS.forEach(parameter => address.searchParams.delete(parameter));
	if (name !== DEFAULT_EXAMPLE) address.hash = name;
	else if (exampleSource(hashName()) !== undefined) address.hash = "";
	if (address.href !== location.href) history.replaceState(history.state, "", address);
}

const hashName = () => decodeURIComponent(location.hash.slice(1));

// the example the address names: ?example=circle (?sample=circle), else #circle
function requestedExample() {
	const parameters = new URLSearchParams(location.search);
	return [...EXAMPLE_PARAMETERS.map(parameter => parameters.get(parameter)), hashName()].find(name => name && exampleSource(name) !== undefined);
}

// #circle typed into the address shows that example
function chooseExampleOfHash() {
	const name = hashName();
	if (exampleSource(name) !== undefined && name !== $("examples").value) chooseExample(name);
}

function fillExamples() {
	const group = (label, names) => element("optgroup", { label }, ...names.map(name => element("option", { value: name }, name)));
	$("examples").replaceChildren(group("tour", Object.keys(EXAMPLES)), group("samples/", Object.keys(SAMPLES).sort()));
	$("examples").onchange = event => chooseExample(event.target.value);
}

function initialize() {
	editor = CodeMirror.fromTextArea($("code"), {
		// fixedGutter moves the gutter on every scroll, which Firefox warns about; wrapped lines never scroll sideways
		lineNumbers: true, lineWrapping: true, fixedGutter: false, mode: "warp", indentWithTabs: true, tabSize: 4,
		extraKeys: SHORTCUTS, // shortcuts.js
	});
	editor.on("change", () => {
		if (!$("auto").checked) return;
		clearTimeout(typingTimer);
		typingTimer = setTimeout(runNow, TYPING_DELAY_MS);
	});
	$("run").onclick = runPressed;
	$("full-screen").onclick = toggleFullScreen;
	$("shown-from").value = stored(SHOWN_FROM_KEY) ?? (stored(HINTS_KEY) === "off" ? "weak" : "hint");
	store(HINTS_KEY, null);
	$("shown-from").onchange = () => {
		store(SHOWN_FROM_KEY, $("shown-from").value);
		runNow();
	};
	$("address").onkeydown = goToAddress;
	$("rendered").onclick = click => followLink(click) || sendElementEvent("click", click, clickDetail(click));
	$("rendered").oninput = input => sendElementEvent("input", input, inputDetail(input.composedPath()[0]));
	$("output").onclick = click => sendPageEvent("click", clickDetail(click));
	$("output").onkeydown = key => sendPageEvent("key", { key: key.key });
	followPointer($("output"));
	$("download").onclick = downloadModule;
	startResizers();
	showBuildSwitch();
	showVersion();
	showReportButton();
	fillExamples();
	startWorker();
	startAssistant(editor, tellEnvironment, startCompletion(editor, askMembers));
	chooseExample(requestedExample() ?? DEFAULT_EXAMPLE);
	addEventListener("hashchange", chooseExampleOfHash);
}

// for the headless probes (probes/web_playground.py, test_in_browser.py --examples): evaluate code as the page does and return the report
window.playground = { evaluate, applyFix, chooseExample, runCode, reportAddress, code: () => editor.getValue(), setCode: source => editor.setValue(source), lastModule: () => lastModule, acknowledge: topic => saveAcknowledged([...acknowledged, topic]), forgetAll: () => saveAcknowledged([]),
	// what a run waits for, for the Firefox driver's timeout report (card firefox-hello-again)
	state: () => ({ worker: workerStage, ready: workerSettled, pending: pending?.id, queued: queued !== undefined, showing }) };

reportBareErrors(self, "page"); // task-workers.js
addEventListener("unhandledrejection", event => {
	if (event.reason !== undefined) return;
	event.preventDefault();
	console.error("warp page: a promise failed without a reason", event.promise, new Error("seen here").stack);
});

(window.pageStarts ?? Promise.resolve()).then(initialize); // index.html: not before its reload for isolation
