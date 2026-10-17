use std::time::{Duration, Instant};
use warp::warp_parser::WarpParser;

const FEW: usize = 500;
const GROWTH: u32 = 4;
/// Linear parsing takes GROWTH times as long for GROWTH times the definitions, quadratic GROWTH²; the margin absorbs a
/// busy machine. While quadratic, 2000 one-line definitions took 9.5 s in a debug build, 500 took 0.6 s
const SLOWDOWN_LIMIT: u32 = GROWTH * 2;
/// Each size parses this often and counts its fastest run: a busy machine (the full suite's parallel tests) only slows
/// some runs, so the fastest is the parse's own time (card linear-time)
const RUNS: usize = 5;

/// The source line of definition number `index`
type Definition = fn(usize) -> String;
/// A block closed by `end` after all the definitions: every definition's search for its closing `end` could run to it
const LATE_END_BLOCK: &str = "items.each do |item|\n\tprint item\nend\n";

fn time_parse(definition: Definition, count: usize, tail: &str) -> Duration {
	let source: String = (1..=count).map(|index| definition(index) + "\n").collect::<String>() + tail;
	let time_once = || {
		let started = Instant::now();
		WarpParser::parse(&source);
		started.elapsed()
	};
	(0..RUNS).map(|_| time_once()).min().unwrap()
}

/// Many one-line function definitions parse in time proportional to their count, as plain assignments do
fn assert_linear(tail: &str) {
	let forms: [(&str, Definition); 4] = [
		("def", |index| format!("def f{index}(x) = x + {index}")),
		("colon-equals", |index| format!("f{index}(x) := x * {index}")),
		("braces", |index| format!("fun f{index}(x) {{ x + {index} }}")),
		("typed", |index| format!("def f{index}(x: int) -> int = x + {index}")),
	];
	for (name, definition) in forms {
		let few = time_parse(definition, FEW, tail);
		let many = time_parse(definition, FEW * GROWTH as usize, tail);
		assert!(many < few * SLOWDOWN_LIMIT, "{name}: {FEW} definitions took {few:?}, {GROWTH} times as many {many:?}");
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
