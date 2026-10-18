use super::test_big_literals::thread_cpu_time;
use std::time::Duration;
use warp::warp_parser::WarpParser;

const FEW: usize = 500;
const GROWTH: usize = 4;
/// Linear parsing grows as the plain assignments do, GROWTH times for GROWTH times the lines, quadratic GROWTH times
/// more; the margin absorbs noise. While quadratic, 2000 one-line definitions took 9.5 s in a debug build, 500 took 0.6 s
const GROWTH_MARGIN: f64 = 2.0;
/// Each size parses this often and counts its fastest run, timed in the CPU time of the parsing thread: the gate's load
/// stretched wall-clock time unevenly (10.3 and 10.8 times for 4 times the definitions, assignments 4.0), even with
/// the four sources taking turns in each round (card linear-time)
const RUNS: usize = 9;

/// The source line number `index`
type Line = fn(usize) -> String;
/// The reference, known to parse in linear time (test_parse_long_sources): under the full suite's load a big source
/// parsed 15 times as slowly as one a quarter its size, so the definitions are measured against it (card linear-time)
const ASSIGNMENT: Line = |index| format!("x{index} = {index}");
/// A block closed by `end` after all the definitions: every definition's search for its closing `end` could run to it
const LATE_END_BLOCK: &str = "items.each do |item|\n\tprint item\nend\n";

fn source_of(line: Line, count: usize, tail: &str) -> String {
	(1..=count).map(|index| line(index) + "\n").collect::<String>() + tail
}

fn time_parse(source: &str) -> Duration {
	let started = thread_cpu_time();
	WarpParser::parse(source);
	thread_cpu_time() - started
}

/// How many times as long GROWTH times the lines take, for the definitions and for the assignments
fn growths(definition: Line, tail: &str) -> (f64, f64) {
	let sources: Vec<String> =
		[(definition, FEW), (definition, FEW * GROWTH), (ASSIGNMENT, FEW), (ASSIGNMENT, FEW * GROWTH)]
			.iter()
			.map(|&(line, count)| source_of(line, count, tail))
			.collect();
	let mut fastest = [Duration::MAX; 4];
	for _ in 0..RUNS {
		for (time, source) in fastest.iter_mut().zip(&sources) {
			*time = (*time).min(time_parse(source));
		}
	}
	let growth = |few: Duration, many: Duration| many.as_secs_f64() / few.as_secs_f64();
	(growth(fastest[0], fastest[1]), growth(fastest[2], fastest[3]))
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
		let (definitions, assignments) = growths(definition, tail);
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
