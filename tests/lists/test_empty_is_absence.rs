//! Card empty-is-absence (decision empty-is-not-missing): ø is absence; an empty list, text or map is a value of its own
//! kind, unequal to ø, and `x is ø` asks whether x is missing. Emptiness is asked with `is empty`, `if no x`, `if not x`
use crate::is;

#[test]
fn an_empty_value_is_not_missing() {
	is!("[] == ø", false);
	is!("\"\" == ø", false);
	is!("[] != ø", true);
	is!("x = []; x == ø", false);
	is!("f(a: any) := a == ø; f([])", false);
	is!("ø == ø", true);
}

#[test]
fn is_nothing_means_missing() {
	is!("[] is ø", false);
	is!("f(a: any) := a is ø; f([])", false);
	is!("f(a: any) := a is ø; f(ø)", true);
	is!("ø is ø", true);
}

#[test]
fn a_list_holding_an_empty_list_holds_no_nothing() {
	is!("[[]].contains(ø)", false);
	is!("xs = [[] 1]; ø in xs", false);
	is!("[ø].contains(ø)", true);
}

#[test]
#[ignore = "card empty-map-literal: `{}` in code is still ø"]
fn braces_in_code_are_an_empty_map() {
	is!("{} == ø", false);
	is!("x = {}; x == ø", false);
	is!("f(a: any) := a == ø; f({})", false);
	is!("type({})", "map");
	is!("x = {}; type(x)", "map");
	is!("f(a: any) := a; type(f({}))", "map");
}

#[test]
fn emptiness_is_asked_loosely() {
	is!("xs = []; xs is empty", true);
	is!("xs = []; if no xs then 1 else 2", 1);
	is!("xs = [1]; if no xs then 1 else 2", 2);
	is!("t = \"\"; if not t then 1 else 2", 1);
	is!("m = {}; m is empty", true);
	is!("xs = [1]; if xs then 1 else 2", 1);
}

#[test]
fn the_empty_set_sign_is_nothing() {
	is!("∅ == ø", true);
	is!("∅ is ø", true);
}
