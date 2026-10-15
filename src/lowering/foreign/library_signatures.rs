//! Signatures of the library words (card reflect-library-signatures, notes/reflection.md step 3): `sqrt.signature`,
//! `first.params` and `help(upper)` answer as they do for a user function. A word below is named and typed as written
//! there, a prelude word by its head in lib/prelude.warp; any other library word is read off its table (libm, text
//! builtins, runtime words): its number of parameters with positional names, and the result kind where the table has one

use crate::type_kinds::Kind;

/// The words whose tables name neither their parameters nor (for an argument-dependent result) their result
const NAMED_SIGNATURES: [(&str, &str); 23] = [
	("sqrt", "(x:number) -> number"),
	("cbrt", "(x:number) -> number"),
	("abs", "(x:number) -> number"),
	("floor", "(x:number) -> int"),
	("ceil", "(x:number) -> int"),
	("round", "(x:number) -> int"),
	("trunc", "(x:float) -> float"),
	// the real functions, exact on exact values (numbers/real.rs), else the f64 ones (ln has no libm name)
	("sin", "(x:float) -> float"),
	("cos", "(x:float) -> float"),
	("tan", "(x:float) -> float"),
	("exp", "(x:float) -> float"),
	("ln", "(x:float) -> float"),
	("min", "(a, b) -> number"),
	("max", "(a, b) -> number"),
	("upper", "(t:text) -> text"),
	("lower", "(t:text) -> text"),
	("reverse", "(xs)"),
	("sort", "(xs:list) -> list"),
	("split", "(t:text, separator:text) -> list of text"),
	("join", "(xs:list, separator:text) -> text"),
	("chars", "(t:text) -> list of text"),
	("ord", "(c:codepoint) -> int"),
	("print", "(x)"),
];
/// The builtin methods of lists, texts and maps no table above names (members.rs LIST_MEMBERS …), so each is a function
/// value like the others (card dir-lists: methods(x) lists what dir(x) does)
const MEMBER_SIGNATURES: [(&str, &str); 19] = [
	("count", "(xs) -> int"),
	("map", "(xs, f)"),
	("filter", "(xs, f)"),
	("reduce", "(xs, f)"),
	("fold", "(xs, start, f)"),
	("any", "(xs, f) -> bool"),
	("all", "(xs, f) -> bool"),
	("find", "(xs, f)"),
	("each", "(xs, f)"),
	("add", "(xs, x)"),
	("insert", "(xs, x, at)"),
	("remove", "(xs, x)"),
	("pop", "(xs)"),
	("clear", "(xs)"),
	("number", "(x) -> number"),
	("string", "(x) -> text"),
	("keys", "(m) -> list"),
	("values", "(m) -> list"),
	("get", "(m, key)"),
];
const PRELUDE: &str = "prelude";
const POSITIONAL_NAMES: [&str; 4] = ["a", "b", "c", "d"];
const LIBM_PARAMETER_TYPE: &str = "float";

/// A library word's parameter names and its signature `(t:text, separator:text) -> list of text`; None for a word
/// that is no library word
pub fn library_signature(word: &str) -> Option<(Vec<String>, String)> {
	let word = crate::library_words::canonical_spelling(word);
	let signature = named(&NAMED_SIGNATURES, word)
		.or_else(|| prelude_signature(word))
		.or_else(|| libm_signature(word))
		.or_else(|| text_builtin_signature(word))
		.or_else(|| {
			crate::library_words::is_library_word(word)
				.then(|| positional(crate::library_words::arity(word), None, Kind::Empty))
		})
		.or_else(|| named(&MEMBER_SIGNATURES, word))?;
	Some((parameter_names(&signature), signature))
}

fn named(signatures: &[(&str, &str)], word: &str) -> Option<String> {
	signatures.iter().find(|(name, _)| *name == word).map(|(_, signature)| signature.to_string())
}

/// The parameter count of a library word whose first parameter is typed (`floor(x:number)`, `upper(t:text)`): a function
/// is never its argument, so `round(floor x)` nests as `round(floor(x))`
pub fn typed_first_arity(word: &str) -> Option<usize> {
	let (params, signature) = library_signature(word)?;
	let first = signature.trim_start_matches('(').split([',', ')']).next()?;
	first.contains(':').then_some(params.len())
}

/// The library words of one typed value with a signature here or in libm: `floor`, `sin`, `upper`, `log10`
pub fn one_value_words() -> impl Iterator<Item = &'static str> {
	let named = NAMED_SIGNATURES.iter().map(|(name, _)| *name);
	named
		.chain(crate::ffi::LIBM_F64_FUNCTIONS.iter().map(|(name, _)| *name))
		.filter(|name| typed_first_arity(name) == Some(1))
}

/// `replace(t, old, new) := …` → `(t, old, new)`
fn prelude_signature(word: &str) -> Option<String> {
	let head = crate::modules::std_docs::word_head(PRELUDE, word)?;
	head.strip_prefix(word).map(str::to_string)
}

fn libm_signature(word: &str) -> Option<String> {
	crate::ffi::LIBM_F64_FUNCTIONS
		.iter()
		.find(|(name, _)| *name == word)
		.map(|(_, arity)| positional(*arity, Some(LIBM_PARAMETER_TYPE), Kind::Float))
}

/// A text builtin of one arity; one of several (`warning(x)`, `warning(x, y)`) has no single signature
fn text_builtin_signature(word: &str) -> Option<String> {
	let [arity] = crate::wasm_emitter::text::text_builtins::text_builtin_arities(word)[..] else {
		return None;
	};
	let result = crate::wasm_emitter::text::text_builtins::text_builtin_kind(word, arity)?;
	Some(positional(arity, None, result))
}

/// `(x)`, `(a:float, b:float) -> float`: one parameter is x, more are a, b, c…
fn positional(arity: usize, parameter_type: Option<&str>, result: Kind) -> String {
	let names: Vec<&str> = match arity {
		1 => vec!["x"],
		_ => POSITIONAL_NAMES.iter().copied().take(arity).collect(),
	};
	let parameters: Vec<String> = names
		.iter()
		.map(|name| parameter_type.map_or_else(|| name.to_string(), |kind| format!("{name}: {kind}")))
		.collect();
	let result = match result {
		Kind::Empty => String::new(),
		kind => format!(" -> {kind}"),
	};
	format!("({}){result}", parameters.join(", "))
}

/// `(t:text, separator:text) -> …` → `[t separator]`, `(x, limit = 0)` → `[x limit]`
fn parameter_names(signature: &str) -> Vec<String> {
	let inside = signature.strip_prefix('(').and_then(|rest| rest.split(')').next()).unwrap_or_default();
	inside
		.split(',')
		.filter_map(|parameter| parameter.split([':', '=']).next())
		.map(str::trim)
		.filter(|name| !name.is_empty())
		.map(str::to_string)
		.collect()
}
