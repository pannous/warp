// The pipeline `xs |> f(a)` is `f(xs, a)` and `xs |> f` is `f(xs)` (F#, Elixir, wiki/Purpose.md), also across lines
use warp::{ints, is};

#[test]
fn a_pipeline_calls_each_stage_with_the_value() {
	is!("[3 1 2] |> sort", ints(vec![1, 2, 3]));
	is!("f(a,b):=a-b; 10 |> f(3)", 7);
	is!("[1 2 3] |> sum > 3", 1);
}

#[test]
fn a_pipeline_over_several_lines() {
	is!("square(x):=x*x; numbers=[1,2,3,4,5]\nr = numbers\n    |> map(square)\n    |> filter(x => x > 5)\n    |> sum\nr", 50);
}
