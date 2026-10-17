use std::time::{Duration, Instant};
use warp::warp_parser::WarpParser;

const FEW: usize = 500;
const GROWTH: u32 = 4;
/// Linear parsing takes GROWTH times as long for GROWTH times the definitions, quadratic GROWTH²; the margin absorbs a
/// busy machine. While quadratic, 2000 one-line definitions took 9.5 s in a debug build, 500 took 0.6 s
const SLOWDOWN_LIMIT: u32 = GROWTH * 2;

/// The source line of definition number `index`
type Definition = fn(usize) -> String;

fn time_parse(definition: Definition, count: usize) -> Duration {
	let source: String = (1..=count).map(|index| definition(index) + "\n").collect();
	let started = Instant::now();
	WarpParser::parse(&source);
	started.elapsed()
}

/// Many one-line function definitions parse in time proportional to their count, as plain assignments do
#[test]
fn many_definitions_parse_in_linear_time() {
	let forms: [(&str, Definition); 4] = [
		("def", |index| format!("def f{index}(x) = x + {index}")),
		("colon-equals", |index| format!("f{index}(x) := x * {index}")),
		("braces", |index| format!("fun f{index}(x) {{ x + {index} }}")),
		("typed", |index| format!("def f{index}(x: int) -> int = x + {index}")),
	];
	for (name, definition) in forms {
		let few = time_parse(definition, FEW);
		let many = time_parse(definition, FEW * GROWTH as usize);
		assert!(many < few * SLOWDOWN_LIMIT, "{name}: {FEW} definitions took {few:?}, {GROWTH} times as many {many:?}");
	}
}
