use std::time::{Duration, Instant};
use warp::diagnostic::Diagnostic;
use warp::node::{Bracket, Node, Separator};
use warp::warp_parser::WarpParser;

const LINES: usize = 20_000;
/// Generous for a debug build on a busy machine; parsing was quadratic in the line count (minutes for this source)
const PATIENCE: Duration = Duration::from_secs(20);

/// A long source parses in time proportional to its length, every statement knowing its line
#[test]
fn a_long_source_parses_in_linear_time() {
	let source: String = (1..=LINES).map(|line| format!("x{line} = {line}\n")).collect();
	let started = Instant::now();
	let program = WarpParser::parse(&source);
	assert!(started.elapsed() < PATIENCE, "{LINES} lines took {:?}", started.elapsed());
	let Node::List(statements, Bracket::None, Separator::Newline) = program.drop_meta() else { panic!("{program:?}") };
	assert_eq!(statements.len(), LINES);
	assert_eq!(Diagnostic::at(&statements[LINES - 1], "").line, LINES);
}
