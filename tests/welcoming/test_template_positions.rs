//! An error inside an expanded library template names the value the word was given, not the template's own column
//! (card template-positions: `sum 1` said 'not a list at 1:78')
use crate::common::fails_with;

#[test]
fn template_errors_name_the_value_given() {
	fails_with("sum 1", "not a list at 1:5");
	fails_with("x = 2\ny = sum x", "not a list at 2:9");
	fails_with("square x = x*x; map 3 square", "not a list at 1:21");
	fails_with("each 3 print", "not a list at 1:6");
}

/// Every template a pass writes outside the parser goes through parse_generated or parse_template, so an error inside
/// one never names the template's own column (reviewer, card template-positions); native only: the browser build has
/// no source folder
#[test]
#[cfg(feature = "native")]
fn generated_templates_parse_without_their_own_positions() {
	fn plain_template_parses(dir: &std::path::Path, found: &mut Vec<String>) {
		for entry in std::fs::read_dir(dir).expect("source dir").flatten() {
			let path = entry.path();
			if path.ends_with("warp_parser") {
				continue;
			}
			if path.is_dir() {
				plain_template_parses(&path, found);
			} else if path.extension().is_some_and(|extension| extension == "rs") {
				let source = std::fs::read_to_string(&path).expect("source file");
				let lines = source.lines().enumerate().filter(|(_, line)| line.contains("parse(&format!"));
				found.extend(lines.map(|(index, _)| format!("{}:{}", path.display(), index + 1)));
			}
		}
	}
	let mut found = vec![];
	plain_template_parses(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src"), &mut found);
	assert!(found.is_empty(), "use warp_parser::parse_generated for these templates: {found:?}");
}
