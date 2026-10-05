//! Uniscript entities in code (wiki/uniscript.md, unicode.md, Features.md): `\:infinity` and `\alpha` are the characters
//! ∞ and α, as identifiers and operators alike (`\alpha = 4` assigns α). Only the simple entities of this table;
//! the full table and the `<:fracture A>` blocks are the uniscript package (probes/uniscript, data/uniscript).
//! Texts and comments keep their backslashes (`"\n"` is a newline, not an entity).

/// Entity names: the LaTeX / unicode-math command names, plus the uniscript English names
const ENTITIES: &[(&str, char)] = &[
	("alpha", 'α'), ("beta", 'β'), ("gamma", 'γ'), ("delta", 'δ'), ("epsilon", 'ε'), ("varepsilon", 'ε'), ("zeta", 'ζ'),
	("eta", 'η'), ("theta", 'θ'), ("iota", 'ι'), ("kappa", 'κ'), ("lambda", 'λ'), ("mu", 'μ'), ("nu", 'ν'), ("xi", 'ξ'),
	("omicron", 'ο'), ("pi", 'π'), ("rho", 'ρ'), ("sigma", 'σ'), ("tau", 'τ'), ("upsilon", 'υ'), ("phi", 'φ'), ("varphi", 'φ'),
	("chi", 'χ'), ("psi", 'ψ'), ("omega", 'ω'),
	("Alpha", 'Α'), ("Beta", 'Β'), ("Gamma", 'Γ'), ("Delta", 'Δ'), ("Epsilon", 'Ε'), ("Zeta", 'Ζ'), ("Eta", 'Η'), ("Theta", 'Θ'),
	("Iota", 'Ι'), ("Kappa", 'Κ'), ("Lambda", 'Λ'), ("Mu", 'Μ'), ("Nu", 'Ν'), ("Xi", 'Ξ'), ("Omicron", 'Ο'), ("Pi", 'Π'),
	("Rho", 'Ρ'), ("Sigma", 'Σ'), ("Tau", 'Τ'), ("Upsilon", 'Υ'), ("Phi", 'Φ'), ("Chi", 'Χ'), ("Psi", 'Ψ'), ("Omega", 'Ω'),
	("infinity", '∞'), ("infty", '∞'), ("aleph", 'ℵ'), ("hbar", 'ℏ'), ("ell", 'ℓ'), ("euler", 'ℯ'), ("degree", '°'),
	("nat", 'ℕ'), ("naturals", 'ℕ'), ("integers", 'ℤ'), ("rationals", 'ℚ'), ("reals", 'ℝ'), ("complex", 'ℂ'),
	("forall", '∀'), ("exists", '∃'), ("nexists", '∄'), ("in", '∈'), ("notin", '∉'), ("ni", '∋'),
	("empty", '∅'), ("emptyset", '∅'), ("subset", '⊂'), ("subseteq", '⊆'), ("supset", '⊃'), ("supseteq", '⊇'),
	("cup", '∪'), ("union", '∪'), ("cap", '∩'), ("intersection", '∩'),
	("wedge", '∧'), ("land", '∧'), ("vee", '∨'), ("lor", '∨'), ("neg", '¬'), ("lnot", '¬'), ("xor", '⊻'),
	("implies", '⇒'), ("Rightarrow", '⇒'), ("iff", '⇔'), ("Leftrightarrow", '⇔'), ("to", '→'), ("rightarrow", '→'),
	("leftarrow", '←'), ("mapsto", '↦'),
	("leq", '≤'), ("le", '≤'), ("geq", '≥'), ("ge", '≥'), ("neq", '≠'), ("ne", '≠'), ("approx", '≈'), ("equiv", '≡'),
	("times", '×'), ("cdot", '⋅'), ("div", '÷'), ("pm", '±'), ("mp", '∓'), ("sqrt", '√'), ("cbrt", '∛'), ("circ", '∘'),
	("sum", '∑'), ("prod", '∏'), ("int", '∫'), ("partial", '∂'), ("nabla", '∇'),
	("star", '⋆'), ("bullet", '•'), ("ldots", '…'), ("dots", '…'), ("prime", '′'), ("therefore", '∴'), ("because", '∵'),
	("perp", '⊥'), ("bot", '⊥'), ("top", '⊤'), ("parallel", '∥'), ("angle", '∠'), ("langle", '⟨'), ("rangle", '⟩'),
];

/// The character an entity name stands for
pub fn entity(name: &str) -> Option<char> {
	ENTITIES.iter().find(|(entity_name, _)| *entity_name == name).map(|(_, character)| *character)
}

/// The entity written at `chars[at..]` (`\:name` or `\name`, the name its whole run of ASCII letters):
/// its name and the number of chars it spans
pub fn entity_name_at(chars: &[char], at: usize) -> Option<(String, usize)> {
	if chars.get(at) != Some(&'\\') {
		return None;
	}
	let name_start = if chars.get(at + 1) == Some(&':') { at + 2 } else { at + 1 };
	let name: String = chars[name_start.min(chars.len())..].iter().take_while(|c| c.is_ascii_alphabetic()).collect();
	(!name.is_empty()).then(|| (name.clone(), name_start - at + name.len()))
}

/// The source with every known entity outside texts and comments replaced by its character; an unknown one stays
/// and the parser names it
pub fn expand_entities(source: &str) -> String {
	if !source.contains('\\') {
		return source.to_string();
	}
	let chars: Vec<char> = source.chars().collect();
	let mut expanded = String::with_capacity(source.len());
	let mut at = 0;
	while at < chars.len() {
		if let Some(end) = crate::wasp_parser::text_or_comment_end(&chars, at) {
			expanded.extend(&chars[at..end]);
			at = end;
			continue;
		}
		match entity_name_at(&chars, at).and_then(|(name, length)| Some((entity(&name)?, length))) {
			Some((character, length)) => {
				expanded.push(character);
				at += length;
			}
			None => {
				expanded.push(chars[at]);
				at += 1;
			}
		}
	}
	expanded
}
