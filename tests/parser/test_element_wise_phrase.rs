// The parser keeps `xs .* 2` as written, the phrase [xs .* 2]; lowering/broadcasting.rs gives it its meaning (a map over
// the elements), so the parser twin in lib/compiler/parser.warp stays syntactic (card cleanup-parser-element-wise)
use crate::is;
use warp::warp_parser::parse;

#[test]
fn element_wise_operators_still_mean_what_they_did() {
	is!("xs=[1 2 3]; xs .* 2", parse("[2 4 6]"));
	is!("xs=[1 2 3]; ys=[4 5 6]; (xs .* 2) .+ ys", parse("[6 9 12]"));
	is!("xs=[1 2 3]; sum(xs .^ 2)", 14);
	is!("f(xs) := xs .+ 1; f([1 2])", parse("[2 3]"));
}
