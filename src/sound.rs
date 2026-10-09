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
/// CoreAudio player, PulseAudio/PipeWire's (libsndfile: no mp3 before 1.1), FFmpeg's and mpv
const FILE_PLAYERS: [(&str, &[&str], &[&str]); 4] = [
	("afplay", &[], &["wav", "mp3", "flac", "aiff", "m4a"]),
	("paplay", &[], &["wav", "ogg", "flac", "aiff"]),
	("ffplay", &["-nodisp", "-autoexit", "-loglevel", "quiet"], &["wav", "mp3", "ogg", "flac", "aiff", "m4a"]),
	("mpv", &["--no-video", "--really-quiet"], &["wav", "mp3", "ogg", "flac", "aiff", "m4a"]),
];
/// The music files playing in the background, stopped by stop_sound
static PLAYING: std::sync::Mutex<Vec<std::process::Child>> = std::sync::Mutex::new(Vec::new());

/// `play "song.mp3"`: a music file played in the background when the user may hear it, else checked and named
pub fn play_file(path: &str) -> Result<(), String> {
	let format = file_format(path)?;
	if !crate::paint::shows_windows() {
		eprintln!("sound file {format}: {path}");
		return Ok(());
	}
	let players = || FILE_PLAYERS.iter().filter(|(_, _, formats)| formats.contains(&format));
	for (player, options, _) in players() {
		let spawned = std::process::Command::new(player).args(*options).arg(path)
			.stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).spawn();
		if let Ok(child) = spawned {
			PLAYING.lock().map_err(|_| "the players' list is poisoned".to_string())?.push(child);
			return Ok(());
		}
	}
	let names: Vec<&str> = players().map(|(player, _, _)| *player).collect();
	Err(format!("no player for {format} found ({}) to play {path}", names.join(", ")))
}

/// Stop the music files playing in the background
pub fn stop_files() {
	if let Ok(mut playing) = PLAYING.lock() {
		for mut child in playing.drain(..) {
			let _ = child.kill();
			let _ = child.wait();
		}
	}
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
