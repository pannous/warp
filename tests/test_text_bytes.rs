//! Text as UTF-8 bytes: `+` concatenates texts and characters, `read(path)` loads a file,
//! `byte_at(text, offset)` and `byte_slice(text, start, end)` address bytes by 0-based offset

use warp::is;

#[test]
fn plus_concatenates_texts_and_characters() {
	is!("'Hello, ' + 'World!'", "Hello, World!");
	is!("'123' + '4' is '1234'", true);
	is!("'a' + 'β'", "aβ");
	is!("x=\"ab\"; y=\"cd\"; x+y", "abcd");
	is!("s=\"\"; s = s + \"𓀀\"; s", "𓀀");
}

#[test]
fn plus_equals_appends_to_a_text() {
	is!("x=\"ab\"; x += \"cd\"; x += \"e\"; x", "abcde");
	is!("f(n):={ s=\"\"; i=0; while i < n do {s += \"ab\"; i++}; s }; f(3)", "ababab");
	is!("f(t:text):= t + \"!\"; f(\"x\")", "x!");
}

#[test]
fn text_plus_number_stays_a_type_error() {
	is!("x=\"ab\"; x + 3", warp::error("type error: text + int: no implicit conversion, convert explicitly, e.g. int(\"5\") + 3"));
}

#[test]
fn byte_at_reads_one_byte() {
	is!("t=\"héllo\"; byte_at(t, 0)", 104);
	is!("t=\"héllo\"; byte_at(t, 1)", 0xC3);
	is!("t=\"héllo\"; byte_at(t, 1) * 256 + byte_at(t, 2)", 0xC3A9);
	is!("t=\"héllo\"; byte_at(t, 6)", warp::error("index out of range"));
}

#[test]
fn byte_slice_cuts_by_byte_offsets() {
	is!("t=\"héllo\"; byte_slice(t, 1, 3)", "é");
	is!("t=\"héllo\"; byte_slice(t, 3, 6)", "llo");
	is!("t=\"héllo\"; byte_slice(t, 2, 1)", warp::error("index out of range"));
}

#[test]
fn read_loads_a_file_as_bytes() {
	is!("use uniscript; index = read(\"packages/uniscript/data/entities.idx\"); byte_slice(index, 0, 4)", "USX1");
	is!("use uniscript; index = read(\"packages/uniscript/data/entities.idx\"); byte_at(index, 4)", 5);
	is!("read(\"no/such/file\")", warp::error("read no/such/file failed: No such file or directory (os error 2)"));
}
