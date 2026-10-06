// A C function returning char * gives a wasp text (card ffi-char-text): its NUL-terminated string is copied into the
// module, NULL is ø; other pointer results stay refused loudly; a libc function used without `use c` says how to import it
#![cfg(feature = "native")]
use warp::is;

#[test]
fn a_c_string_result_is_a_text() {
	is!("use c; getenv(\"NO_SUCH_VARIABLE_XYZ\")", warp::Node::Empty);
	is!("use c; count(getenv(\"PATH\")) > 0", true);
	is!("use c; h = getenv(\"PATH\"); count(h + \"/x\") > 2", true);
	is!("use c; strlen(getenv(\"PATH\")) > 0", true);
	is!("use sqlite3; sqlite3_errstr(1)", "SQL logic error");
	is!("use z; count(zlibVersion()) > 2", true);
}

#[test]
fn other_pointer_results_are_refused() {
	is!("use sqlite3; sqlite3_db_handle(0)", 0); // a handle now (test_ffi_handles), NULL is 0
}

#[test]
fn a_libc_function_without_use_c_says_how_to_import_it() {
	crate::common::fails_with("getenv(\"HOME\")", "getenv is a C function: write `use c`");
	crate::common::fails_with("strlen(\"abc\")", "strlen is a C function");
}
