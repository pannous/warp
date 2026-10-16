//! The keywords and marks the parser and several lowering passes look for, a leaf module: it depends on nothing

pub const COUNT_WORD: &str = "count";
/// The class of a path written without quotes where a word has a method for it (lib/shell.warp, `sort ./names.txt`)
pub const PATH_CLASS: &str = "Path";
pub const PATH_FIELD: &str = "name";
pub const EACH_WORD: &str = "each";
pub const FOR_WORD: &str = "for";
pub const FROM_WORD: &str = "from";
pub const GLOBAL_WORD: &str = "global";
pub const IN_WORD: &str = "in";
pub const MAP_WORD: &str = "map";
pub const OF_WORD: &str = "of";
pub const ON_WORD: &str = "on";
pub const ONCE_WORD: &str = "once";
/// Other systems' event names, listened to as the warp event with a got-it note (card key-aliases): one line per alias
pub const EVENT_ALIASES: [(&str, &str); 9] = [
	("keydown", "key"),
	("key_down", "key"),
	("keypress", "key"),
	("keyup", "key"),
	("key_up", "key"),
	("mousedown", "click"),
	("mouse_down", "click"),
	("mouseup", "click"),
	("mouse_up", "click"),
];
pub const WHENEVER_WORD: &str = "whenever";
pub const RETURN_WORD: &str = "return";
pub const SUM_WORD: &str = "sum";

/// `switch x {…}` / `match x {…}` (lowering/control/switch.rs)
pub const SWITCH_WORDS: [&str; 2] = ["switch", MATCH_WORD];
/// The match lowering passes write (nodes::matched)
pub const MATCH_WORD: &str = "match";
/// The type every value is: `x: any`, `foo of any`, an untyped variant among typed ones
pub const ANY_TYPE: &str = "any";
/// The parameter a lambda without one names: `it`
pub const IMPLICIT_PARAMETER: &str = "it";
/// `emit alarm{level: 3}` runs the `on alarm` handlers and goes on; nobody listening, it does nothing (P163). `send`
/// without `to` is the same; `raise` and `throw` are errors only
pub const EMIT_WORDS: [&str; 2] = ["emit", "send"];
/// Other systems' words for emit, working with a got-it note naming it (a program defining one of them keeps it);
/// `ask` reads as asking a handler for a value: `net * (1 + ask tax_rate)` (card price-net)
pub const EMIT_ALIASES: [&str; 4] = ["fire", "trigger", "signal", "ask"];
/// The parser's attribute on the name of a class declared `class Foo;`
pub const FILE_CLASS_MARK: &str = "file_class";
/// The parser's attribute on the name of a `class Foo` written without a body (class_references.rs)
pub const BARE_CLASS_MARK: &str = "bare_class";
/// The parser's attribute on a sum type's name: its variants without payload
pub const VARIANTS_MARK: &str = "variants";
/// P74 (user, 2026-10-05): "make codepoint the default name": the spelling hints, fixes and docs use
pub const CODEPOINT: &str = "codepoint";
/// `m.keys()`: the keys of a map (library_words)
pub const MAP_KEYS: &str = "map_keys";
/// `slice(x, start, end)`: the items or characters start…end-1, 0-based (`a[1:3]`, `s.slice(1)`)
pub const SLICE: &str = "slice";
/// `x!` that changes nothing unwraps x (library_words: ø is a loud error)
pub const UNWRAP: &str = "unwrap";
/// `instance_of(x, "square")`: 1 when x is an instance of the declared type, else 0 (the emitter reads the type name of
/// the instance at run time)
pub const INSTANCE_OF: &str = "instance_of";
/// The mark of the name taking the rest in `a, *rest = xs` (Python's starred target)
pub const STARRED: &str = "*";
/// `is_type(x, "T")`: whether x is of type T (type_tests.rs)
pub const IS_TYPE: &str = "is_type";
/// `version 1.2` (versions.rs)
pub const VERSION_KEYWORD: &str = "version";
/// The Meta key the parser puts on the name of a function written with type parameters `fn id<T>(…)`: their names
pub const GENERIC_MARK: &str = "generic";
/// The Meta key the parser puts on an operator spelled as a word and applied, `sqrt 9`: where a function is expected
/// the word is that function and 9 the next argument (closures::unfolded_operator_word)
pub const OPERATOR_WORD_MARK: &str = "operator_word";
/// Words that declare a name assignable once: `const x=5`, `final x=5`
pub const CONSTANT_KEYWORDS: [&str; 4] = ["const", "constant", "final", "val"];
/// Rust's `let mut x` and `let mutable x`
pub const MUTABLE_WORDS: [&str; 2] = ["mut", "mutable"];

// Words several passes look for, each spelled once here (card duplicate-constants)
/// Type words
pub const FLOAT_WORD: &str = "float";
pub const LIST_WORD: &str = "list";
/// The type of every number, a ± value among them
pub const NUMBER_WORD: &str = "number";
/// `x is pair`: a `key: value` pair
pub const PAIR_WORD: &str = "pair";
/// The type of an operator node, `parse("1+1")`, `a<b` as data; a `key: value` pair stays a key (decision parse-type)
pub const OPERATION_WORD: &str = "operation";
/// The type of a quoted program part, `f.body`, `parse("1+1")`, whatever its form (decision body-code)
pub const CODE_WORD: &str = "code";
/// `as_code(data e)`: e marked as code (wasm_emitter constructors.rs, type_kinds::CODE_MARK)
pub const AS_CODE: &str = "as_code";
pub const REAL_WORD: &str = "real";
pub const SYMBOL_WORD: &str = "symbol";
pub const TEXT_WORD: &str = "text";
/// Keywords and their companions: `await all`, `x at 2`, `go f()`, `x is int`, `let x = 1`, `raise e`
pub const ALL_WORD: &str = "all";
pub const AT_WORD: &str = "at";
pub const GO_WORD: &str = "go";
pub const IS_WORD: &str = "is";
/// `let x = 1` binds x once, as const does (user, card let-reassign)
pub const LET_WORD: &str = "let";
pub const RAISE_WORD: &str = "raise";
pub const SELF_WORD: &str = "self";
/// Library words: `xs.add(x)`, `chr(n)`, `dir(x)`, `dot(xs, ys)`, `error "e"`, `xs.pop()`, `print x`, `str(x)` …
pub const ADD_WORD: &str = "add";
pub const CHR_WORD: &str = "chr";
pub const DELETE_WORD: &str = "delete";
pub const DIR_WORD: &str = "dir";
pub const DOT_WORD: &str = "dot";
pub const ERROR_WORD: &str = "error";
/// `x failed` reads `is_error(x)`: whether x is an Error value
pub const IS_ERROR_WORD: &str = "is_error";
pub const POP_WORD: &str = "pop";
pub const PRINT_WORD: &str = "print";
pub const SEND_WORD: &str = "send";
pub const SORT_WORD: &str = "sort";
pub const STR_WORD: &str = "str";
pub const DO_WORD: &str = "do";
/// `codepoint(c)`, `ord(c)`, `ordinal(c)`: the code point of a character (`c as int` is only its digit). Inside the
/// compiler it is `ord`, since `codepoint` is also a type word
pub const ORD_WORD: &str = "ord";
pub const SPLIT_WORD: &str = "split";
/// `log(x)` is libm's natural logarithm; `log(x, base)` divides by the base's
pub const LOG_WORD: &str = "log";
/// `b = a.copy()`: values are never shared, so the copy is the value itself
pub const COPY_WORD: &str = "copy";
pub const STYLE_WORD: &str = "style";
/// `min(a, b)` and `max(a, b)` of two values (lowering/collections/min_max.rs): a comparison, each argument computed once
pub const EXTREMUM_WORDS: [&str; 2] = ["min", "max"];

/// A vector's components by letter (decision S1, notes/compile_shaders.md): `p.x` is p#1, `p.xy` is [p#1 p#2], `e.xyy`
pub const SWIZZLE_LETTERS: [char; 4] = ['x', 'y', 'z', 'w'];

/// The 0-based components a swizzle name picks, 1 to 4 of the letters xyzw (`zyx`: 2 1 0), None for any other name
pub fn swizzle_components(name: &str) -> Option<Vec<usize>> {
	if !(1..=SWIZZLE_LETTERS.len()).contains(&name.chars().count()) {
		return None;
	}
	name.chars().map(|letter| SWIZZLE_LETTERS.iter().position(|known| *known == letter)).collect()
}
