//! card consists-characters: "שָלוֹם" consists of 4 characters (grapheme clusters: a letter with its vowel point) and 6
//! code points, so the loop by character and the loop by code point differ, as their counts do. `char` is short for
//! character (user, 2026-10-16: "char should be a shortcut for character, not code point, if they are distinct")
use crate::is;

const SHALOM: &str = "s = \"שָלוֹם\"";

fn shalom(code: &str) -> String {
	format!("{SHALOM}; {code}")
}

#[test]
fn shalom_counts_four_characters_and_six_code_points() {
	for four in ["count s", "s.graphemes", "count(s.chars)", "number of chars in s", "number of characters in s"] {
		is!(&shalom(four), 4);
	}
	for six in ["s.codepoints", "number of codepoints in s"] {
		is!(&shalom(six), 6);
	}
}

#[test]
fn the_chars_of_a_text_are_its_characters() {
	is!(&shalom("s.chars"), warp::parse("[\"שָ\" 'ל' \"וֹ\" 'ם']"));
	is!("\"abc\".chars", warp::parse("['a' 'b' 'c']"));
}

#[test]
fn a_loop_by_character_walks_the_characters() {
	for unit in ["c", "each char", "each character", "each grapheme", "chars", "characters"] {
		is!(&shalom(&format!("n = 0; for {unit} in s {{ n += 1 }}; n")), 4);
	}
	is!(&shalom("[c for c in s]#1"), "שָ");
}

#[test]
fn a_loop_by_code_point_walks_the_code_points() {
	for unit in ["each codepoint", "codepoints"] {
		is!(&shalom(&format!("n = 0; for {unit} in s {{ n += 1 }}; n")), 6);
	}
}

#[test]
fn a_glyph_is_a_character() {
	for four in ["number of glyphs in s", "count(s.glyphs)"] {
		is!(&shalom(four), 4);
	}
	is!(&shalom("n = 0; for glyphs in s { n += 1 }; n"), 4);
	is!(&shalom("n = 0; for each glyph in s { n += 1 }; n"), 4);
}

#[test]
fn letters_are_whole_characters() {
	// q with an acute accent as its own code point (U+0301, q́ has no precomposed form): one letter, not q and a lone accent
	is!("t = \"aq\\u{301}!\"; count(letters(t))", 2);
	is!("t = \"aq\\u{301}!\"; x = letters(t)#2; x.codepoints", 2);
}
