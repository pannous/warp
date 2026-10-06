// `import name` of a WebAssembly core module (name.wasm, or its text name.wat): its exported functions are called with
// the types the module declares, its exported globals read as values, and its state lives in one instance per run
use crate::is;
use crate::common::fails_with;

const FOURTY_TWO: &str = "import tests/fixtures/wasm/fourty_two; ";

fn with_module(code: &str) -> String {
	format!("{FOURTY_TWO}{code}")
}

#[test]
fn an_exported_function_is_called_with_its_declared_types() {
	is!(&with_module("twice(21)"), 42); // i64
	is!(&with_module("half(5.0)"), 2.5); // f64
	is!(&with_module("add32(2, 3)"), 5); // i32
}

#[test]
fn an_exported_global_reads_as_its_value() {
	is!(&with_module("ft * 2"), 84);
	is!(&with_module("twice(ft) + 1"), 85);
	fails_with(&with_module("ft = 3; ft"), "ft is an immutable global of an imported module");
}

#[test]
fn a_module_keeps_its_state_for_the_run() {
	is!(&with_module("tick(); tick(); tick()"), 3);
	is!("import tests/fixtures/wasm/counter.wasm; count_up(2); count_up(3)", 5);
}

#[test]
fn a_module_is_found_by_name_or_path() {
	is!("use \"tests/fixtures/wasm/counter.wasm\"; count_up(5)", 5);
	#[cfg(feature = "native")] // WAT text needs the native build
	is!("import \"tests/fixtures/wasm/fourty_two.wat\"; ft", 42);
	fails_with("import tests/fixtures/wasm/no_such; 1", "module not found");
}

#[test]
fn an_export_is_qualified_by_its_module_name() {
	is!(&with_module("fourty_two.twice(21)"), 42);
	is!(&with_module("fourty_two.double(21)"), 42); // the export, not the float cast double(21)
	is!(&with_module("fourty_two.twice(fourty_two.ft)"), 84);
}

// P139 include runs the entry point, P140 a mutable global is set in the module, P141 a bare call a cast would take is
// ambiguous
#[test]
fn include_runs_main_globals_are_set_and_casts_are_ambiguous() {
	is!("include tests/fixtures/wasm/fourty_two", 42);
	is!("include tests/fixtures/wasm/fourty_two; ft * 2", 84);
	is!(&with_module("level = 5; level"), 5);
	is!(&with_module("level += 2; level * 10"), 30);
	fails_with(&with_module("double(21)"), "double is ambiguous: fourty_two.double(21) for the export, 21 as float for the cast");
}

#[test]
fn an_imported_module_gets_wasi_host_words_and_other_modules() {
	const GREETER: &str = "import tests/fixtures/wasm/greeter; ";
	is!(&format!("{GREETER}greet()"), 3); // WASI fd_write printed "hi\n"
	is!(&format!("{GREETER}five()"), 5); // the host word random_below
	is!(&format!("{GREETER}count_twice(4)"), 8); // counter.wasm, imported by greeter
	// one instance per file: the program's counter.wasm is greeter's
	is!(&format!("import tests/fixtures/wasm/counter.wasm; {GREETER}count_up(1); count_twice(1)"), 3);
}

#[test]
fn a_qualified_global_is_assigned_in_its_module() {
	is!(&with_module("fourty_two.level = 5; level"), 5);
	is!(&with_module("fourty_two.level += 2; fourty_two.level * 10"), 30);
	fails_with(&with_module("fourty_two.ft = 3"), "fourty_two.ft is an immutable global of an imported module");
}
