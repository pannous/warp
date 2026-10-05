// A capturing lambda passed to a task through a parameter: its result kind comes from the captured variable's kind
// ("task apply·node: not an int" when `s + t` with a captured text was not typed); a parameter named like a unit
// (`s`, `m`) is the parameter, not seconds or meters
use warp::*;

#[test]
fn a_capturing_text_lambda_runs_in_a_task() {
	is!("t = \"!\"; shout = s => s + t; apply(g, v) := g(v); job = go apply(shout, \"hey\"); await job", "hey!");
	is!("t = \"!\"; shout = s => \"x\" + t; apply(g, v) := g(v); apply(shout, \"hey\")", "x!");
}

#[test]
fn a_parameter_named_like_a_unit_is_the_parameter() {
	is!("t = 7; shout = s => s + t; shout(1)", 8);
	is!("f(m) := m * 3; f(2)", 6);
}
