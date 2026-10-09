//! The text of a value whose kind is known only at run time (card text-arithmetic): a field of an object that mixes kinds
//! is any-typed, and so is arithmetic on it
use warp::wasm_emitter::eval;

const MIXED: &str = "p = {dist: 0, name: \"run\"}; for i in 1..3 { p.dist += 5 }; ";

fn shown(code: &str) -> String {
	eval(&format!("{MIXED}{code}")).serialize().trim().to_string()
}

#[test]
fn test_arithmetic_on_an_any_typed_field_has_a_text() {
	assert_eq!(shown("text_form(p.dist / 2)"), "\"5\"");
	assert_eq!(shown("text_form(p.dist + 1)"), "\"11\"");
	assert_eq!(shown("x = p.dist * 2; \"x=${x}\""), "\"x=20\"");
	assert_eq!(shown("str(p.dist / 4)"), "\"2.5\"");
}

// card todo-app robustness: a field of an any value is what it holds, not what a class's field of that name declares
// (`request.body.priority` of a form is the text "2" though class Todo declares `priority: int`), and int or number
// of it reads the number the text spells
#[test]
fn a_field_of_an_any_value_ignores_a_class_field_of_that_name() {
	let class = "class T{t: text; p: int}; ";
	crate::is!(&format!("{class}f(r: any) := T(r.t, int(r.p)); f({{t: \"a\", p: \"2\"}}).p"), 2);
	crate::is!(&format!("{class}f(r: any) := T(r.t, int(r.p)); f({{t: \"a\", p: \"23\"}}).p"), 23);
	crate::is!("f(r: any) := number(r.p); f({p: \"2\"})", 2);
	crate::is!("f(r: any) := number(r.p); f({p: \"23\"})", 23);
}
