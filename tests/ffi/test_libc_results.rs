// libc through FFI ("c") returns the C result types, in the browser too, where host.js stands in for libc:
// size_t and long are i64 there (a BigInt), int is i32
use crate::is;

#[test]
fn strlen_returns_a_size() {
	is!("import strlen from \"c\"\nn = strlen(\"hello\")\nn + 1", 6);
}

#[test]
fn strlen_inside_a_program() {
	is!("import abs from \"c\"\nabs(-42)\nimport strlen from \"c\"\nstrlen(\"hello\")\n7", 7);
}

#[test]
fn atol_returns_a_long() {
	is!("import atol from \"c\"\natol(\"12345678901\")", 12345678901i64);
}

#[test]
fn strncmp_compares_a_prefix() {
	is!("import strncmp from \"c\"\nstrncmp(\"hello\", \"help\", 3)", 0);
}

#[test]
fn strcmp_orders_texts() {
	is!("import strcmp from \"c\"\nstrcmp(\"abc\", \"abd\")", -1);
	is!("import strcmp from \"c\"\nstrcmp(\"same\", \"same\")", 0);
}

// the browser calls libc.wasm for these (P147, web/playground/lib): the same results as the system libc natively
#[test]
fn libc_text_and_character_functions() {
	is!("use c\nstrstr(\"haystack\", \"st\")", "stack");
	is!("use c\nstrrchr(\"a/b/c\", 47)", "/c");
	is!("use c\ntoupper(97) + tolower(66)", 163);
	is!("use c\nisdigit(55) > 0", true);
	is!("use c\natoi(\"-12\") + strlen(\"abc\")", -9);
	is!("use c\nstrcspn(\"hello world\", \" \")", 5);
	is!("use c\nstrchr(\"abc\", 120)", warp::Node::Empty);
}

// a header's comment prose declares nothing: glibc's ctype.h says "because tolower (EOF) must be EOF" in a block
// comment, which the cached libc table took for tolower (a text result: "wasm trap: cast failure" in Linux CI)
#[test]
fn comment_prose_in_a_header_declares_nothing() {
	let declared = warp::ffi_parser::parse_header_file("tests/fixtures/c/comment_prose.h", "c");
	assert_eq!(declared.len(), 1, "{declared:?}");
	assert_eq!(declared[0].name, "tolower");
}
