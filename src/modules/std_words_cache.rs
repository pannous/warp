//! The standard modules' words and the prelude's heads kept on disk between runs (card cache-lowered): reading them
//! out of the sources parsed lib/*.warp at every start, about half the compile of a small program. Kept in the module
//! cache (run/module_cache.rs), keyed by the sources and the warp binary, whose parser reads them
#![cfg_attr(not(feature = "native"), allow(dead_code, unused_imports))] // the browser has no module cache
use super::*;

/// The extension of the kept words in the module cache
const WORDS_EXTENSION: &str = "std-words";
const WORD_SEPARATOR: char = ' ';
const HEAD_SEPARATOR: char = ',';

/// Each standard module's words in STD_MODULES order, and each prelude word with its number of parameters and of
/// those with a default
pub(super) struct KeptWords {
	pub(super) words: Vec<Vec<String>>,
	pub(super) heads: Vec<(String, usize, usize)>,
}

/// The words a run before this one kept for these sources and this binary
pub(super) fn kept_words() -> Option<&'static KeptWords> {
	static KEPT: std::sync::OnceLock<Option<KeptWords>> = std::sync::OnceLock::new();
	KEPT.get_or_init(read_kept_words).as_ref()
}

#[cfg(feature = "native")]
fn read_kept_words() -> Option<KeptWords> {
	let bytes = crate::run::module_cache::kept_bytes(&words_key()?, WORDS_EXTENSION)?;
	let text = String::from_utf8(bytes).ok()?;
	let lines: Vec<&str> = text.lines().collect();
	let (heads_line, word_lines) = lines.split_last()?;
	if word_lines.len() != STD_MODULES.len() {
		return None;
	}
	let words = word_lines
		.iter()
		.map(|line| line.split(WORD_SEPARATOR).filter(|word| !word.is_empty()).map(String::from).collect());
	let head = |written: &str| {
		let mut parts = written.split(HEAD_SEPARATOR);
		Some((parts.next()?.to_string(), parts.next()?.parse().ok()?, parts.next()?.parse().ok()?))
	};
	let heads =
		heads_line.split(WORD_SEPARATOR).filter(|written| !written.is_empty()).map(head).collect::<Option<_>>()?;
	Some(KeptWords { words: words.collect(), heads })
}

#[cfg(not(feature = "native"))]
fn read_kept_words() -> Option<KeptWords> {
	None
}

/// Keep the words for the next runs, unless the cache is off
#[cfg(feature = "native")]
pub(super) fn keep_words(words: &[Vec<String>], heads: &[(String, usize, usize)]) {
	let Some(key) = words_key() else { return };
	let word_lines = words.iter().map(|module| module.join(&WORD_SEPARATOR.to_string()));
	let heads_line = heads
		.iter()
		.map(|(name, arity, defaults)| format!("{name}{HEAD_SEPARATOR}{arity}{HEAD_SEPARATOR}{defaults}"))
		.collect::<Vec<_>>()
		.join(&WORD_SEPARATOR.to_string());
	let text = word_lines.chain([heads_line]).collect::<Vec<_>>().join("\n");
	crate::run::module_cache::keep_bytes(&key, WORDS_EXTENSION, text.as_bytes());
}

#[cfg(not(feature = "native"))]
pub(super) fn keep_words(_: &[Vec<String>], _: &[(String, usize, usize)]) {}

/// The sources the words come from and the binary whose parser read them (its path, size and time)
#[cfg(feature = "native")]
fn words_key() -> Option<String> {
	use sha2::{Digest, Sha256};
	let binary = std::env::current_exe().ok()?;
	let metadata = std::fs::metadata(&binary).ok()?;
	let mut hasher = Sha256::new();
	hasher.update(format!("{}\n{}\n{:?}\n", binary.display(), metadata.len(), metadata.modified().ok()?));
	STD_MODULES.iter().for_each(|(_, source)| hasher.update(source));
	hasher.update(std_module(PRELUDE_MODULE).unwrap_or_default());
	Some(hasher.finalize().iter().map(|byte| format!("{byte:02x}")).collect())
}
