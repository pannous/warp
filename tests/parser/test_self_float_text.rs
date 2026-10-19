//! card float-text-warp: runtime.warp float_text and float_digits print run-time floats as Rust's float_text does,
//! at the hard cases of the shortest digits (probes/self_hosting/emitter/core/float_text*.warp)
use crate::common::twins::emitter_differential;
use std::path::PathBuf;

const FLOAT_TEXT_PROGRAMS: [&str; 3] = [
	"probes/self_hosting/emitter/core/float_text.warp",
	"probes/self_hosting/emitter/core/float_text_edges.warp",
	"probes/self_hosting/emitter/core/float_text_shortest.warp",
];
const SHORTEST_PROGRAM: &str = "probes/self_hosting/emitter/core/float_text_shortest.warp";
/// The shortest texts that read back, as Python's repr finds them (positional from 1e-5 up to 1e15)
const SHORTEST_TEXTS: &str = "1e23 1.7976931348623157e308 5e-324 2.2250738585072014e-308 9.223372036854776e18 \
	0.7999999999999999 0.14285714285714285 123.456 -6.666666666666667e-6 9.007199254740992e15 5e-15";

#[test]
fn float_texts_of_emitter_warp_are_rusts() {
	let programs: Vec<PathBuf> = FLOAT_TEXT_PROGRAMS.iter().map(PathBuf::from).collect();
	let inside = emitter_differential(&programs, programs.len());
	assert_eq!(inside.len(), programs.len());
}

#[test]
fn rust_float_text_prints_the_shortest_digits() {
	let source = std::fs::read_to_string(SHORTEST_PROGRAM).unwrap();
	let (_, printed) = warp::host::with_captured_stdout(|| warp::wasm_emitter::eval(&source));
	let expected = SHORTEST_TEXTS.split_whitespace().collect::<Vec<_>>().join(" ");
	assert!(printed.contains(&expected), "printed {printed:?}, expected [{expected}]");
}
