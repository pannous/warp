//! sound_samples(samples, count, rate) natively (card basic-sound, lib/sound.warp): 16-bit mono samples as a WAV file in the
//! system's temporary folder (warp-sound/sound.wav, then sound-2.wav … within one run, as paint's PNGs), played by the
//! system's player when the warp binary may reach the user (paint::shows_windows: never under WARP_NO_WINDOW, CI or
//! tests, which only get the file). The playground plays the same samples with WebAudio (host.js sound_samples).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

/// The samples cross as whole numbers ≥ 0: amplitude + 32768 (lib/sound.warp sample_offset)
pub const SAMPLE_OFFSET: i64 = 32_768;
const FOLDER: &str = "warp-sound";
const FILE_STEM: &str = "sound";
/// macOS, then PulseAudio and ALSA on Linux
const PLAYERS: [&str; 3] = ["afplay", "paplay", "aplay"];
const PCM_FORMAT: u16 = 1;
const CHANNELS: u16 = 1;
const BITS_PER_SAMPLE: u16 = 16;
const BYTES_PER_SAMPLE: u32 = (BITS_PER_SAMPLE / 8) as u32;
const FORMAT_CHUNK_SIZE: u32 = 16;
const HEADER_SIZE_AFTER_RIFF: u32 = 36;
/// The sound calls of this run so far
static SOUNDED: AtomicUsize = AtomicUsize::new(0);
/// The music files' formats by their first bytes: (format, magic bytes, offset of the magic)
const FILE_FORMATS: [(&str, &[u8], usize); 7] = [
	("wav", b"WAVE", 8), ("mp3", b"ID3", 0), ("mp3", b"\xFF\xFB", 0), ("ogg", b"OggS", 0), ("flac", b"fLaC", 0), ("aiff", b"AIFF", 8), ("m4a", b"ftyp", 4),
];
/// The players a music file plays by in the background, the first the system has that knows its format: macOS's
/// CoreAudio player, PulseAudio/PipeWire's (libsndfile: no mp3 before 1.1), FFmpeg's and mpv; each says why it fails
const FILE_PLAYERS: [(&str, &[&str], &[&str]); 4] = [
	("afplay", &[], &["wav", "mp3", "flac", "aiff", "m4a"]),
	("paplay", &[], &["wav", "ogg", "flac", "aiff"]),
	("ffplay", &["-nodisp", "-autoexit", "-loglevel", "error"], &["wav", "mp3", "ogg", "flac", "aiff", "m4a"]),
	("mpv", &["--no-video", "--msg-level=all=error"], &["wav", "mp3", "ogg", "flac", "aiff", "m4a"]),
];
/// The decoders that read a music file without playing it, the first the system has that knows its format
const FILE_PROBES: [(&str, &[&str], &[&str]); 2] = [
	("ffprobe", &["-v", "error"], &["wav", "mp3", "ogg", "flac", "aiff", "m4a"]),
	("afinfo", &[], &["wav", "mp3", "flac", "aiff", "m4a"]),
];
/// A player failing at once (a file it cannot decode) fails play itself; a later failure is printed when it happens
const EARLY_FAILURE: std::time::Duration = std::time::Duration::from_millis(300);
const PLAYER_POLL: std::time::Duration = std::time::Duration::from_millis(50);
/// The last characters of a failing player's error output that its report shows
const FAILURE_TAIL: usize = 300;
type Player = std::sync::Arc<std::sync::Mutex<std::process::Child>>;
/// The music files playing in the background, stopped by stop_sound
static PLAYING: std::sync::Mutex<Vec<Player>> = std::sync::Mutex::new(Vec::new());

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
	let child: Player = std::sync::Arc::new(std::sync::Mutex::new(child));
	let started = std::time::Instant::now();
	while started.elapsed() < EARLY_FAILURE {
		if let Some(failure) = failure_of(&child, player, path) {
			return failure;
		}
		std::thread::sleep(PLAYER_POLL);
	}
	PLAYING.lock().map_err(|_| "the players' list is poisoned".to_string())?.push(child.clone());
	let path = path.to_string();
	std::thread::spawn(move || loop {
		match failure_of(&child, player, &path) {
			Some(Err(failure)) => return eprintln!("sound file {failure}"),
			Some(Ok(())) => return,
			None => std::thread::sleep(PLAYER_POLL),
		}
	});
	Ok(())
}

/// Stop the music files playing in the background
pub fn stop_files() {
	if let Ok(mut playing) = PLAYING.lock() {
		for child in playing.drain(..) {
			if let Ok(mut child) = child.lock() {
				let _ = child.kill();
				let _ = child.wait();
			}
		}
	}
}

/// Whether the system has a decoder that checks a file of this format without playing it
pub fn can_probe(format: &str) -> bool {
	FILE_PROBES.iter().any(|(probe, _, formats)| formats.contains(&format) && which(probe))
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
fn spawned(commands: &[(&'static str, &[&str], &[&str])], format: &str, path: &str, keep_output: bool) -> Option<(&'static str, std::process::Child)> {
	let output = || if keep_output { std::process::Stdio::piped() } else { std::process::Stdio::null() };
	commands.iter().filter(|(_, _, formats)| formats.contains(&format)).find_map(|(command, options, _)| {
		let child = std::process::Command::new(command).args(*options).arg(path).stdin(std::process::Stdio::null())
			.stdout(output()).stderr(std::process::Stdio::piped()).spawn().ok()?;
		Some((*command, child))
	})
}

fn names_for(commands: &[(&str, &[&str], &[&str])], format: &str) -> String {
	commands.iter().filter(|(_, _, formats)| formats.contains(&format)).map(|(command, _, _)| *command).collect::<Vec<_>>().join(", ")
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

/// Write the samples as a WAV and play it when the user may hear it, else say where it is; played to its end, so
/// sounds one after another play in order
pub fn sound(samples: &[u64], rate: u32) -> Result<PathBuf, String> {
	let path = wav_file(samples, rate)?;
	if crate::paint::shows_windows() {
		play(&path)?;
	} else {
		eprintln!("sound {:.2} s: {}", samples.len() as f64 / rate.max(1) as f64, path.display());
	}
	Ok(path)
}

fn wav_file(samples: &[u64], rate: u32) -> Result<PathBuf, String> {
	let folder = std::env::temp_dir().join(FOLDER);
	std::fs::create_dir_all(&folder).map_err(|failure| format!("sound: cannot create {}: {failure}", folder.display()))?;
	let call = SOUNDED.fetch_add(1, Ordering::Relaxed) + 1;
	let path = folder.join(if call == 1 { format!("{FILE_STEM}.wav") } else { format!("{FILE_STEM}-{call}.wav") });
	std::fs::write(&path, wav(samples, rate)).map_err(|failure| format!("sound: cannot write {}: {failure}", path.display()))?;
	Ok(path)
}

/// The first player the system has plays the file to its end
fn play(path: &Path) -> Result<(), String> {
	for player in PLAYERS {
		if let Ok(status) = std::process::Command::new(player).arg(path).status() {
			return if status.success() { Ok(()) } else { Err(format!("sound: {player} could not play {}", path.display())) };
		}
	}
	Err(format!("sound: no player found ({}) for {}", PLAYERS.join(", "), path.display()))
}

/// A 16-bit mono PCM WAV of the offset samples
pub fn wav(samples: &[u64], rate: u32) -> Vec<u8> {
	let data: Vec<u8> = samples.iter().flat_map(|&sample| ((sample as i64 - SAMPLE_OFFSET).clamp(i16::MIN as i64, i16::MAX as i64) as i16).to_le_bytes()).collect();
	let format = [
		PCM_FORMAT.to_le_bytes().as_slice(), &CHANNELS.to_le_bytes(), &rate.to_le_bytes(), &(rate * BYTES_PER_SAMPLE * CHANNELS as u32).to_le_bytes(),
		&(BYTES_PER_SAMPLE as u16 * CHANNELS).to_le_bytes(), &BITS_PER_SAMPLE.to_le_bytes(),
	].concat();
	[
		b"RIFF".as_slice(), &(HEADER_SIZE_AFTER_RIFF + data.len() as u32).to_le_bytes(), b"WAVE",
		b"fmt ", &FORMAT_CHUNK_SIZE.to_le_bytes(), &format,
		b"data", &(data.len() as u32).to_le_bytes(), &data,
	].concat()
}
