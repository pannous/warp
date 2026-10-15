//! `warp format [check] [hash-comments] <file|folder>…`, also `warp lint` (decision warp-format): the canonical
//! layout written in place, or with `check` listed as `file:line: layout: <rule>` (exit 1 when something would
//! change); the soft hints (abbreviations, `//` comments) are listed either way and never fail. Its options are
//! words, never `--flags` (user, 2026-10-15: warp's own arguments have no minus-minus style)
use std::fs;
use std::path::{Path, PathBuf};
use warp::formatting::{format, soft_hints, Options};

pub(crate) const FORMAT_COMMANDS: [&str; 2] = ["format", "lint"];
const CHECK_WORD: &str = "check";
const HASH_COMMENTS_WORD: &str = "hash-comments";
const OPTION_WORDS: [&str; 2] = [CHECK_WORD, HASH_COMMENTS_WORD];
const WARP_EXTENSION: &str = "warp";
/// code.golf solutions count their bytes: a folder of them keeps its layout (a file named explicitly is formatted)
const SKIPPED_FOLDERS: [&str; 1] = ["golf"];

fn warp_files(path: &Path, files: &mut Vec<PathBuf>) {
	if path.is_dir() {
		let mut entries: Vec<PathBuf> =
			fs::read_dir(path).into_iter().flatten().flatten().map(|entry| entry.path()).collect();
		entries.sort();
		for entry in entries {
			let skipped = entry.is_dir()
				&& entry.file_name().is_some_and(|name| SKIPPED_FOLDERS.iter().any(|skip| name == *skip));
			if !skipped && (entry.is_dir() || entry.extension().is_some_and(|extension| extension == WARP_EXTENSION)) {
				warp_files(&entry, files);
			}
		}
	} else {
		files.push(path.to_path_buf());
	}
}

/// The exit code: 1 when a file does not parse or, with `check`, would change. The option words come first; a file
/// named like one is written `./check`
pub(crate) fn format_command(arguments: &[String]) -> i32 {
	let words = arguments.iter().take_while(|argument| OPTION_WORDS.contains(&argument.as_str())).count();
	let (option_words, paths) = arguments.split_at(words);
	let check = option_words.iter().any(|word| word == CHECK_WORD);
	let options = Options { hash_comments: option_words.iter().any(|word| word == HASH_COMMENTS_WORD) };
	let mut files = Vec::new();
	for path in paths {
		warp_files(Path::new(path), &mut files);
	}
	if files.is_empty() {
		eprintln!("warp format [check] [hash-comments] <file|folder>…: name the warp files or folders to format");
		return 2;
	}
	let (mut to_change, mut failed, mut hints) = (0, 0, 0);
	for file in &files {
		let shown = file.display();
		let Ok(code) = fs::read_to_string(file) else {
			println!("{shown}: not formatted: it cannot be read");
			failed += 1;
			continue;
		};
		for hint in soft_hints(&code) {
			println!("{shown}:{}: hint: {}", hint.line, hint.message);
			hints += 1;
		}
		let formatted = match format(&code, options) {
			Ok(formatted) => formatted,
			Err(reason) => {
				println!("{shown}: not formatted: {reason}");
				failed += 1;
				continue;
			}
		};
		if formatted.text == code {
			continue;
		}
		to_change += 1;
		if check {
			formatted.changes.iter().for_each(|change| println!("{shown}:{}: layout: {}", change.line, change.rule));
		} else if let Err(error) = fs::write(file, &formatted.text) {
			println!("{shown}: not formatted: {error}");
			failed += 1;
		} else {
			println!("formatted {shown}: {} lines", formatted.changes.len());
		}
	}
	let verb = if check { "to format" } else { "formatted" };
	println!("warp format: {} files, {to_change} {verb}, {failed} not formatted, {hints} hints", files.len());
	(failed > 0 || (check && to_change > 0)) as i32
}
