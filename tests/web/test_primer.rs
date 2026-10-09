// The language primer (web/playground/primer.md, card language-primer): the system prompt of the playground's
// assistant and the site's /llms.txt. Every snippet runs: a `=> v` block shows v, a plain block runs without error,
// a `compiles` block (servers, paint, C libraries) compiles, and a `printed` block is what the snippet before prints
use std::collections::HashSet;
use warp::web::evaluate;

const PRIMER: &str = include_str!("../../web/playground/primer.md");
const FENCE_START: &str = "```warp";
const VALUE_MARK: &str = " => ";
const COMPILE_MARK: &str = " compiles";
const FENCE_END: &str = "```";
const PRINTED_FENCE: &str = "```printed\n";
const MOST_CHARACTERS: usize = 16_000; // about 4k tokens

struct Snippet {
	code: &'static str,
	head: &'static str,
	printed: Option<&'static str>,
}

fn snippets() -> Vec<Snippet> {
	PRIMER.split(FENCE_START).skip(1).map(|fence| {
		let (head, rest) = fence.split_once('\n').unwrap();
		let (code, after) = rest.split_once(FENCE_END).unwrap();
		let printed = after.strip_prefix('\n').and_then(|next| next.strip_prefix(PRINTED_FENCE)).map(|block| block.split(FENCE_END).next().unwrap());
		Snippet { code, head, printed }
	}).collect()
}

fn failures_of(check: impl Fn(&Snippet) -> Option<String>) -> String {
	snippets().iter().filter_map(check).collect::<Vec<_>>().join("\n")
}

#[test]
fn the_primer_fits_a_system_prompt() {
	assert!(PRIMER.len() < MOST_CHARACTERS, "the primer has {} characters", PRIMER.len());
}

#[test]
fn every_primer_snippet_runs_to_its_value() {
	let failures = failures_of(|snippet| {
		if snippet.head == COMPILE_MARK { return None; }
		let result = evaluate(snippet.code, HashSet::new());
		let shown = result["value"].as_str().unwrap_or_default().to_string();
		let wrong = match snippet.head.strip_prefix(VALUE_MARK) {
			Some(expected) => shown != expected,
			None => shown.is_empty() || shown.starts_with("Error"),
		};
		wrong.then(|| format!("{}  shows {shown}, the primer says {}", snippet.code, snippet.head))
	});
	assert!(failures.is_empty(), "{failures}");
}

#[cfg(feature = "native")] // the FFI import needs the native libraries
#[test]
fn every_primer_compile_snippet_compiles() {
	let failures = failures_of(|snippet| {
		if snippet.head != COMPILE_MARK { return None; }
		let failure = warp::pipeline::compile(snippet.code).err()?;
		Some(format!("{}  fails: {failure:?}", snippet.code))
	});
	assert!(failures.is_empty(), "{failures}");
}

#[cfg(feature = "native")] // runs the warp binary
#[test]
fn every_primer_snippet_prints_what_the_primer_shows() {
	let failures = failures_of(|snippet| {
		let expected = snippet.printed?;
		let shown: String = crate::common::printed(snippet.code).lines().filter(|line| !line.starts_with("» ")).map(|line| format!("{line}\n")).collect();
		(shown != expected).then(|| format!("{}  prints {shown:?}, the primer says {expected:?}", snippet.code))
	});
	assert!(failures.is_empty(), "{failures}");
}
