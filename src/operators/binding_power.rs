//! How tightly each operator binds: the left and right binding powers the parser climbs by
use super::*;

/// Right binding power of `as`: higher than every infix operator, so the target type is a single atom
pub const TYPE_OPERAND_BP: u8 = 250;
/// Left binding power of the suffix operators ² ³ ++ --: below `.` (180) and `#` (170), above `^` (160)
const SUFFIX_BP: u8 = 165;
/// Right binding power of the prefix operators √ ∛ abs: between ^ (160) and the suffix operators (SUFFIX_BP)
const PREFIX_BP: u8 = 162;

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
			Op::Mul | Op::Div | Op::FloorDiv | Op::Mod | Op::Rem => (150, 151),

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
			| Op::FloorDivAssign
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
}
