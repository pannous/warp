//! #26 (user decision 2026-10-03, "Extend all"): upper/lower map all of Unicode's simple case pairs, sort orders any
//! comparable values (ints, floats, texts), reverse works on text by code points.

use crate::is;

#[test]
fn upper_and_lower_cover_unicode_scripts() {
	is!("\"straße\".upper", "STRASSE");
	is!("\"ΑΒΓ\".lower", "αβγ");
	is!("\"привет\".upper", "ПРИВЕТ");
	is!("\"աբգ\".upper", "ԱԲԳ"); // Armenian lower case starts at U+0561
	is!("\"ａｂｃ\".upper", "ＡＢＣ"); // full-width Latin
	is!("\"ႠႡ\".lower", "ⴀⴁ"); // Georgian
	is!("\"𐐨\".upper", "𐐀"); // Deseret, outside the BMP
	is!("\"x1!\".upper", "X1!");
}

#[test]
fn sort_orders_floats() {
	is!("([2.5 1.5 3.25].sort)#1", 1.5);
	is!("([2.5 1.5 3.25].sort)#3", 3.25);
	is!("x=[2.5, -1.0, 0.5]; (x.sort)#1", -1.0);
	is!("([2.5f 1.5f 3.25f].sort)#1", 1.5); // IEEE floats
	is!("([2.5f 1 0.5].sort)#3", 2.5); // a float among exact numbers
}

#[test]
fn sort_orders_mixed_numbers() {
	is!("([2.5 1 3].sort)#1", 1);
	is!("([2.5 1 3].sort)#2", 2.5);
}

#[test]
fn sort_orders_texts() {
	is!("([\"pear\" \"apple\" \"fig\"].sort)#1", "apple");
	is!("([\"pear\" \"apple\" \"fig\"].sort)#3", "pear");
	is!("([\"b\" \"ab\" \"a\"].sort).join(\",\")", "a,ab,b");
}

#[test]
fn sort_still_orders_ints() {
	is!("([3 1 2].sort).join(\",\")", "1,2,3");
}

#[test]
fn reverse_works_on_text_by_code_points() {
	is!("\"hello\".reverse", "olleh");
	is!("\"añb🌍\".reverse", "🌍bña");
	is!("x=\"abc\"; reverse x", "cba");
}
