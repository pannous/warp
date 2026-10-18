//! The standard modules' words and the prelude's heads kept on disk between runs (card cache-lowered): reading them
//! out of the sources parsed lib/*.warp at every start, about half the compile of a small program. Kept in the module
//! cache (run/module_cache.rs), keyed by the sources and the warp binary, whose parser reads them
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

fn read_kept_words() -> Option<KeptWords> {
	let text = kept_text(WORDS_EXTENSION)?;
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

/// Keep the words for the next runs, unless the cache is off
pub(super) fn keep_words(words: &[Vec<String>], heads: &[(String, usize, usize)]) {
	let word_lines = words.iter().map(|module| module.join(&WORD_SEPARATOR.to_string()));
	let heads_line = heads
		.iter()
		.map(|(name, arity, defaults)| format!("{name}{HEAD_SEPARATOR}{arity}{HEAD_SEPARATOR}{defaults}"))
		.collect::<Vec<_>>()
		.join(&WORD_SEPARATOR.to_string());
	keep_text(WORDS_EXTENSION, &word_lines.chain([heads_line]).collect::<Vec<_>>().join("\n"));
}

/// The text a run before this one kept under `extension` for these sources and this binary
#[cfg(feature = "native")]
pub(super) fn kept_text(extension: &str) -> Option<String> {
	String::from_utf8(crate::run::module_cache::kept_bytes(&words_key()?, extension)?).ok()
}

#[cfg(not(feature = "native"))]
pub(super) fn kept_text(_: &str) -> Option<String> {
	None
}

/// Keep the text under `extension` for the next runs, unless the cache is off
#[cfg(feature = "native")]
pub(super) fn keep_text(extension: &str, text: &str) {
	if let Some(key) = words_key() {
		crate::run::module_cache::keep_bytes(&key, extension, text.as_bytes());
	}
}

#[cfg(not(feature = "native"))]
pub(super) fn keep_text(_: &str, _: &str) {}

/// The sources the words come from and the binary whose parser read them (its path, size and time)
#[cfg(feature = "native")]
fn words_key() -> Option<String> {
	use sha2::{Digest, Sha256};
	let mut hasher = Sha256::new();
	hasher.update(binary_stamp()?);
	STD_MODULES.iter().for_each(|(_, source)| hasher.update(source));
	hasher.update(std_module(PRELUDE_MODULE).unwrap_or_default());
	Some(crate::extensions::hex_text(&hasher.finalize()))
}

/// The running warp binary: its path, size and time
#[cfg(feature = "native")]
pub(crate) fn binary_stamp() -> Option<String> {
	let binary = std::env::current_exe().ok()?;
	let metadata = std::fs::metadata(&binary).ok()?;
	Some(format!("{}\n{}\n{:?}\n", binary.display(), metadata.len(), metadata.modified().ok()?))
}
