//! CSS: hex colors and selectors
use super::*;

/// The digits of a CSS hex color: `#fff`, `#8884`, `#1e1e1e`, `#00ff0080`
const CSS_COLOR_DIGITS: [usize; 4] = [3, 4, 6, 8];
/// What follows an id selector of hex letters (`#fed { … }`, `#add, #b`, `#bad:hover`), never a color
const SELECTOR_FOLLOWERS: [char; 8] = ['{', ',', '.', ':', '>', '+', '~', '['];
/// What a style sheet's selector starts after: the sheet's or a rule's brace, a declaration's end, a selector list's comma
const SELECTOR_STARTS_AFTER: [char; 5] = ['{', '}', ';', ',', '\0'];

impl WarpParser {
	/// In a style sheet `#8884` or `#fff` in a value is a CSS hex color, not the count or index operator (card
	/// css-block): its length with the `#`. Ids starting with a letter stay selectors where one is followed by a block
	/// or another selector part (`#fed { … }`); an id never starts with a digit
	pub(crate) fn css_color_length(&self) -> Option<usize> {
		if !self.in_style_sheet || self.current_char() != '#' {
			return None;
		}
		let digits = (1..).take_while(|&offset| self.peek_char(offset).is_ascii_hexdigit()).count();
		let after = self.peek_char(digits + 1);
		if !CSS_COLOR_DIGITS.contains(&digits) || after.is_alphanumeric() || matches!(after, '_' | '-') {
			return None;
		}
		let next = (digits + 1..).map(|offset| self.peek_char(offset)).find(|ch| !ch.is_whitespace()).unwrap_or('\0');
		(self.peek_char(1).is_ascii_digit() || !SELECTOR_FOLLOWERS.contains(&next)).then_some(digits + 1)
	}

	/// `:root { … }`, `:hover { … }` starting a rule of a style sheet: a selector, not the colon of a declaration (card
	/// markup-style-block)
	pub(crate) fn at_pseudo_class_selector(&self) -> bool {
		let previous = self.chars[..self.pos].iter().rev().find(|ch| !ch.is_whitespace()).copied().unwrap_or('\0');
		self.in_style_sheet
			&& self.current_char() == ':'
			&& self.is_identifier_start(1)
			&& SELECTOR_STARTS_AFTER.contains(&previous)
	}
}
