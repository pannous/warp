use crate::is;

#[test]
fn logic_glyphs_are_and_or() {
	is!("true ∨ false", true);
	is!("false ∨ false", false);
	is!("true ∧ false", false);
	is!("true ⋀ true", true);
	is!("false ⋁ true", true);
	is!("true or false", true);
}

#[test]
fn equality_glyphs_compare() {
	is!("3 ≟ 3", true);
	is!("3 ≡ 4", false);
	is!("3 ﹦ 3", true);
	is!("3 == 3", true);
}

#[test]
fn dash_glyphs_subtract() {
	is!("3–1", 2);
	is!("3—1", 2);
	is!("3−1", 2);
	is!("3‑1", 2);
	is!("3 − 1", 2);
	is!("−3 + 5", 2);
	is!("3-1", 2);
}

#[test]
fn glyphs_inside_text_stay_text() {
	is!("\"a–b\"", "a–b");
}
