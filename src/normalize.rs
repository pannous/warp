//! Normalization hints for guiding users toward canonical syntax
//!
//! Wasp accepts many syntactic forms but has preferred canonical forms.
//! This module emits gentle hints to educate users about the preferred way.
//!
//! # Configuration
//! To change which form is canonical, modify the `Style` struct defaults.
//! For example, to prefer `def f(x) {...}` over `f(x) := ...`:
//! ```ignore
//! style.function_def = FunctionStyle::Def;
//! ```

use once_cell::sync::Lazy;
use std::cell::RefCell;
use std::collections::HashSet;
use std::sync::Mutex;

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

thread_local! {
    static HINT_POSITION: RefCell<HintPosition> = RefCell::new(HintPosition::default());
}

/// Set the current position for hint messages
pub fn set_hint_position(line: usize, column: usize) {
	HINT_POSITION.with(|pos| {
		let mut p = pos.borrow_mut();
		p.line = line;
		p.column = column;
	});
}

/// Set the current file for hint messages
pub fn set_hint_file(file: &str) {
	HINT_POSITION.with(|pos| {
		pos.borrow_mut().file = Some(file.to_string());
	});
}

/// Clear the hint position
pub fn clear_hint_position() {
	HINT_POSITION.with(|pos| {
		*pos.borrow_mut() = HintPosition::default();
	});
}

/// Get position string in clickable format (file:line:col or line:col)
fn position_string() -> String {
	HINT_POSITION.with(|pos| {
		let p = pos.borrow();
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
}

/// Preferred style for logical operators
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LogicalStyle {
	/// `and`, `or`, `not`
	Words,
	/// `&&`, `||`, `!`
	Symbols,
}

/// Preferred style for string quotes
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum QuoteStyle {
	/// `'hello'`
	Single,
	/// `"hello"`
	Double,
}

/// Preferred style for indexing
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IndexStyle {
	/// `x#0`
	Hash,
	/// `x[0]`
	Bracket,
}

/// Preferred style for conditionals
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConditionalStyle {
	DontCare,
	/// `if x then y else z`
	IfThenElse,
	/// `x ? y : z`
	Ternary,
}

/// Preferred style for power operator
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PowerStyle {
	/// `x^2`
	Caret,
	/// `x**2`
	DoubleStar,
}

/// Global style configuration
#[derive(Debug, Clone)]
pub struct Style {
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

impl Default for Style {
	fn default() -> Self {
		Self {
			cast: CastStyle::AsOperator,
			function_def: FunctionStyle::ColonEquals,
			var_def: VarStyle::ColonEquals,
			logical: LogicalStyle::Words,
			quotes: QuoteStyle::Single,
			index: IndexStyle::Hash,
			conditional: ConditionalStyle::DontCare,
			power: PowerStyle::Caret,
			prefer_string_over_str: true,
		}
	}
}

/// Global style setting
static STYLE: Lazy<Mutex<Style>> = Lazy::new(|| Mutex::new(Style::default()));

/// Set the global style
pub fn set_style(style: Style) {
	if let Ok(mut s) = STYLE.lock() {
		*s = style;
	}
}

/// Get the current style (cloned)
pub fn style() -> Style {
	STYLE.lock().map(|s| s.clone()).unwrap_or_default()
}

// ============================================================================
// Hint Mode Configuration
// ============================================================================

/// Global set of hints already shown (for "once" mode)
static SHOWN_HINTS: Lazy<Mutex<HashSet<String>>> = Lazy::new(|| Mutex::new(HashSet::new()));

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

/// Global hint mode setting
static HINT_MODE: Lazy<Mutex<HintMode>> = Lazy::new(|| Mutex::new(HintMode::Always));

/// Set the global hint mode
pub fn set_hint_mode(mode: HintMode) {
	if let Ok(mut m) = HINT_MODE.lock() {
		*m = mode;
	}
}

/// Get the current hint mode
pub fn hint_mode() -> HintMode {
	HINT_MODE.lock().map(|m| *m).unwrap_or(HintMode::Always)
}

/// Clear shown hints (useful for testing)
pub fn clear_shown_hints() {
	if let Ok(mut shown) = SHOWN_HINTS.lock() {
		shown.clear();
	}
}

// ============================================================================
// Core Hint Function
// ============================================================================

/// Emit a normalization hint to stderr
pub fn hint(original: &str, canonical: &str, reason: &str) {
	let mode = hint_mode();
	if mode == HintMode::Off {
		return;
	}

	let key = format!("{}|{}", original, canonical);

	if mode == HintMode::Once {
		if let Ok(mut shown) = SHOWN_HINTS.lock() {
			if shown.contains(&key) {
				return;
			}
			shown.insert(key);
		}
	}

	let pos = position_string();
	if pos.is_empty() {
		eprintln!(
			"\x1b[36mhint:\x1b[0m prefer `\x1b[32m{}\x1b[0m` over `\x1b[33m{}\x1b[0m`",
			canonical, original
		);
	} else {
		eprintln!(
			"\x1b[36mhint\x1b[0m \x1b[90m{}\x1b[0m: prefer `\x1b[32m{}\x1b[0m` over `\x1b[33m{}\x1b[0m`",
			pos, canonical, original
		);
	}
	eprintln!("      {}", reason);
}

// ============================================================================
// Hint Functions - Check style before emitting
// ============================================================================

pub mod hints {
	use super::*;

	/// Type constructor vs 'as' operator
	pub fn type_constructor(type_name: &str, value: &str) {
		let s = style();
		if s.cast == CastStyle::AsOperator {
			let original = format!("{}({})", type_name, value);
			let canonical = format!("{} as {}", value, type_name);
			hint(&original, &canonical, "postfix 'as' reads naturally: value as type");
		}
	}

	/// 'as' operator when constructor style is preferred
	pub fn as_operator(value: &str, type_name: &str) {
		let s = style();
		if s.cast == CastStyle::Constructor {
			let original = format!("{} as {}", value, type_name);
			let canonical = format!("{}({})", type_name, value);
			hint(&original, &canonical, "constructor style preferred for casts");
		}
	}

	/// String type name variations (str, String -> string)
	pub fn string_type(used: &str) {
		let s = style();
		if s.prefer_string_over_str && (used == "str" || used == "String") {
			hint(used, "string", "use lowercase 'string' for the string type");
		}
	}

	/// Single quotes when double preferred
	pub fn single_quotes(content: &str) {
		let s = style();
		if s.quotes == QuoteStyle::Double {
			let original = format!("'{}'", content);
			let canonical = format!("\"{}\"", content);
			hint(&original, &canonical, "double quotes preferred for strings");
		}
	}

	/// && operator
	pub fn and_operator(used: &str) {
		let s = style();
		match (used, s.logical) {
			("&&", LogicalStyle::Words) => hint("&&", "and", "word operators are more readable"),
			("and", LogicalStyle::Symbols) => hint("and", "&&", "symbol operators preferred"),
			_ => {}
		}
	}

	/// || operator
	pub fn or_operator(used: &str) {
		let s = style();
		match (used, s.logical) {
			("||", LogicalStyle::Words) => hint("||", "or", "word operators are more readable"),
			("or", LogicalStyle::Symbols) => hint("or", "||", "symbol operators preferred"),
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
			(_, ConditionalStyle::DontCare) => {}
			(true, ConditionalStyle::IfThenElse) => {
				hint("x ? y : z", "if x then y else z", "if-then-else is more readable")
			}
			(false, ConditionalStyle::Ternary) => {
				hint("if x then y else z", "x ? y : z", "ternary operator is more concise")
			}
			_ => {}
		}
	}

	/// Variable definition keywords
	pub fn var_keyword(used: &str) {
		let s = style();
		let canonical = match s.var_def {
			VarStyle::ColonEquals => "x := ...",
			VarStyle::Let => "let x = ...",
			VarStyle::Var => "var x = ...",
		};
		let reason = match s.var_def {
			VarStyle::ColonEquals => "use := for definition",
			VarStyle::Let => "use 'let' for definition",
			VarStyle::Var => "use 'var' for definition",
		};
		match used {
			"let" if s.var_def != VarStyle::Let => hint("let x = ...", canonical, reason),
			"var" if s.var_def != VarStyle::Var => hint("var x = ...", canonical, reason),
			":=" if s.var_def != VarStyle::ColonEquals => hint("x := ...", canonical, reason),
			_ => {}
		}
	}

	/// Function definition keywords
	pub fn function_keyword(used: &str, name: &str, params: &str) {
		let s = style();
		let canonical_form = match s.function_def {
			FunctionStyle::ColonEquals => format!("{}({}) := ...", name, params),
			FunctionStyle::Def => format!("def {}({}): ...", name, params),
			FunctionStyle::Fn => format!("fn {}({}) = ...", name, params),
			FunctionStyle::Fun => format!("fun {}({}) = ...", name, params),
			FunctionStyle::Function => format!("function {}({}) {{ ... }}", name, params),
		};
		let reason = match s.function_def {
			FunctionStyle::ColonEquals => "short := form preferred",
			FunctionStyle::Def => "'def' keyword preferred",
			FunctionStyle::Fn => "'fn' keyword preferred",
			FunctionStyle::Fun => "'fun' keyword preferred",
			FunctionStyle::Function => "'function' keyword preferred",
		};

		let original = match used {
			"def" => format!("def {}({}): ...", name, params),
			"fn" => format!("fn {}({}) = ...", name, params),
			"fun" => format!("fun {}({}) = ...", name, params),
			"function" => format!("function {}({}) {{ ... }}", name, params),
			":=" => format!("{}({}) := ...", name, params),
			_ => return,
		};

		// Only hint if used style differs from preferred
		let used_style = match used {
			"def" => FunctionStyle::Def,
			"fn" => FunctionStyle::Fn,
			"fun" => FunctionStyle::Fun,
			"function" => FunctionStyle::Function,
			":=" => FunctionStyle::ColonEquals,
			_ => return,
		};

		if used_style != s.function_def {
			hint(&original, &canonical_form, reason);
		}
	}

	/// Bracket indexing vs hash indexing
	pub fn index_operator(var: &str, idx: &str, used_bracket: bool) {
		let s = style();
		match (used_bracket, s.index) {
			(true, IndexStyle::Hash) => {
				let original = format!("{}[{}]", var, idx);
				let canonical = format!("{}#{}", var, idx);
				hint(&original, &canonical, "use # for indexing");
			}
			(false, IndexStyle::Bracket) => {
				let original = format!("{}#{}", var, idx);
				let canonical = format!("{}[{}]", var, idx);
				hint(&original, &canonical, "use [] for indexing");
			}
			_ => {}
		}
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

#[cfg(test)]
mod tests {
	use super::*;
	use crate::is;

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
		// Default prefers 'as' operator
		let s = style();
		assert_eq!(s.cast, CastStyle::AsOperator);

		// Swap to constructor style
		let mut new_style = Style::default();
		new_style.cast = CastStyle::Constructor;
		set_style(new_style);

		let s = style();
		assert_eq!(s.cast, CastStyle::Constructor);

		// Reset to default
		set_style(Style::default());
	}

	#[test]
	fn test_hint_position() {
		is!("'abc'", "abc"); // hint:
		// Clear position
		clear_hint_position();
		assert_eq!(position_string(), "");

		// Set position without file
		set_hint_position(10, 5);
		assert_eq!(position_string(), "10:5");

		// Set file
		set_hint_file("test.wasp");
		set_hint_position(42, 13);
		assert_eq!(position_string(), "test.wasp:42:13");

		// Clear again
		clear_hint_position();
		assert_eq!(position_string(), "");
	}
}
