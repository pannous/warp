//! Reflection mode (card use-reflection, decision reflection): `use reflection` (or `use reflection mode`) gives a
//! function value where and how it was made: .file .line .column .origin .compiler .wasm_name, and `print f` a comment
//! line saying so above the definition
use crate::is;

const TWICE: &str = "use reflection\n# doubles a number\ntwice(x) := x*2\n";

#[test]
fn a_function_knows_where_it_was_written() {
	is!(&format!("{TWICE}twice.line"), 3);
	is!(&format!("{TWICE}fs = [function twice]\nfs#1.column"), 1);
	is!(&format!("{TWICE}twice.origin"), "program");
	is!("use reflection mode\nuse math\ngcd.origin", "math");
}

#[test]
fn a_function_knows_what_made_it() {
	is!(&format!("{TWICE}twice.wasm_name"), "twice");
	is!(&format!("{TWICE}twice.compiler"), format!("warp {}", env!("CARGO_PKG_VERSION")).as_str());
}

#[test]
#[cfg(feature = "native")]
fn a_printed_function_says_where_it_was_made() {
	let printed = crate::common::printed(&format!("{TWICE}print(function twice)"));
	let version = env!("CARGO_PKG_VERSION");
	assert_eq!(
		printed.trim(),
		format!("# program:3:1 · warp {version} · wasm twice\n# doubles a number\ntwice(x) := x*2")
	);
}

#[test]
fn without_reflection_mode_a_function_has_no_line() {
	crate::common::fails_with("twice(x) := x*2\nfs = [function twice]\nfs#1.line", "no field line");
}
