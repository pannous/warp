// card sound-samples: audio files read as samples (lib/sound.warp sample, src/sound/audio_files.rs), played as recorded,
// at a rate, sliced and looped; checked by rendering, so silently
#![cfg(feature = "native")]
use super::test_sound_voices::rendered;
use std::path::{Path, PathBuf};
use std::process::Command;

const RATE: usize = 22050;

/// What the program prints, run silently
fn value_of(program: &str) -> String {
	let run = crate::common::warp_command().args(["--no-ask", "eval", program]).output().unwrap();
	String::from_utf8_lossy(&run.stdout).trim().trim_start_matches("» ").to_string()
}

/// A WAV of half a second of 440 Hz, written by warp's own render_sound
fn tone_file(name: &str) -> PathBuf {
	let path = std::env::temp_dir().join(format!("warp-sample-{name}-{}.wav", std::process::id()));
	assert_eq!(value_of(&format!("use sound\ntone(440Hz, 0.5s)\nsave sound as \"{}\"", path.display())), "0.5");
	path
}

fn of_sample(path: &Path, program: &str) -> String {
	format!("use sound\nkick = sample(\"{}\")\n{program}", path.display())
}

#[test]
fn a_wav_plays_back_as_recorded() {
	let path = tone_file("recorded");
	let (_, original) = rendered("original", "use sound\ntone(440Hz, 0.5s)\nsave sound as \"SONG\"");
	let (seconds, played) = rendered("played", &of_sample(&path, "play(kick)\nsave sound as \"SONG\""));
	assert_eq!(seconds, "0.5");
	assert_eq!(played, original);
	assert_eq!(value_of(&of_sample(&path, "[kick.rate, kick.channels, sample_seconds(kick)]")), "[22050 1 0.5]");
}

#[test]
fn a_rate_changes_speed_and_pitch() {
	let path = tone_file("rate");
	let (seconds, samples) = rendered("rated", &of_sample(&path, "play_sample(kick, 2)\nsave sound as \"SONG\""));
	assert_eq!(seconds, "0.25");
	assert_eq!(samples.len(), RATE * 2 / 4, "rendered at the sound's own rate, the highest");
}

#[test]
fn a_render_mixes_rates() {
	let path = tone_file("mixed");
	let (seconds, samples) = rendered(
		"mixed",
		&of_sample(
			&path,
			"play_sample(kick, 2)
tone(440Hz, 0.5s)
save sound as \"SONG\"",
		),
	);
	assert_eq!(seconds, "0.75");
	assert_eq!(samples.len(), RATE * 2 * 3 / 4, "the 22050 Hz tone converted to 44100 Hz");
}

#[test]
fn slices_and_loops() {
	let path = tone_file("slices");
	let program = "[sample_seconds(sample_slice(kick, 0.1s, 0.3s)), count(sample_slices(kick, 5)), sample_seconds(sample_slices(kick, 5)[4]), sample_seconds(sample_looped(kick, 2s, 0.25s, 0.5s))]";
	assert_eq!(value_of(&of_sample(&path, program)), "[0.2 5 0.1 2]");
}

#[test]
fn other_formats_go_through_a_converter() {
	let wav = tone_file("converted");
	let aiff = wav.with_extension("aiff");
	let converted = Command::new("afconvert")
		.args(["-f", "AIFF", "-d", "BEI24"])
		.arg(&wav)
		.arg(&aiff)
		.status()
		.or_else(|_| Command::new("ffmpeg").args(["-v", "error", "-y", "-i"]).arg(&wav).arg(&aiff).status());
	if !converted.is_ok_and(|status| status.success()) {
		eprintln!("neither afconvert nor ffmpeg: skipped");
		return;
	}
	assert_eq!(value_of(&of_sample(&aiff, "[kick.rate, kick.channels, sample_seconds(kick)]")), "[22050 1 0.5]");
}

#[test]
fn not_audio_fails_loudly() {
	let path = std::env::temp_dir().join(format!("warp-sample-text-{}.txt", std::process::id()));
	std::fs::write(&path, "no sound here").unwrap();
	let said = value_of(&of_sample(&path, "kick"));
	assert!(said.contains("is not a sound file"), "{said}");
}
