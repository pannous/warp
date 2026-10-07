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

#[test]
fn use_math_rounds_to_places_factorial_is_prime_lerp() {
	is!("use math; round_to(1.23456, 2)", 1.23);
	is!("use math; factorial(5)", 120);
	is!("use math; [is_prime(7), is_prime(9), is_prime(2), is_prime(1)]", warp::ints(vec![1, 0, 1, 0]));
	is!("use math; lerp(0, 10, 0.25)", 2.5);
	is!("use text; [is_blank(\"  \"), is_blank(\" a\")]", warp::ints(vec![1, 0]));
}

#[test]
fn use_math_parity_digits_choose_mod_pow() {
	is!("use math; [is_even(4), is_odd(4), is_odd(-3)]", warp::ints(vec![1, 0, 1]));
	is!("use math; digits(1234)", warp::ints(vec![1, 2, 3, 4]));
	is!("use math; [choose(5, 2), choose(10, 3), choose(3, 5)]", warp::ints(vec![10, 120, 0]));
	is!("use math; mod_pow(2, 10, 1000)", 24);
}

#[test]
fn use_text_cases_prefixes_counts_palindromes() {
	is!("use text; [is_upper(\"AB\"), is_upper(\"Ab\"), is_lower(\"ab1\"), is_upper(\"12\")]", warp::ints(vec![1, 0, 1, 0]));
	is!("use text; remove_prefix(\"foobar\", \"foo\")", "bar");
	is!("use text; remove_suffix(\"foobar\", \"bar\")", "foo");
	is!("use text; remove_prefix(\"foobar\", \"x\")", "foobar");
	is!("use text; count_of(\"banana\", \"an\")", 2);
	is!("use text; [is_palindrome(\"A man, a plan, a canal: Panama\"), is_palindrome(\"abc\")]", warp::ints(vec![1, 0]));
}

#[test]
fn use_math_percent_angles_roots_factors_bases() {
	is!("use math; percent(1, 4)", 25);
	is!("use math; to_degrees(pi)", 180);
	is!("use math; to_radians(180) == pi", true);
	is!("use math; [isqrt(17), isqrt(16), isqrt(0)]", warp::ints(vec![4, 4, 0]));
	is!("use math; [is_square(16), is_square(17)]", warp::ints(vec![1, 0]));
	is!("use math; divisors(12)", warp::ints(vec![1, 2, 3, 4, 6, 12]));
	is!("use math; prime_factors(360)", warp::ints(vec![2, 2, 2, 3, 3, 5]));
	is!("use math; [to_base(255, 16), to_base(5, 2), to_base(-10, 10)]", warp::wasp_parser::parse(r#"["ff" "101" "-10"]"#));
	is!("use math; [from_base(\"ff\", 16), from_base(\"101\", 2)]", warp::ints(vec![255, 5]));
}

#[test]
fn use_text_cases_wraps_and_cuts() {
	is!("use text; word_count(\"a b  c\")", 3);
	is!("use text; [snake_case(\"helloWorld\"), snake_case(\"Hello World\"), snake_case(\"hello-world\")]", warp::wasp_parser::parse(r#"["hello_world" "hello_world" "hello_world"]"#));
	is!("use text; kebab_case(\"Hello World\")", "hello-world");
	is!("use text; camel_case(\"Hello big World\")", "helloBigWorld");
	is!("use text; indent(\"a\\nb\", 2) == \"  a\\n  b\"", true);
	is!("use text; [is_numeric(\"12.5\"), is_numeric(\"-3\"), is_numeric(\"1.2.3\"), is_numeric(\"a1\"), is_numeric(\"\")]", warp::ints(vec![1, 1, 0, 0, 0]));
	is!("use text; between(\"a[bc]d\", \"[\", \"]\")", "bc");
	is!("use text; wrap(\"aaa bbb ccc\", 7) == \"aaa bbb\\nccc\"", true);
}
