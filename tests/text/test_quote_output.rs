//! Printed and serialized strings use the canonical quote of `Style.quotes`; one-character codepoints keep their single quotes
use crate::is;
use std::sync::Mutex;
use warp::normalize::*;
use warp::wasm_emitter::eval;
use warp::warp_parser::{parse, WarpParser};

/// The style is global: tests that read or swap it run one at a time
static GLOBAL_STYLE: Mutex<()> = Mutex::new(());

fn with_quotes<T>(quotes: QuoteStyle, action: impl FnOnce() -> T) -> T {
	let _guard = GLOBAL_STYLE.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
	set_style(Style { quotes, ..Style::default() });
	let result = action();
	set_style(Style::default());
	result
}

#[test]
fn test_evaluated_text_prints_with_the_canonical_quote() {
	with_quotes(QuoteStyle::Double, || {
		assert_eq!(eval("\"abc\"").serialize(), "\"abc\"");
	});
	with_quotes(QuoteStyle::Single, || assert_eq!(eval("\"abc\"").serialize(), "'abc'"));
}

#[test]
fn test_codepoints_keep_single_quotes() {
	with_quotes(QuoteStyle::Double, || {
		assert_eq!(eval("'a'").serialize(), "'a'");
		assert_eq!(eval("65 as char").serialize(), "'A'");
	});
}

#[test]
fn test_texts_inside_lists_print_with_the_canonical_quote() {
	with_quotes(QuoteStyle::Double, || assert_eq!(eval("[\"ab\" \"cd\"]").serialize(), "[\"ab\" \"cd\"]"));
}

#[test]
fn test_serialized_text_parses_back() {
	for quotes in [QuoteStyle::Double, QuoteStyle::Single] {
		with_quotes(quotes, || {
			let node = WarpParser::parse("[\"hello\" \"a b\"]");
			assert_eq!(parse(&node.serialize()), node);
			let keyed = WarpParser::parse("name:\"Alice\"");
			assert_eq!(parse(&keyed.serialize()), keyed);
		});
	}
}

#[test]
fn test_is_macro_round_trips_text() {
	is!("\"abc\"", "abc");
	is!("x = \"ab\"; x", "ab");
}
