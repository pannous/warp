//! card library-word-tables: the word tables of lib/compiler/words.warp against their Rust twins, over the words the
//! sources use
use crate::common::twins::sources;
use std::collections::BTreeSet;
use warp::warp_parser::parse;
use warp::Node;

/// The texts of a warp list of lib/compiler/words.warp
fn warp_table(name: &str) -> BTreeSet<String> {
	let table = warp::pipeline::eval(&format!("use \"lib/compiler/words.warp\"; {name}"));
	let Node::List(items, ..) = table.drop_meta() else { panic!("{name} is no list: {}", table.serialize()) };
	items
		.iter()
		.map(|item| item.text_value().unwrap_or_else(|| panic!("{name}: no text {}", item.serialize())))
		.collect()
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
