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
use crate::node::{Bracket, Node};
use crate::operators::Op;

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
}

/// A hint recorded instead of only printed, see `capture_hints`
#[derive(Debug, Clone, PartialEq)]
pub struct CapturedHint {
    pub original: String,
    pub canonical: String,
    /// `line:column`, empty when the emitting stage knows no position
    pub position: String,
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
    /// `define f(x): x*2`
    Define,
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

/// Preferred style for the type of a list of elements
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ListTypeStyle {
    /// `ints`, `texts`, `floats`: the plural type word
    Plural,
    /// `list<int>`
    Generic,
    /// `list of int`
    Of,
}

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

impl Default for Style {
    fn default() -> Self {
        Self {
            list_type: ListTypeStyle::Plural,
            cast: CastStyle::AsOperator,
            function_def: FunctionStyle::ColonEquals,
            var_def: VarStyle::ColonEquals,
            logical: LogicalStyle::Words,
            quotes: QuoteStyle::Double,
            index: IndexStyle::Hash,
            conditional: ConditionalStyle::IfThenElse,
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
    CAPTURED_HINTS.with(|captured| {
        if let Some(hints) = captured.borrow_mut().as_mut() {
            hints.push(CapturedHint { original: original.to_string(), canonical: canonical.to_string(), position: pos.clone() });
        }
    });
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
    /// The type word in its canonical spelling: `str` and `String` become `string` when that is preferred
    fn canonical_type_word(used: &str) -> &str {
        if style().prefer_string_over_str && STRING_TYPE_NAMES.contains(&used) { "string" } else { used }
    }

    /// Returns whether a hint was emitted, so that the type word is not hinted a second time
    pub fn type_constructor(type_name: &str, value: &str) -> bool {
        let is_non_canonical = style().cast == CastStyle::AsOperator;
        if is_non_canonical {
            let original = format!("{}({})", type_name, value);
            let canonical = format!("{} as {}", value, canonical_type_word(type_name));
            hint(&original, &canonical, "postfix 'as' reads naturally: value as type");
        }
        is_non_canonical
    }

    /// 'as' operator when constructor style is preferred; returns whether a hint was emitted
    pub fn as_operator(value: &str, type_name: &str) -> bool {
        let is_non_canonical = style().cast == CastStyle::Constructor;
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
        let (canonical_quote, reason) = match style().quotes {
            QuoteStyle::Single => ('\'', "single quotes preferred for strings"),
            QuoteStyle::Double => ('"', "double quotes preferred for strings"),
        };
        if used == '\'' || used == '"' {
            if used != canonical_quote {
                hint(&format!("{used}{content}{used}"), &format!("{canonical_quote}{content}{canonical_quote}"), reason);
            }
        }
    }

    /// The operator as written in the source; `is_prefix` tells `!x` from an infix use
    pub fn operator(written: &str, is_prefix: bool) {
        match (written, is_prefix) {
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
            ("&&" | "&", LogicalStyle::Words) => hint(used, "and", "word operators are more readable"),
            ("and", LogicalStyle::Symbols) => hint("and", "&&", "symbol operators preferred"),
            _ => {}
        }
    }

    /// || operator
    pub fn or_operator(used: &str) {
        let s = style();
        match (used, s.logical) {
            ("||" | "|", LogicalStyle::Words) => hint(used, "or", "word operators are more readable"),
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
    pub fn var_keyword(used: &str, name: &str, value: &str) {
        let preferred = style().var_def;
        let used_style = match used {
            "let" => VarStyle::Let,
            "var" => VarStyle::Var,
            ":=" => VarStyle::ColonEquals,
            _ => return,
        };
        let spell = |var_style: VarStyle| match var_style {
            VarStyle::ColonEquals => format!("{name} := {value}"),
            VarStyle::Let => format!("let {name} = {value}"),
            VarStyle::Var => format!("var {name} = {value}"),
        };
        let reason = match preferred {
            VarStyle::ColonEquals => "use := for definition",
            VarStyle::Let => "use 'let' for definition",
            VarStyle::Var => "use 'var' for definition",
        };
        if used_style != preferred {
            hint(&spell(used_style), &spell(preferred), reason);
        }
    }

    /// The canonical spelling of the list of `element_words` (`["int"]`, or `["list", "int"]` for a list of lists) in `list_style`;
    /// `None` when the style cannot spell it (`ints` has no plural for a list of lists)
    fn list_type_spelling(list_style: ListTypeStyle, element_words: &[&str]) -> Option<String> {
        match (list_style, element_words) {
            (ListTypeStyle::Plural, [element]) => crate::analyzer::type_word_kind(element).map(|_| format!("{element}s")),
            (ListTypeStyle::Plural, _) => None,
            (ListTypeStyle::Generic, words) => {
                let (last, nested) = words.split_last()?;
                let arguments = nested.iter().rev().fold(last.to_string(), |inner, word| format!("{word}<{inner}>"));
                Some(format!("{LIST_TYPE_HEAD}<{arguments}>"))
            }
            (ListTypeStyle::Of, words) => Some(format!("{LIST_TYPE_HEAD} {OF_WORD} {}", words.join(" of "))),
        }
    }

    /// A list type written in `used` style: `list<int>` and `list of int` when `ints` is preferred, `ints` when the words are
    pub fn list_type(used: ListTypeStyle, element_words: &[&str]) {
        let preferred = style().list_type;
        if used == preferred {
            return;
        }
        if let (Some(original), Some(canonical)) = (list_type_spelling(used, element_words), list_type_spelling(preferred, element_words)) {
            let reason = match preferred {
                ListTypeStyle::Plural => "a plural type word is a list of that type",
                ListTypeStyle::Generic => "angle brackets apply the list type to its element type",
                ListTypeStyle::Of => "'list of' reads as a phrase",
            };
            hint(&original, &canonical, reason);
        }
    }

    /// Function definition keywords
    pub fn function_keyword(used: &str, name: &str, params: &str) {
        let preferred = style().function_def;
        let used_style = match used {
            "def" => FunctionStyle::Def,
            "define" => FunctionStyle::Define,
            "fn" => FunctionStyle::Fn,
            "fun" => FunctionStyle::Fun,
            "function" => FunctionStyle::Function,
            ":=" => FunctionStyle::ColonEquals,
            _ => return,
        };
        let spell = |function_style: FunctionStyle| match function_style {
            FunctionStyle::ColonEquals => format!("{name}({params}) := ..."),
            FunctionStyle::Def => format!("def {name}({params}): ..."),
            FunctionStyle::Define => format!("define {name}({params}): ..."),
            FunctionStyle::Fn => format!("fn {name}({params}) = ..."),
            FunctionStyle::Fun => format!("fun {name}({params}) = ..."),
            FunctionStyle::Function => format!("function {name}({params}) {{ ... }}"),
        };
        let reason = match preferred {
            FunctionStyle::ColonEquals => "short := form preferred",
            FunctionStyle::Def => "'def' keyword preferred",
            FunctionStyle::Define => "'define' keyword preferred",
            FunctionStyle::Fn => "'fn' keyword preferred",
            FunctionStyle::Fun => "'fun' keyword preferred",
            FunctionStyle::Function => "'function' keyword preferred",
        };
        if used_style != preferred {
            hint(&spell(used_style), &spell(preferred), reason);
        }
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
const STRING_TYPE_NAMES: [&str; 2] = ["str", "String"];

/// Source text of an operand for a hint: texts in the canonical quote
pub fn operand_text(node: &Node) -> String {
    match node.drop_meta() {
        Node::Text(text) => {
            let quote = if style().quotes == QuoteStyle::Double { '"' } else { '\'' };
            format!("{quote}{text}{quote}")
        }
        other => other.serialize(),
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
fn signature_of(definition: &Node) -> Option<(String, String)> {
    let signature = match definition.drop_meta() {
        Node::Key(signature, _, _) => signature.as_ref(),
        Node::List(parts, _, _) => parts.first()?,
        _ => return None,
    };
    let Node::List(words, _, _) = signature.drop_meta() else { return None };
    let (name, parameters) = words.split_first()?;
    let parameters: Vec<String> = parameters.iter().map(|parameter| parameter.serialize().trim_matches(|c| c == '(' || c == ')').to_string()).collect();
    Some((word_of(name)?.to_string(), parameters.join(", ")))
}

/// Emit the hints for every non-canonical form in the parsed program that the parser cannot see while reading characters
pub fn check_style(program: &Node) {
    walk(program, &mut |node, positioned| match node {
        Node::List(items, _, _) => check_items(items, positioned),
        Node::Key(_, Op::Colon, right) => {
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
            if let Some((name, parameters)) = signature_of(node) {
                set_position_of(positioned);
                hints::function_keyword(":=", &name, &parameters);
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
        (word, [definition, ..]) if crate::operators::is_function_keyword(word) => {
            if let Some((name, parameters)) = signature_of(definition) {
                hints::function_keyword(word, &name, &parameters);
            }
        }
        (type_word, [argument]) if crate::analyzer::type_word_kind(&type_word.to_lowercase()).is_some() => {
            let is_call = match argument.drop_meta() {
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
	use crate::is;
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
		is!("'abc'", "abc");// hint:
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
