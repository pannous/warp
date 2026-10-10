//! sound_samples(samples, count, rate) natively (card basic-sound, lib/sound.warp): 16-bit mono samples as a WAV file in the
//! system's temporary folder (warp-sound/sound-<process id>.wav, then sound-<process id>-2.wav …: runs at the same time
//! never share a file; card sound-wavs), played by the
//! system's player when the warp binary may reach the user (paint::shows_windows: never under WARP_NO_WINDOW, CI or
//! tests, which only get the file). The playground plays the same samples with WebAudio (host.js sound_samples).
//! A sound is queued on the audio clock behind those before it in its voice and `play` returns at once (card sound-pro):
//! each sound waits for its start on a thread of its own and plays, stop_sound drops the waiting ones, the process's
//! end waits for them and for the music files playing. A voice is the program's thread or a task (`go { … }`), which
//! starts where its starter stands: voices sound together. render_sound mixes the sounds since the last render as
//! they sound, by the voices' places on the clock (counted in samples, not in real time): offline, headless too.

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::cell::Cell;
use std::sync::{Arc, Mutex, Once};
use std::time::{Duration, Instant};

/// The samples cross as whole numbers ≥ 0: amplitude + 32768 (lib/sound.warp sample_offset)
pub const SAMPLE_OFFSET: i64 = 32_768;
const FOLDER: &str = "warp-sound";
const FILE_STEM: &str = "sound";
const PCM_FORMAT: u16 = 1;
const CHANNELS: u16 = 1;
const BITS_PER_SAMPLE: u16 = 16;
const BYTES_PER_SAMPLE: u32 = (BITS_PER_SAMPLE / 8) as u32;
const FORMAT_CHUNK_SIZE: u32 = 16;
const HEADER_SIZE_AFTER_RIFF: u32 = 36;
/// The rate of a render without sounds (lib/sound.warp sample_rate)
const DEFAULT_RATE: u32 = 22_050;
/// The sounds and music files of this run so far: a sound's handle is its number among them, from 1
static SOUNDED: AtomicUsize = AtomicUsize::new(0);
/// The handles stop_sound(handle) stopped one by one
static STOPPED: Mutex<Vec<usize>> = Mutex::new(Vec::new());
/// The music files' formats by their first bytes: (format, magic bytes, offset of the magic)
const FILE_FORMATS: [(&str, &[u8], usize); 7] = [
	("wav", b"WAVE", 8), ("mp3", b"ID3", 0), ("mp3", b"\xFF\xFB", 0), ("ogg", b"OggS", 0), ("flac", b"fLaC", 0), ("aiff", b"AIFF", 8), ("m4a", b"ftyp", 4),
];
/// A command, its options before the file, and the formats it knows
type Command = (&'static str, &'static [&'static str], &'static [&'static str]);
/// The players a music file plays by in the background, the first the system has that knows its format: macOS's
/// CoreAudio player, PulseAudio/PipeWire's (libsndfile: no mp3 before 1.1), FFmpeg's, mpv and ALSA's; each says why
/// it fails
const FILE_PLAYERS: [Command; 5] = [
	("afplay", &[], &["wav", "mp3", "flac", "aiff", "m4a"]),
	("paplay", &[], &["wav", "ogg", "flac", "aiff"]),
	("ffplay", &["-nodisp", "-autoexit", "-loglevel", "error"], &["wav", "mp3", "ogg", "flac", "aiff", "m4a"]),
	("mpv", &["--no-video", "--msg-level=all=error"], &["wav", "mp3", "ogg", "flac", "aiff", "m4a"]),
	("aplay", &["-q"], &["wav"]),
];
/// The decoders that read a music file without playing it, the first the system has that knows its format
const FILE_PROBES: [Command; 2] = [
	("ffprobe", &["-v", "error"], &["wav", "mp3", "ogg", "flac", "aiff", "m4a"]),
	("afinfo", &[], &["wav", "mp3", "flac", "aiff", "m4a"]),
];
/// A player failing at once (a file it cannot decode) fails play itself; a later failure is printed when it happens
const EARLY_FAILURE: Duration = Duration::from_millis(300);
const PLAYER_POLL: Duration = Duration::from_millis(50);
/// The last characters of a failing player's error output that its report shows
const FAILURE_TAIL: usize = 300;
type Player = Arc<Mutex<std::process::Child>>;
/// The sounds and music files playing by their handles, stopped by stop_sound
static PLAYING: Mutex<Vec<(usize, Player)>> = Mutex::new(Vec::new());
/// When the queued sounds of all voices end on the audio clock; none before the first
static SOUNDS_END: Mutex<Option<Instant>> = Mutex::new(None);
/// Where a voice stands on the audio clock: when its queued sounds end in real time, and how many seconds of sound
/// it has made in all (its place for render_sound, free of the time computing them took)
#[derive(Clone, Copy, Default)]
pub struct Voice {
	end: Option<Instant>,
	seconds: f64,
	stops: usize,
}
thread_local! {
	/// The voice of this thread: the program's, or a task's from its start (tasks.rs spawn)
	static VOICE: Cell<Voice> = Cell::new(Voice::default());
}
/// Counted up by stop_sound: a queued sound of an earlier count is dropped, and its player's end is no failure
static STOPS: AtomicUsize = AtomicUsize::new(0);
/// The sounds queued for the player thread and not played to their end yet
static PENDING: AtomicUsize = AtomicUsize::new(0);
/// The sounds since the last render_sound: their place in their voice (seconds), sample rate and 16-bit samples (in
/// memory, 44 KB a second at 22050 Hz: the WAV files are shared by all warp processes, another run may overwrite them)
static UNRENDERED: Mutex<Vec<(f64, u32, Vec<u8>)>> = Mutex::new(Vec::new());
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
	let format = file_format(path)?;
	probe(path, format)?;
	let handle = next_handle();
	if !crate::paint::shows_windows() {
		eprintln!("sound file {format}: {path}");
		return Ok(handle);
	}
	let (player, child) = spawned(&FILE_PLAYERS, format, path, false).ok_or_else(|| format!("no player for {format} found ({}) to play {path}", names_for(&FILE_PLAYERS, format)))?;
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
		self.stops != STOPS.load(Ordering::Relaxed) || STOPPED.lock().is_ok_and(|stopped| stopped.contains(&self.handle))
	}
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
	if let Ok(mut end) = SOUNDS_END.lock() {
		*end = None;
	}
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
	killed(|playing| playing == handle);
	Ok(())
}

/// The players of the handles chosen ended
fn killed(chosen: impl Fn(usize) -> bool) {
	let playing: Vec<Player> = PLAYING.lock().map(|playing| playing.iter().filter(|(handle, _)| chosen(*handle)).map(|(_, child)| child.clone()).collect()).unwrap_or_default();
	for child in playing {
		if let Ok(mut child) = child.lock() {
			let _ = child.kill();
		}
	}
}

/// sound_queued(): the seconds the queued sounds still sound on the audio clock
pub fn queued_seconds() -> f64 {
	let end = SOUNDS_END.lock().ok().and_then(|end| *end);
	end.map_or(0.0, |end| end.saturating_duration_since(Instant::now()).as_secs_f64())
}

/// wait_sound, and the process's end: until the player thread played what is queued (headless nothing is)
pub fn wait() {
	while PENDING.load(Ordering::Relaxed) > 0 {
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

/// The sound's seconds added to its voice behind the sounds queued there before it: when it starts in real time, and
/// its place among the voice's sounds. stop_sound since the voice's last sound starts the voice anew, now
fn scheduled(seconds: f64) -> (Instant, f64) {
	let now = Instant::now();
	let stops = STOPS.load(Ordering::Relaxed);
	let mut voice = voice();
	let start = voice.end.filter(|end| *end > now && voice.stops == stops).unwrap_or(now);
	let place = voice.seconds;
	voice = Voice { end: Some(start + Duration::from_secs_f64(seconds)), seconds: place + seconds, stops };
	join_voice(voice);
	if let Ok(mut end) = SOUNDS_END.lock() {
		*end = (*end).filter(|end| *end > now).max(voice.end);
	}
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
	REGISTERED.call_once(|| unsafe { atexit(wait_for_sounds); });
}

/// Whether the system has a decoder that checks a file of this format without playing it
pub fn can_probe(format: &str) -> bool {
	knowing(&FILE_PROBES, format).any(|(probe, _)| which(probe))
}

/// The file decoded by the first probe the system has for its format (none: nothing to check it with)
fn probe(path: &str, format: &'static str) -> Result<(), String> {
	let Some((prober, child)) = spawned(&FILE_PROBES, format, path, true) else { return Ok(()) };
	let output = child.wait_with_output().map_err(|failure| format!("{prober} could not check {path}: {failure}"))?;
	if output.status.success() {
		return Ok(());
	}
	Err(format!("{path} cannot be decoded as {format}: {prober} failed: {}", tail(&output.stderr, &output.stdout)))
}

/// The first command of the list the system has for the format, started on the file, its error output kept (its output
/// too when `keep_output`: afinfo says its failure there; a player's would fill the pipe)
fn spawned(commands: &[Command], format: &str, path: &str, keep_output: bool) -> Option<(&'static str, std::process::Child)> {
	let output = || if keep_output { std::process::Stdio::piped() } else { std::process::Stdio::null() };
	knowing(commands, format).find_map(|(command, options)| {
		let child = std::process::Command::new(command).args(options).arg(path).stdin(std::process::Stdio::null())
			.stdout(output()).stderr(std::process::Stdio::piped()).spawn().ok()?;
		Some((command, child))
	})
}

/// The commands of the list that know the format, with their options
fn knowing<'a>(commands: &'a [Command], format: &'a str) -> impl Iterator<Item = (&'static str, &'static [&'static str])> + 'a {
	commands.iter().filter(move |(_, _, formats)| formats.contains(&format)).map(|(command, options, _)| (*command, *options))
}

fn names_for(commands: &[Command], format: &str) -> String {
	knowing(commands, format).map(|(command, _)| command).collect::<Vec<_>>().join(", ")
}

fn which(command: &str) -> bool {
	std::env::var_os("PATH").is_some_and(|paths| std::env::split_paths(&paths).any(|folder| folder.join(command).is_file()))
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
	let said = String::from_utf8_lossy(if errors.iter().any(|byte| !byte.is_ascii_whitespace()) { errors } else { output });
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
	FILE_FORMATS.iter().find(|(_, magic, at)| start[..read].get(*at..at + magic.len()) == Some(magic)).map(|(format, _, _)| *format)
		.ok_or_else(|| format!("{path} is not a sound file (wav, mp3, ogg, flac, aiff, m4a)"))
}

/// Write the samples as a WAV and queue it on the audio clock: played when the user may hear it, else say where it is
pub fn sound(samples: &[u64], rate: u32) -> Result<PathBuf, String> {
	let data = pcm(samples);
	let handle = next_handle();
	let path = wav_file(&data, rate, handle)?;
	let seconds = samples.len() as f64 / rate.max(1) as f64;
	let (start, place) = scheduled(seconds);
	if let Ok(mut unrendered) = UNRENDERED.lock() {
		unrendered.push((place, rate, data));
	}
	if crate::paint::shows_windows() {
		queued(Queued { path: path.clone(), start, stops: STOPS.load(Ordering::Relaxed), handle });
	} else {
		eprintln!("sound {seconds:.2} s: {}", path.display());
	}
	Ok(path)
}

fn wav_file(data: &[u8], rate: u32, handle: usize) -> Result<PathBuf, String> {
	let folder = std::env::temp_dir().join(FOLDER);
	std::fs::create_dir_all(&folder).map_err(|failure| format!("sound: cannot create {}: {failure}", folder.display()))?;
	let process = std::process::id();
	let path = folder.join(if handle == 1 { format!("{FILE_STEM}-{process}.wav") } else { format!("{FILE_STEM}-{process}-{handle}.wav") });
	std::fs::write(&path, wav_of_data(data, rate)).map_err(|failure| format!("sound: cannot write {}: {failure}", path.display()))?;
	Ok(path)
}

/// render_sound(path): the sounds since the last render (or the start) in one WAV file, each at its place in its
/// voice, the voices mixed; its seconds
pub fn render(path: &str) -> Result<f64, String> {
	let sounds = std::mem::take(&mut *UNRENDERED.lock().map_err(|_| "the rendered sounds are poisoned".to_string())?);
	let rate = sounds.first().map_or(DEFAULT_RATE, |(_, rate, _)| *rate);
	if let Some((_, other, _)) = sounds.iter().find(|(_, sound_rate, _)| *sound_rate != rate) {
		return Err(format!("sounds of {rate} and {other} samples per second cannot share {path}"));
	}
	let first_place = sounds.iter().map(|(place, _, _)| *place).fold(f64::INFINITY, f64::min);
	let mut mixed: Vec<i32> = vec![];
	for (place, _, data) in sounds {
		let start = ((place - first_place) * rate as f64).round() as usize;
		let samples: Vec<i16> = data.chunks(2).map(|pair| i16::from_le_bytes([pair[0], pair[1]])).collect();
		if mixed.len() < start + samples.len() {
			mixed.resize(start + samples.len(), 0);
		}
		mixed[start..].iter_mut().zip(samples).for_each(|(sum, sample)| *sum += sample as i32);
	}
	let data: Vec<u8> = mixed.iter().flat_map(|&sum| (sum.clamp(i16::MIN as i32, i16::MAX as i32) as i16).to_le_bytes()).collect();
	std::fs::write(path, wav_of_data(&data, rate)).map_err(|failure| format!("cannot write {path}: {failure}"))?;
	Ok(mixed.len() as f64 / rate as f64)
}

/// A 16-bit mono PCM WAV of the offset samples
pub fn wav(samples: &[u64], rate: u32) -> Vec<u8> {
	wav_of_data(&pcm(samples), rate)
}

/// The offset samples as 16-bit little-endian PCM
fn pcm(samples: &[u64]) -> Vec<u8> {
	samples.iter().flat_map(|&sample| ((sample as i64 - SAMPLE_OFFSET).clamp(i16::MIN as i64, i16::MAX as i64) as i16).to_le_bytes()).collect()
}

fn wav_of_data(data: &[u8], rate: u32) -> Vec<u8> {
	let format = [
		PCM_FORMAT.to_le_bytes().as_slice(), &CHANNELS.to_le_bytes(), &rate.to_le_bytes(), &(rate * BYTES_PER_SAMPLE * CHANNELS as u32).to_le_bytes(),
		&(BYTES_PER_SAMPLE as u16 * CHANNELS).to_le_bytes(), &BITS_PER_SAMPLE.to_le_bytes(),
	].concat();
	[
		b"RIFF".as_slice(), &(HEADER_SIZE_AFTER_RIFF + data.len() as u32).to_le_bytes(), b"WAVE",
		b"fmt ", &FORMAT_CHUNK_SIZE.to_le_bytes(), &format,
		b"data", &(data.len() as u32).to_le_bytes(), data,
	].concat()
}
