// footgun-pi: `pi = 4; pi` gave 3.14159…: the parser read pi as π and the assignment was dropped. The intent is clear, so
// it compiles: from its assignment on, pi is the variable, with a note that it shadows the constant π; a field named pi
// is a plain field
use crate::is;
use warp::normalize::capture_hints;

const SHADOWING_NOTE: &str = "shadows the constant";

fn shadowing_notes(code: &str) -> usize {
	capture_hints(|| warp::wasm_emitter::eval(code)).1.iter().filter(|hint| hint.reason.contains(SHADOWING_NOTE)).count()
}

#[test]
fn an_assigned_constant_word_is_a_variable() {
	is!("pi = 4; pi", 4);
	is!("pi = 4; pi * 2", 8);
	is!("tau = 1; tau + 1", 2);
	is!("class circle{pi = 3}; c = circle{}; c.pi", 3);
	is!("x = pi; x > 3.14 and x < 3.15", true);
}

#[test]
fn assigning_a_constant_word_says_it_shadows_the_constant() {
	assert_eq!(shadowing_notes("pi = 4; pi"), 1);
	assert_eq!(shadowing_notes("r = 2; r * pi"), 0);
}
