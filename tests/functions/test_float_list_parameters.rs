// card untyped-parameters: an untyped list parameter that every call passes a list of floats takes `floats`, as if
// declared, so its loop reads f64s instead of Nodes (probes/perf/float_loop.warp: ~5x faster); an exact 0.0 accumulator
// that only gets floats then stays a float
use warp::Node;

const PROGRAM: &str = "xs = [i * 0.5 as float for i in 0..4]\ndef total(values) {\n\tsum = 0.0\n\tfor x in values { sum = x + sum }\n\tsum\n}\ntotal(xs)";

fn annotation_of_values(program: &str) -> Option<Node> {
	let lowered = warp::pipeline::lower(program).expect("a program");
	let context = warp::analyzer::function_context(&lowered);
	context.user_functions["total"].params[0].annotation.clone()
}

#[test]
fn a_parameter_given_only_float_lists_takes_floats() {
	assert_eq!(annotation_of_values(PROGRAM), Some(Node::Symbol("floats".into())));
	crate::is!(PROGRAM, 3.0);
}

#[test]
fn a_parameter_given_other_lists_too_stays_untyped() {
	let program = format!("{PROGRAM}\ntotal([1, 2])");
	assert_eq!(annotation_of_values(&program), None);
	crate::is!(&program, 3);
}

#[test]
fn a_call_inside_a_function_leaves_the_parameter_untyped() {
	let program = format!("{PROGRAM}\ndef twice(ys) {{ total(ys) * 2 }}\ntwice(xs)");
	assert_eq!(annotation_of_values(&program), None);
	crate::is!(&program, 6.0);
}
