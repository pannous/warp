//! card heading-links: `[[#Pictures]]` in the guides is the link `[Pictures](#pictures)`, and both reach any heading of the
//! guide, a `## ` chapter or a `### ` section, by one slug (GitHub's, so `(#pictures)` works where the .md is read as
//! plain Markdown too). guide.js's inline() and headingId() under node; every heading link of both guides names a heading
use std::process::Command;

const GUIDE_SCRIPT: &str = include_str!("../../web/playground/guide.js");
const GUIDES: [&str; 2] =
	[include_str!("../../web/playground/guide.md"), include_str!("../../web/playground/guide-expert.md")];
/// the inline HTML of each line given (argv[1], JSON), and the ids of the guide's headings
const RENDERED: &str = r#"
const [lines, headings] = JSON.parse(process.argv[1]);
console.log(JSON.stringify({ html: lines.map(inline), ids: headings.map(headingId) }));
"#;

struct Rendered {
	html: Vec<String>,
	ids: Vec<String>,
}

fn rendered(lines: &[&str], headings: &[&str]) -> Rendered {
	// the page's start (DOM, fetch) left out: only its functions
	let script = GUIDE_SCRIPT.replace("\nstartGuide();", "");
	let output = Command::new("node")
		.arg("-e")
		.arg(format!("{script}\n{RENDERED}"))
		.arg(serde_json::to_string(&(lines, headings)).unwrap())
		.output()
		.expect("node runs");
	assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
	let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("JSON");
	let texts =
		|key: &str| value[key].as_array().unwrap().iter().map(|item| item.as_str().unwrap().to_string()).collect();
	Rendered { html: texts("html"), ids: texts("ids") }
}

#[test]
fn a_wiki_heading_link_is_a_markdown_heading_link() {
	let shown = rendered(&["see [[#Pictures]] above", "see [Pictures](#pictures) above"], &["Pictures"]);
	assert_eq!(shown.html[0], "see <a href=\"#guide-pictures\">Pictures</a> above");
	assert_eq!(shown.html[1], shown.html[0]);
	assert_eq!(shown.ids, ["guide-pictures"]);
}

#[test]
fn a_section_and_a_titled_chapter_have_githubs_slug() {
	let shown = rendered(&["[[#Whole lists at once]]"], &["Whole lists at once", "Hello, world"]);
	assert_eq!(shown.html[0], "<a href=\"#guide-whole-lists-at-once\">Whole lists at once</a>");
	assert_eq!(shown.ids, ["guide-whole-lists-at-once", "guide-hello-world"]);
}

#[test]
fn every_heading_link_of_the_guides_names_a_heading() {
	for guide in GUIDES {
		let headings: Vec<&str> =
			guide.lines().filter_map(|line| line.strip_prefix("## ").or_else(|| line.strip_prefix("### "))).collect();
		let shown = rendered(&guide.lines().collect::<Vec<_>>(), &headings);
		for html in &shown.html {
			for link in html.split("href=\"#").skip(1) {
				let id = &link[..link.find('"').unwrap()];
				assert!(shown.ids.iter().any(|heading| heading == id), "no heading #{id} for: {html}");
			}
		}
	}
}
