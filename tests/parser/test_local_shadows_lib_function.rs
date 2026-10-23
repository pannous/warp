//! card local-shadows-lib-function: a local or parameter named like a lib function (list.warp's `index(xs, x)`, `max`,
//! `print`) is the variable wherever it is bound; reading it never calls the function, on the Rust lane and under
//! `warp pure` and `warp self` (parser.warp's closing_ends_of counts down a local `index`)
use super::test_warp_pure::lane;

/// Locals and a parameter in functions, beside the program's call of list.warp's index
const IN_FUNCTIONS: &str = include_str!("../../probes/shadowing/local_shadows_lib_function.warp");
const IN_FUNCTIONS_PRINTED: [&str; 4] = ["1", "10", "0", "8"];
/// Main-level variables named index and max
const IN_MAIN: &str = include_str!("../../probes/shadowing/main_local_shadows_lib_function.warp");
const IN_MAIN_PRINTED: [&str; 3] = ["4", "10", "3"];

fn lines(text: &str) -> Vec<&str> {
	text.lines().collect()
}

#[test]
fn a_local_shadows_a_lib_function_on_the_rust_lane() {
	assert_eq!(lines(&crate::common::printed(IN_FUNCTIONS)), IN_FUNCTIONS_PRINTED, "{IN_FUNCTIONS}");
	assert_eq!(lines(&crate::common::printed(IN_MAIN)), IN_MAIN_PRINTED, "{IN_MAIN}");
}

#[test]
fn a_local_shadows_a_lib_function_under_warp_pure_and_self() {
	let folder = crate::common::scratch_directory("local_shadows_lib_function");
	std::fs::create_dir_all(&folder).unwrap();
	for (name, code, printed) in
		[("in_functions", IN_FUNCTIONS, &IN_FUNCTIONS_PRINTED[..]), ("in_main", IN_MAIN, &IN_MAIN_PRINTED[..])]
	{
		let program = folder.join(format!("{name}.warp"));
		std::fs::write(&program, code).unwrap();
		for command in ["pure", "self"] {
			let run = lane(command).arg(&program).output().expect("warp runs");
			let output = String::from_utf8_lossy(&run.stdout);
			assert_eq!(lines(&output), printed, "{command} {name}: {}", String::from_utf8_lossy(&run.stderr));
		}
	}
}

/// The local in a function of a used file, beside the lib function the program uses (how compiler.warp uses
/// parser.warp beside list.warp)
#[test]
fn a_used_files_local_shadows_a_lib_function() {
	let folder = crate::common::scratch_directory("local_shadows_lib_function_used");
	std::fs::create_dir_all(&folder).unwrap();
	let used = "lowest(xs: list): int := {\n\tindex = xs.length - 1\n\twhile index > 0 {\n\t\txs[index - 1] = min(xs[index - 1], xs[index])\n\t\tindex = index - 1\n\t}\n\txs[index]\n}\n";
	std::fs::write(folder.join("lowest.warp"), used).unwrap();
	let printed = crate::common::printed_in(&folder, "use list\nuse \"lowest.warp\"\nprint(index([4, 5, 6], 6))\nprint(lowest([5, 3, 4]))");
	assert_eq!(lines(&printed), ["2", "3"]);
}

/// A call of a function's local holding a number is the error of any variable applied to values, never list.warp's
/// index taking the local's arguments (it answered 1)
#[test]
fn a_call_of_a_local_named_like_a_lib_function_calls_no_function() {
	let code = "use list\nprint(index([4, 5, 6], 5))\ng(): int := {\n\tindex = 3\n\tindex([1, 2], 2)\n}\nprint(g())";
	let said = crate::common::said(code);
	assert!(said.contains("`index` is a variable here"), "{said}");
}
