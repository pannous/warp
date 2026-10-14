// card inside-loop: `at` scheduling works as the one statement of a loop's or an if's body
#![cfg(feature = "native")]
use super::test_sound_voices::rendered;

#[test]
fn at_inside_a_loop_places_each_pass() {
	let (seconds, _) = rendered(
		"at_loop",
		"use sound\ntempo = 60\nfor i in 1 to 3 { at i beats play(C4, 0.1s) }\nrender_sound(\"SONG\")",
	);
	assert!(seconds.starts_with("3.1"), "{seconds}");
}

#[test]
fn at_inside_an_if() {
	let (seconds, _) = rendered("at_if", "use sound\nif yes { at 0 bars play(C4, 0.5s) }\nrender_sound(\"SONG\")");
	assert_eq!(seconds, "0.5");
}
