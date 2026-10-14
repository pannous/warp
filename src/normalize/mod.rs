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
use crate::words::{LIST_WORD, OF_WORD};
const STRING_TYPE_NAMES: [&str; 2] = ["str", "String"];

mod check_style;
pub use check_style::*;
pub mod hints;
mod positions;
pub use positions::*;
mod style;
pub use style::*;

pub use crate::errors::diagnostic::clear_shown_hints;

// ============================================================================
// Core Hint Function
// ============================================================================

/// Emit a normalization hint to stderr: `canonical` is what to write instead of `original`
pub fn hint(original: &str, canonical: &str, reason: &str) {
	acknowledgeable_hint(original, canonical, reason, Some(rewrite(original, canonical, reason)));
}

/// The fix of a hint: its preferred form in place of the original
pub(crate) fn rewrite(original: &str, canonical: &str, reason: &str) -> crate::errors::fixits::Fix {
	crate::errors::fixits::fix(reason, original, canonical)
}

/// A hint whose preferred form is no plain rewrite of the original text: said elsewhere (`global x` at main's
/// assignment) or meaning something else (`(a<b) |> f`); `fix` is its "I meant" edit, when the advice names one
pub fn advise(original: &str, preferred: &str, reason: &str, fix: Option<crate::errors::fixits::Fix>) {
	acknowledgeable_hint(original, preferred, reason, fix);
}

/// A hint the user can say "got it" to, for this expression or all hints of its reason (card hints-dismissed)
fn acknowledgeable_hint(original: &str, canonical: &str, reason: &str, fix: Option<crate::errors::fixits::Fix>) {
	let topic = format!("{HINT_TOPIC_PREFIX}{reason}");
	if !crate::errors::diagnostic::is_acknowledged(&topic, original) {
		show_hint(original, canonical, reason, fix, &topic);
	}
}

/// Says the hint (a Diagnostic of level Hint, diagnostic::say) at the hint position, unless hints are off: whether it
/// was printed
pub(crate) fn show_hint(
	original: &str,
	canonical: &str,
	reason: &str,
	fix: Option<crate::errors::fixits::Fix>,
	topic: &str,
) -> bool {
	if hint_mode() == HintMode::Off || compile_context::read(|context| context.hints_muted) {
		return false;
	}
	let position = compile_context::read(|context| context.hint_position.clone());
	let hint = crate::errors::diagnostic::Diagnostic {
		message: reason.to_string(),
		line: position.line,
		column: position.column,
		file: position.file.filter(|_| position.line > 0),
		fixes: fix.into_iter().collect(),
		level: crate::errors::diagnostic::Level::Hint,
		written: Some(original.to_string()),
		preferred: Some(canonical.to_string()),
		..Default::default()
	}
	.about(topic, original);
	crate::errors::diagnostic::say(hint)
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
		set_style(Style { cast: CastStyle::Constructor, ..Style::default() });

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
