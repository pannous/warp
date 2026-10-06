// Class words of other languages are aliases (user 2026-10-06, the alias mechanism): they work, with a got-it note
// naming the wasp word and an "I meant: <wasp word>" fix
use crate::is;

/// The notes (written → wasp word) a program's compilation gives
fn alias_notes(code: &str) -> Vec<(String, String)> {
	let (_, hints) = warp::normalize::capture_hints(|| warp::wasm_emitter::eval(code));
	hints.iter().filter(|hint| hint.fix().is_some()).map(|hint| (hint.original.clone(), hint.canonical.clone())).collect()
}

fn assert_alias(code: &str, written: &str, wasp_word: &str) {
	let notes = alias_notes(code);
	assert!(notes.contains(&(written.to_string(), wasp_word.to_string())), "{written} → {wasp_word} not in {notes:?}");
}

#[test]
fn foreign_constructor_names_say_value() {
	assert_alias("class P { x=0; constructor(x) { self.x = x } }; P(3).x", "constructor", "value");
	assert_alias("class P:\n    def __init__(self, x):\n        self.x = x\nP(3).x", "__init__", "value");
}

#[test]
fn foreign_operator_methods_say_the_wasp_name() {
	assert_alias("class V{x:int; __add__(o) := V(x + o.x)}; (V(1) + V(2)).x", "__add__", "plus");
	assert_alias("class V{x:int; add(o) := V(x + o.x)}; (V(1) + V(2)).x", "add", "plus");
}

#[test]
fn class_modifiers_say_class() {
	assert_alias("data class P(val x: Int)\nP(1).x", "data class", "class");
	assert_alias("open class P { x = 1 }\nP().x", "open class", "class");
}

#[test]
fn member_modifiers_and_field_keywords_are_dropped() {
	assert_alias("data class V(val x: Int) { operator fun plus(o: V) = V(x + o.x) }\n(V(1) + V(2)).x", "operator fun", "fun");
	assert_alias("data class V(val x: Int)\nV(1).x", "val x", "x");
	assert_alias("struct C { var count = 0; mutating func up() { count += 1 } }; c = C(); c.up(); c.count", "var count", "count");
	assert_alias("struct C { var count = 0; mutating func up() { count += 1 } }; c = C(); c.up(); c.count", "mutating func", "func");
}

#[test]
fn the_wasp_words_give_no_note() {
	let code = "class V{x:int; plus(o) := V(x + o.x)}; (V(1) + V(2)).x";
	assert_eq!(alias_notes(code), vec![]);
	is!(code, 3);
}
