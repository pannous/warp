//! card library-word-tables: the word tables of lib/compiler/words.warp against their Rust twins, over the words the
//! sources use
use crate::common::twins::sources;
use std::collections::BTreeSet;
use warp::warp_parser::parse;
use warp::Node;

/// The texts of a warp list of lib/compiler/words.warp, in order
fn warp_list(name: &str) -> Vec<String> {
	let table = warp::pipeline::eval(&format!("use \"lib/compiler/words.warp\"; {name}"));
	let Node::List(items, ..) = table.drop_meta() else { panic!("{name} is no list: {}", table.serialize()) };
	items
		.iter()
		.map(|item| item.text_value().unwrap_or_else(|| panic!("{name}: no text {}", item.serialize())))
		.collect()
}

fn warp_table(name: &str) -> BTreeSet<String> {
	warp_list(name).into_iter().collect()
}

/// Every symbol the sources use, lowercase
fn source_words() -> BTreeSet<String> {
	let mut words = BTreeSet::new();
	for path in sources() {
		let Ok(text) = std::fs::read_to_string(&path) else { continue };
		parse(&text).visit(&mut |part| {
			if let Node::Symbol(name) = part {
				words.insert(name.to_lowercase());
			}
		});
	}
	words
}

#[test]
fn type_words_are_the_rust_type_words() {
	let is_type_word = |word: &str| warp::analyzer::type_word_kind(word).is_some();
	let table = warp_table("type_words");
	let not_rust: Vec<&String> = table.iter().filter(|word| !is_type_word(word)).collect();
	assert!(not_rust.is_empty(), "type_words Rust doesn't take: {not_rust:?}");
	let missing: Vec<String> =
		source_words().into_iter().filter(|word| is_type_word(word) && !table.contains(word)).collect();
	assert!(missing.is_empty(), "type words missing in lib/compiler/words.warp: {missing:?}");
}

#[test]
fn copied_tables_are_their_rust_constants() {
	assert_eq!(warp_list("function_keywords"), warp::operators::FUNCTION_KEYWORDS);
	assert_eq!(warp_list("selector_words"), warp::lowering::of_lookups::SELECTOR_WORDS);
	let element_wise: Vec<&str> = warp::operators::ELEMENT_WISE_OPERATORS.iter().map(|op| op.as_str()).collect();
	assert_eq!(warp_list("element_wise_operators"), element_wise);
}

/// The kind Rust's analyzer gives `word(0, …)` of each number of arguments up to six
fn rust_call_kinds(word: &str) -> Vec<String> {
	let scope = warp::analyzer::Scope::new();
	(0..=6)
		.map(|arity| {
			let call = format!("{word}({})", vec!["0"; arity].join(", "));
			warp::analyzer::infer_type(&parse(&call), &scope).to_string()
		})
		.collect()
}

#[test]
fn call_kinds_are_rusts() {
	let table = warp::pipeline::eval("use \"lib/compiler/words.warp\"; call_kinds");
	let Node::List(entries, ..) = table.drop_meta() else { panic!("call_kinds is no map: {}", table.serialize()) };
	let differing: Vec<String> = entries
		.iter()
		.filter_map(|entry| {
			let Node::Key(word, _, kind) = entry.drop_meta() else { panic!("no `word: kind` {}", entry.serialize()) };
			let (word, kind) = (word.text_value().expect("a word text").to_string(), kind.text_value().expect("a kind text").to_string());
			let rust = rust_call_kinds(&word);
			(!rust.contains(&kind)).then(|| format!("{word}: warp {kind}, rust {rust:?}"))
		})
		.collect();
	assert!(differing.is_empty(), "call_kinds differ from Rust's analyzer:\n{}", differing.join("\n"));
}
