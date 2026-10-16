//! card lambda-held: a function value held by a field, of an untyped class field or a map entry, is called in every
//! call form: `(h.d)(5)`, `h.d(5)`, `5 |> h.d`, and through a variable `g = h.d; g(5)`
use crate::is;

const HOLDER: &str = "class Holder {d}\nh = Holder(x => x + 1)\n";

#[test]
fn a_lambda_in_an_untyped_class_field_is_called() {
	for call in ["(h.d)(5)", "h.d(5)", "5 |> h.d", "g = h.d\ng(5)"] {
		is!(&format!("{HOLDER}{call}"), 6);
	}
}

#[test]
fn a_lambda_in_a_map_entry_is_called() {
	is!("h = {d: x => x * 3}\n(h.d)(5)", 15);
	is!("h = {d: x => x * 3}\nh.d(5)", 15);
}

/// card untyped-field: a field given a function value is passed as one; a field of another kind still says it needs
/// a function
#[test]
fn a_field_holding_a_lambda_is_passed_as_a_function() {
	let apply = "apply(f, v) := f(v)\n";
	is!(&format!("{apply}{HOLDER}apply(h.d, 5)"), 6);
	is!(&format!("{apply}h = {{d: x => x * 3}}\napply(h.d, 5)"), 15);
	let typed = warp::wasm_emitter::eval(&format!("{apply}class P {{n: int}}\np = P(3)\napply(p.n, 5)")).serialize();
	assert!(typed.contains("apply needs a function for parameter f"), "{typed}");
}

/// reviewer: the holder may arrive any way, the field is called by its declared or inferred kind, not by the path
/// assigned the lambda: a parameter, a call's result, a variable assigned a call, a loop variable, a list item
#[test]
fn a_held_lambda_is_called_whatever_holds_it() {
	for program in [
		"call(k) := k.d(5)\ncall(Holder(x => x + 1))",
		"make() := Holder(x => x + 1)\nmake().d(5)",
		"make() := Holder(x => x + 1)\nh = make()\nh.d(5)",
		"out = 0\nfor h in [Holder(x => x + 1)] { out = h.d(5) }\nout",
	] {
		is!(&format!("class Holder {{d}}\n{program}"), 6);
	}
	is!("hs = [{d: x => x * 3}]\nhs#1.d(5)", 15);
}

/// reviewer: a class field named like a library method leaves that method to other receivers; a field given no
/// function value is no function either
#[test]
fn a_field_named_like_a_library_method_keeps_the_method() {
	is!("class Box {join}\nb = Box(1)\n[1, 2].join(\"-\")", "1-2");
	is!("class Doc {split}\nd = Doc(1)\nwords = \"a b\".split(\" \")\n#words", 2);
	is!("class Item {size}\nx = Item(2)\n\"abc\".size()", 3);
	is!("class Box {join}\nb = Box(x => x + 1)\n[1, 2].join(\"-\")", "1-2");
	is!("class Box {join}\nb = Box(x => x + 1)\nb.join(5)", 6);
}

/// card holder-says with untyped-field: a lambda given by a named argument is called in every form
#[test]
fn a_lambda_given_by_name_is_called() {
	for call in ["(h.d)(5)", "h.d(5)", "5 |> h.d"] {
		is!(&format!("class Holder {{d}}\nh = Holder(d: x => x + 1)\n{call}"), 6);
	}
}
