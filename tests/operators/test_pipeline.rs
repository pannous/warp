// The pipeline `xs |> f(a)` is `f(xs, a)` and `xs |> f` is `f(xs)` (F#, Elixir, wiki/Purpose.md), also across lines
use warp::ints;
use crate::is;

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

// a braceless call before the pipeline is its value, as in F#: `square numbers |> sum` is `sum(square numbers)`
// (card square-numbers: the pipeline took only `numbers`); statement words keep the whole pipeline (`print xs |> sum`)
#[test]
fn a_braceless_call_is_the_piped_value() {
	is!("square := it*it; numbers=[1 2 3 4]; square numbers |> filter(x => x > 5) |> sum", 25);
	is!("square := it*it; numbers=[1 2 3 4]; r = square numbers |> sum; r", 30);
	is!("square(x) := x*x; square 3 |> str", "9");
	is!("def square := it * it\nnumbers = [1, 2, 3, 4, 5]\nr = square numbers\n    |> filter(x => x > 5)\n    |> sum\nr", 50);
	is!("xs=[1 2 3]; print xs |> sum; 1", 1);
}
