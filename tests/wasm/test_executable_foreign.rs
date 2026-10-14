// Card exe-foreign (user decision 2026-10-12, notes/aot.md): an executable using python or js starts python3 or node from
// PATH when it runs, as warp does, and fails loudly naming a missing interpreter
#![cfg(feature = "native")]
use std::path::PathBuf;
use std::process::Output;

const OUTPUT_DIRECTORY: &str = env!("CARGO_TARGET_TMPDIR");

/// `warp build --exe` of `source` as <name>.warp, run with the environment `variables`
fn run_executable(name: &str, source: &str, variables: &[(&str, &str)]) -> Output {
	let source_path = PathBuf::from(OUTPUT_DIRECTORY).join(format!("{name}.warp"));
	std::fs::write(&source_path, source).unwrap();
	let build = crate::common::warp_command()
		.env("WARP_RUNTIME_STUB", crate::common::runtime_stub())
		.args(["build", "--exe"])
		.arg(&source_path)
		.output()
		.unwrap();
	assert!(build.status.success(), "{}", String::from_utf8_lossy(&build.stderr));
	std::process::Command::new(source_path.with_extension(std::env::consts::EXE_EXTENSION))
		.envs(variables.iter().copied())
		.output()
		.unwrap()
}

fn printed(name: &str, source: &str) -> String {
	let run = run_executable(name, source, &[]);
	assert!(run.status.success(), "{}", String::from_utf8_lossy(&run.stderr));
	String::from_utf8_lossy(&run.stdout).into_owned()
}

#[test]
fn an_executable_calls_python_and_node() {
	assert_eq!(printed("exe_python_sqrt", "use python math\nmath.sqrt(16)"), "4\n");
	assert_eq!(printed("exe_js_max", "use js Math\nMath.max(3, 7)"), "7\n");
}

#[test]
fn values_cross_both_ways_as_warp_does() {
	assert_eq!(
		printed("exe_python_json", "use python json\njson.loads(\"{\\\"a\\\": [1, 2.5, \\\"x\\\"]}\")"),
		"{a:[1 2.5 \"x\"]}\n"
	);
	assert_eq!(printed("exe_python_big", "use python math\nmath.factorial(25)"), "15511210043330985984000000\n");
	assert_eq!(
		printed("exe_python_handle", "use python datetime\nd = datetime.date(2020, 1, 2)\nd.isoformat()"),
		"2020-01-02\n" // an executable prints a text result plainly
	);
}

#[test]
fn a_missing_interpreter_is_named() {
	let run =
		run_executable("exe_python_missing", "use python math\nmath.sqrt(16)", &[("WARP_PYTHON", "no-such-python3")]);
	let errors = String::from_utf8_lossy(&run.stderr);
	assert!(!run.status.success());
	assert!(errors.contains("cannot start no-such-python3") && errors.contains("WARP_PYTHON"), "{errors}");
}

#[test]
fn the_runtime_reads_and_builds_nodes_with_warps_codes() {
	use warp::type_kinds::{Kind, BOOL_INFO, KIND_BITS, KIND_MASK};
	use warp_runtime::gc_values as runtime;
	let kinds = [
		(runtime::EMPTY, Kind::Empty),
		(runtime::INT, Kind::Int),
		(runtime::FLOAT, Kind::Float),
		(runtime::TEXT, Kind::Text),
		(runtime::CODEPOINT, Kind::Codepoint),
		(runtime::SYMBOL, Kind::Symbol),
		(runtime::KEY, Kind::Key),
		(runtime::BLOCK, Kind::Block),
		(runtime::LIST, Kind::List),
		(runtime::ERROR, Kind::Error),
		(runtime::FUNCTION, Kind::Function),
	];
	for (code, kind) in kinds {
		assert_eq!(code, kind as u8, "{kind:?}");
	}
	assert_eq!((runtime::KIND_MASK, runtime::KIND_BITS, runtime::BOOL_INFO), (KIND_MASK, KIND_BITS, BOOL_INFO));
	assert_eq!(runtime::COLON, warp::operators::op_to_code(&warp::operators::Op::Colon));
	assert_eq!(runtime::CURLY, warp::wasm_emitter::bracket_info(&warp::node::Bracket::Curly));
	assert_eq!(runtime::SQUARE, warp::wasm_emitter::bracket_info(&warp::node::Bracket::Square));
	assert_eq!(runtime::BUILDERS[8..], warp::wasm_emitter::EXACT_BUILDERS[..3]);
	assert_eq!(runtime::TEXT_HEAP, warp::host::TEXT_HEAP_EXPORT);
}
