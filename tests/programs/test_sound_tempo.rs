// card sound-pro, layer 1 (clocked playback): tempo as units. In a program that uses sound `1/4 beat`, `2 beats`,
// `1 bar` are seconds at the tempo (beats per minute, 120 unless the program sets it), 4 beats to the bar
#![cfg(feature = "native")]
use crate::is;

#[test]
fn a_beat_is_a_share_of_a_minute() {
	is!("use sound\nx = 1/4 beat\nx", 0.125);
	is!("use sound\n2 beats", 1.0);
	is!("use sound\nx = 2 bars\nx", 4.0);
	is!("use sound\nbeat_seconds(1) + bar_seconds(1)", 2.5); // the words they lower to
}

#[test]
fn the_tempo_sets_the_beat() {
	is!("use sound\nglobal tempo = 90\nplay(C4, 5ms)\n3 beats", 2.0);
	is!("use sound\nglobal tempo = 240\nplay(C4, 1/8 beat)\nsound_queued() <= 1/32", true);
}

#[test]
fn notes_last_beats() {
	is!("use sound\nmelody([C4 E4], 1/8 beat)\n7", 7);
	is!("use sound\nplay([C4 E4], 1/16 beat)\n7", 7);
	is!("use sound\nf(seconds) := seconds * 2\nplay(C4, 5ms)\nf(1 beat)", 1.0);
}

/// a module's `global` is a setting: the program's top-level `tempo = 60` sets it, no `global` needed
#[test]
fn the_program_sets_the_modules_settings() {
	is!("use sound\ntempo = 60\nplay(C4, 5ms)\n2 beats", 2.0);
	is!("use sound\nnote_seconds = 0.01\nplay C4\nsound_queued() <= 0.01", true);
}
