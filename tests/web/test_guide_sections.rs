// card guide-lists: "Whole lists at once" is a section of the Lists chapter, in the beginner and the expert guide
const GUIDES: [&str; 2] = [include_str!("../../web/playground/guide.md"), include_str!("../../web/playground/guide-expert.md")];

/// The chapter (`## title`) whose text holds the line
fn chapter_holding(guide: &str, line: &str) -> Option<String> {
	let mut chapter = None;
	for text in guide.lines() {
		if let Some(title) = text.strip_prefix("## ") {
			chapter = Some(title.to_string());
		}
		if text == line {
			return chapter;
		}
	}
	None
}

#[test]
fn whole_lists_at_once_is_a_section_of_lists() {
	for guide in GUIDES {
		assert_eq!(chapter_holding(guide, "### Whole lists at once").as_deref(), Some("Lists"));
		assert!(!guide.lines().any(|line| line == "## Whole lists at once"), "no chapter of its own");
	}
}
