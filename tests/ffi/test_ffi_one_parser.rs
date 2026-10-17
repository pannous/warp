//! One C header parser (card cleanup-ffi-headers): the calls of libc and libm as the program sees them, pinned before
//! ffi_parser.rs and ffi/header.rs became one; `use c` now leaves strtol's `char **` endptr out as an import does
//! (card ffi-header)
use crate::is;

fn with_c(code: &str) -> String {
	format!("use c\n{code}")
}

#[test]
fn libm_calls_need_no_import() {
	is!("sqrt(16)", 4);
	is!("cbrt(27)", 3);
	is!("floor(2.5)", 2);
}

#[test]
fn libc_numbers_and_texts() {
	is!(&with_c("strlen(\"abc\")"), 3);
	is!(&with_c("atoi(\"42\")"), 42);
	is!(&with_c("labs(-5) + llabs(-7)"), 12);
	is!(&with_c("fabs(-2.5)"), 2.5);
	is!(&with_c("strerror(2)"), "No such file or directory");
}

#[test]
fn a_file_is_a_handle() {
	is!(&with_c("f = fopen(\"/dev/null\", \"r\")\nfclose(f)"), 0);
}

#[test]
fn a_char_pointer_pointer_parameter_is_left_out() {
	is!(&with_c("strtod(\"3.5\")"), 3.5);
	is!(&with_c("strtol(\"ff\", 16)"), 255);
}

/// The cached table against the headers' tables, one line per function whose signature differs: run with --ignored
/// --nocapture to review what one parser changes
#[test]
#[ignore]
fn table_differences() {
	let cached = warp::ffi::get_ffi_signatures();
	let mut lines: Vec<String> = cached
		.iter()
		.filter(|(_, signature)| ["m", "c", "SDL2"].contains(&signature.library))
		.filter_map(|(name, signature)| {
			let declared = warp::ffi::get_signatures_from_headers(signature.library).get(name);
			let shown = |s: &warp::ffi::FfiSignature| format!("{:?} -> {:?}", s.params, s.results);
			match declared {
				Some(declared) if shown(declared) == shown(signature) => None,
				Some(declared) => Some(format!("{name}: {} | header {}", shown(signature), shown(declared))),
				None => Some(format!("{name}: {} | header none", shown(signature))),
			}
		})
		.collect();
	lines.sort();
	println!("{} differ\n{}", lines.len(), lines.join("\n"));
}

#[test]
fn a_long_double_function_is_refused_by_name() {
	crate::common::fails_with(&with_c("sqrtl(4)"), "sqrtl is a C function using long double");
}
