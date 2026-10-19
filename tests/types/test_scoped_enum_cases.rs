// card node-enum (decision compiler-node-enum): an enum's cases are named in its scope, so the cases of the compiler's
// `enum Node { True, False, Empty, Number(value) }` keep the names of the Rust enum: `Node.True` is the case, a bare
// `True` stays yes; cases also come in rows `Add, Sub, Mul`, and a used file's enum is the importer's too
use crate::is;
use warp::symbol;

const NODE: &str = "enum Node { True, False, Empty, Number(n: int), Text(s: text) }";

fn with_node(code: &str) -> String {
	format!("{NODE}; {code}")
}

#[test]
fn a_constant_word_is_a_case_in_its_enum() {
	is!(&with_node("Node.True"), symbol("True"));
	is!(&with_node("Node::False"), symbol("False"));
	is!(&with_node("Node.Empty == Node.Empty"), true);
	is!(&with_node("True"), true);
	is!("enum Separator { Space, Comma, None }; Separator.None", symbol("None"));
}

#[test]
fn a_case_with_values_by_its_enum() {
	is!(&with_node("Node.Number(3).n"), 3);
	is!(&with_node("x = Node.Number(3); x is Node"), true);
	is!(&with_node("x = Node.Number(4)\nmatch x {\n Node.Number(n) => n * 2\n _ => 0\n}"), 8);
	is!(&with_node("x = Node.True\nmatch x {\n Node.True => 1\n _ => 0\n}"), 1);
}

#[test]
fn cases_in_rows() {
	is!("enum Op {\n\tAdd, Sub\n\tMul, Div\n}\nOp.Mul", symbol("Mul"));
	is!("enum Op {\n\tAdd, Sub\n\tMul, Div\n}\nOp.Div as int", 3);
	is!("enum Op {\n\tAdd, Sub\n\tMul\n}\nenum Key { Pair(op: Op) }\nKey.Pair(Op.Mul).op", symbol("Mul"));
}

#[cfg(feature = "native")] // reads tests/fixtures/scoped_enums.warp
#[test]
fn a_used_files_enum() {
	let used = |code: &str| format!("use \"tests/fixtures/scoped_enums.warp\"\n{code}");
	is!(&used("Tree.True"), symbol("True"));
	is!(&used("Tree.Key(Tree.True, Op.Mul, Tree.Number(3)).op"), symbol("Mul"));
	is!(&used("x = Tree.Empty; x is Tree"), true);
	is!(&used("Separator.None"), symbol("None"));
}

// card enum-match-in-used-file: a used file's match binds its cases' fields, as in the main file
#[cfg(feature = "native")] // reads tests/fixtures/used_cases.warp
#[test]
fn a_match_in_a_used_file_binds_its_cases() {
	is!("use \"tests/fixtures/used_cases.warp\"\nf(Thing::Holds(Box(3)), Thing::Counts(2))", 3);
}
