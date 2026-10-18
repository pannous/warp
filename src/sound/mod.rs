//! sound_samples(samples, count, rate, channels) natively (card basic-sound, lib/sound.warp): 16-bit samples, mono or
//! stereo (interleaved left, right: lib panned, card stereo-pan), as a WAV file in the
//! system's temporary folder (warp-sound/sound-<process id>.wav, then sound-<process id>-2.wav …: runs at the same time
//! never share a file; card sound-wavs), played by the
//! system's player when the warp binary may reach the user (paint::shows_windows: never under WARP_NO_WINDOW, CI or
//! tests, which only get the file). The playground plays the same samples with WebAudio (host-files.js sound_samples).
//! A sound is queued on the audio clock behind those before it in its voice and `play` returns at once (card sound-pro):
//! each sound waits for its start on a thread of its own and plays, stop_sound drops the waiting ones, the process's
//! end waits for them and for the music files playing. A voice is the program's thread or a task (`go { … }`), which
//! starts where its starter stands: voices sound together. save sound mixes the sounds since the last render as
//! they sound, by the voices' places on the clock (counted in samples, not in real time): offline, headless too.

// DONE turn this into a module, albeit built-in one: the sound module is lib/sound.warp (its words), this folder (the
// engine, effects, MIDI, audio files), adapters.rs (its std_io words, which modules/std_adapters.rs
// answers through) and host_words.rs (its native host words, which host/native.rs links) (cards turn-src, src-sound)

pub(crate) mod adapters;
pub mod audio_files;
pub mod effects;
pub(crate) mod host_words;
pub mod midi;
#[cfg(target_os = "macos")]
mod midi_devices;
pub mod sound_engine;
pub mod spectrum;

use crate::graphics::paint::shows_windows;
use effects::Effect;
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, Once};
use std::time::{Duration, Instant};

/// The samples cross as whole numbers ≥ 0: amplitude + 32768 (lib/sound.warp sample_offset)
pub const SAMPLE_OFFSET: i64 = 32_768;
/// Where the WAVs go below the temp dir, named <FILE_STEM>-<process id>[-<handle>].wav (tests sweep those of ended
/// processes, tests/common scratch_root)
pub const FOLDER: &str = "warp-sound";
pub const FILE_STEM: &str = "sound";
const PCM_FORMAT: u16 = 1;
const MONO: u16 = 1;
const BITS_PER_SAMPLE: u16 = 16;
const BYTES_PER_SAMPLE: u32 = (BITS_PER_SAMPLE / 8) as u32;
const FORMAT_CHUNK_SIZE: u32 = 16;
const HEADER_SIZE_AFTER_RIFF: u32 = 36;
/// The rate of a render without sounds (lib/sound.warp sample_rate)
const DEFAULT_RATE: u32 = 22_050;
/// The sounds and music files of this run so far: a sound's handle is its number among them, from 1
static SOUNDED: AtomicUsize = AtomicUsize::new(0);
/// The clock the voices' moments are counted on (moment_seconds), the same for every task of the run
static RUN_CLOCK: std::sync::LazyLock<Instant> = std::sync::LazyLock::new(Instant::now);
/// The handles stop_sound(handle) stopped one by one
static STOPPED: Mutex<Vec<usize>> = Mutex::new(Vec::new());
/// The music file formats warp plays and reads
const FORMATS: [&str; 6] = ["wav", "mp3", "ogg", "flac", "aiff", "m4a"];
/// The music files' formats by their first bytes: (format, magic bytes, offset of the magic)
const FILE_FORMATS: [(&str, &[u8], usize); 7] = [
	("wav", b"WAVE", 8),
	("mp3", b"ID3", 0),
	("mp3", b"\xFF\xFB", 0),
	("ogg", b"OggS", 0),
	("flac", b"fLaC", 0),
	("aiff", b"AIFF", 8),
	("m4a", b"ftyp", 4),
];
/// A command, its options before the file, and the formats it knows
type Command = (&'static str, &'static [&'static str], &'static [&'static str]);
/// The players a music file plays by in the background, the first the system has that knows its format: macOS's
/// CoreAudio player, PulseAudio/PipeWire's (libsndfile: no mp3 before 1.1), FFmpeg's, mpv and ALSA's; each says why
/// it fails
const FILE_PLAYERS: [Command; 5] = [
	("afplay", &[], &["wav", "mp3", "flac", "aiff", "m4a"]),
	("paplay", &[], &["wav", "ogg", "flac", "aiff"]),
	("ffplay", &["-nodisp", "-autoexit", "-loglevel", "error"], &FORMATS),
	("mpv", &["--no-video", "--msg-level=all=error"], &FORMATS),
	("aplay", &["-q"], &["wav"]),
];
/// The decoders that read a music file without playing it, the first the system has that knows its format
const FILE_PROBES: [Command; 2] =
	[("ffprobe", &["-v", "error"], &FORMATS), ("afinfo", &[], &["wav", "mp3", "flac", "aiff", "m4a"])];
/// A player failing at once (a file it cannot decode) fails play itself; a later failure is printed when it happens
const EARLY_FAILURE: Duration = Duration::from_millis(300);
const PLAYER_POLL: Duration = Duration::from_millis(50);
/// The last characters of a failing player's error output that its report shows
const FAILURE_TAIL: usize = 300;
/// The samples a signal computes at once (lib/sound.warp play_blocks: the program's linear block): 23 ms at 22050 Hz
const SIGNAL_BLOCK: usize = 512;
/// How far ahead of the speaker a signal is computed: its latency, and room for a slow block
const SIGNAL_AHEAD: Duration = Duration::from_millis(100);
const SIGNAL_POLL: Duration = Duration::from_millis(5);
/// How far ahead of the speaker a heard voice may queue before its next sound waits (card real-time-playback): room
/// to compute the next one, so `loop { melody … }` plays along instead of running minutes ahead
const PLAY_AHEAD: Duration = Duration::from_millis(500);
type Player = Arc<Mutex<std::process::Child>>;
/// The sounds and music files playing by their handles, stopped by stop_sound
static PLAYING: Mutex<Vec<(usize, Player)>> = Mutex::new(Vec::new());
/// Where a voice stands on the audio clock: when its queued sounds end in real time, and how many seconds of sound
/// it has made in all (its place for save sound, free of the time computing them took); its origin is the real time
/// of place 0, where `at 2 beats` counts from
#[derive(Clone, Copy, Default)]
pub struct Voice {
	end: Option<Instant>,
	seconds: f64,
	origin: Option<Instant>,
	stops: usize,
	/// the run the voice belongs to (start_run): save sound mixes the sounds of its own run only, as the tests run many
	/// programs at once in one process
	run: usize,
}
thread_local! {
	/// The voice of this thread: the program's, or a task's from its start (tasks.rs spawn)
	static VOICE: Cell<Voice> = Cell::new(Voice::default());
	/// The voices an `at` statement left, taken up again at its end
	static BEFORE_AT: RefCell<Vec<Voice>> = const { RefCell::new(Vec::new()) };
	/// The bus this thread's next sounds go through (card shared-buses), 0: none
	static SENT_TO: Cell<usize> = const { Cell::new(0) };
}
/// The buses' effects by their numbers, from 1 (card shared-buses): every thread's sounds may go through each. Each
/// definition since the last save sound at its place on the defining voice, so a render changes the effects there
/// (stage 3); the last one before the render stays, its place -∞
static BUSES: Mutex<Vec<BusHistory>> = Mutex::new(Vec::new());
/// A bus's definitions: each at its place, its effects
type BusHistory = Vec<(f64, Vec<Effect>)>;
/// Counted up by stop_sound: a queued sound of an earlier count is dropped, and its player's end is no failure
static STOPS: AtomicUsize = AtomicUsize::new(0);
/// The sounds queued for the player thread and not played to their end yet
static PENDING: AtomicUsize = AtomicUsize::new(0);
/// The runs started in this process (start_run)
static RUNS: AtomicUsize = AtomicUsize::new(0);
/// The place on the voices where the last save sound of each run ended, by run
static RENDERED_UNTIL: Mutex<BTreeMap<usize, f64>> = Mutex::new(BTreeMap::new());
/// A sound since the last save sound: its place in its voice (seconds), sample rate, channels and 16-bit samples
struct Unrendered {
	run: usize,
	handle: usize,
	bus: usize,
	place: f64,
	rate: u32,
	channels: u16,
	samples: Vec<i16>,
}
/// The sounds since the last save sound, in memory (44 KB a second at 22050 Hz mono: the WAV files are shared by all
/// warp processes, another run may overwrite them)
static UNRENDERED: Mutex<Vec<Unrendered>> = Mutex::new(Vec::new());
/// fade_sound(handle, gain, seconds) as save sound applies it: from its place, where the speaker stood on the caller's
/// voice, the handle's gain (0: every sound's) ramps linearly to `gain` over the seconds
#[derive(Clone, Copy)]
struct Fade {
	run: usize,
	handle: usize,
	place: f64,
	gain: f64,
	seconds: f64,
}
/// The fades since the last save sound
static FADES: Mutex<Vec<Fade>> = Mutex::new(Vec::new());
/// A queued sound: its WAV, when it starts, the stop count it was queued at and its handle
struct Queued {
	path: PathBuf,
	start: Instant,
	stops: usize,
	handle: usize,
}

/// `play "song.mp3"`: a music file played in the background when the user may hear it, else checked and named. A file
/// the system's decoder cannot read fails here, as does a player failing at once; a later failure is printed. Its handle
pub fn play_file(path: &str) -> Result<usize, String> {
	crate::host::sandbox::check_process("sound.play_file")?;
	let format = file_format(path)?;
	probe(path, format)?;
	let handle = next_handle();
	if !shows_windows() {
		eprintln!("sound file {format}: {path}");
		return Ok(handle);
	}
	let (player, child) = spawned(&FILE_PLAYERS, format, path, false)
		.ok_or_else(|| format!("no player for {format} found ({}) to play {path}", names_for(&FILE_PLAYERS, format)))?;
	let child: Player = Arc::new(Mutex::new(child));
	let started = Instant::now();
	while started.elapsed() < EARLY_FAILURE {
		if let Some(failure) = failure_of(&child, player, path) {
			return failure.map(|_| handle);
		}
		std::thread::sleep(PLAYER_POLL);
	}
	listed(handle, &child);
	let queued = Queued { path: path.into(), start: Instant::now(), stops: STOPS.load(Ordering::Relaxed), handle };
	std::thread::spawn(move || watched(child, player, &queued, "sound file"));
	Ok(handle)
}

fn next_handle() -> usize {
	SOUNDED.fetch_add(1, Ordering::Relaxed) + 1
}

/// The handle of the latest sound or music file, 0 before the first
pub fn last_handle() -> usize {
	SOUNDED.load(Ordering::Relaxed)
}

impl Queued {
	/// stop_sound() since it was queued, or stop_sound(its handle)
	fn stopped(&self) -> bool {
		stopped_since(self.handle, self.stops)
	}
}

/// stop_sound() since the stop count `stops`, or stop_sound(handle)
fn stopped_since(handle: usize, stops: usize) -> bool {
	stops != STOPS.load(Ordering::Relaxed) || STOPPED.lock().is_ok_and(|stopped| stopped.contains(&handle))
}

/// The player among those playing by its handle: stop_sound stops it, the process's end waits for it (a ctrl-c then
/// stops both)
fn listed(handle: usize, child: &Player) {
	if let Ok(mut playing) = PLAYING.lock() {
		playing.push((handle, child.clone()));
	}
	wait_at_exit();
}

/// The player played to its end, its failure printed unless stop_sound ended it
fn watched(child: Player, player: &str, queued: &Queued, label: &str) {
	let path = queued.path.display().to_string();
	let ended = loop {
		match failure_of(&child, player, &path) {
			Some(ended) => break ended,
			None => std::thread::sleep(PLAYER_POLL),
		}
	};
	if let (Err(failure), false) = (ended, queued.stopped()) {
		eprintln!("{label} {failure}");
	}
	if let Ok(mut playing) = PLAYING.lock() {
		playing.retain(|(_, other)| !Arc::ptr_eq(other, &child));
	}
}

/// stop_sound: the queued sounds dropped, the playing sounds and music files stopped
pub fn stop() {
	STOPS.fetch_add(1, Ordering::Relaxed);
	sound_engine::stop(0);
	killed(|_| true);
}

/// stop_sound(handle): that sound or music file alone dropped from the queue or stopped
pub fn stop_one(handle: usize) -> Result<(), String> {
	if handle == 0 || handle > last_handle() {
		return Err(format!("stop_sound({handle}): no sound {handle} in this run, which played {}", last_handle()));
	}
	if let Ok(mut stopped) = STOPPED.lock() {
		stopped.push(handle);
	}
	sound_engine::stop(handle);
	killed(|playing| playing == handle);
	Ok(())
}

/// fade_sound(handle, gain, seconds): the playing sound's gain ramps to `gain` (1 as played, 0 silent) over the seconds;
/// handle 0 fades all. Only warp's own output ramps a playing sound (sound_engine.rs); played by afplay it can't
pub fn fade(handle: usize, gain: f64, seconds: f64) -> Result<(), String> {
	if handle > last_handle() {
		return Err(format!("fade_sound({handle}): no sound {handle} in this run, which played {}", last_handle()));
	}
	if !(0.0..=1.0).contains(&gain) || seconds < 0.0 {
		return Err(format!("fade_sound({handle}, {gain}, {seconds}): the gain is 0…1 and the seconds not negative"));
	}
	// headless nothing is heard: the render places the fade at the start, not wherever the program's slowness put it
	let place = if shows_windows() { voice().seconds - queued_seconds() } else { 0.0 };
	if let Ok(mut fades) = FADES.lock() {
		fades.push(Fade { run: voice().run, handle, place, gain, seconds });
	}
	let faded = sound_engine::fade(handle, gain as f32, seconds);
	if !faded && shows_windows() && last_handle() > 0 {
		return Err("fade_sound: the sounds play by afplay here (no output stream of warp's own), which can't change a playing sound's gain".into());
	}
	Ok(())
}

/// The players of the handles chosen ended
fn killed(chosen: impl Fn(usize) -> bool) {
	let playing: Vec<Player> = PLAYING
		.lock()
		.map(|playing| playing.iter().filter(|(handle, _)| chosen(*handle)).map(|(_, child)| child.clone()).collect())
		.unwrap_or_default();
	for child in playing {
		if let Ok(mut child) = child.lock() {
			let _ = child.kill();
		}
	}
}

/// sound_queued(): the seconds this voice's queued sounds still sound on the audio clock (a task asks its own voice)
pub fn queued_seconds() -> f64 {
	let voice = voice();
	let end = voice.end.filter(|_| voice.stops == STOPS.load(Ordering::Relaxed));
	end.map_or(0.0, |end| end.saturating_duration_since(Instant::now()).as_secs_f64())
}

/// wait_sound, and the process's end: until the players and our output stream played what is queued (headless
/// nothing is)
pub fn wait() {
	while PENDING.load(Ordering::Relaxed) > 0 || sound_engine::busy() {
		std::thread::sleep(PLAYER_POLL);
	}
}

/// This thread's voice, for a task it starts: the task's sounds begin where this voice stands
pub fn voice() -> Voice {
	VOICE.with(Cell::get)
}

/// The voice of a task's thread, taken over from the thread that started it
pub fn join_voice(voice: Voice) {
	VOICE.with(|own| own.set(voice));
}

/// `bus()` and a bus's effects changed (card shared-buses): the bus numbered `bus` (0: a new one) gets the effects, from
/// now on, live from this voice's moment, where the sounds queued before it end; its number
pub fn define_bus(bus: usize, effects: Vec<Effect>) -> Result<usize, String> {
	let (start, place) = moment();
	let mut buses = BUSES.lock().map_err(|_| "the sound buses are poisoned")?;
	match bus {
		0 => {
			buses.push(vec![(place, effects.clone())]);
			sound_engine::set_bus(buses.len(), &effects);
			Ok(buses.len())
		}
		_ => {
			buses.get_mut(bus - 1).ok_or_else(|| format!("no sound bus {bus}"))?.push((place, effects.clone()));
			sound_engine::change_bus(bus, start, &effects);
			Ok(bus)
		}
	}
}

/// The bus this thread's next sounds go through, 0: none; the bus they went through before
pub fn send_to(bus: usize) -> usize {
	SENT_TO.with(|sent| sent.replace(bus))
}

/// The bus's latest effects
fn bus_effects(bus: usize) -> Vec<Effect> {
	let buses = BUSES.lock();
	let latest = |buses: &Vec<BusHistory>| buses.get(bus.checked_sub(1)?)?.last().map(|(_, effects)| effects.clone());
	buses.ok().and_then(|buses| latest(&buses)).unwrap_or_default()
}

/// The bus's effects at the render's first place, and their changes after it at their frames from there; the
/// definitions before the render's end forgotten but the last
fn bus_changes(bus: usize, first_place: f64, rate: u32) -> (Vec<Effect>, Vec<(usize, Vec<Effect>)>) {
	// taken and replaced by its last definition under one lock: a voice's definition meanwhile is not lost
	let forgotten = |history: &mut BusHistory| {
		let last = history.last().map(|(_, effects)| vec![(f64::NEG_INFINITY, effects.clone())]).unwrap_or_default();
		std::mem::replace(history, last)
	};
	let Some(history) = BUSES.lock().ok().and_then(|mut buses| buses.get_mut(bus.checked_sub(1)?).map(forgotten))
	else {
		return (Vec::new(), Vec::new());
	};
	let (before, after): (Vec<_>, Vec<_>) = history.iter().partition(|(place, _)| *place <= first_place);
	let effects = before.last().or(after.first()).map(|(_, effects)| effects.clone()).unwrap_or_default();
	let frame = |place: f64| ((place - first_place) * rate as f64).round() as usize;
	let changes = after.iter().map(|(place, effects)| (frame(*place), effects.clone())).collect();
	(effects, changes)
}

/// `at seconds …` (card sample-accurate): the statement's sounds start that many seconds after the voice's first
/// sound, its place in renders exactly, on the speakers not before now; at_end returns to where the voice stood
pub fn at(seconds: f64) {
	let voice = voice();
	BEFORE_AT.with(|before| before.borrow_mut().push(voice));
	let origin = voice.origin.unwrap_or_else(Instant::now);
	let stops = STOPS.load(Ordering::Relaxed);
	join_voice(Voice {
		end: Some(origin + Duration::from_secs_f64(seconds.max(0.0))),
		seconds: seconds.max(0.0),
		origin: Some(origin),
		stops,
		run: voice.run,
	});
}

pub fn at_end() {
	if let Some(before) = BEFORE_AT.with(|before| before.borrow_mut().pop()) {
		join_voice(Voice { origin: voice().origin, ..before }); // the origin `at` gave a voice without sounds stays
	}
}

/// A moment on this voice (card midi-clock): where the sounds queued before it end, or an `at`'s time; its place on
/// the voice. Nothing is queued, the voice stays where it stands
pub fn moment() -> (Instant, f64) {
	scheduled(0.0)
}

/// This voice's moment in seconds on the run's clock, comparable across tasks: a live loop takes the body redefined
/// at or before the moment its cycle starts (card live-redefine)
pub fn moment_seconds() -> f64 {
	let (start, _) = moment();
	start.saturating_duration_since(*RUN_CLOCK).as_secs_f64()
}

/// `run` at this voice's moment: at once when that is now, else on a thread of its own sleeping until then (the
/// process's end and wait_sound wait for it, stop_sound drops it as it drops a queued sound; its failure goes to stderr)
pub fn on_clock(run: impl FnOnce() -> Result<(), String> + Send + 'static) -> Result<(), String> {
	let (start, _) = moment();
	if start <= Instant::now() {
		return run();
	}
	wait_at_exit();
	PENDING.fetch_add(1, Ordering::Relaxed);
	let stops = STOPS.load(Ordering::Relaxed);
	std::thread::spawn(move || {
		std::thread::sleep(start.saturating_duration_since(Instant::now()));
		if STOPS.load(Ordering::Relaxed) == stops {
			if let Err(failure) = run() {
				eprintln!("{failure}");
			}
		}
		PENDING.fetch_sub(1, Ordering::Relaxed);
	});
	Ok(())
}

/// The voice goes on that many seconds without a sound (midi_note's length); heard, paced as a sound is
pub fn rest(seconds: f64) {
	paced_when_heard();
	scheduled(seconds.max(0.0));
}

/// Heard, until the voice is within PLAY_AHEAD of the speaker, except in an `at`, which places its sounds itself (an
/// arrangement's later bars come before its earlier ones)
fn paced_when_heard() {
	if shows_windows() && BEFORE_AT.with(|before| before.borrow().is_empty()) {
		paced(PLAY_AHEAD);
	}
}

/// The sound's seconds added to its voice behind the sounds queued there before it: when it starts in real time, and
/// its place among the voice's sounds. stop_sound since the voice's last sound starts the voice anew, now
fn scheduled(seconds: f64) -> (Instant, f64) {
	let now = Instant::now();
	let stops = STOPS.load(Ordering::Relaxed);
	let mut voice = voice();
	let start = voice.end.filter(|end| *end > now && voice.stops == stops).unwrap_or(now);
	let place = voice.seconds;
	let origin = voice.origin.unwrap_or_else(|| start - Duration::from_secs_f64(place));
	voice = Voice {
		end: Some(start + Duration::from_secs_f64(seconds)),
		seconds: place + seconds,
		origin: Some(origin),
		stops,
		run: voice.run,
	};
	join_voice(voice);
	(start, place)
}

/// The WAV played at its start on a thread of its own (the process's end waits for it)
fn queued(sound: Queued) {
	wait_at_exit();
	PENDING.fetch_add(1, Ordering::Relaxed);
	std::thread::spawn(move || {
		std::thread::sleep(sound.start.saturating_duration_since(Instant::now()));
		if !sound.stopped() {
			play_wav(&sound);
		}
		let _ = std::fs::remove_file(&sound.path); // played or stopped: per-process names would pile up otherwise
		PENDING.fetch_sub(1, Ordering::Relaxed);
	});
}

/// One queued WAV played to its end by the first player the system has
fn play_wav(queued: &Queued) {
	let shown = queued.path.display().to_string();
	match spawned(&FILE_PLAYERS, "wav", &shown, false) {
		Some((player, child)) => {
			let child: Player = Arc::new(Mutex::new(child));
			listed(queued.handle, &child);
			watched(child, player, queued, "sound")
		}
		None => eprintln!("sound: no player found ({}) for {shown}", names_for(&FILE_PLAYERS, "wav")),
	}
}

/// The process's end waits for the queued sounds and the playing music files (once registered for both)
fn wait_at_exit() {
	static REGISTERED: Once = Once::new();
	extern "C" fn wait_for_sounds() {
		wait();
		while PLAYING.lock().is_ok_and(|playing| !playing.is_empty()) {
			std::thread::sleep(PLAYER_POLL);
		}
	}
	extern "C" {
		fn atexit(callback: extern "C" fn()) -> i32;
	}
	// SAFETY: atexit only stores the function pointer; wait_for_sounds reads atomics and a lock and sleeps
	REGISTERED.call_once(|| unsafe {
		atexit(wait_for_sounds);
	});
}

/// Whether the system has a decoder that checks a file of this format without playing it
pub fn can_probe(format: &str) -> bool {
	knowing(&FILE_PROBES, format).any(|(probe, _)| which(probe))
}

/// The file decoded by the first probe the system has for its format (none: nothing to check it with)
fn probe(path: &str, format: &'static str) -> Result<(), String> {
	let Some((prober, child)) = spawned(&FILE_PROBES, format, path, true) else {
		return Ok(());
	};
	let output = child.wait_with_output().map_err(|failure| format!("{prober} could not check {path}: {failure}"))?;
	if output.status.success() {
		return Ok(());
	}
	Err(format!("{path} cannot be decoded as {format}: {prober} failed: {}", tail(&output.stderr, &output.stdout)))
}

/// The first command of the list the system has for the format, started on the file, its error output kept (its output
/// too when `keep_output`: afinfo says its failure there; a player's would fill the pipe)
fn spawned(
	commands: &[Command],
	format: &str,
	path: &str,
	keep_output: bool,
) -> Option<(&'static str, std::process::Child)> {
	if let Err(denial) = crate::host::sandbox::check_process("a sound player") {
		eprintln!("{denial}"); // a background player has no program to raise it in
		return None;
	}
	let output = || {
		if keep_output {
			std::process::Stdio::piped()
		} else {
			std::process::Stdio::null()
		}
	};
	knowing(commands, format).find_map(|(command, options)| {
		let child = std::process::Command::new(command)
			.args(options)
			.arg(path)
			.stdin(std::process::Stdio::null())
			.stdout(output())
			.stderr(std::process::Stdio::piped())
			.spawn()
			.ok()?;
		Some((command, child))
	})
}

/// The commands of the list that know the format, with their options
fn knowing<'a>(
	commands: &'a [Command],
	format: &'a str,
) -> impl Iterator<Item = (&'static str, &'static [&'static str])> + 'a {
	commands
		.iter()
		.filter(move |(_, _, formats)| formats.contains(&format))
		.map(|(command, options, _)| (*command, *options))
}

fn names_for(commands: &[Command], format: &str) -> String {
	knowing(commands, format).map(|(command, _)| command).collect::<Vec<_>>().join(", ")
}

fn which(command: &str) -> bool {
	std::env::var_os("PATH")
		.is_some_and(|paths| std::env::split_paths(&paths).any(|folder| folder.join(command).is_file()))
}

/// None while the player plays; once it ended Ok, or the failure naming the player and its error output's tail
fn failure_of(child: &Player, player: &str, path: &str) -> Option<Result<(), String>> {
	let mut child = child.lock().ok()?;
	let status = child.try_wait().ok()??;
	if status.success() {
		return Some(Ok(()));
	}
	let mut errors = Vec::new();
	if let Some(mut stderr) = child.stderr.take() {
		use std::io::Read;
		let _ = stderr.read_to_end(&mut errors);
	}
	Some(Err(format!("{path}: {player} failed ({status}): {}", tail(&errors, &[]))))
}

/// The last characters of a command's error output (or its output, where afinfo writes its failure)
fn tail(errors: &[u8], output: &[u8]) -> String {
	let said =
		String::from_utf8_lossy(if errors.iter().any(|byte| !byte.is_ascii_whitespace()) { errors } else { output });
	let said = said.trim();
	let start = said.char_indices().rev().nth(FAILURE_TAIL).map_or(0, |(at, _)| at);
	said[start..].replace('\n', " ")
}

/// The format of a music file by its first bytes
fn file_format(path: &str) -> Result<&'static str, String> {
	use std::io::Read;
	let mut start = [0u8; 12];
	let mut file = std::fs::File::open(path).map_err(|failure| format!("cannot open {path}: {failure}"))?;
	let read = file.read(&mut start).map_err(|failure| format!("cannot read {path}: {failure}"))?;
	FILE_FORMATS
		.iter()
		.find(|(_, magic, at)| start[..read].get(*at..at + magic.len()) == Some(magic))
		.map(|(format, _, _)| *format)
		.ok_or_else(|| format!("{path} is not a sound file ({})", FORMATS.join(", ")))
}

/// Queue the samples on the audio clock: when the user may hear them in our own output stream (sound_engine.rs), else
/// as a WAV for the system's player; headless the WAV only, its place said
pub fn sound(samples: &[u64], rate: u32, channels: u16) -> Result<(), String> {
	sounded(next_handle(), pcm(samples), rate, channels)
}

/// play_signal(f, seconds) (card real-time): the samples of f(t), t the sample's time in seconds, each computed when it
/// is due, in blocks of SIGNAL_BLOCK (play_blocks); `sample_at(channel, t)` for each of the channels, left first. Its
/// handle
pub fn play_signal(
	seconds: f64,
	rate: u32,
	channels: u16,
	mut sample_at: impl FnMut(usize, f64) -> Result<f64, String>,
) -> Result<usize, String> {
	play_blocks(seconds, rate, SIGNAL_BLOCK, channels, |first, count| {
		let mut blocks = vec![Vec::with_capacity(count); channels as usize];
		for index in first..first + count {
			for (channel, block) in blocks.iter_mut().enumerate() {
				block.push(sample_at(channel, index as f64 / rate as f64)?);
			}
		}
		Ok(blocks)
	})
}

/// play_blocks(fill, seconds) (sound layer 8, block-wise): the samples `fill(first, count)` gives, a block of each
/// channel, `count` of them from the sample `first` on, `block` at a time: on warp's own output each block queued
/// SIGNAL_AHEAD before the device plays it (stop_sound ends it), else all of them as one sound. Its handle
pub fn play_blocks(
	seconds: f64,
	rate: u32,
	block: usize,
	channels: u16,
	mut fill: impl FnMut(usize, usize) -> Result<Vec<Vec<f64>>, String>,
) -> Result<usize, String> {
	let total = (seconds.max(0.0) * rate as f64).round() as usize;
	let handle = next_handle();
	let stops = STOPS.load(Ordering::Relaxed);
	let streamed = shows_windows() && sound_engine::available();
	let mut samples = Vec::new();
	for first in (0..total).step_by(block.max(1)) {
		if streamed {
			paced(SIGNAL_AHEAD);
		}
		if stopped_since(handle, stops) {
			break;
		}
		let count = block.min(total - first);
		let blocks = fill(first, count)?;
		samples.extend(
			(0..count)
				.flat_map(|index| blocks.iter().map(move |block| block.get(index).copied().unwrap_or(0.0)))
				.map(signal_pcm),
		);
		if streamed {
			sounded(handle, std::mem::take(&mut samples), rate, channels)?;
		}
	}
	if !samples.is_empty() {
		sounded(handle, samples, rate, channels)?;
	}
	Ok(handle)
}

/// A signal's value, full scale -1…1, as a 16-bit amplitude
pub(crate) fn signal_pcm(value: f64) -> i16 {
	(value.clamp(-1.0, 1.0) * i16::MAX as f64).round() as i16
}

/// Until this voice's queued sounds end within `ahead` (stop_sound ends the wait)
pub fn paced(ahead: Duration) {
	while queued_seconds() > ahead.as_secs_f64() {
		std::thread::sleep(SIGNAL_POLL);
	}
}

/// The 16-bit samples queued as the sound of the handle, paced when heard
fn sounded(handle: usize, samples: Vec<i16>, rate: u32, channels: u16) -> Result<(), String> {
	let audible = shows_windows();
	paced_when_heard();
	let seconds = samples.len() as f64 / channels as f64 / rate.max(1) as f64;
	let (start, place) = scheduled(seconds);
	let bus = SENT_TO.with(Cell::get);
	let effects = bus_effects(bus);
	if audible && sound_engine::play(handle, (bus, &effects), start, &samples, rate, channels) {
		wait_at_exit();
	} else {
		let path = wav_file(&wav_of(&samples, rate, channels), handle)?;
		if audible {
			queued(Queued { path, start, stops: STOPS.load(Ordering::Relaxed), handle });
		} else {
			eprintln!("sound {seconds:.2} s: {}", path.display());
		}
	}
	if let Ok(mut unrendered) = UNRENDERED.lock() {
		unrendered.push(Unrendered { run: voice().run, handle, bus, place, rate, channels, samples });
	}
	Ok(())
}

fn wav_file(wav: &[u8], handle: usize) -> Result<PathBuf, String> {
	let stem = format!("{FILE_STEM}-{}", std::process::id());
	crate::extensions::utils::numbered_temporary_file(FOLDER, &stem, handle, "wav", wav, "sound")
}

/// save sound as path: the sounds since the last render (or the start) in one WAV file, each at its place in its
/// voice, the voices mixed (in stereo when one of them is: a mono sound on both speakers alike); its seconds
pub fn render(path: &str) -> Result<f64, String> {
	crate::host::sandbox::check_write(path)?;
	let run = voice().run;
	let sounds = of_run(&UNRENDERED, run, |sound| sound.run)?;
	// the highest rate of the sounds, the others converted to it (a 44.1 kHz sample beside 22.05 kHz synthesis)
	let rate = sounds.iter().map(|sound| sound.rate).max().unwrap_or(DEFAULT_RATE);
	let channels = sounds.iter().map(|sound| sound.channels).max().unwrap_or(MONO);
	// from where the last render ended, so `at 1s beep()` keeps its second of silence; an `at` before it starts earlier
	let mut rendered_until = RENDERED_UNTIL.lock().map_err(|_| POISONED.to_string())?;
	let first_place = sounds.iter().map(|sound| sound.place).fold(*rendered_until.entry(run).or_default(), f64::min);
	let fades = of_run(&FADES, run, |fade| fade.run)?;
	// each bus's sounds mixed apart, then through its effects together (card shared-buses); bus 0 is the output
	let mut buses: BTreeMap<usize, Vec<i32>> = BTreeMap::new();
	for sound in sounds {
		let mixed = buses.entry(sound.bus).or_default();
		let start = ((sound.place - first_place) * rate as f64).round() as usize * channels as usize;
		let own: Vec<Fade> =
			fades.iter().filter(|fade| fade.handle == 0 || fade.handle == sound.handle).copied().collect();
		let gain = |frame: usize| {
			if own.is_empty() {
				1.0
			} else {
				faded_gain(&own, sound.place + frame as f64 / rate as f64)
			}
		};
		let samples: Vec<i16> = at_rate(&sound.samples, sound.channels, sound.rate, rate)
			.chunks(sound.channels as usize)
			.enumerate()
			.flat_map(|(index, frame)| {
				let gain = gain(index);
				(0..channels as usize)
					.map(move |channel| (frame[channel.min(frame.len() - 1)] as f64 * gain).round() as i16)
			})
			.collect();
		added_into(mixed, start, samples.into_iter().map(i32::from));
	}
	let mut mixed = buses.remove(&0).unwrap_or_default();
	for (bus, sum) in buses {
		let amplitudes: Vec<f32> = sum.iter().map(|&sample| sample as f32 / i16::MAX as f32).collect();
		let (effects, changes) = bus_changes(bus, first_place, rate);
		let processed = effects::processed(&amplitudes, &effects, &changes, rate, channels);
		added_into(&mut mixed, 0, processed.into_iter().map(|sample| (sample * i16::MAX as f32).round() as i32));
	}
	let seconds = mixed.len() as f64 / channels as f64 / rate as f64;
	rendered_until.insert(run, first_place + seconds);
	let samples: Vec<i16> = mixed.iter().map(|&sum| sum.clamp(i16::MIN as i32, i16::MAX as i32) as i16).collect();
	std::fs::write(path, wav_of(&samples, rate, channels))
		.map_err(|failure| format!("cannot write {path}: {failure}"))?;
	Ok(seconds)
}

const POISONED: &str = "the rendered sounds are poisoned";

/// The samples added into the mix from `start` on, the mix grown to hold them
fn added_into(mix: &mut Vec<i32>, start: usize, samples: impl ExactSizeIterator<Item = i32>) {
	let end = start + samples.len();
	if mix.len() < end {
		mix.resize(end, 0);
	}
	mix[start..].iter_mut().zip(samples).for_each(|(sum, sample)| *sum += sample);
}

/// The items of the run taken out of the shared list, the other runs' left in it
fn of_run<T>(shared: &Mutex<Vec<T>>, run: usize, run_of: impl Fn(&T) -> usize) -> Result<Vec<T>, String> {
	let mut items = shared.lock().map_err(|_| POISONED.to_string())?;
	Ok(items.extract_if(.., |item| run_of(item) == run).collect())
}

/// A program starts running on this thread: a voice of a run of its own, at place 0, sending to no bus; its tasks take
/// the voice over (join_voice)
pub fn start_run() {
	join_voice(Voice { run: RUNS.fetch_add(1, Ordering::Relaxed) + 1, ..Voice::default() });
	SENT_TO.with(|bus| bus.set(0));
}

/// The gain at the place after the fades, in the order they were called: each ramps linearly from the gain it finds
/// there to its own
fn faded_gain(fades: &[Fade], place: f64) -> f64 {
	let ramped = |fade: &Fade, from: f64, place: f64| {
		let share = if fade.seconds > 0.0 { ((place - fade.place) / fade.seconds).clamp(0.0, 1.0) } else { 1.0 };
		from + (fade.gain - from) * share
	};
	let mut ramp = (&Fade { run: 0, handle: 0, place: 0.0, gain: 1.0, seconds: 0.0 }, 1.0);
	for fade in fades.iter().filter(|fade| fade.place <= place) {
		ramp = (fade, ramped(ramp.0, ramp.1, fade.place));
	}
	ramped(ramp.0, ramp.1, place)
}

/// Interleaved samples at another rate, each channel's linearly interpolated between its neighbours
fn at_rate(samples: &[i16], channels: u16, from: u32, to: u32) -> Vec<i16> {
	let channels = channels.max(1) as usize;
	if from == to || samples.len() < channels {
		return samples.to_vec();
	}
	let frames = samples.len() / channels;
	let converted_frames = (frames as f64 * to as f64 / from as f64).round() as usize;
	(0..converted_frames)
		.flat_map(|frame| {
			let position = frame as f64 * from as f64 / to as f64;
			let before = (position.floor() as usize).min(frames - 1);
			let after = (before + 1).min(frames - 1);
			let share = position - before as f64;
			(0..channels).map(move |channel| {
				let (a, b) = (samples[before * channels + channel] as f64, samples[after * channels + channel] as f64);
				(a + (b - a) * share).round() as i16
			})
		})
		.collect()
}

/// A 16-bit mono PCM WAV of the offset samples
pub fn wav(samples: &[u64], rate: u32) -> Vec<u8> {
	wav_of(&pcm(samples), rate, MONO)
}

/// The offset samples as 16-bit amplitudes
fn pcm(samples: &[u64]) -> Vec<i16> {
	samples
		.iter()
		.map(|&sample| (sample as i64 - SAMPLE_OFFSET).clamp(i16::MIN as i64, i16::MAX as i64) as i16)
		.collect()
}

/// A 16-bit PCM WAV of the samples, a stereo one's interleaved left, right
fn wav_of(samples: &[i16], rate: u32, channels: u16) -> Vec<u8> {
	let data: Vec<u8> = samples.iter().flat_map(|sample| sample.to_le_bytes()).collect();
	let data = data.as_slice();
	let format = [
		PCM_FORMAT.to_le_bytes().as_slice(),
		&channels.to_le_bytes(),
		&rate.to_le_bytes(),
		&(rate * BYTES_PER_SAMPLE * channels as u32).to_le_bytes(),
		&(BYTES_PER_SAMPLE as u16 * channels).to_le_bytes(),
		&BITS_PER_SAMPLE.to_le_bytes(),
	]
	.concat();
	[
		b"RIFF".as_slice(),
		&(HEADER_SIZE_AFTER_RIFF + data.len() as u32).to_le_bytes(),
		b"WAVE",
		b"fmt ",
		&FORMAT_CHUNK_SIZE.to_le_bytes(),
		&format,
		b"data",
		&(data.len() as u32).to_le_bytes(),
		data,
	]
	.concat()
}
