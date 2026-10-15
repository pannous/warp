//! `warp format [check] [hash-comments] <file|folder>…`, also `warp lint` (decision warp-format): the canonical
//! layout written in place, or with `check` listed as `file:line: layout: <rule>` (exit 1 when something would
//! change); the soft hints (abbreviations, `//` comments) are listed either way and never fail. `warp beautify` is
//! the opinionated verb (card warp-format-strict): the layout plus the best-practice rewrites, so far hash-comments.
//! Its options are words, never `--flags` (user, 2026-10-15: warp's own arguments have no minus-minus style); only
//! help is understood in every spelling
use std::fs;
use std::path::{Path, PathBuf};
use warp::formatting::{format, soft_hints, Options};

pub(crate) const FORMAT_COMMANDS: [&str; 2] = ["format", "lint"];
pub(crate) const BEAUTIFY_COMMAND: &str = "beautify";
const CHECK_WORD: &str = "check";
const HASH_COMMENTS_WORD: &str = "hash-comments";
const OPTION_WORDS: [&str; 2] = [CHECK_WORD, HASH_COMMENTS_WORD];
const HELP_WORDS: [&str; 3] = ["help", "--help", "-h"];
pub(crate) const FORMAT_USAGE: &str = "warp format [check] [hash-comments] <file|folder>…  The canonical layout of warp code, only where the meaning stays (warp lint: the same; check: list what would change, exit 1; hash-comments: // comments become #)
  warp beautify [check] <file|folder>…  The layout plus the best-practice rewrites: // comments become #";
const WARP_EXTENSION: &str = "warp";
/// code.golf solutions count their bytes: a folder of them keeps its layout (a file named explicitly is formatted)
const SKIPPED_FOLDERS: [&str; 1] = ["golf"];

pub(crate) fn is_format_command(word: &str) -> bool {
	FORMAT_COMMANDS.contains(&word) || word == BEAUTIFY_COMMAND
}

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
pub(crate) fn format_command(command: &str, arguments: &[String]) -> i32 {
	if arguments.iter().any(|argument| HELP_WORDS.contains(&argument.as_str())) {
		println!("  {FORMAT_USAGE}");
		return 0;
	}
	let words = arguments.iter().take_while(|argument| OPTION_WORDS.contains(&argument.as_str())).count();
	let (option_words, paths) = arguments.split_at(words);
	let check = option_words.iter().any(|word| word == CHECK_WORD);
	let beautify = command == BEAUTIFY_COMMAND;
	let options = Options { hash_comments: beautify || option_words.iter().any(|word| word == HASH_COMMENTS_WORD) };
	let mut files = Vec::new();
	for path in paths {
		warp_files(Path::new(path), &mut files);
	}
	if files.is_empty() {
		eprintln!("warp {command}: name the warp files or folders to format\n  {FORMAT_USAGE}");
		return 2;
	}
	let (mut to_change, mut failed, mut hints_left) = (0, 0, 0);
	for file in &files {
		let shown = file.display();
		let Ok(code) = fs::read_to_string(file) else {
			println!("{shown}: not formatted: it cannot be read");
			failed += 1;
			continue;
		};
		let hints: Vec<_> =
			soft_hints(&code).into_iter().filter(|hint| !(options.hash_comments && hint.beautify_fixes)).collect();
		for hint in &hints {
			let fix = if hint.beautify_fixes {
				format!(": `warp {BEAUTIFY_COMMAND} {shown}` rewrites them")
			} else {
				String::new()
			};
			println!("{shown}:{}: hint: {}{fix}", hint.line, hint.message);
		}
		hints_left += hints.len();
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
		let lines = formatted.changes.len();
		if check {
			formatted.changes.iter().for_each(|change| println!("{shown}:{}: layout: {}", change.line, change.rule));
		} else if let Err(error) = fs::write(file, &formatted.text) {
			println!("{shown}: not formatted: {error}");
			failed += 1;
		} else if hints.is_empty() {
			println!("formatted {shown}: {lines} lines");
		} else {
			// "formatted" would read as done while the hints above still ask for something
			println!("{shown}: layout of {lines} lines fixed, {} hints left", hints.len());
		}
	}
	let verb = if check { "to format" } else { "changed" };
	println!(
		"warp {command}: {} files, {to_change} {verb}, {failed} not formatted, {hints_left} hints left",
		files.len()
	);
	(failed > 0 || (check && to_change > 0)) as i32
}
