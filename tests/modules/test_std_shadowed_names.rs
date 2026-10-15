//! A function's local named like a module word is the local (card libm-function): math's angle(x, y) beside
//! `angle = 0.5` in a function
use crate::is;

#[test]
fn a_local_named_like_a_module_word_is_the_local() {
	is!("use math; def s(x) { angle = 0.5; cos(angle) }; s(1)", 0.8775825618903728);
	is!("use math; def s() { angle = 0.25; sqrt(angle) }; s()", 0.5);
	// the function angle stays callable beside the local (angle(1, 0) is 0 exactly: the browser's atan2 differs in the
	// last bit elsewhere)
	is!("use math; def s() { angle = 0.5; angle * 2 }; s() + angle(1, 0)", 1.0);
}

#[test]
fn a_local_named_like_a_program_function_is_the_local() {
	is!("f(x) := x + 1; g(n) := { f = [n]; f }; g(3)", warp::warp_parser::parse("[3]"));
}

/// cards local-variable, local-variable-named: a local in one function hides no library or module word in another
#[test]
fn a_local_in_one_function_leaves_the_word_to_the_others() {
	is!("def f() { lower = \"abc\"; lower.length }; def g(w) := w.lower(); g(\"AB\")", "ab");
	is!("f() := { words = \"a b\"; count(words) }; g(t) := words(t); g(\"x y\") == [\"x\", \"y\"]", true);
	is!("def f() { words = \"a b\"; count(words) }; def g(t) := words(t); count(g(\"x y z\")) + f()", 6);
}

/// a local or parameter named like a function of the program is indexed there, not called (`words#1` beside the text
/// module's words, card local-variable-named)
#[test]
fn a_local_named_like_a_function_is_indexed_not_called() {
	is!("def size(t) { 0 }; def collect(p) { size = [p, 2]; size#2 }; collect(3)", 2);
	is!("def size(t) { 0 }; def g(size) { size#2 }; g([4, 5])", 5);
	is!("def padded(t) { pad_left(t, 3) }; def collect(p) { words = [p]; if words.length == 1 { return words#1 }; words }; collect(3)", 3);
	is!("def f(x) { x * 2 }; def g(y) { f -y }; g(3)", -6);
}

/// a parameter named like a standard module's word is the parameter: no module is needed for it (card
/// parameter-module-word)
#[test]
fn a_parameter_named_like_a_module_word_needs_no_module() {
	is!("f(subject, pattern) := subject; f(1, 2)", 1);
	is!("f(x, melody) := x + melody; f(1, 2)", 3);
	is!("f(x, live_loop) := x; f(1, 2)", 1);
	let error = warp::wasm_emitter::eval("f(x) := melody(x); f(1)").serialize();
	assert!(error.contains("use sound"), "{error}");
}

/// card use-sound: a program function named like a module's helper is the program's; the module's own code still calls
/// its own (lexical scope, as for a program variable): mix sums with sound's added, lowpass filters with its biquad
#[test]
fn a_program_function_named_like_a_module_helper_leaves_the_module_its_own() {
	is!("use sound\nadded(x, y) := 7\nmix([0.1], [0.2])#1 ~ 0.3 and added(1, 2) == 7", true);
	is!("use sound\nbiquad(a, k, f, q) := [9]\ncount(osc(C4, 0.01s) |> lowpass(500))", 220);
	is!("use sound\nwave(shape, p) := 0.0\nmax(chord_amplitudes([440], 0.01, \"sine\")) > 0 and wave(1, 2) == 0", true);
}
