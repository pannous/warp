//! The playground's controls where the user looks for them: the warn level select beside the hints and warnings it
//! filters, in the output pane (card put-warn), and a clear button for the program beside the bug button (card
//! there-should)
const PAGE: &str = include_str!("../../web/playground/index.html");
const PLAYGROUND: &str = include_str!("../../web/playground/playground.js");

fn position(text: &str) -> usize {
	PAGE.find(text).unwrap_or_else(|| panic!("index.html has {text}"))
}

#[test]
fn the_warn_level_sits_above_the_diagnostics() {
	let (output, level, diagnostics) =
		(position("id=\"output\""), position("id=\"shown-from\""), position("id=\"diagnostics\""));
	assert!(output < level && level < diagnostics, "the level select is in the output pane, just above the list");
	assert!(!PAGE[position("<nav class=\"toolbar\">")..output].contains("shown-from"), "not in the toolbar");
}

#[test]
fn a_clear_button_sits_beside_the_bug_button() {
	let (bug, clear, toolbar_end) = (position("id=\"bug\""), position("id=\"clear-code\""), position("</nav>"));
	assert!(bug < clear && clear < toolbar_end, "after the bug button, in the toolbar");
	assert!(PLAYGROUND.contains("$(\"clear-code\").onclick"), "the button clears the editor");
}
