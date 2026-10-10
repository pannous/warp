// card sound-pro, layer 3 (synthesis): an ADSR envelope (attack, decay, sustain, release in seconds and a share),
// a noise wave, gain in decibels (`loudness = -6 dB`)
#![cfg(feature = "native")]
use crate::is;

#[test]
fn the_envelope_rises_decays_sustains_and_releases() {
	let adsr = "use sound\nattack = 0.01\ndecay = 0.01\nsustain = 0.5\nrelease = 0.1\n";
	is!(&format!("{adsr}round(envelope(110, 22050) * 100)"), 50); // half way up the attack
	is!(&format!("{adsr}round(envelope(330, 22050) * 100)"), 75); // half way down the decay
	is!(&format!("{adsr}round(envelope(2000, 22050) * 100)"), 50); // the sustain
	is!(&format!("{adsr}round(envelope(22050 - 1 - 1102, 22050) * 100)"), 25); // half way through the release
	is!("use sound\nenvelope(0, 100) + envelope(99, 100) == 0", true); // no click at either end
}

#[test]
fn noise_is_a_wave() {
	is!("use sound\nxs = [wave(\"noise\", 0.25) for i in 0..100]\nmin(xs) >= -1 and max(xs) <= 1 and min(xs) < max(xs)", true);
	is!("wave_shape = \"noise\"\nplay C4 for 5ms\n7", 7);
}

#[test]
fn gain_in_decibels() {
	is!("use sound\nx = -6 dB\nround(x * 1000)", 501);
	is!("use sound\nround(-6dB * 1000)", 501);
	is!("loudness = -20 dB\nplay C4 for 5ms\nloudness", 0.1);
}
