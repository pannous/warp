//! A name a function's body assigns is its local there, whichever function of that name the program also has: its
//! own, or a used file's (card local-variable-named, found composing lib/compiler's stages into one program)
use crate::is;

#[test]
fn a_local_named_like_a_function_is_the_local() {
	is!("digits(a, b, c, d) := 1; f(t: text) := { digits = t; slice digits 1 ø }; f(\"007\")", "07");
	is!("digits(a, b, c, d) := 1; f(t: text) := { digits = t; digits[1..] }; f(\"007\")", "07");
}

#[test]
fn a_local_named_like_another_used_files_function_is_the_local() {
	is!(
		"use \"tests/fixtures/bound_names/lexing.warp\"; use \"tests/fixtures/bound_names/trimming.warp\"; trimmed(\"007\")",
		"7"
	);
}

#[test]
fn a_parameter_is_not_another_functions_local_of_that_name() {
	is!("f() := { payload = ø; 1 }; g(payload: int) := 2 + payload; f() + g(3)", 6);
}

#[test]
fn a_local_named_like_a_function_heads_a_list_as_its_value() {
	is!("held(a, suffix) := a; f(x) := { held = x; [held, 1] }; f(2)", warp::warp_parser::parse("[2 1]"));
}

/// A function ending in a local named like a function returns the local's value, no function value: a variable it is
/// assigned to elsewhere does not turn a same-named local function into closure calls (card standalone: clauses.warp's
/// `test = test_of(…)` beside emitter.warp's `test = x => …`)
#[test]
fn a_local_named_like_a_function_returned_is_no_function_value() {
	is!("condition(x) := x\nj(cs: list): any := { condition = cs#1; condition }\nt_of(n: int): any := j([n])\nother(n: int): any := { test = t_of(n); test }\nboth(a, b, code) := { test = x => x + code; test(a) + test(b) }\nboth(1, 2, 10)", 23);
}

/// A lambda reads a local of its function also when a function elsewhere has the local's name (card standalone:
/// analyzer.warp's `w => w in numbers` beside lexer.warp's numbers(text))
#[test]
fn a_lambda_captures_a_local_named_like_a_function() {
	is!("numbers(t: text): list := [t]\nwide(words: list): bool := { numbers = {\"int\": 1}; words.all(w => w in numbers) }\nwide([\"int\"])", true);
}

/// A loop variable named like a function is the loop's item: `call#1` reads its first element, no call of call(#1)
#[test]
fn a_loop_variable_named_like_a_function_is_indexed() {
	is!("call(n: int): int := n\nnamed(name: text): text := name\nf(xs: list): text := { out = \"\"; for call in xs { out = named(call#1) }; out }\nf([[\"a\", 1]])", "a");
}

/// A lambda kept in a local of one function is that function's, whatever another body's same-named variable holds
/// (card standalone: emitter.warp's `part = i => …; part(0)` beside `for part in parts` over function values elsewhere)
#[test]
fn a_local_lambda_is_called_beside_a_same_named_function_holder() {
	is!("all_parts(): int := { fs = [x => x * 2]; total = 0; for part in fs { total += part(1) }; total }\nboth(k: int): int := { part = i => i + k; part(0) + part(1) }\nall_parts() + both(10)", 23);
}

/// A local holding a call's value and sliced is the local, beside a `def` of three parameters by that name (card
/// local-variable-named: `items = items_of(n); items[1..]` in a file using parser.warp gave 'slice takes 3 arguments')
#[test]
fn a_local_holding_a_calls_value_is_sliced_beside_a_def_of_its_name() {
	is!("def items(p: list, closer: text, bracket: text): list := p\nitems_of(xs: list): any := xs\nt(xs: list): any := { items = items_of(xs); arguments = items[1..]; #arguments }\nt([3, 9, 4])", 2);
}

/// A function nested by a lambda reads its enclosing body's local, beside a function of that name elsewhere (card
/// standalone: emitter.warp's `held = …; part = i => as_node(held) …` beside multiple_assignments.warp's held(name, suffix)
/// gave 'held needs 2 arguments')
#[test]
fn a_lambda_reads_an_enclosing_local_named_like_a_function() {
	let held = "held(name: text, suffix: text): text := name + suffix\n";
	is!(
		&format!("{held}f(n: int): any := {{ held = [1, 2]; part = i => held; part(n) }}\nf(1)"),
		warp::warp_parser::parse("[1 2]")
	);
	is!(&format!("{held}f(n: int): any := {{ held = [1, 2]; part = i => #held + i; part(n) }}\nf(1)"), 3);
	is!(&format!("{held}f(n: int): any := {{ held = [1, 2]; part = i => held#1 + i; part(n) }}\nf(1)"), 2);
}
