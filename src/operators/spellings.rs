//! How operators are written: every spelling of each `Op` (words, glyphs, ASCII), negated and element-wise spellings, and the method names of operators
use super::*;

/// Every spelling of an infix or prefix operator the parser reads: symbols, Unicode glyphs and words (wiki/alias.md,
/// wiki/operator.md). The canonical spelling is Op::as_str, every other one an alias; the parser takes the longest
/// spelling that fits (lookahead.rs peek_operator), the style hints name the canonical one (normalize.rs). The math
/// spellings (× ≤ √ …) are lib/math.warp's alias lines instead (aliases.rs, decision operator-alias)
pub const OPERATOR_SPELLINGS: [(&str, Op); 120] = [
	(":", Op::Colon),
	(".", Op::Dot),
	("?.", Op::SafeDot),
	("??", Op::Coalesce),
	("::", Op::Scope),
	(":=", Op::Define),
	("be", Op::Define), // wiki/be.md
	("=", Op::Assign),
	("<-", Op::Assign), // R's `x <- 3` (P145)
	("->", Op::Arrow),
	("=>", Op::FatArrow),
	("+", Op::Add),
	("plus", Op::Add),
	("add", Op::Add), // only between operands (lookahead operator_in_context, card add-subtract)
	("-", Op::Sub),
	("minus", Op::Sub),
	("subtract", Op::Sub),
	// the dashes 0x2010..0x2015 all mean `-` (the minus sign − is lib/math.warp's alias line)
	("‐", Op::Sub),
	("‑", Op::Sub),
	("‒", Op::Sub),
	("–", Op::Sub),
	("—", Op::Sub),
	("―", Op::Sub),
	("*", Op::Mul),
	// ("times", Op::Mul), // card word-operators: `3 times {…}` is a loop
	("/", Op::Div),
	("divided by", Op::Div),
	// floor division `7 /! 2`, `x /!= 2`: the parser makes them floor_quotient calls; `//` is always a comment and Pascal's
	// `7 div 2` notes the warp spelling (decision floor-division)
	("/!", Op::FloorDiv),
	("/!=", Op::FloorDivAssign),
	("div", Op::FloorDiv),
	("%", Op::Mod),
	// `a mod b` is Euclidean (0 ≤ r < |b|), `a rem b` the truncated remainder (sign of the dividend, as C)
	("mod", Op::Mod), // rest overloaded
	("modulo", Op::Mod),
	("rem", Op::Rem),
	("^", Op::Pow),
	("**", Op::Pow),
	("⌞", Op::LogBase),
	("⌟", Op::LogOf),
	("<<", Op::Shl),
	(">>", Op::Shr),
	("+=", Op::AddAssign),
	("-=", Op::SubAssign),
	("*=", Op::MulAssign),
	("/=", Op::DivAssign),
	("%=", Op::ModAssign),
	("^=", Op::PowAssign),
	("**=", Op::PowAssign),
	("&&=", Op::AndAssign),
	("and=", Op::AndAssign),
	("||=", Op::OrAssign),
	("or=", Op::OrAssign),
	("^^=", Op::XorAssign),
	("xor=", Op::XorAssign),
	("<", Op::Lt),
	("less than", Op::Lt),
	("is less than", Op::Lt),
	(">", Op::Gt),
	("greater than", Op::Gt),
	("is greater than", Op::Gt),
	("<=", Op::Le),
	("is at most", Op::Le),
	(">=", Op::Ge),
	("is at least", Op::Ge),
	("==", Op::Eq),
	("equals", Op::Eq),
	("is", Op::Eq), // wiki/equality.md: `is` compares by value like ==
	("!=", Op::Ne),
	// identity (P208): `same` is the word, `===` its alias; no `identical` (user)
	("===", Op::Identical),
	("same", Op::Identical),
	("same as", Op::Identical),
	("is the same as", Op::Identical),
	("!==", Op::NotIdentical),
	("is not the same as", Op::NotIdentical),
	("circa", Op::Similar),
	("approximately", Op::Similar),
	("~", Op::Rough),
	("~~", Op::Rough),
	("and", Op::And),
	("&&", Op::And),
	("or", Op::Or),
	("||", Op::Or),
	("xor", Op::Xor),
	("not", Op::Not),
	("!", Op::Not),
	("¬", Op::Not),
	("&", Op::And),
	("|", Op::Or),
	("+-", Op::PlusMinus),
	("++", Op::Inc),
	("--", Op::Dec),
	("?", Op::Question),
	// after a statement `if`, `unless`, `while` and `until` (lookahead.rs: outside a for header) guard it
	("if", Op::If),
	("unless", Op::If),
	("while", Op::While),
	("then", Op::Then),
	("else", Op::Else),
	("do", Op::Do),
	("#", Op::Hash),
	("..", Op::Range),
	("..<", Op::Range),   // Swift's exclusive range
	("upto", Op::Range),  // wiki/range.md: `1 upto 10` excludes 10
	("until", Op::Range), // Kotlin's `for i in 0 until n`
	("to", Op::To),
	("...", Op::To),
	("…", Op::To),
	("..=", Op::To), // Rust's inclusive range (card rust-inclusive-range)
	("through", Op::To),
	("as", Op::As),
	// element-wise arithmetic `xs .+ 4` maps the operator over the list (D3: `xs + 4` asks, `xs + [4]` joins)
	(".+", Op::Add),
	(".-", Op::Sub),
	(".*", Op::Mul),
	("./", Op::Div),
	(".^", Op::Pow),
	("|>", Op::Pipe),
	("in", Op::In),
	("is in", Op::In), // `item is in basket` reads as `item in basket`
	("contains", Op::In),
	("has", Op::In),
	("includes", Op::In),
	("↑", Op::ArrowUp),
	("↓", Op::ArrowDown),
	// lib/math.warp's alias lines spell -> => <- ↑ ↓ with some of these arrows (card user-listed):
	// Arrows ˂ ˃ ˄ ˅ ͍ ͎ ͢   ⃔ ⃕ ⃖ ⃗ ⃡  ← ↑ → ↓ ↔ ↕ ↖ ↗ ↘ ↙ ↚ ↛ ↜ ↝ ↞ ↟ ↠ ↡ ↢ ↣ ↤ ↥ ↦ ↧ ↨ ↩ ↪ ↫ ↬ ↭ ↮ ↯ ↰ ↱ ↲ ↳ ↴ ↵ ↶ ↷ ↸ ↹ ↺ ⇄ ⇅ ⇆ ⇇ ⇈ ⇉ ⇊ ⇍ ⇎ ⇏ ⇐ ⇑ ⇒ ⇓ ⇔ ⇕ ⇖ ⇗ ⇘ ⇙ ⇚ ⇛ ⇜ ⇝ ⇞ ⇟ ⇠ ⇡ ⇢ ⇣ ⇤ ⇦ ⇧ ⇨ ⇩ ⇪ ⇫ ⇬ ⇭ ⇮ ⇯ ⇰ ⇱ ⇲ ⇳ ⌁ ⌃ ⌄ ⌤ ⍇ ⍈ ⍐ ⍗ ⎋ ➔ ➘ ➙ ➚ ➛ ➜ ➝ ➞ ➟ ➠ ➡ ➢ ➣ ➤ ➥ ➦ ➧ ➨ ➩ ➪ ➫ ➬ ➭ ➮ ➯ ➱ ➲ ➳ ➴ ➵ ➶ ➷ ➸ ➹ ➺ ➻ ➼ ➽ ➾ ⽮ ￩ ￪ ￫ ￬ ⬀ ⬁ ⬂ ⬃ ⬄ ⬅ ⬆ ⬇ ⬈ ⬉ ⬊ ⬋ ⬌ ⬍ ⬎ ⬏ ⬐ ⬑ ⭠ ⭡ ⭢ ⭣ ⭤ ⭥ ⬰ ⬱ ⬲ ⬳ ⬴ ⬵ ⬶ ⬷ ⬸ ⬹ ⬺ ⬻ ⬼ ⬽ ⬾ ⬿ ⭀ ⭁ ⭂ ⭃ ⭄ ⭅ ⭆ ⭇ ⭈ ⭉ ⭊ ⭋ ⭌ ⟰ ⟱ ⟲ ⟳ ⟴ ⟵ ⟶ ⟷ ⟸ ⟹ ⟺ ⟻ ⟼ ⟽ ⟾ ⟿ ⤾ ⤿ ⥀ ⥁ ⥂ ⥃ ⥄ ⥅ ⥆ ⥇ ⥈ ⥉ ⥊ ⥋ ⥌ ⥍ ⥎ ⥏ ⥐ ⥑ ⥒ ⥓ ⥔ ⥕ ⥖
];
/// Spellings of a negated operator, each with the spelling it negates: `a nand b` is `not (a and b)` (wiki/operator.md:
/// `nand ¬&`, Unicode's ⊼), `x is not 4` and `x isn't 4` are `not (x is 4)`
pub(crate) const NEGATED_SPELLINGS: [(&str, &str); 8] = [
	("nand", "and"),
	("¬&", "and"),
	("⊼", "and"),
	("is not", "is"),
	("isn't", "is"),
	("isn’t", "is"),
	("not in", "in"), // Python's `x not in xs` (card parse-free)
	("∉", "in"),
];
/// Glyphs that are fine as written, so no style hint suggests the ASCII spelling of their operator
pub(crate) const PLAIN_GLYPHS: &str = "×÷≤≥≠≈⋍±¬⌞⌟…∈∉∋";
/// `xs .+ 4`: an arithmetic operator behind a dot applies to each element (D3)
pub const ELEMENT_WISE_OPERATORS: [Op; 5] = [Op::Add, Op::Sub, Op::Mul, Op::Div, Op::Pow];
/// The methods an operator on an instance calls (wiki/operator.md aliases, Python's special methods)
pub const OPERATOR_METHODS: [(Op, &[&str]); 10] = [
	(Op::Add, &["plus", "add", "__add__"]),
	(Op::Sub, &["minus", "subtract", "__sub__"]),
	(Op::Mul, &["times", "multiply", "__mul__"]),
	(Op::Div, &["divide", "div", "__truediv__"]),
	(Op::Mod, &["mod", "modulo", "__mod__"]),
	(Op::Lt, &["less", "smaller", "__lt__"]),
	(Op::Gt, &["more", "bigger", "__gt__"]),
	(Op::Similar, &["approximately"]),
	(Op::Rough, &["similar"]),
	(Op::Hash, &["index"]), // `F#4` of a note object F (decision music-plain-details)
];

/// The spelling a negated spelling negates: `and` for `nand`
pub fn negated_spelling(spelling: &str) -> Option<&'static str> {
	NEGATED_SPELLINGS.iter().find(|(negated, _)| *negated == spelling).map(|(_, positive)| *positive)
}

/// Every spelling with its operator, the negated ones with the operator they negate
pub fn all_spellings() -> impl Iterator<Item = (&'static str, Op)> {
	let negated =
		NEGATED_SPELLINGS.into_iter().filter_map(|(negated, positive)| Some((negated, spelled_operator(positive)?)));
	OPERATOR_SPELLINGS.into_iter().chain(infix_math_aliases()).chain(negated)
}

fn infix_math_aliases() -> impl Iterator<Item = (&'static str, Op)> {
	super::math_aliases()
		.iter()
		.filter(|alias| alias.position == super::AliasPosition::Infix)
		.map(|alias| (alias.spelling.as_str(), alias.op))
}

/// The operator a spelling stands for: `and` and `∧` for And, `+` for Add
pub fn spelled_operator(spelling: &str) -> Option<Op> {
	table_operator(spelling).or_else(|| infix_math_aliases().find(|(known, _)| *known == spelling).map(|(_, op)| op))
}

/// The operator a spelling of the parser's own table stands for, the target of an alias line
pub(crate) fn table_operator(spelling: &str) -> Option<Op> {
	OPERATOR_SPELLINGS.iter().find(|(known, _)| *known == spelling).map(|(_, op)| *op)
}

/// A glyph spelling of an operator the style hints replace by the canonical spelling: `–` by `-`, `∧` by `and`
pub fn glyph_operator(glyph: char) -> Option<Op> {
	let mut spelling = [0; 4];
	spelled_operator(glyph.encode_utf8(&mut spelling)).filter(|_| !glyph.is_ascii() && !PLAIN_GLYPHS.contains(glyph))
}

/// A word that spells an arithmetic operator: `plus`, `minus`, `add`, `subtract`, `div`, `mod` (card add-subtract)
pub fn is_arithmetic_word(word: &str) -> bool {
	word.chars().all(char::is_alphabetic)
		&& spelled_operator(word).is_some_and(|op| op.is_arithmetic() || op == Op::FloorDiv)
}

/// A word that spells an operator, alone or as the first word of a phrase: `and`, `mod`, `is` (of `is less than`)
pub fn is_operator_word(word: &str) -> bool {
	all_spellings()
		.any(|(spelling, _)| spelling.split([' ', '=']).next() == Some(word) && word.starts_with(char::is_alphabetic))
}

/// The word the parser keeps an element-wise operator as, `.*` of `*`
pub fn element_wise_word(op: Op) -> String {
	format!(".{}", op.as_str())
}

/// The operator an element-wise word applies to each element, `*` of `.*`
pub fn element_wise_operator(word: &str) -> Option<Op> {
	ELEMENT_WISE_OPERATORS.into_iter().find(|op| element_wise_word(*op) == word)
}

/// The method an operator on an instance calls, `plus` for `+`
pub fn operator_method(op: Op) -> Option<&'static str> {
	OPERATOR_METHODS.iter().find(|(known, _)| *known == op).map(|(_, names)| names[0])
}

impl Op {
	/// The string representation of this operator
	pub fn as_str(&self) -> &'static str {
		match self {
			// Structural
			Op::Colon => ":",
			Op::Dot => ".",
			Op::SafeDot => "?.",
			Op::Coalesce => "??",
			Op::Scope => "::",
			Op::Define => ":=",
			Op::Assign => "=",
			Op::Arrow => "->",
			Op::FatArrow => "=>",
			Op::Subscript => "[]",
			Op::ArrowUp => "↑",
			Op::ArrowDown => "↓",

			// Arithmetic
			Op::Add => "+",
			Op::Sub => "-",
			Op::PlusMinus => "±",
			Op::Pipe => "|>",
			Op::In => "in",
			Op::Mul => "*",
			Op::Div => "/",
			Op::FloorDiv => "/!",
			Op::Mod => "%",
			Op::Rem => "rem",
			Op::Pow => "^",
			Op::LogBase => "⌞",
			Op::LogOf => "⌟",
			Op::Shl => "<<",
			Op::Shr => ">>",

			// Compound assignment
			Op::AddAssign => "+=",
			Op::SubAssign => "-=",
			Op::MulAssign => "*=",
			Op::DivAssign => "/=",
			Op::FloorDivAssign => "/!=",
			Op::ModAssign => "%=",
			Op::PowAssign => "^=",
			Op::AndAssign => "&&=",
			Op::OrAssign => "||=",
			Op::XorAssign => "^^=",

			// Comparison
			Op::Lt => "<",
			Op::Gt => ">",
			Op::Le => "<=",
			Op::Ge => ">=",
			Op::Eq => "==",
			Op::Ne => "!=",
			Op::Identical => "===",
			Op::NotIdentical => "!==",
			Op::Similar => "≈",
			Op::Rough => "~",

			// Logical
			Op::And => "and",
			Op::Or => "or",
			Op::Xor => "xor",
			Op::Not => "not",
			Op::BitAnd => "&",
			Op::BitOr => "|",

			// Prefix
			Op::Neg => "-",
			Op::Sqrt => "√",
			Op::Cbrt => "∛",
			Op::Abs => "‖",

			// Suffix
			Op::Inc => "++",
			Op::Dec => "--",

			// Ternary/Index/Range
			Op::Question => "?",
			Op::Hash => "#",
			Op::Range => "..",
			Op::To => "to",

			// Type conversion
			Op::As => "as",

			// Conditional
			Op::If => "if",
			Op::Then => "then",
			Op::Else => "else",

			// Loop
			Op::While => "while",
			Op::Do => "do",

			Op::None => "",
		}
	}
}
