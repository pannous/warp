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

/// P162: `init` is the constructor; every common constructor name works as its alias
#[test]
fn every_common_constructor_name_says_init() {
	let aliases = [
		("value", "class P { x=0; value(x) { self.x = x } }; P(3).x"),
		("constructor", "class P { x=0; constructor(x) { self.x = x } }; P(3).x"),
		("__init__", "class P:\n    def __init__(self, x):\n        self.x = x\nP(3).x"),
		("initialize", "class P { x=0; def initialize(x) { self.x = x } }; P(3).x"),
		("__construct", "class P { x=0; function __construct(x) { self.x = x } }; P(3).x"),
		("New", "class P { x=0; New(x) { self.x = x } }; P(3).x"),
		("Create", "class P { x=0; Create(x) { self.x = x } }; P(3).x"),
		("new", "class P { x=0; fn new(x) { self.x = x } }; P(3).x"),
		("P", "class P { int x; P(int x) { this.x = x; } }; P(3).x"),
	];
	for (alias, code) in aliases {
		assert_alias(code, alias, "init");
		is!(code, 3);
	}
	is!("class P { x=0; init(x) { self.x = x } }; P(3).x", 3);
	assert_eq!(alias_notes("class P { x=0; init(x) { self.x = x } }; P(3).x"), vec![]);
}

/// A name the program also calls as a method stays a method: `p.new(2)` is no construction
#[test]
fn a_constructor_alias_called_as_a_method_stays_a_method() {
	is!("class P { x=1; new(k) := P(x + k) }; p = P(1); p.new(2).x", 3);
	is!("class P { x=1; Create(k) := P(x * k) }; P(2).Create(3).x", 6);
}

/// `new Point(1, 2)` builds what `Point(1, 2)` builds, with a note that new is superfluous
#[test]
fn new_before_a_construction_is_superfluous() {
	let code = "class Point { x=0; y=0 }; p = new Point(1, 2); p.y";
	is!(code, 2);
	assert_alias(code, "new Point", "Point");
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

/// P167: positional braces `Point{1, 2}` (Go) of a known class build `Point(1, 2)`, with a note
#[test]
fn positional_braces_build_a_known_class() {
	is!("type Point struct {\n    X int\n    Y int\n}\nPoint{1, 2}.Y", 2);
	let code = "class P{x:int; y:int}; P{3, 4}.x";
	is!(code, 3);
	assert_alias(code, "P{3, 4}", "P(3, 4)");
}

/// P157: a ported type parameter `class Box<T>` compiles untyped, with a note that wasp infers types
#[test]
fn a_type_parameter_of_a_class_is_noted() {
	let code = "class Box<T> { item: T }\nBox(3).item";
	is!(code, 3);
	assert_alias(code, "Box<T>", "Box");
}
