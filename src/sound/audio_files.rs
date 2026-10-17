//! Audio files read as samples (card sound-samples, lib/sound.warp `sample(path)`): a WAV is read here, any other
//! format (mp3, ogg, flac, aiff, m4a) is first converted to a 16-bit WAV by the first converter the system has that
//! knows it. The frames cross to the program as whole numbers ≥ 0 (amplitude + 32768, as sound_samples takes them),
//! channels interleaved, after the rate and the channel count.

use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

const RIFF_HEADER: usize = 12;
const CHUNK_HEADER: usize = 8;
const PCM: u16 = 1;
const IEEE_FLOAT: u16 = 3;
/// WAVE_FORMAT_EXTENSIBLE: the real format is the first two bytes of the sub-format GUID
const EXTENSIBLE: u16 = 0xFFFE;
const EXTENSIBLE_SUBFORMAT_AT: usize = 24;
/// A converter, its arguments around the input and output paths, and the formats it reads
type Converter = (&'static str, &'static [&'static str], &'static [&'static str], &'static [&'static str]);
/// macOS's afconvert, then FFmpeg
const CONVERTERS: [Converter; 2] = [
	("afconvert", &[], &["-f", "WAVE", "-d", "LEI16"], &["mp3", "flac", "aiff", "m4a"]),
	(
		"ffmpeg",
		&["-v", "error", "-y", "-i"],
		&["-f", "wav", "-acodec", "pcm_s16le"],
		&["mp3", "ogg", "flac", "aiff", "m4a"],
	),
];

/// The conversions of this process so far, one scratch WAV name each
static CONVERSIONS: AtomicUsize = AtomicUsize::new(0);

/// The file's rate, channels and offset frames, flat: [rate, channels, frame values …]
pub fn load(path: &str) -> Result<Vec<i64>, String> {
	let format = super::file_format(path)?;
	let bytes = if format == "wav" {
		std::fs::read(path).map_err(|failure| format!("cannot read {path}: {failure}"))?
	} else {
		converted(path, format)?
	};
	let (rate, channels, amplitudes) = decoded(&bytes).map_err(|problem| format!("{path}: {problem}"))?;
	let offset = amplitudes.into_iter().map(|amplitude| amplitude as i64 + super::SAMPLE_OFFSET);
	Ok([rate as i64, channels as i64].into_iter().chain(offset).collect())
}

/// A file's first channel, amplitudes around 0, and its rate
pub struct Mono {
	pub rate: f64,
	pub amplitudes: Vec<f32>,
}

/// A decoded file: its path, its modification time then, its first channel
type Decoded = (String, Option<SystemTime>, Arc<Mono>);
/// The files decoded so far: a file written again is decoded again
static DECODED: Mutex<Vec<Decoded>> = Mutex::new(Vec::new());

/// The file's first channel, decoded once per version of it (lib/sound.warp spectrum of a file, sound_seconds)
pub fn mono(path: &str) -> Result<Arc<Mono>, String> {
	let modified = std::fs::metadata(path).and_then(|file| file.modified()).ok();
	let mut known = DECODED.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
	if let Some((.., mono)) = known.iter().find(|(known_path, when, _)| known_path == path && *when == modified) {
		return Ok(mono.clone());
	}
	let flat = load(path)?;
	let (rate, channels) = (flat[0] as f64, flat[1].max(1) as usize);
	let amplitudes = flat[2..].iter().step_by(channels).map(|value| (value - super::SAMPLE_OFFSET) as f32).collect();
	let mono = Arc::new(Mono { rate, amplitudes });
	known.retain(|(known_path, ..)| known_path != path);
	known.push((path.to_string(), modified, mono.clone()));
	Ok(mono)
}

/// The file as a 16-bit WAV by the first converter that knows its format
fn converted(path: &str, format: &str) -> Result<Vec<u8>, String> {
	let knowing: Vec<&Converter> = CONVERTERS.iter().filter(|(_, _, _, formats)| formats.contains(&format)).collect();
	let Some((command, before, after, _)) = knowing.iter().find(|(command, ..)| super::which(command)) else {
		let names: Vec<&str> = knowing.iter().map(|(command, ..)| *command).collect();
		return Err(format!("sample {path}: no converter for {format} found (install one of: {})", names.join(", ")));
	};
	crate::host::sandbox::check_process("sound.load")?;
	let wav = std::env::temp_dir().join(format!(
		"warp-sample-{}-{}.wav",
		std::process::id(),
		CONVERSIONS.fetch_add(1, Ordering::Relaxed)
	));
	let output = Command::new(command)
		.args(*before)
		.arg(path)
		.args(*after)
		.arg(&wav)
		.output()
		.map_err(|failure| format!("sample {path}: {command} could not run ({failure})"))?;
	let bytes = std::fs::read(&wav);
	let _ = std::fs::remove_file(&wav);
	if !output.status.success() {
		return Err(format!(
			"sample {path}: {command} failed ({}): {}",
			output.status,
			String::from_utf8_lossy(&output.stderr).trim()
		));
	}
	bytes.map_err(|failure| format!("sample {path}: {command} wrote nothing ({failure})"))
}

/// A WAV's rate, channels and 16-bit amplitudes (8, 16, 24, 32-bit integer and 32, 64-bit float data)
pub fn decoded(bytes: &[u8]) -> Result<(u32, u16, Vec<i16>), String> {
	if bytes.len() < RIFF_HEADER || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
		return Err("not a WAV file".into());
	}
	let mut format = None;
	let mut at = RIFF_HEADER;
	while at + CHUNK_HEADER <= bytes.len() {
		let id = &bytes[at..at + 4];
		let size = u32_at(bytes, at + 4) as usize;
		let body = &bytes[at + CHUNK_HEADER..(at + CHUNK_HEADER + size).min(bytes.len())];
		match id {
			b"fmt " if body.len() >= 16 => format = Some(body),
			b"data" => {
				let format = format.ok_or("its data comes before its format")?;
				return amplitudes(format, body);
			}
			_ => {}
		}
		at += CHUNK_HEADER + size + size % 2; // chunks are padded to an even size
	}
	Err("no audio data".into())
}

fn amplitudes(format: &[u8], data: &[u8]) -> Result<(u32, u16, Vec<i16>), String> {
	let mut tag = u16_at(format, 0);
	if tag == EXTENSIBLE && format.len() >= EXTENSIBLE_SUBFORMAT_AT + 2 {
		tag = u16_at(format, EXTENSIBLE_SUBFORMAT_AT);
	}
	let channels = u16_at(format, 2).max(1);
	let rate = u32_at(format, 4);
	let bits = u16_at(format, 14);
	let width = (bits as usize).div_ceil(8);
	let amplitude: fn(&[u8]) -> i16 = match (tag, bits) {
		(PCM, 8) => |b| ((b[0] as i16) - 128) << 8,
		(PCM, 16) => |b| i16::from_le_bytes([b[0], b[1]]),
		(PCM, 24) => |b| i16::from_le_bytes([b[1], b[2]]),
		(PCM, 32) => |b| i16::from_le_bytes([b[2], b[3]]),
		(IEEE_FLOAT, 32) => |b| super::signal_pcm(f32::from_le_bytes([b[0], b[1], b[2], b[3]]) as f64),
		(IEEE_FLOAT, 64) => |b| super::signal_pcm(f64::from_le_bytes(b[0..8].try_into().unwrap_or_default())),
		_ => {
			return Err(format!(
				"its format {tag} with {bits} bits is not supported (PCM 8, 16, 24, 32 or float 32, 64)"
			))
		}
	};
	Ok((rate, channels, data.chunks_exact(width).map(amplitude).collect()))
}

fn u16_at(bytes: &[u8], at: usize) -> u16 {
	u16::from_le_bytes([bytes[at], bytes[at + 1]])
}

fn u32_at(bytes: &[u8], at: usize) -> u32 {
	u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}
