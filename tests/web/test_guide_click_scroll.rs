//! card clicking-guide: a guide chapter opened by a click scrolls to the top of the guide, after the chapter open before
//! it closed; one the page opens itself (the address, the first chapter) keeps its place. guide.js's openedChapter
//! under node, the DOM stubbed
use std::process::Command;

const GUIDE_SCRIPT: &str = include_str!("../../web/playground/guide.js");
/// opens a chapter, clicked or not (argv[1]), and prints how it scrolled
const OPENED: &str = r#"
const clicked = process.argv[1] === "clicked";
let scrolled = null;
globalThis.document = { querySelectorAll: () => [] };
globalThis.location = { hash: "" };
globalThis.history = { replaceState() {} };
globalThis.$ = () => ({});
const summary = { textContent: "Lists" };
const chapter = { open: true, id: "guide-lists", dataset: clicked ? { clicked: "" } : {}, contains: () => false,
	querySelector: () => summary, scrollIntoView: options => { scrolled = options; } };
openedChapter({ target: chapter });
console.log(JSON.stringify(scrolled));
"#;

fn scrolled(how: &str) -> String {
	let script = GUIDE_SCRIPT.replace("\nstartGuide();", "");
	let output =
		Command::new("node").arg("-e").arg(format!("{script}\n{OPENED}")).arg(how).output().expect("node runs");
	assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
	String::from_utf8_lossy(&output.stdout).trim().to_string()
}

#[test]
fn a_clicked_chapter_scrolls_to_the_top() {
	assert_eq!(scrolled("clicked"), r#"{"block":"start"}"#);
}

#[test]
fn a_chapter_the_page_opens_keeps_its_place() {
	assert_eq!(scrolled("opened"), "null");
}
