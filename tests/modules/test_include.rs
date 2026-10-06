use warp::wasm_emitter::eval;
use warp::Node;
use crate::is;

fn error_text(code: &str) -> String {
	match eval(code) {
		Node::Error(reason) => format!("{reason:?}"),
		other => panic!("{code}: expected an error, got {other:?}"),
	}
}

#[test]
fn include_splices_the_whole_file_and_runs_it() {
	is!("include \"tests/fixtures/counter.wasp\"; counter", 11);
	is!("include tests/fixtures/counter; counter", 11);
	is!("include tests/fixtures/counter.wasp; counter", 11);
}

#[test]
fn use_imports_only_the_declarations() {
	is!("use tests/fixtures/counter; counter", 10);
}

#[test]
fn require_and_import_are_use() {
	is!("require square; square 7", 49);
	is!("import square; square 7", 49);
	is!("use square; square 7", 49);
	is!("require math; floor(4.5)", 4);
}

#[test]
fn a_nested_include_is_found_next_to_the_including_file() {
	is!("include tests/fixtures/nested/outer; outer_value", 2);
}

#[test]
fn including_a_file_twice_includes_it_once_and_cycles_terminate() {
	is!("include tests/fixtures/counter; include tests/fixtures/counter; counter", 11);
	is!("include tests/fixtures/cycle_a; a_done + b_done", 3);
}

#[test]
fn a_missing_include_lists_the_searched_paths() {
	let message = error_text("include no_such_file");
	assert!(message.contains("no_such_file"), "{message}");
	for directory in ["./include", "./lib", "./src", "./source"] {
		assert!(message.contains(directory), "{directory} missing: {message}");
	}
}
