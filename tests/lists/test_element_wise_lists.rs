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
