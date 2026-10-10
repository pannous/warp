// card winamp-like: spectrum(samples, seconds, bands) gives how loud each frequency band sounds in the samples at that
// moment, from 0 to 1 (bands spaced evenly in pitch from band_low to band_high Hz), computed in warp, so the same
// natively and in the playground; samples/visualizer.warp paints it as bars while the song plays
use crate::is;

const SOUND: &str = "use sound\n";

#[test]
fn a_tone_fills_its_own_band() {
	is!(&format!("{SOUND}bars = spectrum(chord_samples([A4], 0.1, \"sine\"), 0.02, 8)\nbars[nearest_band(A4, 8)] == max(bars)"), true);
	is!(&format!("{SOUND}bars = spectrum(chord_samples([C3], 0.1, \"sine\"), 0.02, 8)\nbars[nearest_band(C3, 8)] == max(bars)"), true);
	is!(&format!("{SOUND}count(spectrum(chord_samples([A4], 0.1, \"sine\"), 0.02, 12))"), 12);
}

#[test]
fn silence_and_the_end_are_empty() {
	is!(&format!("{SOUND}max(spectrum([sample_offset for i in 0..2000], 0, 4)) == 0"), true);
	is!(&format!("{SOUND}max(spectrum(chord_samples([A4], 0.1, \"sine\"), 5, 4)) == 0"), true);
}

#[cfg(feature = "native")]
#[test]
fn the_visualizer_paints_while_it_plays() {
	let sample = concat!(env!("CARGO_MANIFEST_DIR"), "/samples/visualizer.warp");
	let run = crate::common::warp_command().args(["--no-ask", "run", sample]).output().unwrap();
	let errors = String::from_utf8_lossy(&run.stderr);
	assert!(run.status.success(), "{errors}");
	assert!(errors.contains("sound "), "{errors}");
	assert!(errors.matches("paint").count() >= 3 || String::from_utf8_lossy(&run.stdout).matches("paint").count() >= 3, "{errors}");
}
