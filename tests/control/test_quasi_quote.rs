//! Code patterns and templates for passes written in warp (notes/quasi_quote.md): code inside `data(…)` stays as written
//! (card data-pattern-sealed), a hole names a variable even when it is named like a unit (card unit-hole-name), and
//! `$x` in a template splices the value of x (card data-splice)
use crate::is;

const SPLIT: &str = "f(c) := match c {\n\tdata($m + 0) => key(m, \"+\", 1)\n\t_ => c\n}\n";

#[test]
fn quoted_phrases_stay_as_written() {
	is!("data(3 times {print 1}) == parse_code(\"3 times {print 1}\")", true);
	is!("data(a of b) == parse_code(\"a of b\")", true);
	is!("data(set a to 3) == parse_code(\"set a to 3\")", true);
}

#[test]
fn a_hole_is_an_operand_of_a_phrase() {
	is!("match data(3 times {print 1}) {\n\tdata($n times $b) => str(n)\n\t_ => \"no\"\n}", "3");
	is!("match data(3 times {print 1}) {\n\tdata($n times $b) => b == data {print 1}\n\t_ => no\n}", true);
}

#[test]
fn a_hole_named_like_a_unit_is_the_bound_code() {
	is!(&format!("{SPLIT}g(x) := f(x)\ng(data(q + 0)) == data(q + 1)"), true);
	is!("x = data(q + 0)\nx is data($m + 0)\nstr(m)", "q");
	is!("x = data(q + 0)\nx is data($h + 0)\nstr(h)", "q");
}

#[test]
fn a_template_splices_a_variable() {
	is!("a = data(y)\ndata($a + 1) == data(y + 1)", true);
	is!("a = data(y * 2)\ndata(f($a, 3)) == data(f(y * 2, 3))", true);
	is!("n = data(k)\ndata([$n, 2]) == data([k, 2])", true);
}

#[test]
fn a_template_splices_an_expression() {
	is!("k = data(width)\ndata(m.pop($(str(k)))) == data(m.pop(\"width\"))", true);
}

#[test]
fn a_replacement_template_splices_the_holes() {
	is!("(data(y * 2 * 1 + 2) replace data($a * 1) with data($a + 10)) == data(y * 2 + 10 + 2)", true);
}

#[test]
fn a_times_phrase_becomes_its_loop() {
	let rounds = "rounds(c) := match c {\n\tdata($n times $b) => data(for i in 0..$n $b)\n\t_ => c\n}\n";
	is!(&format!("{rounds}rounds(data(3 times {{print 1}})) == data(for i in 0..3 {{print 1}})"), true);
}

#[test]
fn an_arm_template_splices_the_holes() {
	is!("swap(c) := match c {\n\tdata($a - $b) => data($b - $a)\n\t_ => c\n}\nswap(data(x - 1)) == data(1 - x)", true);
}

#[test]
fn a_rest_hole_binds_and_splices_the_remaining_items() {
	let rest = "tail(c) := match c {\n\tdata($f($first, $rest...)) => data($f($rest...))\n\t_ => c\n}\n";
	is!(&format!("{rest}tail(data(g(1, 2, 3))) == data(g(2, 3))"), true);
	is!(&format!("{rest}tail(data(g(1))) == data(g())"), true);
}

#[test]
fn a_template_in_a_lambda_splices_its_parameter() {
	is!(
		"wrap(xs, f) := xs.map(x => data($f($x)))\nwrap([data(a), data(b)], data(g)) == [data(g(a)), data(g(b))]",
		true
	);
}

#[test]
fn a_quoted_literal_is_the_literal() {
	is!("n = data(3)\nn", 3);
	is!("data(3) + 1", 4);
	is!("t = data(\"s\")\nt", "s");
	is!("f(x) := x\nf(data(3))", 3);
}

#[test]
fn the_value_after_an_assignment_takes_the_whole_phrase() {
	is!("f(a) := a\nx = f([1, 2]) where it > 1\nx", warp::ints(vec![2]));
	is!("x = data(y * 1) replace data($a * 1) with a\nx == data y", true);
}

#[test]
fn a_replacement_walks_into_quoted_blocks() {
	is!("data(f {print 1}) replace data($q) with data(7)", 7);
	is!(
		"(data(3 times {print 1}) replace data($n times $b) with data(for i in 0..$n $b)) == data(for i in 0..3 {print 1})",
		true
	);
	is!("m = {a: 1}\nm is map", true);
}
