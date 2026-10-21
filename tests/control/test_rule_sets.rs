//! `simplify = rules { P => Q; … }` is a rule set as a value: `simplify(code)` rewrites at any depth, children first,
//! each node by its first fitting rule until none fits (card rewrite-rules, matching roadmap step 4)
use crate::is;

#[test]
fn a_rule_set_rewrites_code_to_a_normal_form() {
	is!("simplify = rules { data($a * 1) => a; data($a + 0) => a }\nstr(simplify(data(y * 1 + 0)))", "y");
	is!("simplify = rules { data($a * 1) => a; data($a + 0) => a }\nstr(simplify(data(y * 1 * 1 + k * 1)))", "y+k");
}

#[test]
fn a_rule_set_on_lines_with_a_guard() {
	is!(
		"folded := rules {\n\tdata($a + $b) if a is int and b is int => a + b\n\tdata($a * 1) => a\n}\nfolded(data(2 + 3)) + 1",
		6
	);
}

#[test]
fn a_rule_set_rewrites_data() {
	is!("unwrapped = rules { [1, x] => x }\nys = unwrapped([[1, 2], [3, [1, [1, 5]]]])\nys#1 + ys#2#2", 7);
}

#[test]
fn an_unnamed_rule_set_is_a_function() {
	is!("apply(f, x) := f(x)\nstr(apply(rules { data($a * 1) => a }, data(n * 1)))", "n");
}
