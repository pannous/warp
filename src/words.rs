//! The keywords and marks the parser and several lowering passes look for, a leaf module: it depends on nothing

pub const COUNT_WORD: &str = "count";
pub const FOR_WORD: &str = "for";
pub const FROM_WORD: &str = "from";
pub const GLOBAL_WORD: &str = "global";
pub const IN_WORD: &str = "in";
pub const MAP_WORD: &str = "map";
pub const OF_WORD: &str = "of";
pub const ON_WORD: &str = "on";
pub const RETURN_WORD: &str = "return";
pub const SUM_WORD: &str = "sum";

/// `switch x {…}` / `match x {…}` (lowering/switch.rs)
pub const SWITCH_WORDS: [&str; 2] = ["switch", "match"];
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
/// The Meta key the parser puts on the head of a `to` phrase: its pattern as text, `_ to _`
pub const PHRASE_MARK: &str = "phrase";
/// A parameter slot in a pattern
pub const SLOT: &str = "_";
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
/// Property words that count the elements of a value: `size of x`, `x size`, `x.size`; `size` is a synonym of `count`
/// Words that declare a name assignable once: `const x=5`, `final x=5`
pub const CONSTANT_KEYWORDS: [&str; 4] = ["const", "constant", "final", "val"];
