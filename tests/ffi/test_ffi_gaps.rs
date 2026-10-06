// C FFI gaps (card ffi-gaps, notes/stdlib_connectors.md): a library whose header is not named after it (z → zlib.h),
// libc's character classes (ctype), a one-letter library in quotes ("z" parses as a character), sqlite3 without the
// panic on its header, and a loud refusal of pointer results (a char* crossed as a truncated i32)
#![cfg(feature = "native")]
use warp::is;

#[test]
fn a_library_header_named_otherwise_is_found() {
	is!("use z; compressBound(100)", 113);
	is!("use zlib; compressBound(100)", 113);
	is!("import compressBound from \"z\"; compressBound(1000)", 1013);
	is!("use z; adler32(1, \"abc\", 3)", 38600999);
}

#[test]
fn libc_character_classes_are_linked() {
	is!("import toupper from \"c\"; toupper(97)", 65);
	is!("import tolower from \"c\"; tolower(65)", 97);
	is!("use c; isdigit(55)", 1);
	is!("import strlen from \"c\"; strlen(\"abc\")", 3);
}

#[test]
fn sqlite3_links_without_a_panic() {
	is!("use sqlite3; sqlite3_libversion_number() > 3000000", true);
}

#[test]
fn a_pointer_result_is_refused_loudly() {
	// a C string result is a text now (test_ffi_text_results); a pointer to anything else stays refused
	is!("import getenv from \"c\"; count(getenv(\"PATH\")) > 0", true);
	is!("use c; count(getenv(\"PATH\")) > 0", true);
	is!("use sqlite3; sqlite3_db_handle(0)", 0); // a handle now (test_ffi_handles), NULL is 0
}
