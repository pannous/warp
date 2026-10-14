use serde::{Deserialize, Serialize};
use std::fmt;

/// Keywords that introduce function definitions
pub const FUNCTION_KEYWORDS: [&str; 6] = ["fun", "fn", "def", "define", "function", "func"];
/// Right binding power of `as`: higher than every infix operator, so the target type is a single atom
pub const TYPE_OPERAND_BP: u8 = 250;
/// Left binding power of the suffix operators ² ³ ++ --: below `.` (180) and `#` (170), above `^` (160)
const SUFFIX_BP: u8 = 165;
/// Right binding power of the prefix operators √ ∛ abs: between ^ (160) and the suffix operators (SUFFIX_BP)
const PREFIX_BP: u8 = 162;

/// Every spelling of an infix or prefix operator the parser reads: symbols, Unicode glyphs and words (wiki/alias.md,
/// wiki/operator.md). The canonical spelling is Op::as_str, every other one an alias; the parser takes the longest
/// spelling that fits (lookahead.rs peek_operator), the style hints name the canonical one (normalize.rs)
pub const OPERATOR_SPELLINGS: [(&str, Op); 136] = [
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
	// ("add", Op::Add), // card word-operators: clashes with xs.add(…), add:(…) members, `add x to y`
	("-", Op::Sub),
	("minus", Op::Sub),
	// ("subtract", Op::Sub), // card word-operators
	// the dashes 0x2010..0x2015 and the minus sign 0x2212 all mean `-`
	("‐", Op::Sub),
	("‑", Op::Sub),
	("‒", Op::Sub),
	("–", Op::Sub),
	("—", Op::Sub),
	("―", Op::Sub),
	("−", Op::Sub),
	("*", Op::Mul),
	("×", Op::Mul),
	// the unit product of a printed quantity (`6 m·kg`); generated names use `·` only after parsing
	("·", Op::Mul),
	("⋅", Op::Mul),
	// ("times", Op::Mul), // card word-operators: `3 times {…}` is a loop
	("/", Op::Div),
	("÷", Op::Div),
	("divided by", Op::Div),
	// Python's floor division `7//2`, `x//=2` and `7 div 2`: the parser makes them floor_quotient calls
	("//", Op::Div), // todo cleaner? ^^
	("div", Op::Div),
	("//=", Op::DivAssign), // todo /= ≠ //=
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
	("≤", Op::Le),
	("is at most", Op::Le),
	(">=", Op::Ge),
	("≥", Op::Ge),
	("is at least", Op::Ge),
	("==", Op::Eq),
	("≟", Op::Eq),
	("≡", Op::Eq),
	("﹦", Op::Eq),
	("equals", Op::Eq),
	("is", Op::Eq), // wiki/equality.md: `is` compares by value like ==
	("!=", Op::Ne),
	("≠", Op::Ne),
	// identity (P208): `same` is the word, `===` its alias; no `identical` (user)
	("===", Op::Identical),
	("same", Op::Identical),
	("same as", Op::Identical),
	("is the same as", Op::Identical),
	("!==", Op::NotIdentical),
	("is not the same as", Op::NotIdentical),
	("≈", Op::Similar),
	("⋍", Op::Similar),
	("circa", Op::Similar),
	("approximately", Op::Similar),
	("~", Op::Rough),
	("~~", Op::Rough),
	("and", Op::And),
	("&&", Op::And),
	("∧", Op::And),
	("⋀", Op::And),
	("or", Op::Or),
	("||", Op::Or),
	("∨", Op::Or),
	("⋁", Op::Or),
	("xor", Op::Xor),
	("⊻", Op::Xor),
	("not", Op::Not),
	("!", Op::Not),
	("¬", Op::Not),
	("&", Op::And),
	("|", Op::Or),
	("±", Op::PlusMinus),
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
	// Arrows ˂ ˃ ˄ ˅ ͍ ͎ ͢   ⃔ ⃕ ⃖ ⃗ ⃡  ← ↑ → ↓ ↔ ↕ ↖ ↗ ↘ ↙ ↚ ↛ ↜ ↝ ↞ ↟ ↠ ↡ ↢ ↣ ↤ ↥ ↦ ↧ ↨ ↩ ↪ ↫ ↬ ↭ ↮ ↯ ↰ ↱ ↲ ↳ ↴ ↵ ↶ ↷ ↸ ↹ ↺ ⇄ ⇅ ⇆ ⇇ ⇈ ⇉ ⇊ ⇍ ⇎ ⇏ ⇐ ⇑ ⇒ ⇓ ⇔ ⇕ ⇖ ⇗ ⇘ ⇙ ⇚ ⇛ ⇜ ⇝ ⇞ ⇟ ⇠ ⇡ ⇢ ⇣ ⇤ ⇦ ⇧ ⇨ ⇩ ⇪ ⇫ ⇬ ⇭ ⇮ ⇯ ⇰ ⇱ ⇲ ⇳ ⌁ ⌃ ⌄ ⌤ ⍇ ⍈ ⍐ ⍗ ⎋ ➔ ➘ ➙ ➚ ➛ ➜ ➝ ➞ ➟ ➠ ➡ ➢ ➣ ➤ ➥ ➦ ➧ ➨ ➩ ➪ ➫ ➬ ➭ ➮ ➯ ➱ ➲ ➳ ➴ ➵ ➶ ➷ ➸ ➹ ➺ ➻ ➼ ➽ ➾ ⽮ ￩ ￪ ￫ ￬ ⬀ ⬁ ⬂ ⬃ ⬄ ⬅ ⬆ ⬇ ⬈ ⬉ ⬊ ⬋ ⬌ ⬍ ⬎ ⬏ ⬐ ⬑ ⭠ ⭡ ⭢ ⭣ ⭤ ⭥ ⬰ ⬱ ⬲ ⬳ ⬴ ⬵ ⬶ ⬷ ⬸ ⬹ ⬺ ⬻ ⬼ ⬽ ⬾ ⬿ ⭀ ⭁ ⭂ ⭃ ⭄ ⭅ ⭆ ⭇ ⭈ ⭉ ⭊ ⭋ ⭌ ⟰ ⟱ ⟲ ⟳ ⟴ ⟵ ⟶ ⟷ ⟸ ⟹ ⟺ ⟻ ⟼ ⟽ ⟾ ⟿ ⤾ ⤿ ⥀ ⥁ ⥂ ⥃ ⥄ ⥅ ⥆ ⥇ ⥈ ⥉ ⥊ ⥋ ⥌ ⥍ ⥎ ⥏ ⥐ ⥑ ⥒ ⥓ ⥔ ⥕ ⥖
];

/// Spellings of a negated operator, each with the spelling it negates: `a nand b` is `not (a and b)` (wiki/operator.md:
/// `nand ¬&`, Unicode's ⊼), `x is not 4` is `not (x is 4)`
const NEGATED_SPELLINGS: [(&str, &str); 4] = [("nand", "and"), ("¬&", "and"), ("⊼", "and"), ("is not", "is")];

/// The spelling a negated spelling negates: `and` for `nand`
pub fn negated_spelling(spelling: &str) -> Option<&'static str> {
	NEGATED_SPELLINGS
		.iter()
		.find(|(negated, _)| *negated == spelling)
		.map(|(_, positive)| *positive)
}

/// Every spelling with its operator, the negated ones with the operator they negate
pub fn all_spellings() -> impl Iterator<Item = (&'static str, Op)> {
	let negated = NEGATED_SPELLINGS
		.into_iter()
		.filter_map(|(negated, positive)| Some((negated, spelled_operator(positive)?)));
	OPERATOR_SPELLINGS.into_iter().chain(negated)
}

/// Glyphs that are fine as written, so no style hint suggests the ASCII spelling of their operator
const PLAIN_GLYPHS: &str = "×÷≤≥≠≈⋍±¬⌞⌟…";

/// The operator a spelling stands for: `and` and `∧` for And, `+` for Add
pub fn spelled_operator(spelling: &str) -> Option<Op> {
	OPERATOR_SPELLINGS
		.iter()
		.find(|(known, _)| *known == spelling)
		.map(|(_, op)| *op)
}

/// A glyph spelling of an operator the style hints replace by the canonical spelling: `–` by `-`, `∧` by `and`
pub fn glyph_operator(glyph: char) -> Option<Op> {
	let mut spelling = [0; 4];
	spelled_operator(glyph.encode_utf8(&mut spelling)).filter(|_| !glyph.is_ascii() && !PLAIN_GLYPHS.contains(glyph))
}

/// A word that spells an operator, alone or as the first word of a phrase: `and`, `mod`, `is` (of `is less than`)
pub fn is_operator_word(word: &str) -> bool {
	all_spellings()
		.any(|(spelling, _)| spelling.split([' ', '=']).next() == Some(word) && word.starts_with(char::is_alphabetic))
}

/// `xs .+ 4`: an arithmetic operator behind a dot applies to each element (D3)
pub const ELEMENT_WISE_OPERATORS: [Op; 5] = [Op::Add, Op::Sub, Op::Mul, Op::Div, Op::Pow];

/// The word the parser keeps an element-wise operator as, `.*` of `*`
pub fn element_wise_word(op: Op) -> String {
	format!(".{}", op.as_str())
}

/// The operator an element-wise word applies to each element, `*` of `.*`
pub fn element_wise_operator(word: &str) -> Option<Op> {
	ELEMENT_WISE_OPERATORS
		.into_iter()
		.find(|op| element_wise_word(*op) == word)
}

/// The methods an operator on an instance calls (wiki/operator.md aliases, Python's special methods)
pub const OPERATOR_METHODS: [(Op, &[&str]); 9] = [
	(Op::Add, &["plus", "add", "__add__"]),
	(Op::Sub, &["minus", "subtract", "__sub__"]),
	(Op::Mul, &["times", "multiply", "__mul__"]),
	(Op::Div, &["divide", "div", "__truediv__"]),
	(Op::Mod, &["mod", "modulo", "__mod__"]),
	(Op::Lt, &["less", "smaller", "__lt__"]),
	(Op::Gt, &["more", "bigger", "__gt__"]),
	(Op::Similar, &["approximately"]),
	(Op::Rough, &["similar"]),
];

/// The method an operator on an instance calls, `plus` for `+`
pub fn operator_method(op: Op) -> Option<&'static str> {
	OPERATOR_METHODS
		.iter()
		.find(|(known, _)| *known == op)
		.map(|(_, names)| names[0])
}

pub fn is_function_keyword(s: &str) -> bool {
	FUNCTION_KEYWORDS.contains(&s)
}

// node[i]

// Warp ABI GC Node representation design:
// This is a single struct that can represent any node type

// todo move node layout to warp_abi.rs
// todo ... any change to node layout must be reflected in wasm_gc_reader.rs warp_abi.md ...

/* restructure the whole emitter emit_node_instructions serialization to use
(type $Node (struct
	(field $kind i64)
	(field $data anyref)
	(field $value (ref null $$Node))
))
**/

/// Operator for Key nodes - distinguishes different binding operations
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Op {
	// Structural operators (existing)
	Colon,     // :   type annotation and object construction person:{name:"Joe" age:42}
	Dot,       // .   member access
	SafeDot,   // ?.  member access that is ø when the receiver is ø
	Coalesce,  // ??  the left value unless it is ø, then the right (Swift, C#, JS)
	Scope,     // ::  scope resolution
	Define,    // :=  definition
	Assign,    // =   assignment
	Arrow,     // ->  arrow/lambda/return type
	FatArrow,  // =>  only while parsing: the spelling binds and reads as in JS; parse() makes it Arrow, one arrow for warp
	ArrowUp,   // ↑  e.g. exponential x^n
	ArrowDown, // ↓ e.g. logarithmic   log(x)/log(base)

	// Arithmetic operators
	Add,     // +
	Sub,     // -
	Mul,     // *  ×  ⋅
	Div,     // /  ÷
	Mod,     // %  mod  Euclidean: 0 ≤ a % b < |b|
	Rem,     // rem  truncated remainder, sign of the dividend (C, Java, JS, Rust %)
	Pow,     // ^  **
	LogBase, // ⌞  b⌞x: log base b of x (P226)
	LogOf,   // ⌟  x⌟: the natural log of x, x⌟b: log base b of x (P226)
	Shl,     // <<  shift left: a * 2^n
	Shr,     // >>  shift right: floor(a / 2^n)

	// Compound assignment operators (x op= y → x = x op y)
	AddAssign, // +=
	SubAssign, // -=
	MulAssign, // *=
	DivAssign, // /=
	ModAssign, // %=
	PowAssign, // ^=  **=
	AndAssign, // &&=  and=
	OrAssign,  // ||=  or=
	XorAssign, // ^=   xor=

	// Comparison operators
	Lt,           // <
	Gt,           // >
	Le,           // <=  ≤
	Ge,           // >=  ≥
	Eq,           // ==
	Ne,           // !=  ≠
	Identical,    // ===  equal and of the same type: 0 === false is false (card zero-false)
	NotIdentical, // !==
	Similar,      // ≈  ⋍  circa  approximately: equal within the relative `tolerance` (default 1e-9)
	Rough, // ~  ~~: looser than ≈, within `rough_tolerance` (default 1%), texts without surrounding punctuation (P211)

	// Logical operators
	And, // and  &&  ∧
	Or,  // or   ||  ⋁
	Xor, // xor  ⊻
	Not, // not  !  ¬
	// `&` and `|`: the bits of two whole numbers, else the logical and/or (card operator-shall)
	BitAnd, // &
	BitOr,  // |

	// Prefix operators (unary)
	Neg,  // - (unary minus)
	Sqrt, // √
	Cbrt, // ∛
	Abs,  // ‖...‖

	// Suffix operators
	Inc,    // ++
	Dec,    // --
	Square, // ²
	Cube,   // ³

	// Ternary
	Question, // ? (ternary condition)

	// Conditional
	If,   // if
	Then, // then
	Else, // else

	// Loop
	While, // while
	Do,    // do (used with while)

	// Index/Range
	Hash,  // #  (1-based index)
	Range, // ..
	To,    // to  ...  …

	// Type conversion
	As, // as  (type cast)

	PlusMinus, // ±  +-  a value with tolerance

	// Parsed only, the parser builds their nodes: the pipeline `xs |> f` is the call `f(xs)`, the membership `x in xs`
	// and `xs contains x` the list [x in xs]
	Pipe, // |>
	In,   // in

	// User(Node), allow user defined operators, name==symbol
	None, // implicit/unknown
}

impl Op {
	/// Binding power: (left_bp, right_bp)
	/// Higher = tighter binding. Right > left means right-associative.
	/// Suffix operators: (left_bp, 0) - only binds to left
	/// Prefix operators: (0, right_bp) - only binds to right
	pub const fn binding_power(&self) -> (u8, u8) {
		match self {
			// Suffix operators (no right operand), below member access and indexing: `xs#1++`, `bags#1.n++` change the
			// whole place and `p.x²` squares it, not its index or field name
			Op::Square | Op::Cube | Op::Inc | Op::Dec => (SUFFIX_BP, 0),

			// Member access (tightest infix)
			Op::Dot | Op::SafeDot => (180, 181),
			Op::Scope => (175, 176),
			// index operator #: its index is an atom, so `w#1.upper()` is `(w#1).upper()`
			Op::Hash => (170, 182),

			// Power (right-assoc: 2^3^4 = 2^(3^4))
			Op::Pow | Op::ArrowUp | Op::ArrowDown => (160, 159),
			Op::LogBase | Op::LogOf => (160, 161),

			// Multiplicative (left-assoc)
			Op::Mul | Op::Div | Op::Mod | Op::Rem => (150, 151),

			// Shifts sit between additive and range: 1 << 2 + 3 → 1 << (2+3)
			Op::Shl | Op::Shr => (135, 136),

			// Additive (left-assoc)
			Op::Add | Op::Sub | Op::PlusMinus => (140, 141),

			// Range
			Op::Range | Op::To => (130, 131),

			// Type conversion converts the whole arithmetic expression to its left (C#, TypeScript), not the nearest
			// operand (Rust, Kotlin): 2 * 1.5 as int → (2*1.5) as int → 3, 1.5 as int == 1; ungrouped mixes are linted
			// the target type is one atom: 0.1 as float + 0.2 → (0.1 as float) + 0.2, never 0.1 as (float + 0.2)
			Op::As => (125, TYPE_OPERAND_BP),

			// Ordering: one level, the parser chains them: a<b<c → a<b and b<c
			Op::Lt | Op::Gt | Op::Le | Op::Ge => (120, 121),

			// Equality binds weaker and never chains: a<b == c<d compares the two results, a==b==c is ambiguous
			Op::Eq | Op::Ne | Op::Identical | Op::NotIdentical | Op::Similar | Op::Rough | Op::In => (115, 116),
			// below arithmetic, above comparison: `xs |> sum > 3` is `sum(xs) > 3`
			Op::Pipe => (127, 128),

			// Logical not binds weaker than comparison: not a==b → not (a==b)
			Op::Not => (0, 105),

			// Logical (left-assoc with and > or)
			Op::And | Op::BitAnd => (100, 101),
			Op::Xor => (95, 96),
			Op::Or | Op::BitOr => (90, 91),
			Op::Coalesce => (93, 92), // right-assoc: a ?? b ?? c → a ?? (b ?? c)

			// Ternary: ? needs lower right bp so : can bind within
			Op::Question => (85, 79), // right-assoc, lower than Colon's left bp (80)

			// Conditional: if-then-else has similar precedence to ternary
			Op::If => (0, 78),    // prefix: if binds condition until then
			Op::Then => (77, 76), // then binds until else
			Op::Else => (75, 74), // else binds the rest

			// Loop: while condition do body
			Op::While => (0, 78), // prefix: while binds condition until do/block
			Op::Do => (77, 10),   // do binds very loosely to capture whole body including assignments

			// Structural/Key operators (existing, adjusted for consistency)
			Op::Colon => (80, 81),    // type annotation: a:b:c → a:(b:c)
			Op::Arrow => (70, 69),    // right-assoc: a->b->c → a->(b->c); a return type `-> (Int, Int)` stays one type
			Op::FatArrow => (70, 58), // `=>` as JS: the body takes an assignment, x => total += x
			Op::Define => (60, 59),   // right-assoc: a:=b:=c → a:=(b:=c)
			Op::Assign => (60, 59),   // right-assoc: a=b=c → a=(b=c)

			// Compound assignment (same precedence as assignment)
			Op::AddAssign
			| Op::SubAssign
			| Op::MulAssign
			| Op::DivAssign
			| Op::ModAssign
			| Op::PowAssign
			| Op::AndAssign
			| Op::OrAssign
			| Op::XorAssign => (60, 59),

			// Prefix operators (no left operand, binds to right): looser than a suffix power (√x² is √(x²)) and member
			// access (√p.x is √(p.x)), tighter than ^ (√x^2 is (√x)^2)
			Op::Sqrt | Op::Cbrt | Op::Abs => (0, PREFIX_BP),
			// Unary minus binds weaker than power: -2^2 → -(2^2)
			Op::Neg => (0, 155),

			Op::None => (0, 0),
		}
	}

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
			Op::Square => "²",
			Op::Cube => "³",

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

	/// Check if this is a prefix-only operator
	pub fn is_prefix(&self) -> bool {
		matches!(self, Op::Neg | Op::Not | Op::Sqrt | Op::Cbrt | Op::Abs)
	}

	/// Check if this is a suffix-only operator
	pub fn is_suffix(&self) -> bool {
		matches!(self, Op::Inc | Op::Dec | Op::Square | Op::Cube)
	}

	/// The exponent a suffix power operator stands for: x² is x^2, x³ is x^3
	pub fn suffix_exponent(&self) -> Option<i64> {
		match self {
			Op::Square => Some(2),
			Op::Cube => Some(3),
			_ => None,
		}
	}

	/// Whether this comparison holds for two values ordered so (`<=` holds for Less and Equal)
	pub fn holds(&self, ordering: std::cmp::Ordering) -> bool {
		use std::cmp::Ordering::*;
		match self {
			Op::Eq | Op::Identical => ordering == Equal,
			Op::Ne | Op::NotIdentical => ordering != Equal,
			Op::Lt => ordering == Less,
			Op::Le => ordering != Greater,
			Op::Gt => ordering == Greater,
			_ => ordering != Less, // Ge
		}
	}

	/// Check if this is a binary arithmetic operator
	pub fn is_arithmetic(&self) -> bool {
		matches!(
			self,
			Op::Add | Op::Sub | Op::Mul | Op::Div | Op::Mod | Op::Rem | Op::Pow
		)
	}

	/// Check if this is a shift operator, defined on exact Ints only
	pub fn is_shift(&self) -> bool {
		matches!(self, Op::Shl | Op::Shr)
	}

	/// Check if this is a comparison operator
	pub fn is_comparison(&self) -> bool {
		matches!(
			self,
			Op::Eq | Op::Ne | Op::Identical | Op::NotIdentical | Op::Lt | Op::Gt | Op::Le | Op::Ge
		)
	}

	/// A comparison or a similarity (≈, ~): `combine 1.1 with 2.2 ≈ 3.3` compares the phrase's value
	pub fn is_relation(&self) -> bool {
		self.is_comparison() || matches!(self, Op::Similar | Op::Rough)
	}

	/// Ordering comparisons chain (a<b<c), equality does not
	pub fn is_ordering(&self) -> bool {
		matches!(self, Op::Lt | Op::Gt | Op::Le | Op::Ge)
	}

	pub fn is_equality(&self) -> bool {
		matches!(self, Op::Eq | Op::Ne)
	}

	/// Check if this is a logical operator (and, or, xor)
	pub fn is_logical(&self) -> bool {
		matches!(self, Op::And | Op::Or | Op::Xor)
	}

	/// `&` and `|` as the logical and/or they are on anything but two whole numbers; every other operator itself
	pub fn logical_twin(&self) -> Op {
		match self {
			Op::BitAnd => Op::And,
			Op::BitOr => Op::Or,
			_ => *self,
		}
	}

	pub fn is_bitwise(&self) -> bool {
		matches!(self, Op::BitAnd | Op::BitOr)
	}

	/// Check if this is a compound assignment operator (+=, -=, etc.)
	pub fn is_compound_assign(&self) -> bool {
		matches!(
			self,
			Op::AddAssign
				| Op::SubAssign
				| Op::MulAssign
				| Op::DivAssign
				| Op::ModAssign
				| Op::PowAssign
				| Op::AndAssign
				| Op::OrAssign
				| Op::XorAssign
		)
	}

	/// `=`, `+=` and the like, `++`, `--`: an operator that writes its variable
	pub fn is_write(&self) -> bool {
		*self == Op::Assign || self.is_compound_assign() || matches!(self, Op::Inc | Op::Dec)
	}

	/// Get the base operator for a compound assignment (AddAssign -> Add, etc.)
	pub fn base_op(&self) -> Op {
		match self {
			Op::AddAssign => Op::Add,
			Op::SubAssign => Op::Sub,
			Op::MulAssign => Op::Mul,
			Op::DivAssign => Op::Div,
			Op::ModAssign => Op::Mod,
			Op::PowAssign => Op::Pow,
			Op::AndAssign => Op::And,
			Op::OrAssign => Op::Or,
			Op::XorAssign => Op::Xor,
			_ => *self,
		}
	}
}

impl fmt::Display for Op {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{}", self.as_str())
	}
}

/// Every operator by its code in the kind field of a Key node (`(code << 8) | Kind::Key`); the first five are the
/// codes earlier modules stored, the rest follow, so a quoted expression (`data 1+2`) reads back with its operator
pub const OP_CODES: [Op; 66] = [
	Op::None,
	Op::Colon,
	Op::Assign,
	Op::Define,
	Op::Dot,
	Op::SafeDot,
	Op::Scope,
	Op::Arrow,
	Op::Add,
	Op::Sub,
	Op::Mul,
	Op::Div,
	Op::Mod,
	Op::Rem,
	Op::Pow,
	Op::Shl,
	Op::Shr,
	Op::AddAssign,
	Op::SubAssign,
	Op::MulAssign,
	Op::DivAssign,
	Op::ModAssign,
	Op::PowAssign,
	Op::AndAssign,
	Op::OrAssign,
	Op::XorAssign,
	Op::Lt,
	Op::Gt,
	Op::Le,
	Op::Ge,
	Op::Eq,
	Op::Ne,
	Op::Similar,
	Op::And,
	Op::Or,
	Op::Xor,
	Op::Not,
	Op::Neg,
	Op::Sqrt,
	Op::Cbrt,
	Op::Abs,
	Op::Inc,
	Op::Dec,
	Op::Square,
	Op::Cube,
	Op::Question,
	Op::If,
	Op::Then,
	Op::Else,
	Op::While,
	Op::Do,
	Op::Hash,
	Op::Range,
	Op::To,
	Op::As,
	Op::PlusMinus,
	Op::Coalesce,
	Op::Identical,
	Op::NotIdentical,
	Op::Rough,
	Op::LogBase,
	Op::LogOf,
	Op::BitAnd,
	Op::BitOr,
	Op::ArrowUp,
	Op::ArrowDown,
];

/// Encode Op as i64 for storage in kind field
pub fn op_to_code(op: &Op) -> i64 {
	OP_CODES.iter().position(|known| known == op).unwrap_or(0) as i64
}

/// The operator written `text` (Op::as_str's inverse): `*` is Mul
pub fn op_named(text: &str) -> Option<Op> {
	OP_CODES.iter().find(|op| op.as_str() == text).copied()
}

/// Decode i64 back to Op
pub fn code_to_op(code: i64) -> Op {
	usize::try_from(code)
		.ok()
		.and_then(|index| OP_CODES.get(index))
		.copied()
		.unwrap_or(Op::None)
}

/// What Node::serialize writes between an entry's key and value, by operator code: `x+1`, `0.1 as float`, nothing for
/// an instance `p{x:1}` (Op::None); a code no operator has reads as `:`
pub fn written_operator(code: usize) -> String {
	match OP_CODES.get(code).map(Op::as_str) {
		None => Op::Colon.as_str().to_string(),
		Some(word) if word.starts_with(char::is_alphabetic) => format!(" {word} "),
		Some(symbol) => symbol.to_string(),
	}
}
