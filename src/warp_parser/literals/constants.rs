//! Constants: truth glyphs `✓`/`✗`, assignments to built-in constants
use super::*;

/// In data only JSON's words are literals; aliases like `yes`, `no`, `none`, `pi` stay symbols.
const DATA_WORD_LITERALS: [&str; 3] = ["true", "false", "null"];
/// The glyphs that are true or false (card keyword-glyphs), longest spelling first: `✔️` is `✔` with an emoji variation
/// selector. Most start an emoji, which would otherwise be a codepoint (atoms.rs, parse_emoji)
const TRUTH_GLYPHS: [(&str, bool); 13] = [
	("✔️", true),
	("✓️", true),
	("☑️", true),
	("⊤", true),
	("✓", true),
	("✔", true),
	("☑", true),
	("✅", true),
	("🗸", true),
	("🗹", true),
	("⊥", false),
	("❌", false),
	("✗", false),
];

pub(crate) fn truth_glyph(glyph: &str) -> Option<Node> {
	TRUTH_GLYPHS
		.iter()
		.find(|(spelling, _)| *spelling == glyph)
		.map(|(_, truth)| if *truth { Node::True } else { Node::False })
}

pub(crate) fn check_constants(s: &str, data_mode: bool) -> Option<Node> {
	let is_word = s.chars().all(|c| c.is_ascii_alphabetic());
	if data_mode && is_word && !DATA_WORD_LITERALS.contains(&s) {
		return None;
	}
	match s.to_lowercase().as_str() {
		"true" | "yes" => Some(Node::True),
		"false" | "no" | "" => Some(Node::False),
		glyph if TRUTH_GLYPHS.iter().any(|(spelling, _)| *spelling == glyph) => truth_glyph(glyph),
		"ø" | "null" | "nul" | "none" | "nil" | "nill" | "nix" | "nada" | "nothing" | "empty" | "void" => Some(Empty),
		// exact generators (extensions/reals.rs); a bare `e` or `i` stays a free name
		"π" | "pi" => Some(real(Exact::pi())),
		"τ" | "tau" => Some(real(Exact::pi().scale(&Rational::integer(2)))),
		"euler" | "ℯ" => Some(real(Exact::euler())),
		"ⅈ" => Some(real(Exact::imaginary())),
		// hyperreals (wiki/hyperreals.md): the glyphs only, `epsilon` stays a free name like `e`
		"ε" => Some(real(Exact::epsilon())),
		"ω" => Some(real(Exact::omega())),
		_ => None,
	}
}

impl WarpParser {
	/// `⊤`, `✅`, `❌` …: the truth value a glyph at the position spells, and its length in chars
	pub(crate) fn truth_glyph_at(&self) -> Option<(Node, usize)> {
		TRUTH_GLYPHS.iter().find_map(|(glyph, _)| {
			let length = glyph.chars().count();
			let ahead: String = self.chars[self.pos..].iter().take(length).collect();
			(ahead == *glyph).then(|| (super::truth_glyph(glyph).expect("a truth glyph"), length))
		})
	}
}
