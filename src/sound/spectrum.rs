//! How loud each frequency band sounds in a window of samples (lib/sound.warp `spectrum`, card winamp-like; decision
//! sound-dsp: the DSP in Rust): the window Hann-weighted, each band's level by Goertzel's recurrence. One function for
//! a list of samples and an audio file, natively (src/sound/adapters.rs) and in the playground (src/web/mod.rs
//! web_spectrum)

const LOUDEST_SAMPLE: f64 = 32767.0;

/// The levels (0 to 1, a full-scale sine is 1) of `bands` bands spaced evenly in pitch from low to high Hz in the
/// window's amplitudes (around 0, full scale 32767) at `rate`; all 0 for fewer than 2 amplitudes
pub fn window_levels(window: &[f64], rate: f64, bands: usize, low: f64, high: f64) -> Vec<f64> {
	if window.len() < 2 {
		return vec![0.0; bands];
	}
	let last = (window.len() - 1) as f64;
	// Hann-weighted, so a tone between two bands does not leak into the far ones
	let hann = |at: usize| 0.5 - 0.5 * (std::f64::consts::TAU * at as f64 / last).cos();
	let weighted: Vec<f64> = window.iter().enumerate().map(|(at, amplitude)| amplitude * hann(at)).collect();
	let frequency = |band: usize| low * (high / low).powf(band as f64 / (bands.max(2) - 1) as f64);
	(0..bands).map(|band| level(&weighted, frequency(band), rate)).collect()
}

/// The window of `window_seconds` starting `seconds` into amplitudes at `rate`, as window_levels takes it
pub fn window_at(amplitudes: &[f32], rate: f64, seconds: f64, window_seconds: f64) -> Vec<f64> {
	let start = ((seconds.max(0.0) * rate).round() as usize).min(amplitudes.len());
	let end = (start + (window_seconds * rate).round() as usize).min(amplitudes.len());
	amplitudes[start..end].iter().map(|&amplitude| amplitude as f64).collect()
}

/// How loud frequency sounds in the weighted window: the size of one term of the discrete Fourier transform, by
/// Goertzel's recurrence (one multiply and add a sample, no sine or cosine in the loop)
fn level(weighted: &[f64], frequency: f64, rate: f64) -> f64 {
	let coefficient = 2.0 * (std::f64::consts::TAU * frequency / rate).cos();
	let (mut last, mut before) = (0.0, 0.0);
	for x in weighted {
		(last, before) = (x + coefficient * last - before, last);
	}
	let power = (last * last + before * before - coefficient * last * before).max(0.0);
	4.0 * power.sqrt() / weighted.len() as f64 / LOUDEST_SAMPLE
}
