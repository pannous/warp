//! How loud each frequency band of an audio file sounds at a moment (lib/sound.warp `spectrum("song.mp3", seconds,
//! bands)`, decision sound-dsp: the DSP in Rust): the file decoded once and kept, the window of its first channel
//! Hann-weighted, each band's level by Goertzel's recurrence, as lib's spectrum of a list of samples computes it

use std::sync::{Arc, Mutex};

/// The seconds a window spans: lib's spectrum_window of 512 samples at 22050 Hz
const WINDOW_SECONDS: f64 = 512.0 / 22050.0;
const LOUDEST_SAMPLE: f64 = 32767.0;

/// A file's first channel, amplitudes around 0, and its rate
struct Mono {
	rate: f64,
	amplitudes: Vec<f32>,
}

/// The files decoded so far, by path
static DECODED: Mutex<Vec<(String, Arc<Mono>)>> = Mutex::new(Vec::new());

/// The levels (0 to 1, a full-scale sine is 1) of `bands` bands spaced evenly in pitch from low to high Hz
pub fn file_spectrum(path: &str, seconds: f64, bands: usize, low: f64, high: f64) -> Result<Vec<f64>, String> {
	let mono = decoded(path)?;
	let length = (WINDOW_SECONDS * mono.rate).round() as usize;
	let start = ((seconds.max(0.0) * mono.rate).round() as usize).min(mono.amplitudes.len());
	let window = &mono.amplitudes[start..(start + length).min(mono.amplitudes.len())];
	if window.len() < 2 {
		return Ok(vec![0.0; bands]);
	}
	let last = (window.len() - 1) as f64;
	let hann = |at: usize| 0.5 - 0.5 * (std::f64::consts::TAU * at as f64 / last).cos();
	let weighted: Vec<f64> = window.iter().enumerate().map(|(at, amplitude)| *amplitude as f64 * hann(at)).collect();
	let frequency = |band: usize| low * (high / low).powf(band as f64 / (bands.max(2) - 1) as f64);
	Ok((0..bands).map(|band| level(&weighted, frequency(band), mono.rate)).collect())
}

/// How long the file sounds, in seconds
pub fn file_seconds(path: &str) -> Result<f64, String> {
	let mono = decoded(path)?;
	Ok(mono.amplitudes.len() as f64 / mono.rate)
}

/// How loud frequency sounds in the weighted window: one term of the discrete Fourier transform
fn level(weighted: &[f64], frequency: f64, rate: f64) -> f64 {
	let coefficient = 2.0 * (std::f64::consts::TAU * frequency / rate).cos();
	let (mut last, mut before) = (0.0, 0.0);
	for x in weighted {
		(last, before) = (x + coefficient * last - before, last);
	}
	let power = (last * last + before * before - coefficient * last * before).max(0.0);
	4.0 * power.sqrt() / weighted.len() as f64 / LOUDEST_SAMPLE
}

fn decoded(path: &str) -> Result<Arc<Mono>, String> {
	let mut known = DECODED.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
	if let Some((_, mono)) = known.iter().find(|(known_path, _)| known_path == path) {
		return Ok(mono.clone());
	}
	let flat = super::audio_files::load(path)?;
	let (rate, channels) = (flat[0] as f64, flat[1].max(1) as usize);
	let amplitudes = flat[2..].iter().step_by(channels).map(|value| (value - super::SAMPLE_OFFSET) as f32).collect();
	let mono = Arc::new(Mono { rate, amplitudes });
	known.push((path.to_string(), mono.clone()));
	Ok(mono)
}
