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

use std::cell::RefCell;
use std::collections::HashSet;
use crate::node::{Bracket, Node};
use crate::operators::{glyph_operator, Op};

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
    static CAPTURED_HINTS: RefCell<Option<Vec<CapturedHint>>> = const { RefCell::new(None) };
    static HINTS_MUTED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Run `action` without hints on this thread: a stage that repeats work already hinted (the emitter's rerun) says nothing twice
pub fn without_hints<T>(action: impl FnOnce() -> T) -> T {
    let muted = HINTS_MUTED.with(|cell| cell.replace(true));
    let result = action();
    HINTS_MUTED.with(|cell| cell.set(muted));
    result
}

/// A hint recorded instead of only printed, see `capture_hints`
#[derive(Debug, Clone, PartialEq)]
pub struct CapturedHint {
    pub original: String,
    pub canonical: String,
    /// `line:column`, empty when the emitting stage knows no position
    pub position: String,
    pub reason: String,
    /// Whether `canonical` replaces `original` in the source (a rewrite), or is advice about it (`global x`)
    pub rewrites: bool,
}

impl CapturedHint {
    /// The preferred form as an applicable fix of the original; advice is no fix
    pub fn fix(&self) -> Option<crate::fixits::Fix> {
        self.rewrites.then(|| crate::fixits::fix(&self.reason, &self.original, &self.canonical))
    }

    /// The line and column of `position` (`file:line:column` or `line:column`), 0:0 for none
    pub fn line_and_column(&self) -> (usize, usize) {
        let mut parts = self.position.rsplit(':').map(|part| part.parse().unwrap_or(0));
        let column = parts.next().unwrap_or(0);
        (parts.next().unwrap_or(0), column)
    }
}

/// Run `action` and return the hints it emitted on this thread (hints are still printed)
pub fn capture_hints<T>(action: impl FnOnce() -> T) -> (T, Vec<CapturedHint>) {
    CAPTURED_HINTS.with(|captured| *captured.borrow_mut() = Some(Vec::new()));
    let result = action();
    let hints = CAPTURED_HINTS.with(|captured| captured.borrow_mut().take()).unwrap_or_default();
    (result, hints)
}

/// Set the current position for hint messages
pub fn set_hint_position(line: usize, column: usize) {
    HINT_POSITION.with(|pos| {
        let mut p = pos.borrow_mut();
        p.line = line;
        p.column = column;
    });
}

/// The line of the current hint position, 0 when unknown
pub fn hint_line() -> usize {
    HINT_POSITION.with(|pos| pos.borrow().line)
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
preferred_axes!(CastStyle, FunctionStyle, VarStyle, LogicalStyle, QuoteStyle, IndexStyle, ConditionalStyle, PowerStyle, ListTypeStyle);

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
            cast: CastStyle::Any, // user #13: `str(x)` is fine
            function_def: FunctionStyle::Any, // user 2026-10-03: `f(x) := …`, `def f(x): …` … are all fine
            var_def: VarStyle::Any, // user #12: `let t = …` and `t := …` are both legitimate
            logical: LogicalStyle::Words,
            quotes: QuoteStyle::Any, // user #11: "I don't care" about 'x' or "x"
            index: IndexStyle::Hash,
            conditional: ConditionalStyle::IfThenElse,
            power: PowerStyle::Caret,
            prefer_string_over_str: false, // user #13: str(x) gets no hint
        }
    }
}

// The style, hint mode and shown hints configure the compilations of one thread (the CLI has one): a parallel
// test's setting must not leak into another test's compilation
thread_local! {
    static STYLE: RefCell<Style> = RefCell::new(Style::default());
    static SHOWN_HINTS: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
    static HINT_MODE: std::cell::Cell<HintMode> = const { std::cell::Cell::new(HintMode::Always) };
}

/// Set the style of this thread's compilations
pub fn set_style(style: Style) {
    STYLE.with(|current| *current.borrow_mut() = style);
}

/// The style of this thread's compilations (cloned)
pub fn style() -> Style {
    STYLE.with(|current| current.borrow().clone())
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

/// Hints and notes are printed by default; the CLI's `--no-hints` or WARP_HINTS=0 turn printing off for the whole
/// process (user 2026-10-09, card hints-toggle). Off, they are still recorded: `capture_hints` (the playground's
/// report, its fix buttons, the tests) sees every hint
static HINTS_PRINTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);
/// Whether this process printed a hint: the CLI then ends with how to hide them
static ANY_HINT_PRINTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Print hints and notes to stderr (on, the default) or only record them (off)
pub fn print_hints(on: bool) {
    HINTS_PRINTED.store(on, std::sync::atomic::Ordering::Relaxed);
}

/// Whether hints and notes are printed to stderr
pub fn hints_printed() -> bool {
    HINTS_PRINTED.load(std::sync::atomic::Ordering::Relaxed)
}

/// Whether this process printed a hint or note
pub fn any_hint_printed() -> bool {
    ANY_HINT_PRINTED.load(std::sync::atomic::Ordering::Relaxed)
}

/// Set the hint mode of this thread's compilations
pub fn set_hint_mode(mode: HintMode) {
    HINT_MODE.with(|current| current.set(mode));
}

/// The hint mode of this thread's compilations
pub fn hint_mode() -> HintMode {
    HINT_MODE.with(|current| current.get())
}

/// Clear shown hints (useful for testing)
pub fn clear_shown_hints() {
    SHOWN_HINTS.with(|shown| shown.borrow_mut().clear());
}

// ============================================================================
// Core Hint Function
// ============================================================================

/// Emit a normalization hint to stderr: `canonical` is what to write instead of `original`
pub fn hint(original: &str, canonical: &str, reason: &str) {
    emit_hint(original, canonical, reason, true);
}

/// A hint whose preferred form does not replace the original text (it is said elsewhere: `global x`)
pub fn advise(original: &str, preferred: &str, reason: &str) {
    emit_hint(original, preferred, reason, false);
}

fn emit_hint(original: &str, canonical: &str, reason: &str, rewrites: bool) {
    let mode = hint_mode();
    if mode == HintMode::Off || HINTS_MUTED.with(|muted| muted.get()) {
        return;
    }

    let key = format!("{}|{}", original, canonical);

    if mode == HintMode::Once
        && !SHOWN_HINTS.with(|shown| shown.borrow_mut().insert(key)) {
            return;
        }

    crate::diagnostic::note_said();
    let pos = position_string();
    CAPTURED_HINTS.with(|captured| {
        if let Some(hints) = captured.borrow_mut().as_mut() {
            hints.push(CapturedHint { original: original.to_string(), canonical: canonical.to_string(), position: pos.clone(), reason: reason.to_string(), rewrites });
        }
    });
    if !hints_printed() {
        return;
    }
    ANY_HINT_PRINTED.store(true, std::sync::atomic::Ordering::Relaxed);
    use crate::diagnostic::{paint, Color};
    let position = if pos.is_empty() { String::new() } else { format!(" {}", paint(Color::Gray, &pos)) };
    // a note about the text as written (`await job within 100 ms`) prefers nothing else
    match canonical == original {
        true => eprintln!("{}{position}: {}", paint(Color::Cyan, "note"), paint(Color::Green, original)),
        false => eprintln!("{}{position}: prefer {} over {}", paint(Color::Cyan, "hint"), paint(Color::Green, canonical), paint(Color::Yellow, original)),
    }
    eprintln!("      {}", reason);
}

// ============================================================================
// Hint Functions - Check style before emitting
// ============================================================================

pub mod hints {
    use super::*;

    /// Type constructor vs 'as' operator
    /// The type word in its canonical spelling: `str` and `String` become `string` when that is preferred
    fn canonical_type_word(used: &str) -> &str {
        if style().prefer_string_over_str && STRING_TYPE_NAMES.contains(&used) { "string" } else { used }
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
        if s.prefer_string_over_str && (used == "str" || used == "String") {
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
        hint(&format!("{used}{content}{used}"), &format!("{canonical_quote}{content}{canonical_quote}"), reason);
    }

    /// The operator as written in the source; `is_prefix` tells `!x` from an infix use
    pub fn operator(written: &str, is_prefix: bool) {
        let mut characters = written.chars();
        if let (Some(glyph), None) = (characters.next(), characters.next()) {
            if let Some((_, canonical)) = glyph_operator(glyph) {
                match canonical {
                    "and" => and_operator(written),
                    "or" => or_operator(written),
                    "xor" => {}
                    _ => hint(written, canonical, "standard spelling of the operator"),
                }
                return;
            }
        }
        match (written, is_prefix) {
            ("¬&", false) => hint(written, "nand", "standard spelling of the operator"),
            ("&&" | "&" | "and", false) => and_operator(written),
            ("||" | "|" | "or", false) => or_operator(written),
            ("!" | "not", true) => not_operator(written),
            ("**" | "^", false) => power_operator(written),
            ("?", false) => conditional(true),
            ("if", true) => conditional(false),
            _ => {}
        }
    }

    /// && operator
    pub fn and_operator(used: &str) {
        let s = style();
        match (used, s.logical) {
            ("&&" | "&" | "∧" | "⋀", LogicalStyle::Words) => hint(used, "and", "word operators are more readable"),
            ("and" | "∧" | "⋀", LogicalStyle::Symbols) => hint(used, "&&", "symbol operators preferred"),
            _ => {}
        }
    }

    /// || operator
    pub fn or_operator(used: &str) {
        let s = style();
        match (used, s.logical) {
            ("||" | "|" | "∨" | "⋁", LogicalStyle::Words) => hint(used, "or", "word operators are more readable"),
            ("or" | "∨" | "⋁", LogicalStyle::Symbols) => hint(used, "||", "symbol operators preferred"),
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
            (true, ConditionalStyle::IfThenElse) => {
                crate::diagnostic::educate_once(CONDITIONAL_TOPIC, "x ? y : z", "if x then y else z", "if-then-else is more readable")
            }
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
        let Some(preferred) = style().var_def.instead_of(used_style) else { return };
        let spell = |var_style: VarStyle| match var_style {
            VarStyle::ColonEquals => format!("{name} := {value}"),
            VarStyle::Let => format!("let {name} = {value}"),
            VarStyle::Var => format!("var {name} = {value}"),
            VarStyle::Any => format!("{name} = {value}"),
        };
        let reason = match preferred {
            VarStyle::ColonEquals => "use := for definition",
            VarStyle::Let => "use 'let' for definition",
            VarStyle::Var => "use 'var' for definition",
            VarStyle::Any => return,
        };
        match used_style {
            // shown until the user acknowledges it once (diagnostic::educate_once)
            VarStyle::Let => crate::diagnostic::educate_once(LET_TOPIC, &spell(used_style), &spell(preferred),
                &format!("{reason}; in warp `let` is immutable (unlike JS), use var or plain = for variables that change")),
            _ => hint(&spell(used_style), &spell(preferred), reason),
        }
    }

    /// The canonical spelling of the list of `element_words` (`["int"]`, or `["list", "int"]` for a list of lists) in `list_style`;
    /// `None` when the style cannot spell it (`ints` has no plural for a list of lists)
    fn list_type_spelling(list_style: ListTypeStyle, element_words: &[&str]) -> Option<String> {
        match (list_style, element_words) {
            (ListTypeStyle::Plural, [element]) => crate::analyzer::type_word_kind(element).map(|_| format!("{element}s")),
            (ListTypeStyle::Plural | ListTypeStyle::Any, _) => None,
            (ListTypeStyle::Generic, words) => {
                let (last, nested) = words.split_last()?;
                let arguments = nested.iter().rev().fold(last.to_string(), |inner, word| format!("{word}<{inner}>"));
                Some(format!("{LIST_TYPE_HEAD}<{arguments}>"))
            }
            (ListTypeStyle::Of, words) => Some(format!("{LIST_TYPE_HEAD} {OF_WORD} {}", words.join(" of "))),
            (ListTypeStyle::Bracket, words) => {
                let (last, nested) = words.split_last()?;
                Some(nested.iter().rev().fold(format!("[{last}]"), |inner, _| format!("[{inner}]")))
            }
        }
    }

    /// A list type written in `used` style: `list<int>` and `list of int` when `ints` is preferred, `ints` when the words are
    pub fn list_type(used: ListTypeStyle, element_words: &[&str]) {
        let Some(preferred) = style().list_type.instead_of(used) else { return };
        if let (Some(original), Some(canonical)) = (list_type_spelling(used, element_words), list_type_spelling(preferred, element_words)) {
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
        let Some(preferred) = style().function_def.instead_of(used_style) else { return };
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
            FunctionStyle::Fn | FunctionStyle::Fun | FunctionStyle::ColonEquals | FunctionStyle::Any => BodyForm::Assign,
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
        hint(&spell(used_style, written_body), &spell(preferred, conventional_body(preferred)), reason);
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
        Node::List(items, bracket, separator) => match items.as_slice() {
            [word, dividend, divisor] if matches!(word.drop_meta(), Node::Symbol(name) if name == crate::warp_parser::FLOOR_QUOTIENT) => {
                Node::Symbol(format!("{}//{}", operand_text(dividend), operand_text(divisor)))
            }
            _ => Node::List(items.into_iter().map(as_written).collect(), bracket, separator),
        },
        Node::Key(left, op, right) => Node::Key(Box::new(as_written(*left)), op, Box::new(as_written(*right))),
        Node::Meta { node, data } => Node::Meta { node: Box::new(as_written(*node)), data },
        // a witness `area·square` was written `area`
        Node::Symbol(name) if name.contains('·') => Node::Symbol(name.split('·').next().unwrap_or_default().to_string()),
        other => other,
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
            Node::Key(left, _, _) => start_of(left),
            Node::List(items, _, _) => items.first().and_then(start_of),
            _ => None,
        }
    }
    match start_of(node) {
        Some((line, column)) => set_hint_position(line, column),
        None => clear_hint_position(),
    }
}

fn word_of(node: &Node) -> Option<&str> {
    match node.drop_meta() {
        Node::Symbol(word) => Some(word),
        _ => None,
    }
}

/// `x:list of int=[1 2]` is the items `x:list`, `of`, `int=[1 2]`: the element words after `of`, and the node of the head word `list`
fn of_list_type(items: &[Node], of_index: usize) -> Option<(&Node, Vec<&str>)> {
    let Node::Key(_, Op::Colon, head) = items.get(of_index.checked_sub(1)?)?.drop_meta() else { return None };
    (word_of(head)? == LIST_TYPE_HEAD).then_some(())?;
    let mut words = Vec::new();
    for item in &items[of_index + 1..] {
        match item.drop_meta() {
            Node::Symbol(word) if word == OF_WORD => continue,
            Node::Symbol(word) => words.push(word.as_str()),
            Node::Key(word, _, _) => {
                words.push(word_of(word)?);
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
    let Node::List(words, _, _) = signature.drop_meta() else { return None };
    let (name, parameters) = words.split_first()?;
    let parameters: Vec<String> = parameters.iter().map(|parameter| parameter.serialize().trim_matches(|c| c == '(' || c == ')').to_string()).collect();
    Some((word_of(name)?.to_string(), parameters.join(", "), body_form))
}

/// Emit the hints for every non-canonical form in the parsed program that the parser cannot see while reading characters
pub fn check_style(program: &Node) {
    walk(program, &mut |node, positioned| match node {
        // `[int, 3]` and `[double, f]` are items, never the call int(3)
        Node::List(items, bracket, separator) if *bracket != Bracket::Square && *separator != crate::node::Separator::Colon => check_items(items, positioned),
        Node::Key(_, Op::Colon, right) => {
            if let Node::List(items, Bracket::Square, _) = right.drop_meta() {
                if let [element] = items.as_slice() {
                    if let Some(word) = word_of(element) {
                        set_position_of(right);
                        hints::list_type(ListTypeStyle::Bracket, &[word]);
                    }
                }
                return;
            }
            let Some(type_word) = word_of(right) else { return };
            set_position_of(right);
            if let Some(element) = crate::analyzer::plural_element_type(type_word) {
                hints::list_type(ListTypeStyle::Plural, &[element]);
            }
            hints::string_type(type_word);
        }
        Node::Key(value, Op::As, type_node) => {
            if let Some(type_word) = word_of(type_node) {
                set_position_of(positioned);
                if !hints::as_operator(&operand_text(value), type_word) {
                    hints::string_type(type_word);
                }
            }
        }
        Node::Key(target, Op::Define, value) if word_of(target).is_some() => {
            set_position_of(positioned);
            hints::var_keyword(":=",&operand_text(target), &operand_text(value));
        }
        Node::Key(_, Op::Define, _) => {
            if let Some((name, parameters, _)) = signature_of(node) {
                set_position_of(positioned);
                hints::function_keyword(":=", &name, &parameters, BodyForm::Assign);
            }
        }
        Node::Key(target, Op::Dot, call) => {
            if matches!(call.drop_meta(), Node::List(words, _, _) if words.len() == 1 && word_of(&words[0]) == Some("length")) {
                set_position_of(positioned);
                hints::length_operator(&operand_text(target), true);
            }
        }
        _ => {}
    });
}

fn check_items(items: &[Node], positioned: &Node) {
    if let Some(of_index) = items.iter().position(|item| word_of(item) == Some(OF_WORD)) {
        if let Some((head, words)) = of_list_type(items, of_index) {
            set_position_of(head);
            hints::list_type(ListTypeStyle::Of, &words);
        }
    }
    let [keyword, rest @ ..] = items else { return };
    let Some(keyword_word) = word_of(keyword) else { return };
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
            hint(&format!("len({counted})"), &format!("#{counted}"), "use # prefix for length");
        }
        (word, [definition, ..]) if crate::operators::is_function_keyword(word) => {
            if let Some((name, parameters, body_form)) = signature_of(definition) {
                hints::function_keyword(word, &name, &parameters, body_form);
            }
        }
        // `codepoint(c)` is a library word (P74), not the type's constructor
        (type_word, [argument]) if crate::analyzer::type_word_kind(&type_word.to_lowercase()).is_some() && !crate::library_words::is_library_word(type_word) => {
            let is_call = match argument.drop_meta() {
                // `int g(int x) {…}` defines g, `int(…)` would convert
                Node::List(parts, Bracket::Round, _) if matches!(parts.as_slice(), [_, body] if matches!(body.drop_meta(), Node::List(_, Bracket::Curly, _))) => false,
                // `text(p:person)` heads the Printable operation of a declared type (P31)
                Node::List(parts, Bracket::Round, _) if matches!(parts.as_slice(), [part] if matches!(part.drop_meta(), Node::Key(_, Op::Colon, _))) => false,
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
		set_style(Style { cast: CastStyle::Constructor, ..Style::default() });

		let s = style();
		assert_eq!(s.cast, CastStyle::Constructor);

		// Reset to default
		set_style(Style::default());
	}

	#[test]
	fn test_hint_position() {
		assert_eq!(crate::wasm_emitter::eval("'abc'"), "abc");// hint:
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
