// card code-patterns: match on code as the warp compiler needs it; a statement pattern `for $v in $it { $body }`
// matches the loop's phrase, an operator hole `$l $op $r` the operator of a key (rest holes and if/else matched before)
use crate::is;

#[test]
fn a_statement_pattern_matches_a_loop() {
	is!(
		"walk = parse(\"for x in xs { print x }\")\nmatch walk { data(for $v in $it { $body }) => it; _ => \"no\" }",
		"xs"
	);
	is!(
		"walk = parse(\"for x in xs { print x }\")\nmatch walk { data(for $v in $it { $body }) => v; _ => \"no\" }",
		"x"
	);
	is!("walk = parse(\"while c { x }\")\nmatch walk { data(for $v in $it { $body }) => v; _ => \"no\" }", "no");
}

#[test]
fn an_operator_hole_binds_the_operator() {
	is!("binary = parse(\"a - b\")\nmatch binary { data($l $op $r) => op; _ => \"no\" }", "-");
	is!("binary = parse(\"a - b\")\nmatch binary { data($l $op $r) => r; _ => \"no\" }", "b");
	is!("match parse(\"f(1)\") { data($l $op $r) => op; _ => \"no\" }", "no");
}
