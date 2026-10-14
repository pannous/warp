//! Normalization hints for guiding users toward canonical syntax
//!
//! Warp accepts many syntactic forms but has preferred canonical forms.
//! This module emits gentle hints to educate users about the preferred way.
//!
//! # Configuration
//! To change which form is canonical, modify the `Style` struct defaults.
//! For example, to prefer `def f(x) {...}` over `f(x) := ...`:
//! ```ignore
//! style.function_def = FunctionStyle::Def;
//! ```
//! Every axis also takes `Any`: all its forms are fine and never hinted (function definitions by default).

use crate::compiler::compile_context;
use crate::node::{Bracket, Node};
use crate::operators::{glyph_operator, Op};

/// The "got it" topic of a plain hint is its reason: `a` (all of this kind) silences every hint giving that reason
const HINT_TOPIC_PREFIX: &str = "hint:";
const AND_SYMBOLS: [&str; 4] = ["&&", "&", "∧", "⋀"];
const OR_SYMBOLS: [&str; 4] = ["||", "|", "∨", "⋁"];

// ============================================================================
// Position Tracking for Hints
// ============================================================================

/// Current source position for hint messages (thread-local)
#[derive(Debug, Clone, Default)]
pub struct HintPosition {
	pub file: Option<String>,
	pub line: usize,
	pub column: usize,
}

/// Run `action` without hints on this thread: a stage that repeats work already hinted (the emitter's rerun) says nothing twice
pub fn without_hints<T>(action: impl FnOnce() -> T) -> T {
	compile_context::scoped(|context| &mut context.hints_muted, true, action)
}

/// A hint as tests and hosts read it: a view of a Diagnostic of level Hint (see `capture_hints`)
#[derive(Debug, Clone, PartialEq)]
pub struct CapturedHint {
	pub original: String,
	pub canonical: String,
	/// `line:column`, empty when the emitting stage knows no position
	pub position: String,
	pub reason: String,
	/// The "I meant" edit: `canonical` in place of `original` for a rewrite; for advice the edit its caller names
	/// (`global x=1` at main's assignment), none when the advice replaces no text (`on … before the loop`)
	pub fix: Option<crate::fixits::Fix>,
}

impl CapturedHint {
	/// The preferred form as an applicable fix
	pub fn fix(&self) -> Option<crate::fixits::Fix> {
		self.fix.clone()
	}

	/// The line and column of `position` (`file:line:column` or `line:column`), 0:0 for none
	pub fn line_and_column(&self) -> (usize, usize) {
		let mut parts = self.position.rsplit(':').map(|part| part.parse().unwrap_or(0));
		let column = parts.next().unwrap_or(0);
		(parts.next().unwrap_or(0), column)
	}
}

impl From<crate::diagnostic::Diagnostic> for CapturedHint {
	fn from(hint: crate::diagnostic::Diagnostic) -> Self {
		let position = match hint.line {
			0 => String::new(),
			line => crate::compiler::meta::place_text(hint.file.as_deref(), line, hint.column),
		};
		CapturedHint {
			original: hint.written.unwrap_or_default(),
			canonical: hint.preferred.unwrap_or_default(),
			position,
			reason: hint.message,
			fix: hint.fixes.into_iter().next(),
		}
	}
}

/// Run `action` and return the hints it emitted on this thread (hints are still printed)
pub fn capture_hints<T>(action: impl FnOnce() -> T) -> (T, Vec<CapturedHint>) {
	let (result, hints) = crate::diagnostic::capture_hints(action);
	(result, hints.into_iter().map(CapturedHint::from).collect())
}

/// Set the current position for hint messages
pub fn set_hint_position(line: usize, column: usize) {
	compile_context::change(|context| {
		context.hint_position.line = line;
		context.hint_position.column = column;
	});
}

/// Set the current file for hint messages
pub fn set_hint_file(file: &str) {
	compile_context::change(|context| context.hint_position.file = Some(file.to_string()));
}

/// Clear the hint position
pub fn clear_hint_position() {
	compile_context::change(|context| context.hint_position = HintPosition::default());
}

/// Get position string in clickable format (file:line:col or line:col)
#[cfg(test)]
fn position_string() -> String {
	compile_context::read(|context| {
		let p = &context.hint_position;
		if p.line == 0 {
			return String::new();
		}
		match &p.file {
			Some(f) => format!("{}:{}:{}", f, p.line, p.column),
			None => format!("{}:{}", p.line, p.column),
		}
	})
}

// ============================================================================
// Style Configuration - Change these to swap canonical forms
// ============================================================================

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

// ============================================================================
// Hint Mode Configuration
// ============================================================================

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

pub use crate::diagnostic::clear_shown_hints;

// ============================================================================
// Core Hint Function
// ============================================================================

/// Emit a normalization hint to stderr: `canonical` is what to write instead of `original`
pub fn hint(original: &str, canonical: &str, reason: &str) {
	acknowledgeable_hint(original, canonical, reason, Some(rewrite(original, canonical, reason)));
}

/// The fix of a hint: its preferred form in place of the original
pub(crate) fn rewrite(original: &str, canonical: &str, reason: &str) -> crate::fixits::Fix {
	crate::fixits::fix(reason, original, canonical)
}

/// A hint whose preferred form is no plain rewrite of the original text: said elsewhere (`global x` at main's
/// assignment) or meaning something else (`(a<b) |> f`); `fix` is its "I meant" edit, when the advice names one
pub fn advise(original: &str, preferred: &str, reason: &str, fix: Option<crate::fixits::Fix>) {
	acknowledgeable_hint(original, preferred, reason, fix);
}

/// A hint the user can say "got it" to, for this expression or all hints of its reason (card hints-dismissed)
fn acknowledgeable_hint(original: &str, canonical: &str, reason: &str, fix: Option<crate::fixits::Fix>) {
	let topic = format!("{HINT_TOPIC_PREFIX}{reason}");
	if !crate::diagnostic::is_acknowledged(&topic, original) {
		show_hint(original, canonical, reason, fix, &topic);
	}
}

/// Says the hint (a Diagnostic of level Hint, diagnostic::say) at the hint position, unless hints are off: whether it
/// was printed
pub(crate) fn show_hint(
	original: &str,
	canonical: &str,
	reason: &str,
	fix: Option<crate::fixits::Fix>,
	topic: &str,
) -> bool {
	if hint_mode() == HintMode::Off || compile_context::read(|context| context.hints_muted) {
		return false;
	}
	let position = compile_context::read(|context| context.hint_position.clone());
	let hint = crate::diagnostic::Diagnostic {
		message: reason.to_string(),
		line: position.line,
		column: position.column,
		file: position.file.filter(|_| position.line > 0),
		fixes: fix.into_iter().collect(),
		level: crate::diagnostic::Level::Hint,
		written: Some(original.to_string()),
		preferred: Some(canonical.to_string()),
		..Default::default()
	}
	.about(topic, original);
	crate::diagnostic::say(hint)
}

// ============================================================================
// Hint Functions - Check style before emitting
// ============================================================================

pub mod hints {
	use super::*;

	/// Type constructor vs 'as' operator
	/// The type word in its canonical spelling: `str` and `String` become `string` when that is preferred
	fn canonical_type_word(used: &str) -> &str {
		if style().prefer_string_over_str && STRING_TYPE_NAMES.contains(&used) {
			"string"
		} else {
			used
		}
	}

	/// Returns whether a hint was emitted, so that the type word is not hinted a second time
	pub fn type_constructor(type_name: &str, value: &str) -> bool {
		let is_non_canonical = style().cast.instead_of(CastStyle::Constructor).is_some();
		if is_non_canonical {
			let original = format!("{}({})", type_name, value);
			let canonical = format!("{} as {}", value, canonical_type_word(type_name));
			hint(&original, &canonical, "postfix 'as' reads naturally: value as type");
		}
		is_non_canonical
	}

	/// 'as' operator when constructor style is preferred; returns whether a hint was emitted
	pub fn as_operator(value: &str, type_name: &str) -> bool {
		let is_non_canonical = style().cast.instead_of(CastStyle::AsOperator).is_some();
		if is_non_canonical {
			let original = format!("{} as {}", value, type_name);
			let canonical = format!("{}({})", canonical_type_word(type_name), value);
			hint(&original, &canonical, "constructor style preferred for casts");
		}
		is_non_canonical
	}

	/// String type name variations (str, String -> string)
	pub fn string_type(used: &str) {
		let s = style();
		if s.prefer_string_over_str && STRING_TYPE_NAMES.contains(&used) {
			hint(used, "string", "use lowercase 'string' for the string type");
		}
	}

	/// The quote character a string literal was written with
	pub fn quotes(used: char, content: &str) {
		let used_style = match used {
			'\'' => QuoteStyle::Single,
			'"' => QuoteStyle::Double,
			_ => return,
		};
		let reason = match style().quotes.instead_of(used_style) {
			Some(QuoteStyle::Single) => "single quotes preferred for strings",
			Some(QuoteStyle::Double) => "double quotes preferred for strings",
			_ => return,
		};
		let canonical_quote = text_quote();
		hint(
			&format!("{used}{content}{used}"),
			&format!("{canonical_quote}{content}{canonical_quote}"),
			reason,
		);
	}

	/// The operator as written in the source; `is_prefix` tells `!x` from an infix use
	pub fn operator(written: &str, is_prefix: bool) {
		let mut characters = written.chars();
		if let (Some(glyph), None) = (characters.next(), characters.next()) {
			if let Some(op) = glyph_operator(glyph) {
				match op.as_str() {
					"and" => logical_operator(written, "and", AND_SYMBOLS),
					"or" => logical_operator(written, "or", OR_SYMBOLS),
					"xor" => {}
					canonical => hint(written, canonical, "standard spelling of the operator"),
				}
				return;
			}
		}
		match (written, is_prefix) {
			("¬&", false) => hint(written, "nand", "standard spelling of the operator"),
			("&&" | "&" | "and", false) => logical_operator(written, "and", AND_SYMBOLS),
			("||" | "|" | "or", false) => logical_operator(written, "or", OR_SYMBOLS),
			("!" | "not", true) => not_operator(written),
			("**" | "^", false) => power_operator(written),
			("?", false) => conditional(true),
			("if", true) => conditional(false),
			_ => {}
		}
	}

	/// `and` / `&&`, `or` / `||`: the hint toward the style's spelling; `symbols` = the canonical symbol, its short
	/// form, then the logic glyphs
	fn logical_operator(used: &str, word: &str, symbols: [&str; 4]) {
		match style().logical {
			LogicalStyle::Words if symbols.contains(&used) => hint(used, word, "word operators are more readable"),
			LogicalStyle::Symbols if used == word || symbols[2..].contains(&used) => {
				hint(used, symbols[0], "symbol operators preferred")
			}
			_ => {}
		}
	}

	/// ! or not operator
	pub fn not_operator(used: &str) {
		let s = style();
		match (used, s.logical) {
			("!", LogicalStyle::Words) => hint("!", "not", "word operators are more readable"),
			("not", LogicalStyle::Symbols) => hint("not", "!", "symbol operators preferred"),
			_ => {}
		}
	}

	/// Power operator ** vs ^
	pub fn power_operator(used: &str) {
		let s = style();
		match (used, s.power) {
			("**", PowerStyle::Caret) => hint("**", "^", "use ^ for exponentiation"),
			("^", PowerStyle::DoubleStar) => hint("^", "**", "use ** for exponentiation"),
			_ => {}
		}
	}

	/// Ternary vs if-then-else
	pub fn conditional(used_ternary: bool) {
		let s = style();
		match (used_ternary, s.conditional) {
			// shown until the user acknowledges it once (card hint-dismiss)
			(true, ConditionalStyle::IfThenElse) => crate::diagnostic::educate_once(
				CONDITIONAL_TOPIC,
				"x ? y : z",
				"if x then y else z",
				"if-then-else is more readable",
			),
			(false, ConditionalStyle::Ternary) => {
				hint("if x then y else z", "x ? y : z", "ternary operator is more concise")
			}
			_ => {}
		}
	}

	/// Variable definition keywords
	pub fn var_keyword(used: &str, name: &str, value: &str) {
		let used_style = match used {
			"let" => VarStyle::Let,
			"var" => VarStyle::Var,
			":=" => VarStyle::ColonEquals,
			_ => return,
		};
		let Some(preferred) = style().var_def.instead_of(used_style) else {
			return;
		};
		let spell = |var_style: VarStyle| match var_style {
			VarStyle::ColonEquals => format!("{name} := {value}"),
			VarStyle::Let => format!("let {name} = {value}"),
			VarStyle::Var => format!("var {name} = {value}"),
			VarStyle::Any => format!("{name} = {value}"),
		};
		let reason = match preferred {
			// a style hint never changes the meaning: let and := cannot change, a variable stays one (card var-hint)
			VarStyle::ColonEquals | VarStyle::Let if used_style == VarStyle::Var => {
				return hint(
					&spell(used_style),
					&spell(VarStyle::Any),
					"a variable is a plain assignment (let and := cannot change)",
				);
			}
			VarStyle::ColonEquals => "use := for definition",
			VarStyle::Let => "use 'let' for definition",
			VarStyle::Var => "use 'var' for definition",
			VarStyle::Any => return,
		};
		match used_style {
			// shown until the user acknowledges it once (diagnostic::educate_once)
			VarStyle::Let => crate::diagnostic::educate_once(
				LET_TOPIC,
				&spell(used_style),
				&spell(preferred),
				&format!(
					"{reason}; in warp `let` is immutable (unlike JS), use var or plain = for variables that change"
				),
			),
			_ => hint(&spell(used_style), &spell(preferred), reason),
		}
	}

	/// The canonical spelling of the list of `element_words` (`["int"]`, or `["list", "int"]` for a list of lists) in `list_style`;
	/// `None` when the style cannot spell it (`ints` has no plural for a list of lists)
	fn list_type_spelling(list_style: ListTypeStyle, element_words: &[&str]) -> Option<String> {
		match (list_style, element_words) {
			(ListTypeStyle::Plural, [element]) => {
				crate::analyzer::type_word_kind(element).map(|_| format!("{element}s"))
			}
			(ListTypeStyle::Plural | ListTypeStyle::Any, _) => None,
			(ListTypeStyle::Generic, words) => {
				let (last, nested) = words.split_last()?;
				let arguments = nested
					.iter()
					.rev()
					.fold(last.to_string(), |inner, word| format!("{word}<{inner}>"));
				Some(format!("{LIST_TYPE_HEAD}<{arguments}>"))
			}
			(ListTypeStyle::Of, words) => Some(format!("{LIST_TYPE_HEAD} {OF_WORD} {}", words.join(" of "))),
			(ListTypeStyle::Bracket, words) => {
				let (last, nested) = words.split_last()?;
				Some(
					nested
						.iter()
						.rev()
						.fold(format!("[{last}]"), |inner, _| format!("[{inner}]")),
				)
			}
		}
	}

	/// A list type written in `used` style: `list<int>` and `list of int` when `ints` is preferred, `ints` when the words are
	pub fn list_type(used: ListTypeStyle, element_words: &[&str]) {
		let Some(preferred) = style().list_type.instead_of(used) else {
			return;
		};
		if let (Some(original), Some(canonical)) = (
			list_type_spelling(used, element_words),
			list_type_spelling(preferred, element_words),
		) {
			let reason = match preferred {
				ListTypeStyle::Plural => "a plural type word is a list of that type",
				ListTypeStyle::Generic => "angle brackets apply the list type to its element type",
				ListTypeStyle::Of => "'list of' reads as a phrase",
				ListTypeStyle::Bracket => "square brackets around the element type read as a list",
				ListTypeStyle::Any => return,
			};
			hint(&original, &canonical, reason);
		}
	}

	/// Function definition keywords
	pub fn function_keyword(used: &str, name: &str, params: &str, written_body: BodyForm) {
		let used_style = match used {
			"def" => FunctionStyle::Def,
			"define" => FunctionStyle::Define,
			"fn" => FunctionStyle::Fn,
			"fun" => FunctionStyle::Fun,
			"function" => FunctionStyle::Function,
			":=" => FunctionStyle::ColonEquals,
			_ => return,
		};
		let Some(preferred) = style().function_def.instead_of(used_style) else {
			return;
		};
		let spell = |function_style: FunctionStyle, body: BodyForm| {
			let body_text = match body {
				BodyForm::Colon => ": ...",
				BodyForm::Assign => " = ...",
				BodyForm::Block => " { ... }",
			};
			match function_style {
				FunctionStyle::ColonEquals | FunctionStyle::Any => format!("{name}({params}) := ..."),
				FunctionStyle::Def => format!("def {name}({params}){body_text}"),
				FunctionStyle::Define => format!("define {name}({params}){body_text}"),
				FunctionStyle::Fn => format!("fn {name}({params}){body_text}"),
				FunctionStyle::Fun => format!("fun {name}({params}){body_text}"),
				FunctionStyle::Function => format!("function {name}({params}){body_text}"),
			}
		};
		let conventional_body = |function_style: FunctionStyle| match function_style {
			FunctionStyle::Def | FunctionStyle::Define => BodyForm::Colon,
			FunctionStyle::Fn | FunctionStyle::Fun | FunctionStyle::ColonEquals | FunctionStyle::Any => {
				BodyForm::Assign
			}
			FunctionStyle::Function => BodyForm::Block,
		};
		let reason = match preferred {
			FunctionStyle::ColonEquals => "short := form preferred",
			FunctionStyle::Def => "'def' keyword preferred",
			FunctionStyle::Define => "'define' keyword preferred",
			FunctionStyle::Fn => "'fn' keyword preferred",
			FunctionStyle::Fun => "'fun' keyword preferred",
			FunctionStyle::Function => "'function' keyword preferred",
			FunctionStyle::Any => return,
		};
		hint(
			&spell(used_style, written_body),
			&spell(preferred, conventional_body(preferred)),
			reason,
		);
	}

	/// `idx` shifted by `offset` for the other index base: `s[0]` is `s#1`, brackets count from 0 and `#` from 1
	fn shifted_index(idx: &str, offset: i64) -> String {
		match idx.parse::<i64>() {
			Ok(number) => (number + offset).to_string(),
			Err(_) => format!("({idx}{offset:+})"),
		}
	}

	/// Bracket indexing (counts from 0) vs hash indexing (counts from 1)
	pub fn index_operator(var: &str, idx: &str, used_bracket: bool) {
		let looks_up_a_key = idx.starts_with(['"', '\'']); // `ages["alice"]` counts no position
													 // cards g-2WPo, g_YiSA (user): only a number, where `xs#2` is as short as `xs[1]`; `xs[i]` and `xs[i+1]` stay
		if looks_up_a_key || !is_number_index(idx) {
			return;
		}
		let s = style();
		match (used_bracket, s.index) {
			(true, IndexStyle::Hash) => {
				let original = format!("{}[{}]", var, idx);
				let canonical = format!("{}#{}", var, shifted_index(idx, 1));
				hint(&original, &canonical, "use # for indexing");
			}
			(false, IndexStyle::Bracket) => {
				let original = format!("{}#{}", var, idx);
				let canonical = format!("{}[{}]", var, shifted_index(idx, -1));
				hint(&original, &canonical, "use [] for indexing");
			}
			_ => {}
		}
	}

	fn is_number_index(idx: &str) -> bool {
		let digits = idx.strip_prefix('-').unwrap_or(idx);
		!digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit())
	}

	/// Length method vs # operator
	pub fn length_operator(var: &str, used_method: bool) {
		let s = style();
		match (used_method, s.index) {
			(true, IndexStyle::Hash) => {
				let original = format!("{}.length()", var);
				let canonical = format!("#{}", var);
				hint(&original, &canonical, "use # prefix for length");
			}
			(false, IndexStyle::Bracket) => {
				let original = format!("#{}", var);
				let canonical = format!("{}.length()", var);
				hint(&original, &canonical, "use .length() for length");
			}
			_ => {}
		}
	}
}

// ============================================================================
// Style check of a parsed program: the forms that are only visible in the AST
// ============================================================================

/// Type word of the list types; `list<int>`, `list of int` and `ints` all denote a list of int
const LIST_TYPE_HEAD: &str = "list";
const OF_WORD: &str = "of";
/// The acknowledge-once note that `let` is immutable in warp
pub const LET_TOPIC: &str = "let";
/// The acknowledge-once note on `x ? y : z` against `if x then y else z`
pub const CONDITIONAL_TOPIC: &str = "conditional";
const STRING_TYPE_NAMES: [&str; 2] = ["str", "String"];

/// The quote character of the canonical string style
pub fn text_quote() -> char {
	match style().quotes {
		QuoteStyle::Single => '\'',
		QuoteStyle::Double | QuoteStyle::Any => '"',
	}
}

/// Source text of an operand for a hint
pub fn operand_text(node: &Node) -> String {
	match node.drop_meta() {
		Node::Text(text) => format!("{0}{text}{0}", text_quote()),
		other => as_written(other.clone()).serialize(),
	}
}

/// The node as its source spelled it where a pass lowered sugar: `floor_quotient(a, b)` is `a//b`, `area·square` is `area`
fn as_written(node: Node) -> Node {
	match node {
		Node::List(ref items, _, _)
			if let [word, dividend, divisor] = items.as_slice()
				&& matches!(word.drop_meta(), Node::Symbol(name) if name == crate::warp_parser::FLOOR_QUOTIENT) =>
		{
			Node::Symbol(format!("{}//{}", operand_text(dividend), operand_text(divisor)))
		}
		// a witness `area·square` was written `area`
		Node::Symbol(name) if name.contains('·') => {
			Node::Symbol(name.split('·').next().unwrap_or_default().to_string())
		}
		other => crate::lowering::nodes::children_rewritten(other, as_written),
	}
}

/// Visit every node as `(node without metadata, node with metadata)`: the second one knows the source position
fn walk<'a>(node: &'a Node, action: &mut dyn FnMut(&'a Node, &'a Node)) {
	let bare = node.drop_meta();
	action(bare, node);
	match bare {
		Node::Key(left, _, right) => {
			walk(left, action);
			walk(right, action);
		}
		Node::List(items, _, _) => items.iter().for_each(|item| walk(item, action)),
		_ => {}
	}
}

/// The position where the node starts: its own, else that of its leftmost child; unknown positions are cleared, never stale
pub fn set_position_of(node: &Node) {
	fn start_of(node: &Node) -> Option<(usize, usize)> {
		if let Some(info) = node.get_lineinfo() {
			return Some((info.line_nr, info.column));
		}
		match node {
			Node::Meta { node, .. } => start_of(node),
			// a word a pass made (`times` of `3 times 4`) has no position: the first part that has one
			Node::Key(left, _, right) => start_of(left).or_else(|| start_of(right)),
			Node::List(items, _, _) => items.iter().find_map(start_of),
			_ => None,
		}
	}
	match start_of(node) {
		Some((line, column)) => set_hint_position(line, column),
		None => clear_hint_position(),
	}
}

/// `x:list of int=[1 2]` is the items `x:list`, `of`, `int=[1 2]`: the element words after `of`, and the node of the head word `list`
fn of_list_type(items: &[Node], of_index: usize) -> Option<(&Node, Vec<&str>)> {
	let Node::Key(_, Op::Colon, head) = items.get(of_index.checked_sub(1)?)?.drop_meta() else {
		return None;
	};
	(crate::warp_parser::list_type_head(head.symbol_name()?) == LIST_TYPE_HEAD).then_some(())?;
	let mut words = Vec::new();
	for item in &items[of_index + 1..] {
		match item.drop_meta() {
			Node::Symbol(word) if word == OF_WORD => continue,
			Node::Symbol(word) => words.push(word.as_str()),
			Node::Key(word, _, _) => {
				words.push(word.symbol_name()?);
				break;
			}
			_ => return None,
		}
	}
	(!words.is_empty()).then_some((head, words))
}

/// The name and parameter list of a definition after its keyword: `def (f x):…`, `fn (f x)=…`, `function ((f (x)) {…})`
fn signature_of(definition: &Node) -> Option<(String, String, BodyForm)> {
	let (signature, body_form) = match definition.drop_meta() {
		Node::Key(signature, Op::Colon, _) => (signature.as_ref(), BodyForm::Colon),
		Node::Key(signature, _, _) => (signature.as_ref(), BodyForm::Assign),
		Node::List(parts, _, _) => (parts.first()?, BodyForm::Block),
		_ => return None,
	};
	let Node::List(words, _, _) = signature.drop_meta() else {
		return None;
	};
	let (name, parameters) = words.split_first()?;
	let parameters: Vec<String> = parameters
		.iter()
		.map(|parameter| parameter.serialize().trim_matches(|c| c == '(' || c == ')').to_string())
		.collect();
	Some((name.symbol_name()?.to_string(), parameters.join(", "), body_form))
}

/// Emit the hints for every non-canonical form in the parsed program that the parser cannot see while reading characters
pub fn check_style(program: &Node) {
	walk(program, &mut |node, positioned| match node {
		// `[int, 3]` and `[double, f]` are items, never the call int(3)
		Node::List(items, bracket, separator)
			if *bracket != Bracket::Square && *separator != crate::node::Separator::Colon =>
		{
			check_items(items, positioned)
		}
		Node::Key(_, Op::Colon, right) => {
			if let Node::List(items, Bracket::Square, _) = right.drop_meta() {
				if let [element] = items.as_slice() {
					if let Some(word) = element.symbol_name() {
						set_position_of(right);
						hints::list_type(ListTypeStyle::Bracket, &[word]);
					}
				}
				return;
			}
			let Some(type_word) = right.symbol_name() else { return };
			set_position_of(right);
			if let Some(element) = crate::analyzer::plural_element_type(type_word) {
				hints::list_type(ListTypeStyle::Plural, &[element]);
			}
			hints::string_type(type_word);
		}
		Node::Key(value, Op::As, type_node) => {
			if let Some(type_word) = type_node.symbol_name() {
				set_position_of(positioned);
				if !hints::as_operator(&operand_text(value), type_word) {
					hints::string_type(type_word);
				}
			}
		}
		Node::Key(target, Op::Define, value) if target.symbol_name().is_some() => {
			set_position_of(positioned);
			hints::var_keyword(":=", &operand_text(target), &operand_text(value));
		}
		Node::Key(_, Op::Define, _) => {
			if let Some((name, parameters, _)) = signature_of(node) {
				set_position_of(positioned);
				hints::function_keyword(":=", &name, &parameters, BodyForm::Assign);
			}
		}
		Node::Key(target, Op::Dot, call) => {
			if matches!(call.drop_meta(), Node::List(words, _, _) if words.len() == 1 && (words[0]).symbol_name() == Some("length"))
			{
				set_position_of(positioned);
				hints::length_operator(&operand_text(target), true);
			}
		}
		_ => {}
	});
}

fn check_items(items: &[Node], positioned: &Node) {
	if let Some(of_index) = items.iter().position(|item| item.symbol_name() == Some(OF_WORD)) {
		if let Some((head, words)) = of_list_type(items, of_index) {
			set_position_of(head);
			let _ = crate::warp_parser::noted_list_type_head(head.symbol_name().unwrap_or_default());
			hints::list_type(ListTypeStyle::Of, &words);
		}
	}
	let [keyword, rest @ ..] = items else { return };
	let Some(keyword_word) = keyword.symbol_name() else {
		return;
	};
	set_position_of(positioned);
	match (keyword_word, rest) {
		("let" | "var", [declaration, ..]) => {
			if let Node::Key(name, Op::Assign | Op::Define, value) = declaration.drop_meta() {
				hints::var_keyword(keyword_word, &operand_text(name), &operand_text(value));
			}
		}
		("len", [argument]) => {
			let counted = operand_text(argument);
			let counted = counted.trim_matches(|c| c == '(' || c == ')');
			hint(
				&format!("len({counted})"),
				&format!("#{counted}"),
				"use # prefix for length",
			);
		}
		(word, [definition, ..]) if crate::operators::is_function_keyword(word) => {
			if let Some((name, parameters, body_form)) = signature_of(definition) {
				hints::function_keyword(word, &name, &parameters, body_form);
			}
		}
		// `codepoint(c)` is a library word (P74), not the type's constructor
		(type_word, [argument])
			if crate::analyzer::type_word_kind(&type_word.to_lowercase()).is_some()
				&& !crate::library_words::is_library_word(type_word) =>
		{
			let is_call = match argument.drop_meta() {
				// `int g(int x) {…}` defines g, `int(…)` would convert
				Node::List(parts, Bracket::Round, _) if matches!(parts.as_slice(), [_, body] if matches!(body.drop_meta(), Node::List(_, Bracket::Curly, _))) => {
					false
				}
				// `text(p:person)` heads the Printable operation of a declared type (P31)
				Node::List(parts, Bracket::Round, _) if matches!(parts.as_slice(), [part] if matches!(part.drop_meta(), Node::Key(_, Op::Colon, _))) => {
					false
				}
				Node::Key(_, Op::Colon, _) => false,
				Node::List(_, Bracket::Round, _) => true,
				Node::Symbol(_) | Node::Key(_, Op::Assign | Op::Define, _) => false, // `int x` and `int x = 1` declare
				_ => true,
			};
			if is_call {
				let value = match argument.drop_meta() {
					Node::List(inner, Bracket::Round, _) if inner.len() == 1 => &inner[0],
					other => other,
				};
				if !hints::type_constructor(type_word, &operand_text(value)) {
					hints::string_type(type_word);
				}
			}
		}
		_ => {}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_hint_mode() {
		set_hint_mode(HintMode::Off);
		assert_eq!(hint_mode(), HintMode::Off);

		set_hint_mode(HintMode::Once);
		assert_eq!(hint_mode(), HintMode::Once);

		set_hint_mode(HintMode::Always);
		assert_eq!(hint_mode(), HintMode::Always);
	}

	#[test]
	fn test_style_swap() {
		// Default leaves the cast spelling open (user #13: str(x) is fine)
		let s = style();
		assert_eq!(s.cast, CastStyle::Any);

		// Swap to constructor style
		set_style(Style {
			cast: CastStyle::Constructor,
			..Style::default()
		});

		let s = style();
		assert_eq!(s.cast, CastStyle::Constructor);

		// Reset to default
		set_style(Style::default());
	}

	#[test]
	fn test_hint_position() {
		assert_eq!(crate::wasm_emitter::eval("'abc'"), "abc"); // hint:
														 // Clear position
		clear_hint_position();
		assert_eq!(position_string(), "");

		// Set position without file
		set_hint_position(10, 5);
		assert_eq!(position_string(), "10:5");

		set_hint_file("test.warp");
		set_hint_position(42, 13);
		assert_eq!(position_string(), "test.warp:42:13");

		// Clear again
		clear_hint_position();
		assert_eq!(position_string(), "");
	}
}
