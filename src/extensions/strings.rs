use std::fmt::Debug; // for println!("{:?}", item)
use std::fmt::Display; // for println!("{}", item)
					   // use crate::put;

#[allow(dead_code)]
#[allow(non_snake_case)]
pub fn String(s: &str) -> String {
	// ⚠️ pseudo constructor no conflict with std::string::String ??
	s.to_string()
}

#[allow(dead_code)]
#[allow(non_snake_case)]
pub fn S(s: &str) -> String {
	// String.from(s)
	s.to_string()
}

// byte literal b'A' b"hello" &[u8]
pub trait CharExtensions {
	fn upper(&self) -> char;
	fn s(&self) -> String;
}
impl CharExtensions for char {
	fn upper(&self) -> char {
		self.to_uppercase().next().unwrap()
	}
	fn s(&self) -> String {
		self.to_string()
	}
}

pub trait StringExtensions {
	fn at(&self, nr: i32) -> char;
	fn codepoint_at(&self, nr: i32) -> char;
	fn char(&self, nr: usize) -> char;
	fn upper(&self) -> String;
	fn reverse(&self) -> String;
	fn map(&self, f: fn(char) -> char) -> String;
	fn substring(&self, start: usize, end: usize) -> &str;
	fn first_char(&self) -> char;
	fn last_char(&self) -> char;
	fn first(&self) -> char;
	fn start(&self) -> char;
	fn head(&self) -> char;
	fn byte_at(&self, nr: usize) -> u8;
	fn str(&self) -> String;
	// allow negative index into chars : -2 = next to last
	fn s(&self) -> String;
	// from is reserved for String.from("…") constructor
	// fn from(&self, start: usize) -> &str;
	fn start_from(&self, start: usize) -> &str;
	fn set(&self, index: usize, c: char) -> String;
	// return the slice after the first occurrence of `pat`, or empty slice if not found
	fn after(&self, pat: &str) -> &str;
}

impl StringExtensions for String {
	fn at(&self, nr: i32) -> char {
		let wrapped_index = (nr + self.chars().count() as i32) as usize % self.chars().count();
		self.chars().nth(wrapped_index).unwrap()
	}
	fn codepoint_at(&self, nr: i32) -> char {
		self.at(nr)
	}
	fn char(&self, nr: usize) -> char {
		self.chars().nth(nr).unwrap()
	}
	fn upper(&self) -> String {
		self.to_uppercase()
	}
	fn reverse(&self) -> String {
		self.chars().rev().collect()
	}
	fn map(&self, f: fn(char) -> char) -> String {
		self.chars().map(f).collect()
	}
	fn substring(&self, start: usize, end: usize) -> &str {
		// just use the range operator directly
		&self[start..end]
	}
	fn first_char(&self) -> char {
		self.chars().next().unwrap()
	}
	fn last_char(&self) -> char {
		self.chars().last().unwrap()
	}
	fn first(&self) -> char {
		self.chars().next().unwrap()
	}
	fn start(&self) -> char {
		self.chars().next().unwrap()
	}
	fn head(&self) -> char {
		self.chars().next().unwrap()
	}
	fn byte_at(&self, nr: usize) -> u8 {
		self.as_bytes()[nr]
	}
	fn str(&self) -> String {
		self.to_owned()
	}
	// Function implementations
	fn s(&self) -> String {
		self.to_owned()
	}
	// fn from(&self, start: usize) -> &str { &self[start..] }
	fn start_from(&self, start: usize) -> &str {
		&self[start..]
	}
	fn set(&self, at: usize, value: char) -> String {
		self.clone().replace_range(at..at + 1, &value.to_string());
		self.to_string()
	}
	fn after(&self, pat: &str) -> &str {
		match self.find(pat) {
			Some(idx) => &self[idx + pat.len()..],
			None => &self[self.len()..],
		}
	}
	// fn start_from(&self, start: usize) -> &str { panic!("just use &s[start..] ") }
}

impl StringExtensions for str {
	fn at(&self, nr: i32) -> char {
		let wrapped_index = (nr + self.chars().count() as i32) as usize % self.chars().count();
		self.chars().nth(wrapped_index).unwrap()
	}
	fn codepoint_at(&self, nr: i32) -> char {
		self.at(nr)
	}
	fn char(&self, nr: usize) -> char {
		self.chars().nth(nr).unwrap()
	}
	fn upper(&self) -> String {
		self.to_uppercase()
	}
	fn reverse(&self) -> String {
		self.chars().rev().collect()
	}
	fn map(&self, f: fn(char) -> char) -> String {
		self.chars().map(f).collect()
	}
	fn substring(&self, start: usize, end: usize) -> &str {
		// just use the range operator directly
		&self[start..end]
	}
	fn first_char(&self) -> char {
		self.chars().next().unwrap()
	}
	fn last_char(&self) -> char {
		self.chars().last().unwrap()
	}
	fn first(&self) -> char {
		self.chars().next().unwrap()
	}
	fn start(&self) -> char {
		self.chars().next().unwrap()
	}
	fn head(&self) -> char {
		self.chars().next().unwrap()
	}
	fn byte_at(&self, nr: usize) -> u8 {
		self.as_bytes()[nr]
	}
	fn str(&self) -> String {
		self.to_string()
	}
	// allow negative index into chars : -2 = next to last
	fn s(&self) -> String {
		self.to_string()
	}
	fn start_from(&self, start: usize) -> &str {
		&self[start..]
	}
	// fn start_from(&self, start: usize) -> &str { panic!("just use &s[start..] ") }

	fn set(&self, at: usize, value: char) -> String {
		let mut changed = self.to_string();
		changed.replace_range(at..at + 1, &value.to_string());
		changed
	}

	fn after(&self, pat: &str) -> &str {
		match self.find(pat) {
			Some(idx) => &self[idx + pat.len()..],
			None => &self[self.len()..],
		}
	}
}

// by value
// call with &arg if you encounter "Borrow of moved value" error (later)
pub fn print_list<T: Display + Debug>(list: impl IntoIterator<Item = T>) {
	for item in list {
		println!("{}", item);
	}
}



// use std::cmp::PartialEq;

/// `'l'.is("l")`: a character equal to a one-character text (used by the unit tests below)
#[cfg(test)]
trait PartialEqStr {
	fn is(&self, other: &str) -> bool;
}

#[cfg(test)]
impl PartialEqStr for char {
	fn is(&self, other: &str) -> bool {
		other.len() == 1 && other.starts_with(*self)
	}
}

// Test it see tests/string_tests.rs !!


/// Code points that continue the grapheme cluster before them: combining marks, variation selectors,
/// emoji skin tone modifiers, tags, ZWNJ and ZWJ (UAX #29 Extend, SpacingMark and ZWJ, abridged to common scripts).
/// The emitted `grapheme_end` runtime and `grapheme_clusters` share this table, so both count the same unit.
pub const GRAPHEME_EXTEND: [(u32, u32); 27] = [
	(0x0300, 0x036F), // combining diacritical marks
	(0x0483, 0x0489), // Cyrillic
	(0x0591, 0x05BD), // Hebrew points
	(0x05C1, 0x05C2),
	(0x05C4, 0x05C5),
	(0x0610, 0x061A), // Arabic
	(0x064B, 0x065F),
	(0x0670, 0x0670),
	(0x06D6, 0x06DC),
	(0x0900, 0x0903), // Devanagari signs and vowel marks
	(0x093A, 0x093C),
	(0x093E, 0x094F),
	(0x0951, 0x0957),
	(0x0962, 0x0963),
	(0x0E31, 0x0E31), // Thai
	(0x0E34, 0x0E3A),
	(0x0E47, 0x0E4E),
	(0x1AB0, 0x1AFF), // combining marks extended and supplement
	(0x1DC0, 0x1DFF),
	(0x200C, 0x200D), // ZWNJ, ZWJ
	(0x20D0, 0x20FF), // combining marks for symbols
	(0x3099, 0x309A), // kana voicing marks
	(0xFE00, 0xFE0F), // variation selectors
	(0xFE20, 0xFE2F), // combining half marks
	(0x1F3FB, 0x1F3FF), // emoji skin tone modifiers
	(0xE0020, 0xE007F), // tags (subdivision flags)
	(0xE0100, 0xE01EF), // variation selectors supplement
];
/// After a ZWJ these join the cluster (emoji ZWJ sequences, abridged Extended_Pictographic)
pub const GRAPHEME_PICTOGRAPHIC: [(u32, u32); 2] = [(0x2600, 0x27BF), (0x1F000, 0x1FAFF)];
/// Flags are pairs of regional indicators
pub const REGIONAL_INDICATORS: (u32, u32) = (0x1F1E6, 0x1F1FF);
pub const ZERO_WIDTH_JOINER: u32 = 0x200D;

fn in_ranges(c: u32, ranges: &[(u32, u32)]) -> bool {
	ranges.iter().any(|&(low, high)| c.wrapping_sub(low) <= high - low)
}

/// Controls end a cluster at once; CR LF is the one pair that stays together
pub fn is_control(c: u32) -> bool {
	c < 0x20 || (0x7F..0xA0).contains(&c)
}

/// Whether `next` continues a cluster whose last code point is `previous`
pub fn grapheme_joins(previous: u32, next: u32, unpaired_regional_indicator: bool) -> bool {
	in_ranges(next, &GRAPHEME_EXTEND)
		|| (previous == ZERO_WIDTH_JOINER && in_ranges(next, &GRAPHEME_PICTOGRAPHIC))
		|| (unpaired_regional_indicator && in_ranges(next, &[REGIONAL_INDICATORS]))
}

/// The user-perceived characters of `text` (extended grapheme clusters, abridged): `'👍🏽'` is one,
/// `"🇩🇪"` is one, `"e\u{301}"` is one. `#`, `count`, `length` count these; `size` counts bytes.
pub fn grapheme_clusters(text: &str) -> Vec<&str> {
	let mut clusters = vec![];
	let mut chars = text.char_indices().peekable();
	while let Some((start, first)) = chars.next() {
		let mut previous = first as u32;
		let mut end = start + first.len_utf8();
		if previous == 0x0D && matches!(chars.peek(), Some((_, '\n'))) {
			chars.next();
			end += 1;
		} else if !is_control(previous) {
			let mut unpaired = in_ranges(previous, &[REGIONAL_INDICATORS]);
			while let Some(&(at, next)) = chars.peek() {
				if !grapheme_joins(previous, next as u32, unpaired) {
					break;
				}
				chars.next();
				unpaired = false;
				previous = next as u32;
				end = at + next.len_utf8();
			}
		}
		clusters.push(&text[start..end]);
	}
	clusters
}

/// Edit distance (insertions, deletions, substitutions, adjacent swaps) between two words
pub fn edit_distance(a: &str, b: &str) -> usize {
	let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
	let mut rows = vec![vec![0usize; b.len() + 1]; a.len() + 1];
	for (i, row) in rows.iter_mut().enumerate() {
		row[0] = i;
	}
	for (j, cell) in rows[0].iter_mut().enumerate() {
		*cell = j;
	}
	for i in 1..=a.len() {
		for j in 1..=b.len() {
			let cost = usize::from(a[i - 1] != b[j - 1]);
			rows[i][j] = (rows[i - 1][j] + 1).min(rows[i][j - 1] + 1).min(rows[i - 1][j - 1] + cost);
			if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
				rows[i][j] = rows[i][j].min(rows[i - 2][j - 2] + 1);
			}
		}
	}
	rows[a.len()][b.len()]
}

/// The first of the names one letter away from `word` (two for words of six letters or more): `pirnt` → print
pub fn near_miss(word: &str, names: impl IntoIterator<Item = String>) -> Option<String> {
	let allowed = if word.chars().count() < 6 { 1 } else { 2 };
	names.into_iter().filter(|name| name != word).find(|name| edit_distance(word, name) <= allowed)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_char_str_eq() {
		assert!('l'.is("l"));
		// assert!('1'.is(1));
		// assert!("l".is('l'));
	}

	#[test]
	fn test_str_plus() {
		// eq!("a"+"b", "ab");
		assert_eq!("a".s() + "b", "ab");
		// eq!("a".s()+2, "a2");
	}
}
