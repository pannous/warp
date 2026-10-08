// The expert guide (web/playground/guide-expert.md, card g_YD2U): the beginner guide's chapters in the same order, so
// the page's toggle keeps the open chapter, and every snippet shows its value and prints what the guide says
use std::collections::HashSet;
use warp::web::evaluate;

const EXPERT_GUIDE: &str = include_str!("../../web/playground/guide-expert.md");
const BEGINNER_GUIDE: &str = include_str!("../../web/playground/guide.md");
const FENCE_START: &str = "```warp";
const VALUE_MARK: &str = " => ";
const FENCE_END: &str = "```";
const PRINTED_FENCE: &str = "```printed\n";
const CHAPTER: &str = "## ";

/// (code, shown value, printed lines) of every snippet
fn snippets() -> Vec<(&'static str, Option<&'static str>, Option<&'static str>)> {
	EXPERT_GUIDE.split(FENCE_START).skip(1).map(|fence| {
		let (info, rest) = fence.split_once('\n').unwrap();
		let (code, after) = rest.split_once(FENCE_END).unwrap();
		let printed = after.strip_prefix('\n').and_then(|next| next.strip_prefix(PRINTED_FENCE)).map(|block| block.split(FENCE_END).next().unwrap());
		(code, info.strip_prefix(VALUE_MARK), printed)
	}).collect()
}

fn chapters(guide: &str) -> Vec<&str> {
	guide.lines().filter_map(|line| line.strip_prefix(CHAPTER)).collect()
}

#[test]
fn the_expert_guide_has_the_beginner_chapters() {
	assert_eq!(chapters(EXPERT_GUIDE), chapters(BEGINNER_GUIDE));
}

#[test]
fn every_expert_snippet_shows_its_value() {
	let failures: Vec<String> = snippets().into_iter().filter_map(|(code, value, _)| {
		let expected = value?;
		let shown = evaluate(code, HashSet::new())["value"].as_str().unwrap_or_default().to_string();
		(shown != expected).then(|| format!("{code}  shows {shown}, the guide says {expected}"))
	}).collect();
	assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[cfg(feature = "native")] // runs the warp binary
#[test]
fn every_expert_snippet_prints_what_the_guide_shows() {
	let failures: Vec<String> = snippets().into_iter().filter_map(|(code, _, printed)| {
		let expected = printed?;
		let shown: String = crate::common::printed(code).lines().filter(|line| !line.starts_with("» ")).map(|line| format!("{line}\n")).collect();
		(shown != expected).then(|| format!("{code}  prints {shown:?}, the guide says {expected:?}"))
	}).collect();
	assert!(failures.is_empty(), "{}", failures.join("\n"));
}
