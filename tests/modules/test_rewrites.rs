// `rewrite PATTERN => TEMPLATE` (card compiler-modules, src/modules/rewrites.rs): code extending the compiler, the way
// lib/channels.warp gives programs the channel words
use crate::is;

#[test]
fn a_program_rewrites_its_own_phrases() {
	is!("rewrite double $x => $x * 2\ndouble 21", 42);
	is!("rewrite total $xs... => sum([$xs...])\ntotal 1 2 3", 6);
}

// `where $c = counter()`: only variables assigned that code are rewritten
#[test]
fn a_rule_with_an_origin_rewrites_only_its_variables() {
	is!("rewrite $c.bump() => $c + 1 where $c = counter()\ncounter() := 10\nn = counter()\nother = 5\n[n.bump(), other]", warp::ints(vec![11, 5]));
}

// `{ send 7 to ch }`: a statement pattern matches a block of just that statement, which stays a block
#[test]
fn a_statement_rule_keeps_a_block() {
	is!("rewrite twice $x => $x * 2\nf() := { twice 4 }\nf()", 8);
}

// `$n half` matches the run inside a longer phrase, also the value of `x = 8 half`
#[test]
fn a_phrase_rule_rewrites_runs_inside_phrases() {
	is!("rewrite $n half => $n / 2\nx = 8 half\n[x, 6 half]", warp::ints(vec![4, 3]));
}

#[test]
fn a_rule_of_several_statements_takes_the_statements_place() {
	is!("rewrite twice $s => ($s; $s)\nn = 0\ntwice n += 1\nn", 2);
}

#[test]
fn a_typed_hole_takes_only_code_of_its_kind() {
	is!("rewrite size_of $t: text => count($t)\nsize_of \"abc\"", 3);
}

// a program defining channel() keeps its own: the standard rules of a word the program binds stay out
#[test]
fn own_definitions_shadow_standard_rules() {
	is!("channel() := 5\nchannel()", 5);
}
