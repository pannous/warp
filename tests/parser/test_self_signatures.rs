//! Back-end focus (notes/self_hosting.md): lib/compiler/signatures.warp reads the builtins' typed prelude lines as the
//! analyzer's signature table; each answer is the Rust one's (library_words.rs emitted_call_kind, argument_result,
//! call_result_kind)
use warp::library_words::{argument_result, call_result_kind, emitted_call_kind, ArgumentResult};
use warp::type_kinds::Kind;

const MOST_ARGUMENTS: usize = 4;
const FIRST_KINDS: [(&str, Kind); 4] =
	[("int", Kind::Int), ("text", Kind::Text), ("list", Kind::List), ("empty", Kind::Empty)];

fn kind_name(kind: Option<Kind>) -> String {
	kind.map_or("-".to_string(), |kind| kind.to_string())
}

/// `count/1: int`, `sum follows: element 1`, `reverse of text: text`, one line per answer, as Rust gives them
fn rust_answers() -> Vec<String> {
	let mut answers = vec![];
	let mut seen = std::collections::HashSet::new();
	let names = warp::ffi::runtime_word_lines().iter().chain(warp::ffi::emitter_word_lines()).map(|line| line.name);
	for name in names.filter(|name| seen.insert(*name)) {
		for arguments in 0..=MOST_ARGUMENTS {
			answers.push(format!("{name}/{arguments}: {}", kind_name(emitted_call_kind(name, arguments))));
		}
		let follows = argument_result(name).map_or("-".to_string(), |(how, index)| {
			let how = match how {
				ArgumentResult::Same => "same",
				ArgumentResult::Element => "element",
			};
			format!("{how} {}", index + 1)
		});
		answers.push(format!("{name} follows: {follows}"));
		if warp::ffi::runtime_word_lines().iter().any(|line| line.name == name) {
			for (first, kind) in FIRST_KINDS {
				answers.push(format!("{name} of {first}: {}", kind_name(call_result_kind(name, || Some(kind)))));
			}
		}
	}
	answers
}

const WARP_ANSWERS: &str = r#"use "lib/compiler/signatures.warp"
answers = []
seen = []
for line in typed_lines {
	name = line.name
	if not (seen contains name) {
		seen += [name]
		for arguments in 0 to 4 { answers += ["${name}/${arguments}: " + (emitted_call_kind(name, arguments) ?? "-")] }
		follows = argument_result(name)
		answers += ["${name} follows: " + (if follows == ø then "-" else "${follows#1} ${follows#2}")]
		if #lines_of(runtime_lines, name) > 0 {
			for first in ["int", "text", "list", "empty"] { answers += ["${name} of ${first}: " + (overload_kind(name, first) ?? "-")] }
		}
	}
}
join(answers, "\n")"#;

#[test]
fn the_signature_table_answers_as_rust() {
	let warp = warp::pipeline::eval(WARP_ANSWERS);
	let warp = warp.text_value().unwrap_or_else(|| panic!("no answers: {}", warp.serialize()));
	let warp: Vec<&str> = warp.lines().collect();
	let rust = rust_answers();
	let differing: Vec<String> = rust
		.iter()
		.zip(warp.iter().copied().chain(std::iter::repeat("(none)")))
		.filter(|(rust, warp)| rust.as_str() != *warp)
		.map(|(rust, warp)| format!("rust {rust} | warp {warp}"))
		.collect();
	assert!(
		differing.is_empty() && rust.len() == warp.len(),
		"{} vs {} answers:\n{}",
		rust.len(),
		warp.len(),
		differing.join("\n")
	);
}

#[test]
fn module_source_is_the_embedded_standard_module() {
	crate::is!("module_source(\"prelude\").contains(\"count(xs: list): int from emitter\")", true);
}
