//! Signatures of the library words (card reflect-library-signatures, notes/reflection.md step 3): `sqrt.signature`,
//! `first.params` and `help(upper)` answer as they do for a user function. A word below is named and typed as written
//! there, a prelude word by its head in lib/prelude.warp; any other library word is read off its table (libm, text
//! builtins, runtime words): its number of parameters with positional names, and the result kind where the table has one

use crate::type_kinds::Kind;

/// print, which no line or table names; the words warp computes itself are lib/math.warp's and lib/prelude.warp's
/// `from warp` lines (builtin_signature)
const PRINT_SIGNATURE: (&str, &str) = ("print", "(x)");
/// The builtin methods of lists, texts and maps no table above names (members.rs LIST_MEMBERS …), so each is a function
/// value like the others (card dir-lists: methods(x) lists what dir(x) does)
const MEMBER_SIGNATURES: [(&str, &str); 19] = [
	("count", "(xs) -> int"),
	("map", "(xs, f: function)"),
	("filter", "(xs, f: function)"),
	("reduce", "(xs, f: function)"),
	("fold", "(xs, start, f: function)"),
	("any", "(xs, f: function) -> bool"),
	("all", "(xs, f: function) -> bool"),
	("find", "(xs, f: function)"),
	("each", "(xs, f: function)"),
	("add", "(xs, x)"),
	("insert", "(xs, x, at: int)"),
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
/// `more...`: a rest parameter, any number of values (variadic.rs)
const REST_MARK: &str = "...";

/// A library word's parameter names and its signature `(t:text, separator:text) -> list of text`; None for a word
/// that is no library word
pub fn library_signature(word: &str) -> Option<(Vec<String>, String)> {
	let word = crate::library_words::canonical_spelling(word);
	let signature = named(&[PRINT_SIGNATURE], word)
		.or_else(|| builtin_signature(word))
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
	typed_first_parameters(word).filter(|(fewest, most)| fewest == most).map(|(fewest, _)| fewest)
}

/// The fewest and most values a library word with a typed first parameter takes: `floor` (1, 1), `max` (2, unbounded)
pub fn typed_first_parameters(word: &str) -> Option<(usize, usize)> {
	let (params, signature) = library_signature(word)?;
	first_parameter_type(&signature)?;
	let is_variadic = params.last().is_some_and(|last| last.ends_with(REST_MARK));
	Some(if is_variadic { (params.len() - 1, usize::MAX) } else { (params.len(), params.len()) })
}

/// The types a library word is defined for, by its typed first parameter, else by the builtin methods of each type
/// (members.rs): `floor` number, `upper` text, `count` list, text and map; empty for a word of any value (`print`)
/// or no library word. A library word is immutable for these types (decision library-words-typed)
pub fn library_types(word: &str) -> Vec<String> {
	let word = crate::library_words::canonical_spelling(word);
	// each overload's (`reverse` of a list and of a text)
	let overloads: Vec<String> = crate::ffi::runtime_word_lines()
		.iter()
		.filter(|line| line.name == word)
		.filter_map(|line| line.first_type().map(str::to_string))
		.collect();
	if !overloads.is_empty() {
		return overloads;
	}
	if let Some(typed) = library_signature(word).and_then(|(_, signature)| first_parameter_type(&signature)) {
		return vec![typed];
	}
	crate::lowering::members::MEMBERS_BY_TYPE
		.iter()
		.filter(|(_, members)| members.contains(&word))
		.map(|(type_name, _)| type_name.to_string())
		.collect()
}

/// `text` of `(t:text, separator:text) -> …`
fn first_parameter_type(signature: &str) -> Option<String> {
	signature_parameter_types(signature).into_iter().next().flatten()
}

/// The type of each parameter of a library word, None where it takes any value: filter's `[None, function]`
pub fn library_parameter_types(word: &str) -> Vec<Option<String>> {
	library_signature(word).map(|(_, signature)| signature_parameter_types(&signature)).unwrap_or_default()
}

/// `(t:text, separator:text) -> …` → `[text, text]`; a float parameter (libm's) takes any number
fn signature_parameter_types(signature: &str) -> Vec<Option<String>> {
	let inside = signature.strip_prefix('(').and_then(|rest| rest.split(')').next()).unwrap_or_default();
	let type_of = |parameter: &str| {
		let type_name = parameter.split('=').next()?.split_once(':')?.1.trim();
		Some(if type_name == LIBM_PARAMETER_TYPE { crate::words::NUMBER_WORD } else { type_name }.to_string())
	};
	inside.split(',').filter(|parameter| !parameter.trim().is_empty()).map(type_of).collect()
}

/// The number of values a library word takes, fewest and most, when its signature names them
pub fn library_arity(word: &str) -> Option<(usize, usize)> {
	let (params, signature) = library_signature(word)?;
	let is_variadic = params.last().is_some_and(|last| last.ends_with(REST_MARK));
	let defaulted = signature.matches('=').count();
	Some(if is_variadic { (params.len() - 1, usize::MAX) } else { (params.len() - defaulted, params.len()) })
}

/// The library words of one typed value with a signature here or in libm: `floor`, `sin`, `upper`, `log10`
pub fn one_value_words() -> impl Iterator<Item = &'static str> {
	builtin_lines()
		.map(|line| line.name)
		.chain(crate::ffi::libm_f64_functions().iter().map(|(name, _)| *name))
		.filter(|name| typed_first_arity(name) == Some(1))
}

/// `floor(x: number): int from warp` → `(x:number) -> int`: a word warp computes itself, as lib/math.warp or
/// lib/prelude.warp writes it
fn builtin_signature(word: &str) -> Option<String> {
	builtin_lines().find(|line| line.name == word).map(|line| line.signature())
}

fn builtin_lines() -> impl Iterator<Item = crate::ffi::ForeignLine<'static>> {
	crate::ffi::math_foreign_lines(crate::ffi::BUILTIN_LIBRARY).chain(crate::ffi::runtime_word_lines().iter().copied())
}

/// `replace(t, old, new) := …` → `(t, old, new)`
fn prelude_signature(word: &str) -> Option<String> {
	let head = crate::modules::std_docs::word_head(PRELUDE, word)?;
	head.strip_prefix(word).map(str::to_string)
}

fn libm_signature(word: &str) -> Option<String> {
	crate::ffi::libm_f64_functions()
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
