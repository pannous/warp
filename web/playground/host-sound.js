// Sound and MIDI in the page (a part of host.js, which says how parts work; needs host-files.js, whose files it writes
// and reads): the host word sound_samples and std's sound and midi modules (lib/sound.warp)

// src/sound/mod.rs: 16-bit mono PCM, samples offset by SAMPLE_OFFSET; a render without sounds has the default rate
const SAMPLE_OFFSET = 32768;
const DEFAULT_SAMPLE_RATE = 22050;
const WAV_HEADER_BYTES = 44;
// play of a signal (src/sound/mod.rs play_signal): computed this many samples at a time
const SIGNAL_BLOCK = 512;
const FULL_SCALE = 32767;
// a published site ships only the host parts its words need (src/host/host_parts.rs), without the compiler that measures
// a spectrum or the page that decodes a sound file
const WITHOUT_PLAYGROUND = "measured in the playground and by the warp CLI, not on a published site yet";
// played to someone (the page was clicked or typed in), a signal is computed at most this far ahead of the speaker,
// checked every SIGNAL_POLL_MS (src/sound/mod.rs SIGNAL_AHEAD, SIGNAL_POLL)
const SIGNAL_AHEAD_SECONDS = 0.1;
const SIGNAL_POLL_MS = 10;
// heard, a voice queues at most this far ahead of the speaker before its next sound waits (src/sound/mod.rs PLAY_AHEAD)
const PLAY_AHEAD_SECONDS = 0.5;
// a linear float array's block: [count: i64][count f64 cells] (src/wasm_emitter/lists/linear_arrays.rs)
const LINEAR_CELL_BYTES = 8;

// MIDI (card midi-out, src/sound/midi.rs, which says how notes cross): Standard MIDI Files, format 0, 480 ticks a quarter at 120 beats a minute
const MIDI_TICKS_PER_QUARTER = 480;
const MIDI_MICROSECONDS_PER_QUARTER = 500000;
const MIDI_TICKS_PER_SECOND = MIDI_TICKS_PER_QUARTER * 1e6 / MIDI_MICROSECONDS_PER_QUARTER;
const MIDI_NOTE_ON = 0x90;
const MIDI_NOTE_OFF = 0x80;
const MIDI_FIELDS = 4; // start ms, key, length ms, velocity
const midiTicks = milliseconds => Math.round(Math.max(0, milliseconds) * MIDI_TICKS_PER_SECOND / 1000);
const midiVariableLength = value => {
	const groups = [value & 0x7f];
	for (value >>= 7; value > 0; value >>= 7) groups.push(value & 0x7f | 0x80);
	return groups.reverse();
};
const midiChunk = (kind, body) => {
	const bytes = new Uint8Array(8 + body.length);
	const view = new DataView(bytes.buffer);
	[...kind].forEach((letter, i) => bytes[i] = letter.charCodeAt(0));
	view.setUint32(4, body.length);
	bytes.set(body, 8);
	return bytes;
};
function midiFileOf(flat) {
	if (flat.length % MIDI_FIELDS) throw new Error(`notes come as [start, key, length, velocity] four numbers each, got ${flat.length} numbers`);
	const events = [];
	for (let i = 0; i < flat.length; i += MIDI_FIELDS) {
		const [start, key, length, velocity] = flat.slice(i, i + MIDI_FIELDS);
		if (key < 0 || key > 127) throw new Error(`MIDI keys are 0 to 127, got ${key}`);
		const on = midiTicks(start);
		events.push([on, 1, [MIDI_NOTE_ON, key, Math.min(127, Math.max(1, velocity))]], [Math.max(on, midiTicks(start + length)), 0, [MIDI_NOTE_OFF, key, 0]]);
	}
	events.sort((a, b) => a[0] - b[0] || a[1] - b[1]);
	const track = [0, 0xff, 0x51, 3, MIDI_MICROSECONDS_PER_QUARTER >> 16 & 255, MIDI_MICROSECONDS_PER_QUARTER >> 8 & 255, MIDI_MICROSECONDS_PER_QUARTER & 255];
	let now = 0;
	for (const [tick, , message] of events) {
		track.push(...midiVariableLength(tick - now), ...message);
		now = tick;
	}
	track.push(0, 0xff, 0x2f, 0);
	const header = new Uint8Array([0, 0, 0, 1, MIDI_TICKS_PER_QUARTER >> 8, MIDI_TICKS_PER_QUARTER & 255]);
	return Uint8Array.from([...midiChunk("MThd", header), ...midiChunk("MTrk", track)]);
}
function midiNotesOf(bytes) {
	const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.length);
	const ascii = at => String.fromCharCode(...bytes.slice(at, at + 4));
	if (ascii(0) !== "MThd") throw new Error("not a MIDI file (it does not start with MThd)");
	const tracks = view.getUint16(10), division = view.getUint16(12);
	if (division & 0x8000) throw new Error("MIDI files timed in SMPTE frames are not supported, only ticks per quarter note");
	const tempi = [[0, MIDI_MICROSECONDS_PER_QUARTER]], messages = [];
	let at = 8 + view.getUint32(4);
	for (let t = 0; t < tracks; t++) {
		const end = at + 8 + view.getUint32(at + 4), kind = ascii(at);
		at += 8;
		if (kind !== "MTrk") { at = end; continue; }
		const ended = () => { throw new Error("the file ends inside an event"); };
		const byte = () => at < end ? bytes[at++] : ended();
		const length = () => { let value = 0; for (let i = 0; i < 4; i++) { const b = byte(); value = value << 7 | b & 0x7f; if (!(b & 0x80)) return value; } throw new Error("a delay is longer than four bytes"); };
		let tick = 0, status = 0;
		while (at < end) {
			tick += length();
			const first = byte();
			if (first & 0x80) status = first; else at--;
			if (status === 0xff) {
				const type = byte(), size = length(), data = bytes.slice(at, at += size);
				if (type === 0x51 && size === 3) tempi.push([tick, data[0] << 16 | data[1] << 8 | data[2]]);
			} else if (status === 0xf0 || status === 0xf7) at += length();
			else {
				const kind = status & 0xf0, channel = status & 15, one = byte();
				const two = kind === 0xc0 || kind === 0xd0 ? 0 : byte();
				if (kind === MIDI_NOTE_ON) messages.push([tick, two > 0, channel, one, two]);
				else if (kind === MIDI_NOTE_OFF) messages.push([tick, false, channel, one, 0]);
			}
		}
		at = end;
	}
	tempi.sort((a, b) => a[0] - b[0]);
	const milliseconds = tick => {
		let seconds = 0, from = 0, tempo = MIDI_MICROSECONDS_PER_QUARTER;
		for (const [when, next] of tempi) {
			if (when >= tick) break;
			seconds += (when - from) * tempo / 1e6 / division;
			[from, tempo] = [when, next];
		}
		return Math.round((seconds + (tick - from) * tempo / 1e6 / division) * 1000);
	};
	messages.sort((a, b) => a[0] - b[0]);
	const sounding = [], notes = [];
	for (const [tick, on, channel, key, velocity] of messages) {
		if (on) { sounding.push([tick, channel, key, velocity]); continue; }
		const index = sounding.findIndex(([, c, k]) => c === channel && k === key);
		if (index < 0) continue;
		const [start, , , started] = sounding.splice(index, 1)[0];
		notes.push([milliseconds(start), key, milliseconds(tick) - milliseconds(start), started]);
	}
	return notes.sort((a, b) => a[0] - b[0] || a[1] - b[1]).flat();
}
const wholeNumbers = list => Array.from(list, Number);

// a 16-bit mono PCM WAV of the offset samples (src/sound/mod.rs wav)
// the sounds mixed at their places on their voices (src/sound/mod.rs render): a task's sound along with the program's; a
// sound without a place (no voices where it ran) follows the one before. From where the run's last render ended, so
// `at 1s beep()` keeps its second of silence
// the sounds mixed at their places, in stereo when one of them is (a mono sound on both speakers alike, as src/sound/mod.rs
// render): the samples interleaved and the channels
// play(hum, 2s) (card real-time): `blockOf(first, count)` the blocks of each channel's values (-1…1) of `count`
// samples from `first` on. Played to someone (heard: playground.js tellSystemValues), each block is a sound of
// the signal's handle, computed when the voice holds less than SIGNAL_AHEAD_SECONDS, the wait a sleep (no runaway
// time); else all of them at once, one sound. Its handle
function playedSignal(holder, seconds, rate, channels, block, blockOf) {
	const total = Math.round(Math.max(0, Number(seconds)) * rate);
	const paced = Boolean(self.pageSystemValues?.heard);
	let samples = [];
	let handle;
	const played = () => {
		if (samples.length) handle = holder.sounded(samples, rate, channels, handle);
		samples = [];
	};
	for (let first = 0; first < total; first += block) {
		while (paced && (self.soundsQueued?.() ?? 0) > SIGNAL_AHEAD_SECONDS) holder.sleep(SIGNAL_POLL_MS);
		const count = Math.min(block, total - first);
		const blocks = blockOf(first, count);
		for (let index = 0; index < count; index++)
			for (const cells of blocks) samples.push(Math.round(Math.max(-1, Math.min(1, Number(cells[index] ?? 0))) * FULL_SCALE) + SAMPLE_OFFSET);
		if (paced) played();
	}
	played();
	return handle ?? 0;
}

// a function the program gives a sound word, of a float to a float: its export, called directly (closure_apply, which
// builds nodes for each call, only for a closure without one)
const exportedOr = (program, callable) => program[callable.warpName] ?? callable;

// the cells of the linear float array at `address` in the program's memory
function linearDoubles(program, address) {
	const view = new DataView(program.memory.buffer);
	const count = Number(view.getBigInt64(address, true));
	return Array.from({ length: count }, (_, index) => view.getFloat64(address + LINEAR_CELL_BYTES * (index + 1), true));
}

function mixed(sounds, rate, run) {
	const channels = Math.max(1, ...sounds.map(sound => sound.channels ?? 1));
	const framesOf = sound => sound.samples.length / (sound.channels ?? 1);
	let end = run.renderedUntil ?? 0;
	const starts = sounds.map(sound => {
		const place = sound.place ?? end;
		end = Math.max(end, place + framesOf(sound) / rate);
		return place;
	});
	const first = Math.min(run.renderedUntil ?? 0, ...starts);
	const offsets = starts.map(place => Math.round((place - first) * rate));
	const amplitudes = new Float64Array(channels * Math.max(0, ...sounds.map((sound, i) => offsets[i] + framesOf(sound))));
	const fades = run.soundFades ?? [];
	run.soundFades = [];
	sounds.forEach((sound, i) => {
		const own = sound.channels ?? 1;
		const ownFades = fades.filter(fade => fade.handle === 0 || fade.handle === sound.handle);
		for (let frame = 0; frame < framesOf(sound); frame++) {
			const gain = ownFades.length ? fadedGain(ownFades, starts[i] + frame / rate) : 1;
			for (let channel = 0; channel < channels; channel++)
				amplitudes[(offsets[i] + frame) * channels + channel] += Math.round((Number(sound.samples[frame * own + Math.min(channel, own - 1)]) - SAMPLE_OFFSET) * gain);
		}
	});
	run.renderedUntil = first + amplitudes.length / channels / rate;
	return { samples: Array.from(amplitudes, amplitude => amplitude + SAMPLE_OFFSET), channels };
}

// the gain at the place after the fades (src/sound/mod.rs faded_gain), in the order they were called: each ramps linearly
// from the gain it finds there to its own
function fadedGain(fades, place) {
	const ramped = (fade, from, at) => from + (fade.gain - from) * (fade.seconds > 0 ? Math.min(1, Math.max(0, (at - fade.place) / fade.seconds)) : 1);
	let ramp = [{ place: 0, gain: 1, seconds: 0 }, 1];
	for (const fade of fades.filter(fade => fade.place <= place)) ramp = [fade, ramped(...ramp, fade.place)];
	return ramped(...ramp, place);
}

// interleaved samples at another rate, each channel's linearly interpolated between its neighbours (src/sound/mod.rs at_rate)
function atRate(samples, channels, from, to) {
	if (from === to || samples.length < channels) return samples;
	const frames = samples.length / channels;
	const converted = Math.round(frames * to / from);
	const out = new Array(converted * channels);
	for (let frame = 0; frame < converted; frame++) {
		const position = frame * from / to;
		const before = Math.min(Math.floor(position), frames - 1), after = Math.min(before + 1, frames - 1), share = position - before;
		for (let channel = 0; channel < channels; channel++) {
			const a = Number(samples[before * channels + channel]), b = Number(samples[after * channels + channel]);
			out[frame * channels + channel] = Math.round(a + (b - a) * share);
		}
	}
	return out;
}

// a WAV's [rate, channels, offset 16-bit frames …] (src/sound/audio_files.rs decoded): PCM 8, 16, 24, 32 or float 32, 64
function wavFrames(bytes, name) {
	const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
	const ascii = (at, length) => String.fromCharCode(...bytes.subarray(at, at + length));
	if (bytes.length < 12 || ascii(0, 4) !== "RIFF" || ascii(8, 4) !== "WAVE") throw new Error(`sample ${name}: the playground reads WAV samples only`);
	let format;
	for (let at = 12; at + 8 <= bytes.length;) {
		const size = view.getUint32(at + 4, true);
		if (ascii(at, 4) === "fmt ") format = at + 8;
		if (ascii(at, 4) === "data") {
			if (format === undefined) throw new Error(`sample ${name}: its data comes before its format`);
			let tag = view.getUint16(format, true);
			if (tag === 0xFFFE) tag = view.getUint16(format + 24, true);
			const channels = view.getUint16(format + 2, true) || 1, rate = view.getUint32(format + 4, true), bits = view.getUint16(format + 14, true);
			const scaled = amplitude => Math.round(Math.max(-1, Math.min(1, amplitude)) * 32767);
			const amplitude = { "1 8": i => (view.getUint8(i) - 128) << 8, "1 16": i => view.getInt16(i, true), "1 24": i => view.getInt16(i + 1, true),
				"1 32": i => view.getInt16(i + 2, true), "3 32": i => scaled(view.getFloat32(i, true)), "3 64": i => scaled(view.getFloat64(i, true)) }[`${tag} ${bits}`];
			if (!amplitude) throw new Error(`sample ${name}: its format ${tag} with ${bits} bits is not supported (PCM 8, 16, 24, 32 or float 32, 64)`);
			const width = bits / 8, end = Math.min(bytes.length, at + 8 + size), flat = [rate, channels];
			for (let i = at + 8; i + width <= end; i += width) flat.push(amplitude(i) + SAMPLE_OFFSET);
			return flat;
		}
		at += 8 + size + size % 2;
	}
	throw new Error(`sample ${name}: no audio data`);
}

function wavOf(samples, rate, channels = 1) {
	const bytes = new Uint8Array(WAV_HEADER_BYTES + 2 * samples.length);
	const view = new DataView(bytes.buffer);
	const ascii = (at, word) => [...word].forEach((letter, i) => view.setUint8(at + i, letter.charCodeAt(0)));
	ascii(0, "RIFF");
	view.setUint32(4, bytes.length - 8, true);
	ascii(8, "WAVEfmt ");
	[[16, 16, 4], [20, 1, 2], [22, channels, 2], [24, rate, 4], [28, 2 * channels * rate, 4], [32, 2 * channels, 2], [34, 16, 2]]
		.forEach(([at, value, size]) => size === 4 ? view.setUint32(at, value, true) : view.setUint16(at, value, true));
	ascii(36, "data");
	view.setUint32(40, 2 * samples.length, true);
	samples.forEach((sample, i) => view.setInt16(WAV_HEADER_BYTES + 2 * i, Math.max(-32768, Math.min(32767, Number(sample) - SAMPLE_OFFSET)), true));
	return bytes;
}

// ["delay", 0.25, 0.4, 0.5] with plain numbers, as JSON takes them
const plainEffect = effect => Array.from(effect, setting => typeof setting === "bigint" ? Number(setting) : setting);

// the samples through the effects of the bus the run sends to (card shared-buses stage 2), a delay's or reverb's tail
// added; each sound apart, which is the bus's sound for its linear effects (filters, delay, reverb, gain), while compress
// and distort act on each sound alone
function onBus(holder, samples, rate, channels) {
	const chain = holder.run.busChains?.[holder.run.sentBus ?? 0];
	if (!chain?.length) return samples;
	const floats = Float32Array.from(samples, sample => (Number(sample) - SAMPLE_OFFSET) / SAMPLE_OFFSET);
	const processed = holder.effected(chain, floats, rate, channels);
	return Array.from(processed, sample => Math.round(Math.max(-1, Math.min(1, sample)) * (SAMPLE_OFFSET - 1)) + SAMPLE_OFFSET);
}

// a sound file's first channel, amplitudes around 0, decoded once per run (lib/sound.warp spectrum of a file,
// sound_seconds): a WAV here, any other format by the page, since a Worker has no decodeAudioData (task-workers.js
// decodeSoundInto, while the Worker waits)
function monoOf(holder, path) {
	const name = besideProgram(contentText(path), programFileOf(holder.run)); // host-files.js
	const decoded = holder.run.decodedSounds ??= {};
	return decoded[name] ??= /\.wav$/i.test(name) ? wavMono(name) : pageDecoded(name);
}

function wavMono(name) {
	const flat = wavFrames(readBytes(name), name), [rate, channels] = flat;
	const amplitudes = Float32Array.from({ length: Math.floor((flat.length - 2) / channels) }, (_, at) => flat[2 + at * channels] - SAMPLE_OFFSET);
	return { rate, amplitudes };
}

function pageDecoded(name) {
	const written = writtenFiles.has(filePath(name));
	const ask = self.askPageDecode && (shared => self.askPageDecode(shared, fileUrl(name), written ? readBytes(name) : undefined));
	if (!self.askPage) throw new Error(`sound ${name}: ${WITHOUT_PLAYGROUND}`);
	const { rate, amplitudes } = self.askPage(`sound ${name}`, ask); // host-tasks.js
	return { rate, amplitudes: Float32Array.from(amplitudes) };
}

addHostPart({
	words: (holder, hooks, { program }) => {
		holder.effected = (chain, floats, rate, channels) => effectsProcessed(hooks, chain, floats, rate, channels); // host-compiler.js
		holder.spectrum = (window, rate, bands, low, high) => {
			if (typeof spectrumLevels !== "function") throw new Error(`spectrum: ${WITHOUT_PLAYGROUND}`);
			return spectrumLevels(hooks, window, rate, Number(bands), Number(low), Number(high)); // host-compiler.js
		};
		// samples (amplitude + SAMPLE_OFFSET, stereo interleaved) queued on the voice, for the page and save sound; a
		// signal's blocks keep its first one's handle. Heard, once the voice is within PLAY_AHEAD_SECONDS of the speaker
		// (but in an `at`, which places its sounds itself). The handle
		holder.sounded = (samples, rate, channels, handle) => {
			while (self.pageSystemValues?.heard && !self.insideAt?.() && (self.soundsQueued?.() ?? 0) > PLAY_AHEAD_SECONDS) holder.sleep(SIGNAL_POLL_MS);
			const { place, at } = self.voiceSounded?.(samples.length / channels / rate) ?? {};
			const sound = { samples: onBus(holder, samples, rate, channels), rate, channels };
			hooks.sound?.({ ...sound, at, handle, loop: self.currentVoice?.().loop }); // a live loop's (card playground-live)
			const own = handle ?? self.lastSoundHandle?.() ?? 0;
			(holder.run.unrenderedSounds ??= []).push({ ...sound, place, handle: own });
			return own;
		};
		return {
			// sound_samples(samples, count, rate, channels) (src/host.rs, lib/sound.warp; stereo interleaved left, right):
			// the page plays them with WebAudio (playground.js playSound); a run without a page (tests, node) has no
			// speakers and stays silent, as natively save sound writes them, played or not, into a WAV (STD_ADAPTERS.sound.render)
			sound_samples: (samples, count, rate, channels) => {
				holder.sounded(intsOfList(program(), samples, Number(count)) ?? plainOfTree(readNode(program(), samples)), Number(rate), Number(channels));
			},
		};
	},
	adapters: {
		// MIDI (card midi-out, lib/sound.warp): files are bytes as save sound's; a message goes to the page, which holds
		// the Web MIDI access (playground.js sendMidi), timed on the voice's clock. A Worker cannot wait for the page's device list or its input
		midi: {
			write_file: (path, notes) => { keepWritten(contentText(path), midiFileOf(wholeNumbers(notes))); return null; },
			read_file: path => midiNotesOf(readBytes(contentText(path))),
			send: (port, message) => {
				if (!self.sendMidi) return null; // a run without a page (tests, node) has no devices, silent as sound is
				// at the voice's moment on the shared clock, after the sounds before it (card midi-clock)
				self.sendMidi(contentText(port), wholeNumbers(message), self.voiceSounded?.(0).at);
				return null;
			},
			outputs: () => { throw new Error("midi.outputs: the playground's page owns the MIDI devices and a program cannot wait for their list; midi_on and midi_send use the first output"); },
			inputs: () => { throw new Error("midi.inputs: the playground's page owns the MIDI devices and a program cannot wait for their list"); },
			receive: () => { throw new Error("midi.receive: MIDI input does not reach a playground program yet (Web MIDI answers on the page, not in the program's Worker)"); },
		},
		// `play "song.mp3"`, `stop_sound` (lib/sound.warp, card sound-library): the page plays it with an <audio>
		// (worker.js self.playSoundFile, playground.js); a run without a page (tests, node) stays silent, as natively
		sound: {
			// a written file (a rendered WAV) goes to the page as its bytes, a served or remote one by its URL; a local file
			// that is not there fails as natively (card browser-play)
			play_file(path) {
				const name = besideProgram(contentText(path), programFileOf(this.holder.run)); // host-files.js
				const written = writtenFiles.has(filePath(name));
				if (!written && !/^https?:/.test(name) && !servedFileExists(name)) throw new Error(`cannot open ${name}: ${FILE_NOT_FOUND}`);
				self.playSoundFile?.(fileUrl(name), written ? readBytes(name) : undefined);
				return self.lastSoundHandle?.() ?? 0;
			},
			// stop_sound(handle) the sound play gave that handle (card sound-pro), handle 0 all of them
			stop: handle => { Number(handle) ? self.stopSound?.(Number(handle)) : self.stopSoundFiles?.(); return null; },
			last: () => self.lastSoundHandle?.() ?? 0,
			// fade_sound(handle, gain, seconds) (src/sound/mod.rs fade): the page ramps the playing sound, save sound from where
			// the speaker stood on this voice
			fade(handle, gain, seconds) {
				[handle, gain, seconds] = [handle, gain, seconds].map(Number);
				const last = self.lastSoundHandle?.() ?? 0;
				if (handle > last) throw new Error(`fade_sound(${handle}): no sound ${handle} in this run, which played ${last}`);
				if (!(gain >= 0 && gain <= 1) || seconds < 0) throw new Error(`fade_sound(${handle}, ${gain}, ${seconds}): the gain is 0…1 and the seconds not negative`);
				const place = (self.currentVoice?.().seconds ?? 0) - (self.soundsQueued?.() ?? 0);
				(this.holder.run.soundFades ??= []).push({ handle, place, gain, seconds });
				self.fadeSound?.(handle, gain, seconds);
				return null;
			},
			// a bus (card shared-buses, src/sound/effects.rs): numbered as natively, its chain ([["reverb", 0.4], …]) kept
			// for the sounds sent to it (onBus)
			bus(number, chain) {
				const id = Number(number) || (this.holder.run.buses = (this.holder.run.buses ?? 0) + 1);
				(this.holder.run.busChains ??= {})[id] = Array.from(chain ?? [], plainEffect);
				return id;
			},
			// a list signal through one effect (lib/sound.warp effected): the DSP in warp.wasm as natively, on amplitudes of
			// the full scale
			effect(effect, signal, rate, fullScale) {
				const scale = Number(fullScale) || 1;
				const floats = Float32Array.from(signal, amplitude => Number(amplitude) / scale);
				return Array.from(this.holder.effected([plainEffect(effect)], floats, Number(rate), 1), sample => sample * scale);
			},
			// the bus before, as natively, so out(signal, bus) restores it
			send(number) {
				const before = this.holder.run.sentBus ?? 0;
				this.holder.run.sentBus = Number(number);
				return before;
			},
			// the page's audio clock as the worker keeps it (worker.js); a worker cannot wait for the page's audio
			queued: () => self.soundsQueued?.() ?? 0,
			at: seconds => { self.voiceAt?.(Number(seconds)); return null; },
			rest: seconds => { self.voiceSounded?.(Number(seconds)); return null; }, // midi_note's length (card midi-clock)
			at_end: () => { self.voiceAtEnd?.(); return null; },
			// the voice's moment on the clock every Worker shares (src/sound/mod.rs moment_seconds, card live-redefine)
			moment: () => self.voiceMoment?.() ?? 0,
			// a named live loop's cycle starts (lib live_cycle_started, host-tasks.js liveCycleStarted)
			live_cycle: name => { self.liveCycleStarted?.(contentText(name)); return null; },
			wait: () => null,
			// sample("kick.wav") (card sound-samples, src/sound/audio_files.rs): [rate, channels, offset frames …]; the playground
			// decodes WAV only, a Worker has no decoder for the other formats
			load: path => wavFrames(readBytes(contentText(path)), contentText(path)),
			// spectrum(samples, seconds, bands): the bands of the window, the samples around sample_offset (src/sound/spectrum.rs in warp.wasm);
			// a window past the end is the empty list, ø: null here
			spectrum(window, rate, bands, low, high) {
				return this.holder.spectrum(Array.from(window ?? [], sample => Number(sample) - SAMPLE_OFFSET), Number(rate), bands, low, high);
			},
			// spectrum("song.mp3", seconds, bands) of the file's window then, and how long a file sounds
			file_spectrum(path, seconds, windowSeconds, bands, low, high) {
				const { rate, amplitudes } = monoOf(this.holder, path);
				const start = Math.min(amplitudes.length, Math.round(Math.max(0, Number(seconds)) * rate));
				const window = amplitudes.subarray(start, Math.min(amplitudes.length, start + Math.round(Number(windowSeconds) * rate)));
				return this.holder.spectrum(window, rate, bands, low, high);
			},
			file_seconds(path) {
				const { rate, amplitudes } = monoOf(this.holder, path);
				return amplitudes.length / rate;
			},
			// play(hum, 2s): hum(t) for each sample (left, then right of a pair [left, right]), t its time in seconds
			signal(functions, seconds, rate) {
				const signals = [functions].flat().map(signal => exportedOr(this.holder.exports, signal));
				return playedSignal(this.holder, seconds, Number(rate), signals.length, SIGNAL_BLOCK, (first, count) => {
					const blocks = signals.map(() => []);
					for (let index = first; index < first + count; index++) signals.forEach((signal, channel) => blocks[channel].push(signal(index / Number(rate))));
					return blocks;
				});
			},
			// play_blocks(fill, addresses, 2s) (lib/sound.warp): the program's fill(t)
			// fills the linear float arrays at `addresses` (one, or left and right), read from its memory
			blocks(caller, addresses, seconds, rate) {
				const blocks = [addresses].flat().map(Number);
				const program = this.holder.exports;
				const fill = exportedOr(program, caller);
				return playedSignal(this.holder, seconds, Number(rate), blocks.length, linearDoubles(program, blocks[0]).length, first => {
					fill(first / Number(rate));
					return blocks.map(address => linearDoubles(program, address));
				});
			},
			// the run's sounds since its start or the last render, one WAV (src/sound/mod.rs render): its seconds; the page
			// offers it for download
			render(path) {
				const sounds = this.holder.run.unrenderedSounds ?? [];
				this.holder.run.unrenderedSounds = [];
				// the highest rate of the sounds, the others converted to it (src/sound/mod.rs render)
				const rate = sounds.length ? Math.max(...sounds.map(sound => sound.rate)) : DEFAULT_SAMPLE_RATE;
				const converted = sounds.map(sound => ({ ...sound, rate, samples: atRate(sound.samples, sound.channels ?? 1, sound.rate, rate) }));
				const { samples, channels } = mixed(converted, rate, this.holder.run);
				keepWritten(contentText(path), wavOf(samples, rate, channels));
				return samples.length / channels / rate;
			},
		},
	},
});
