use std::time::{Duration, Instant};
use warp::warp_parser::WarpParser;

const FEW: usize = 500;
const GROWTH: usize = 4;
/// Linear parsing grows as the plain assignments do, GROWTH times for GROWTH times the lines, quadratic GROWTH times
/// more; the margin absorbs noise. While quadratic, 2000 one-line definitions took 9.5 s in a debug build, 500 took 0.6 s
const GROWTH_MARGIN: f64 = 2.0;
/// Each size parses this often and counts its fastest run: a busy machine (the full suite's parallel tests) only slows
/// some runs, so the fastest is the parse's own time (card linear-time)
const RUNS: usize = 5;

/// The source line number `index`
type Line = fn(usize) -> String;
/// The reference, known to parse in linear time (test_parse_long_sources): under the full suite's load a big source
/// parsed 15 times as slowly as one a quarter its size, so the definitions are measured against it (card linear-time)
const ASSIGNMENT: Line = |index| format!("x{index} = {index}");
/// A block closed by `end` after all the definitions: every definition's search for its closing `end` could run to it
const LATE_END_BLOCK: &str = "items.each do |item|\n\tprint item\nend\n";

fn time_parse(line: Line, count: usize, tail: &str) -> Duration {
	let source: String = (1..=count).map(|index| line(index) + "\n").collect::<String>() + tail;
	let time_once = || {
		let started = Instant::now();
		WarpParser::parse(&source);
		started.elapsed()
	};
	(0..RUNS).map(|_| time_once()).min().unwrap()
}

/// How many times as long GROWTH times the lines take
fn growth(line: Line, tail: &str) -> f64 {
	time_parse(line, FEW * GROWTH, tail).as_secs_f64() / time_parse(line, FEW, tail).as_secs_f64()
}

/// Many one-line function definitions parse in time proportional to their count, as plain assignments do
fn assert_linear(tail: &str) {
	let forms: [(&str, Line); 4] = [
		("def", |index| format!("def f{index}(x) = x + {index}")),
		("colon-equals", |index| format!("f{index}(x) := x * {index}")),
		("braces", |index| format!("fun f{index}(x) {{ x + {index} }}")),
		("typed", |index| format!("def f{index}(x: int) -> int = x + {index}")),
	];
	for (name, definition) in forms {
		let (definitions, assignments) = (growth(definition, tail), growth(ASSIGNMENT, tail));
		assert!(
			definitions < assignments * GROWTH_MARGIN,
			"{name}: {GROWTH} times the definitions took {definitions:.1} times as long, assignments {assignments:.1}"
		);
	}
}

#[test]
fn many_definitions_parse_in_linear_time() {
	assert_linear("");
}

/// Also with a `do … end` block after them
#[test]
fn many_definitions_before_a_late_end_parse_in_linear_time() {
	assert_linear(LATE_END_BLOCK);
}
