//! card gpu-vectors step 1 (notes/gpu.md): an element-wise operator between two lists pairs their items, `xs .* ys`;
//! lists of different lengths are an error; `dot(xs, ys)` is `sum(xs .* ys)`
use crate::is;
use warp::warp_parser::parse;
use warp::wasm_emitter::eval;
use warp::Node;

#[test]
fn an_element_wise_operator_pairs_two_lists() {
	is!("xs=[1 2 3]; ys=[4 5 6]; xs .+ ys", parse("[5 7 9]"));
	is!("xs=[1 2 3]; ys=[4 5 6]; xs .* ys", parse("[4 10 18]"));
	is!("[1 2] .- [3 5]", parse("[-2 -3]"));
	is!("xs=[1 2 3]; ys=[4 5 6]; (xs .* 2) .+ ys", parse("[6 9 12]"));
	is!("xs = [1.5 2.5]; ys = [0.5 0.5]; xs ./ ys", parse("[3 5]"));
}

#[test]
fn reductions_of_paired_lists() {
	is!("xs=[1 2 3]; ys=[4 5 6]; sum(xs .* ys)", 32);
	is!("xs=[1 2 3]; ys=[4 5 6]; dot(xs, ys)", 32);
}

#[test]
fn zero_filled_lists_pair_too() {
	is!("xs = float[3]; ys = [1.5 2.5 3.5]; sum(xs .+ ys)", 7.5);
}

#[test]
fn lists_of_different_lengths_are_an_error() {
	let failed = eval("xs=[1 2 3]; ys=[1 2]; xs .* ys");
	assert!(matches!(&failed, Node::Error(message) if message.to_string().contains("length")), "{failed:?}");
}

// sum of an element-wise expression is one loop adding the items (the list is never built), same values
#[test]
fn a_sum_of_an_element_wise_expression_is_fused() {
	is!("xs=[1 2 3]; sum(xs .* 2)", 12);
	is!("xs=[1 2]; sum((xs .* 2) .+ 1)", 8);
	is!("sum(ø .* 2)", 0);
	is!("xs=[1 2 3]; ys=[4 5 6]; dot(xs, ys)", 32);
	assert!(warp::pipeline::lower("xs=[1 2 3]; sum(xs .* 2)").expect("lowered").serialize().contains("fused_sum"));
	is!("sum(xs) := 7; xs=[1 2]; sum(xs .* 2)", 7);
}

// card float-typed: an exact decimal stored in a float list keeps it a float list, its items floats in arithmetic
#[test]
fn a_float_list_given_a_decimal_stays_a_float_list() {
	is!("xs = float[3]; xs#1 = 0.5; s = 0; for x in xs { s = s + x * 4 }; s", 2.0);
	is!("xs = float[3]; xs#1 = 0.5; s = 7; s = 1.5 + xs#1; s", 2.0);
	is!("xs = float[3]; xs#1 = 0.5; sum(xs .* 4)", 2.0);
}

// card list-literal: a list literal on the right of a dotted operator pairs, the dotted form is no list * number question
#[test]
fn a_list_literal_pairs_with_a_dotted_operator() {
	is!("[2 2] .* [2 4]", parse("[4 8]"));
	is!("sum([1.5 2] .* [2 4])", 11);
}

// `float[n]` of a variable count is a list too: before, `xs .* ys` of two was `x * ys`, a float * list error
#[test]
fn zero_filled_lists_of_a_variable_count_pair() {
	let filled = "n = 3; xs = float[n]; ys = float[n]; for i in 1 to n { xs#i = i; ys#i = 2 }\n";
	is!(&format!("{filled}dot(xs, ys)"), 12.0);
	is!(&format!("{filled}zs = xs .* ys; zs#3"), 6.0);
}
