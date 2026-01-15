///! Normalization hints for guiding users toward canonical syntax
///!
///! Wasp accepts many syntactic forms but has preferred canonical forms.
///! This module emits gentle hints to educate users about the preferred way.

use std::collections::HashSet;
use std::sync::Mutex;
use once_cell::sync::Lazy;

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

    eprintln!("\x1b[36mhint:\x1b[0m prefer `\x1b[32m{}\x1b[0m` over `\x1b[33m{}\x1b[0m`", canonical, original);
    eprintln!("      {}", reason);
}

/// Common normalization hints as constants for consistency
pub mod hints {
    use super::hint;

    /// Type constructor vs 'as' operator
    pub fn type_constructor(type_name: &str, value: &str) {
        let original = format!("{}({})", type_name, value);
        let canonical = format!("{} as {}", value, type_name);
        hint(&original, &canonical, "postfix 'as' reads naturally: value as type");
    }

    /// String type name variations
    pub fn string_type(used: &str) {
        if used == "str" || used == "String" {
            hint(used, "string", "use lowercase 'string' for the string type");
        }
    }

    /// Double quotes vs single quotes
    pub fn double_quotes(content: &str) {
        let original = format!("\"{}\"", content);
        let canonical = format!("'{}'", content);
        hint(&original, &canonical, "single quotes preferred for strings");
    }

    /// C-style operators vs word operators
    pub fn c_style_and() {
        hint("&&", "and", "word operators are more readable");
    }

    pub fn c_style_or() {
        hint("||", "or", "word operators are more readable");
    }

    pub fn c_style_not() {
        hint("!", "not", "word operators are more readable");
    }

    pub fn c_style_ne() {
        hint("!=", "<>", "mathematical notation for not-equal");
    }

    /// Power operator
    pub fn double_star_power() {
        hint("**", "^", "use ^ for exponentiation");
    }

    /// Ternary vs if-then-else
    pub fn ternary_operator() {
        hint("? :", "if ... then ... else ...", "if-then-else is more readable");
    }

    /// Let/var keywords
    pub fn let_keyword() {
        hint("let x = ...", "x := ...", "no 'let' keyword needed, use := for definition");
    }

    pub fn var_keyword() {
        hint("var x = ...", "x := ...", "no 'var' keyword needed, use := for definition");
    }

    /// Function definition styles
    pub fn function_keyword() {
        hint("function f(...) {...}", "f(...) := ...", "short definition form preferred");
    }

    pub fn def_keyword() {
        hint("def f(...): ...", "f(...) := ...", "short definition form preferred");
    }

    pub fn fn_keyword() {
        hint("fn f(...) = ...", "f(...) := ...", "short definition form preferred");
    }

    /// Array indexing
    pub fn bracket_index(var: &str, idx: &str) {
        let original = format!("{}[{}]", var, idx);
        let canonical = format!("{}#{}", var, idx);
        hint(&original, &canonical, "use # for indexing");
    }

    /// Length method vs # operator
    pub fn length_method(var: &str) {
        let original = format!("{}.length()", var);
        let canonical = format!("#{}", var);
        hint(&original, &canonical, "use # prefix for length");
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
}
