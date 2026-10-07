//! The standard library modules math and text (notes/stdlib.md)
use crate::is;

#[test]
fn use_math_brings_libm_and_the_module() {
	is!("use math; gcd(12, 18)", 6);
	is!("use math; lcm(4, 6)", 12);
	is!("use math; clamp(15, 0, 10) + clamp(-2, 0, 10) + clamp(5, 0, 10)", 15);
	is!("use math; sign(-3)", -1);
	is!("use math; floor(2.7)", 2);
}

#[test]
fn use_text_brings_its_words() {
	is!("use text; \"[\" + pad_left(\"ab\", 5) + \"]\"", "[   ab]");
	is!("use text; \"[\" + pad_right(\"ab\", 5) + \"]\"", "[ab   ]");
	is!("use text; repeat(\"ab\", 3)", "ababab");
}

#[test]
fn use_text_formats_a_template() {
	is!("use text; format(\"{} has {} items\", [\"cart\", 3])", "cart has 3 items");
	is!("use text; format(\"{}{}\", [1, 2])", "12");
	is!("use text; format(\"none\", [])", "none");
}

#[test]
fn use_text_brings_words_lines_capitalize_center() {
	// words is a comprehension: a module's source gets the program's early passes (pipeline::lower_module_source)
	is!("use text; count(words(\" a b  c \"))", 3);
	is!("use text; count(lines(\"a\\nb\"))", 2);
	is!("use text; capitalize(\"abc\")", "Abc");
	is!("use text; \"[\" + center(\"ab\", 6) + \"]\"", "[  ab  ]");
}

#[test]
fn use_text_brings_title_slug_truncate() {
	is!("use text; title(\"hello big world\")", "Hello Big World");
	is!("use text; slug(\"Hello, Big World!\")", "hello-big-world");
	is!("use text; truncate(\"abcdef\", 4)", "abc…");
	is!("use text; truncate(\"abc\", 4)", "abc");
}

#[test]
fn comprehensions_in_module_and_program_ask_nothing() {
	warp::diagnostic::take_warnings();
	let value = warp::wasm_emitter::eval("use text; [capitalize(w) for w in words(\"ab cd\")]#2");
	let warnings: Vec<String> = warp::diagnostic::take_warnings().iter().map(|warning| warning.to_string()).collect();
	assert_eq!(value, warp::Node::text("Cd"));
	assert!(warnings.iter().all(|warning| !warning.contains("new local")), "{warnings:?}");
}
