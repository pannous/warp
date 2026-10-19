// card untyped-parameters: an untyped list parameter that every call passes a list of floats takes `floats`, as if
// declared, so its loop reads f64s instead of Nodes (probes/perf/float_loop.warp: ~5x faster); an exact 0.0 accumulator
// that only gets floats then stays a float

const PROGRAM: &str = "xs = [i * 0.5 as float for i in 0..4]\ndef total(values) {\n\tsum = 0.0\n\tfor x in values { sum = x + sum }\n\tsum\n}\ntotal(xs)";

#[test]
fn a_parameter_given_only_float_lists_takes_floats() {
	crate::is!(PROGRAM, 3.0);
}

#[test]
fn a_parameter_given_other_lists_too_stays_untyped() {
	let program = format!("{PROGRAM}\ntotal([1, 2])");
	crate::is!(&program, 3);
}

#[test]
fn a_call_inside_a_function_leaves_the_parameter_untyped() {
	let program = format!("{PROGRAM}\ndef twice(ys) {{ total(ys) * 2 }}\ntwice(xs)");
	crate::is!(&program, 6.0);
}
