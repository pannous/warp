//! sound_samples(samples, count, rate) natively (card basic-sound, lib/sound.warp): 16-bit mono samples as a WAV file in the
//! system's temporary folder (warp-sound/sound.wav, then sound-2.wav … within one run, as paint's PNGs), played by the
//! system's player when the warp binary may reach the user (paint::shows_windows: never under WARP_NO_WINDOW, CI or
//! tests, which only get the file). The playground plays the same samples with WebAudio (host.js sound_samples).
//! A sound is queued on the audio clock behind those before it and `play` returns at once (card sound-pro): a player
//! thread plays the queue in order, stop_sound drops it, the process's end waits for it. render_sound writes the
//! sounds since the last render one after another into one WAV, offline: headless too.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
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
/// The sound calls of this run so far
static SOUNDED: AtomicUsize = AtomicUsize::new(0);
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
/// The sounds and music files playing, stopped by stop_sound
static PLAYING: Mutex<Vec<Player>> = Mutex::new(Vec::new());
/// When the queued sounds end on the audio clock; none before the first
static SOUNDS_END: Mutex<Option<Instant>> = Mutex::new(None);
/// Counted up by stop_sound: a queued sound of an earlier count is dropped, and its player's end is no failure
static STOPS: AtomicUsize = AtomicUsize::new(0);
/// The sounds queued for the player thread and not played to their end yet
static PENDING: AtomicUsize = AtomicUsize::new(0);
/// The sounds since the last render_sound, in order: their sample rate and 16-bit samples (in memory, 44 KB a second
/// at 22050 Hz: the WAV files are shared by all warp processes, another run may overwrite them)
static UNRENDERED: Mutex<Vec<(u32, Vec<u8>)>> = Mutex::new(Vec::new());
/// The player thread's queue: a sound's WAV and the stop count it was queued at
static QUEUE: OnceLock<Mutex<std::sync::mpsc::Sender<(PathBuf, usize)>>> = OnceLock::new();

/// `play "song.mp3"`: a music file played in the background when the user may hear it, else checked and named. A file
/// the system's decoder cannot read fails here, as does a player failing at once; a later failure is printed
pub fn play_file(path: &str) -> Result<(), String> {
	let format = file_format(path)?;
	probe(path, format)?;
	if !crate::paint::shows_windows() {
		eprintln!("sound file {format}: {path}");
		return Ok(());
	}
	let (player, child) = spawned(&FILE_PLAYERS, format, path, false).ok_or_else(|| format!("no player for {format} found ({}) to play {path}", names_for(&FILE_PLAYERS, format)))?;
	let child: Player = Arc::new(Mutex::new(child));
	let started = Instant::now();
	while started.elapsed() < EARLY_FAILURE {
		if let Some(failure) = failure_of(&child, player, path) {
			return failure;
		}
		std::thread::sleep(PLAYER_POLL);
	}
	let (path, stops) = (path.to_string(), STOPS.load(Ordering::Relaxed));
	std::thread::spawn(move || watched(child, player, &path, stops, "sound file"));
	Ok(())
}

/// The player played to its end, its failure printed unless stop_sound ended it
fn watched(child: Player, player: &str, path: &str, stops: usize, label: &str) {
	if let Ok(mut playing) = PLAYING.lock() {
		playing.push(child.clone());
	}
	let ended = loop {
		match failure_of(&child, player, path) {
			Some(ended) => break ended,
			None => std::thread::sleep(PLAYER_POLL),
		}
	};
	if let (Err(failure), true) = (ended, stops == STOPS.load(Ordering::Relaxed)) {
		eprintln!("{label} {failure}");
	}
	if let Ok(mut playing) = PLAYING.lock() {
		playing.retain(|other| !Arc::ptr_eq(other, &child));
	}
}

/// stop_sound: the queued sounds dropped, the playing sounds and music files stopped
pub fn stop() {
	STOPS.fetch_add(1, Ordering::Relaxed);
	if let Ok(mut end) = SOUNDS_END.lock() {
		*end = None;
	}
	let playing: Vec<Player> = PLAYING.lock().map(|playing| playing.clone()).unwrap_or_default();
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

/// The sound's seconds added to the audio clock behind the sounds queued before it
fn scheduled(seconds: f64) {
	if let Ok(mut end) = SOUNDS_END.lock() {
		let now = Instant::now();
		*end = Some(end.filter(|end| *end > now).unwrap_or(now) + Duration::from_secs_f64(seconds));
	}
}

/// The WAV handed to the player thread, started with the first sound (the process's end then waits for the queue)
fn queued(path: PathBuf) -> Result<(), String> {
	let queue = QUEUE.get_or_init(|| {
		let (sender, receiver) = std::sync::mpsc::channel::<(PathBuf, usize)>();
		std::thread::spawn(move || {
			for (path, stops) in receiver {
				if stops == STOPS.load(Ordering::Relaxed) {
					play_wav(&path, stops);
				}
				PENDING.fetch_sub(1, Ordering::Relaxed);
			}
		});
		wait_at_exit();
		Mutex::new(sender)
	});
	PENDING.fetch_add(1, Ordering::Relaxed);
	let sent = queue.lock().map_err(|_| "sound: the queue is poisoned".to_string())?.send((path, STOPS.load(Ordering::Relaxed)));
	sent.map_err(|_| "sound: the player thread ended".to_string())
}

/// One queued WAV played to its end by the first player the system has
fn play_wav(path: &Path, stops: usize) {
	let shown = path.display().to_string();
	match spawned(&FILE_PLAYERS, "wav", &shown, false) {
		Some((player, child)) => watched(Arc::new(Mutex::new(child)), player, &shown, stops, "sound"),
		None => eprintln!("sound: no player found ({}) for {shown}", names_for(&FILE_PLAYERS, "wav")),
	}
}

fn wait_at_exit() {
	extern "C" fn wait_for_queue() {
		wait();
	}
	extern "C" {
		fn atexit(callback: extern "C" fn()) -> i32;
	}
	// SAFETY: atexit only stores the function pointer; wait reads atomics and sleeps
	unsafe { atexit(wait_for_queue) };
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
	let path = wav_file(&data, rate)?;
	if let Ok(mut unrendered) = UNRENDERED.lock() {
		unrendered.push((rate, data));
	}
	let seconds = samples.len() as f64 / rate.max(1) as f64;
	scheduled(seconds);
	if crate::paint::shows_windows() {
		queued(path.clone())?;
	} else {
		eprintln!("sound {seconds:.2} s: {}", path.display());
	}
	Ok(path)
}

fn wav_file(data: &[u8], rate: u32) -> Result<PathBuf, String> {
	let folder = std::env::temp_dir().join(FOLDER);
	std::fs::create_dir_all(&folder).map_err(|failure| format!("sound: cannot create {}: {failure}", folder.display()))?;
	let call = SOUNDED.fetch_add(1, Ordering::Relaxed) + 1;
	let path = folder.join(if call == 1 { format!("{FILE_STEM}.wav") } else { format!("{FILE_STEM}-{call}.wav") });
	std::fs::write(&path, wav_of_data(data, rate)).map_err(|failure| format!("sound: cannot write {}: {failure}", path.display()))?;
	Ok(path)
}

/// render_sound(path): the sounds since the last render (or the start) one after another in one WAV file, its seconds
pub fn render(path: &str) -> Result<f64, String> {
	let sounds = std::mem::take(&mut *UNRENDERED.lock().map_err(|_| "the rendered sounds are poisoned".to_string())?);
	let rate = sounds.first().map_or(DEFAULT_RATE, |(rate, _)| *rate);
	if let Some((other, _)) = sounds.iter().find(|(sound_rate, _)| *sound_rate != rate) {
		return Err(format!("sounds of {rate} and {other} samples per second cannot share {path}"));
	}
	let data: Vec<u8> = sounds.into_iter().flat_map(|(_, data)| data).collect();
	std::fs::write(path, wav_of_data(&data, rate)).map_err(|failure| format!("cannot write {path}: {failure}"))?;
	Ok(data.len() as f64 / BYTES_PER_SAMPLE as f64 / rate as f64)
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
