// card bizarre-global: no rare global words. `play`, `beep`, `stop_sound` and `wait_sound` play without setup; the
// rare words of the sound module (melody, tone, live_loop, render_sound, midi_send, …) need `use sound` (P171)
#![cfg(feature = "native")]
use crate::common::fails_with;
use crate::is;

#[test]
fn play_and_beep_need_no_use() {
	is!("play(C4, 5ms)\n7", 7);
	is!("beep()\nwait_sound()\n7", 7);
}

#[test]
fn a_rare_sound_word_needs_use_sound() {
	fails_with("melody([C4 E4], 5ms)", "melody is in the standard module sound: write `use sound`");
	fails_with(
		"play(C4, 5ms)\ntone(220Hz, 5ms, \"square\")",
		"tone is in the standard module sound: write `use sound`",
	);
	is!("use sound\nmelody([C4 E4], 5ms)\n7", 7);
}

#[test]
fn an_own_word_named_like_a_rare_sound_word_is_the_program_s() {
	is!("melody(x) := x * 2\nmelody 4", 8);
}
