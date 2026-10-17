//! The canonical forms, one axis each (`Style`), and the hint mode: change their defaults to swap what is canonical
use super::*;

/// Preferred style for type casting
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CastStyle {
	/// `x as int` (postfix)
	AsOperator,
	/// `int(x)` (constructor call)
	Constructor,
	/// every form is fine: no hint
	Any,
}

/// Preferred style for function definitions
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FunctionStyle {
	/// `f(x) := x*2`
	ColonEquals,
	/// `def f(x): x*2` or `def f(x) { x*2 }`
	Def,
	/// `fn f(x) = x*2`
	Fn,
	/// `fun f(x) = x*2`
	Fun,
	/// `function f(x) { x*2 }`
	Function,
	/// `define f(x): x*2`
	Define,
	/// every form is fine: no hint
	Any,
}

/// How the body of a function definition was written after its signature
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BodyForm {
	/// `def f(x): x*2`
	Colon,
	/// `fn f(x) = x*2`
	Assign,
	/// `def f(x) { x*2 }`
	Block,
}

/// Preferred style for variable definitions
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VarStyle {
	/// `x := 5`
	ColonEquals,
	/// `let x = 5`
	Let,
	/// `var x = 5`
	Var,
	/// every form is fine: no hint
	Any,
}

/// Preferred style for logical operators
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LogicalStyle {
	/// `and`, `or`, `not`
	Words,
	/// `&&`, `||`, `!`
	Symbols,
	/// every form is fine: no hint
	Any,
}

/// Preferred style for string quotes
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum QuoteStyle {
	/// `'hello'`
	Single,
	/// `"hello"`
	Double,
	/// every form is fine: no hint
	Any,
}

/// Preferred style for indexing
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IndexStyle {
	/// `x#0`
	Hash,
	/// `x[0]`
	Bracket,
	/// every form is fine: no hint
	Any,
}

/// Preferred style for conditionals
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConditionalStyle {
	/// `if x then y else z`
	IfThenElse,
	/// `x ? y : z`
	Ternary,
	/// every form is fine: no hint
	Any,
}

/// Preferred style for power operator
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PowerStyle {
	/// `x^2`
	Caret,
	/// `x**2`
	DoubleStar,
	/// every form is fine: no hint
	Any,
}

/// Preferred style for the type of a list of elements
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ListTypeStyle {
	/// `ints`, `texts`, `floats`: the plural type word
	Plural,
	/// `list<int>`
	Generic,
	/// `list of int`
	Of,
	/// `[int]`
	Bracket,
	/// every form is fine: no hint
	Any,
}

/// A style axis: one preferred form, or `Any` ("I don't care, both are fine": every form compiles, never a hint)
pub trait Preferred: Copy + PartialEq {
	const ANY: Self;

	/// The preferred form to hint toward when `used` is written: none when it is the preferred one or the axis is `Any`
	fn instead_of(self, used: Self) -> Option<Self> {
		(self != Self::ANY && self != used).then_some(self)
	}
}

macro_rules! preferred_axes {
    ($($axis:ident),*) => { $(impl Preferred for $axis { const ANY: Self = $axis::Any; })* };
}
preferred_axes!(
	CastStyle,
	FunctionStyle,
	VarStyle,
	LogicalStyle,
	QuoteStyle,
	IndexStyle,
	ConditionalStyle,
	PowerStyle,
	ListTypeStyle
);

/// Global style configuration
#[derive(Debug, Clone)]
pub struct Style {
	pub list_type: ListTypeStyle,
	pub cast: CastStyle,
	pub function_def: FunctionStyle,
	pub var_def: VarStyle,
	pub logical: LogicalStyle,
	pub quotes: QuoteStyle,
	pub index: IndexStyle,
	pub conditional: ConditionalStyle,
	pub power: PowerStyle,
	pub prefer_string_over_str: bool,
}

impl Style {
	/// One spelling per form, every axis decided: what a formatter writes and what the hint machinery is tested
	/// with; the default leaves open what the user called legitimate (#11, #12, #13)
	pub fn canonical() -> Self {
		Self {
			cast: CastStyle::AsOperator,
			var_def: VarStyle::ColonEquals,
			quotes: QuoteStyle::Double,
			prefer_string_over_str: true,
			..Self::default()
		}
	}
}

impl Default for Style {
	fn default() -> Self {
		Self {
			list_type: ListTypeStyle::Plural,
			cast: CastStyle::Any,             // user #13: `str(x)` is fine
			function_def: FunctionStyle::Any, // user 2026-10-03: `f(x) := …`, `def f(x): …` … are all fine
			var_def: VarStyle::Any,           // user #12: `let t = …` and `t := …` are both legitimate
			logical: LogicalStyle::Words,
			quotes: QuoteStyle::Any, // user #11: "I don't care" about 'x' or "x"
			index: IndexStyle::Hash,
			conditional: ConditionalStyle::IfThenElse,
			power: PowerStyle::Caret,
			prefer_string_over_str: false, // user #13: str(x) gets no hint
		}
	}
}

/// Set the style of this thread's compilations
pub fn set_style(style: Style) {
	compile_context::change(|context| context.style = style);
}

/// The style of this thread's compilations (cloned)
pub fn style() -> Style {
	compile_context::read(|context| context.style.clone())
}

/// Hint display mode
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HintMode {
	/// Show hints every time
	Always,
	/// Show each unique hint only once per session
	Once,
	/// Disable all hints
	Off,
}

/// Set the hint mode of this thread's compilations
pub fn set_hint_mode(mode: HintMode) {
	compile_context::change(|context| context.hint_mode = mode);
}

/// The hint mode of this thread's compilations
pub fn hint_mode() -> HintMode {
	compile_context::read(|context| context.hint_mode)
}
