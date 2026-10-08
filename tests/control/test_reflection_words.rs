//! Reflection words as dot forms (card reflection step 1, notes/reflection.md): `f.effects` is `effects of f`,
//! `e.listeners` is `listeners of e`; a real field of that name wins (card listeners-tick: an unknown name is loud)
use crate::common::fails_with;
use crate::is;
use warp::ints;
use warp::Node;

#[test]
fn a_functions_effects_as_a_dot_form() {
	is!("f(x) := print x\nf.effects", Node::Symbol("IO".into()));
}

#[test]
fn an_events_listeners_as_a_dot_form() {
	is!("on alarm {1}; on alarm {2}; alarm.listeners", ints(vec![1, 2]));
	is!("on tick {1}; count tick.listeners", 1);
	is!("x = 0; on set x {print value}; count x.listeners", 1);
}

#[test]
fn a_real_field_named_listeners_wins() {
	is!("p = {listeners: 7}; p.listeners", 7);
}

#[test]
fn listeners_of_a_name_nothing_listens_to_is_loud() {
	fails_with("listeners of alarm", "nothing listens to alarm");
}

// step 2: compile-time objects, the answer read off the class or map literal
const POINT: &str = "class P{x:int; y:int; sq() := x*x+y*y}; p = P(1, 2)";

#[test]
fn an_instances_class_and_type() {
	is!(&format!("{POINT}; p.class"), "P");
	is!(&format!("{POINT}; p.type"), "P");
}

#[test]
fn an_instances_fields_and_their_aliases() {
	is!(&format!("{POINT}; p.fields"), warp::texts(vec!["x", "y"]));
	is!(&format!("{POINT}; p.attributes"), warp::texts(vec!["x", "y"]));
	is!(&format!("{POINT}; p.members"), warp::texts(vec!["x", "y"]));
	is!(&format!("{POINT}; P.fields"), warp::texts(vec!["x", "y"]));
}

#[test]
fn a_classes_methods() {
	is!(&format!("{POINT}; p.methods"), warp::texts(vec!["sq"]));
	is!(&format!("{POINT}; P.methods"), warp::texts(vec!["sq"]));
}

#[test]
fn dir_of_an_instance_a_class_and_a_map() {
	is!(&format!("{POINT}; dir(p)"), warp::texts(vec!["x", "y", "sq"]));
	is!(&format!("{POINT}; dir P"), warp::texts(vec!["x", "y", "sq"]));
	is!("m = {a:1, b:2}; dir(m)", warp::texts(vec!["a", "b"]));
	is!("m = {a:1, b:2}; m.fields", warp::texts(vec!["a", "b"]));
}

#[test]
fn inherited_fields_come_first() {
	is!("class A{x:int}; class B extends A{y:int; f() := y}; b = B(1, 2); b.fields", warp::texts(vec!["x", "y"]));
}

#[test]
fn a_real_field_named_fields_wins() {
	is!("class Form{fields:int}; f = Form(3); f.fields", 3);
}

// step 3: a function's parameters and signature, from its definition
#[test]
fn a_functions_params() {
	is!("f(a:int, b:int) := a+b\nf.params", warp::texts(vec!["a", "b"]));
	is!("def g(x){x*2}\ng.parameters", warp::texts(vec!["x"]));
}

#[test]
fn a_functions_signature() {
	is!("f(a:int, b:int) := a+b\nf.signature", "(a:int, b:int) -> int");
	is!("def greet(name:text){\"hi \" + name}\ngreet.signature", "(name:text) -> text");
}

// step 4: x.doc is x.@comment (under `use comments`), also of a function
#[test]
fn a_bindings_doc() {
	is!("use comments\n// the answer\nx = 42\nx.doc", "the answer");
	is!("use comments\n// doubles\nf(x) := x*2\nf.doc", "doubles");
	is!("use comments\n// doubles\ndef f(x){x*2}\nf.doc", "doubles");
}

#[test]
fn a_real_field_named_doc_wins() {
	is!("use comments\n// a page\np = {doc: 7}\np.doc", 7);
}

// step 4: a core module's exports, read off the module at compile time
#[test]
fn a_modules_exports() {
	is!("import tests/fixtures/wasm/fourty_two; (\"twice\" in fourty_two.exports) > 0", true);
	is!("import tests/fixtures/wasm/fourty_two; (\"ft\" in dir(fourty_two)) > 0", true);
	is!("import tests/fixtures/wasm/fourty_two; fourty_two.exports#1", "add32");
}
