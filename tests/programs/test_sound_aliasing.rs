// card reason-beep: a square's and a sawtooth's jumps are band-limited (lib/sound.warp step_smoothing, PolyBLEP). Their
// harmonics above half the sample rate folded back as an inharmonic buzz: beep's 880 Hz square at 22050 samples a second
// had 3.9 % of its energy off its harmonics, a sawtooth 5.4 %; a sine has 0.6 % (its fades), these now 0.7 %
// (probes/sound/beep_spectrum.py)
#![cfg(feature = "native")]
use super::test_sound_voices::rendered;

const RATE: f64 = 22050.0;
const FUNDAMENTAL: f64 = 880.0;
const BIN_HERTZ: usize = 10;
const NEAR_HARMONIC_HERTZ: f64 = 30.0;
const MOST_OFF_HARMONIC_SHARE: f64 = 0.015;

/// The power of the samples at frequency (one bin of a discrete Fourier transform)
fn power_at(samples: &[i16], frequency: f64) -> f64 {
	let turn = -2.0 * std::f64::consts::PI * frequency / RATE;
	let (real, imaginary) = samples.iter().enumerate().fold((0.0, 0.0), |(real, imaginary), (i, &sample)| {
		let angle = turn * i as f64;
		(real + f64::from(sample) * angle.cos(), imaginary + f64::from(sample) * angle.sin())
	});
	real * real + imaginary * imaginary
}

/// The share of the energy at frequencies that are no harmonic of the fundamental
fn off_harmonic_share(samples: &[i16]) -> f64 {
	let spectrum: Vec<(f64, f64)> = (BIN_HERTZ..RATE as usize / 2)
		.step_by(BIN_HERTZ)
		.map(|frequency| (frequency as f64, power_at(samples, frequency as f64)))
		.collect();
	let near_harmonic = |frequency: f64| {
		(frequency - (frequency / FUNDAMENTAL).round() * FUNDAMENTAL).abs() <= NEAR_HARMONIC_HERTZ
			&& frequency > FUNDAMENTAL / 2.0
	};
	let total: f64 = spectrum.iter().map(|(_, power)| power).sum();
	spectrum.iter().filter(|(frequency, _)| !near_harmonic(*frequency)).map(|(_, power)| power).sum::<f64>() / total
}

#[test]
fn square_and_sawtooth_do_not_alias() {
	for (name, sounding) in [("beep", "beep()"), ("sawtooth", "tone(880, 0.15, \"sawtooth\")")] {
		let (_, samples) = rendered(name, &format!("use sound\n{sounding}\nsave sound as \"SONG\""));
		let share = off_harmonic_share(&samples);
		assert!(share < MOST_OFF_HARMONIC_SHARE, "{name}: {:.1} % off the harmonics", share * 100.0);
	}
}
