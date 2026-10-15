//! `warp format [check] [hash-comments] <file|folder>…`, also `warp lint` (decision warp-format): the canonical
//! layout written in place, or with `check` listed as `file:line: layout: <rule>` (exit 1 when something would
//! change); the soft hints (abbreviations, `//` comments) are listed either way and never fail. `warp beautify` is
//! the opinionated verb (card warp-format-strict): the layout plus the best-practice rewrites (hash-comments, synonyms
//! canonical), shown as a diff first (decision beautify-preview): applied after "apply? y/n" in a terminal, with the
//! word `yes` without asking, never when scripted.
//! Its options are words, never `--flags` (user, 2026-10-15: warp's own arguments have no minus-minus style); only
//! help is understood in every spelling
use std::fs;
use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};
use warp::formatting::{format, soft_hints, Formatted, Options};

pub(crate) const FORMAT_COMMANDS: [&str; 2] = ["format", "lint"];
pub(crate) const BEAUTIFY_COMMAND: &str = "beautify";
/// `warp fix <file>` = beautify for now (user 2026-10-15; the agent fixing the rest: card warp-fix)
pub(crate) const BEAUTIFY_COMMANDS: [&str; 2] = [BEAUTIFY_COMMAND, "fix"];
const CHECK_WORD: &str = "check";
const HASH_COMMENTS_WORD: &str = "hash-comments";
/// `warp beautify yes <file>`: applied without the preview's question
const YES_WORD: &str = "yes";
const OPTION_WORDS: [&str; 3] = [CHECK_WORD, HASH_COMMENTS_WORD, YES_WORD];
const HELP_WORDS: [&str; 3] = ["help", "--help", "-h"];
pub(crate) const FORMAT_USAGE: &str = "warp format [check] [hash-comments] <file|folder|code>…  The canonical layout of warp code, only where the meaning stays (warp lint: the same; check: list what would change, exit 1; hash-comments: // comments become #)
  warp beautify [check|yes] <file|folder|code>…  The layout plus the best-practice rewrites (// comments become #, other languages' words the warp words), shown as a diff and applied after \"apply? y/n\" (yes: without asking; warp fix: the same)";
const WARP_EXTENSION: &str = "warp";
/// What hints on code given inline name instead of a file
const CODE_LABEL: &str = "code";
/// code.golf solutions count their bytes: a folder of them keeps its layout (a file named explicitly is formatted)
const SKIPPED_FOLDERS: [&str; 1] = ["golf"];

pub(crate) fn is_format_command(word: &str) -> bool {
	FORMAT_COMMANDS.contains(&word) || BEAUTIFY_COMMANDS.contains(&word)
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

/// The soft hints of `code` that the options leave, printed as `<shown>:<line>: hint: …`; a hint beautify resolves
/// names `warp beautify <target>`
fn printed_hints(shown: &str, target: &str, code: &str, options: Options) -> usize {
	let hints: Vec<_> =
		soft_hints(code).into_iter().filter(|hint| !(options.hash_comments && hint.beautify_fixes)).collect();
	for hint in &hints {
		let fix = if hint.beautify_fixes {
			format!(": `warp {BEAUTIFY_COMMAND} {target}` rewrites them")
		} else {
			String::new()
		};
		println!("{shown}:{}: hint: {}{fix}", hint.line, hint.message);
	}
	hints.len()
}

/// The changed lines of `old` → `new` as `-`/`+` lines under `@@ <file>:<line>`, by their longest common subsequence
fn print_diff(shown: &str, old: &str, new: &str) {
	let (old, new): (Vec<&str>, Vec<&str>) = (old.lines().collect(), new.lines().collect());
	let mut common = vec![vec![0usize; new.len() + 1]; old.len() + 1];
	for i in (0..old.len()).rev() {
		for j in (0..new.len()).rev() {
			common[i][j] =
				if old[i] == new[j] { common[i + 1][j + 1] + 1 } else { common[i + 1][j].max(common[i][j + 1]) };
		}
	}
	let (mut i, mut j, mut in_hunk) = (0, 0, false);
	while i < old.len() || j < new.len() {
		if i < old.len() && j < new.len() && old[i] == new[j] {
			(i, j, in_hunk) = (i + 1, j + 1, false);
			continue;
		}
		if !in_hunk {
			println!("@@ {shown}:{}", i + 1);
			in_hunk = true;
		}
		if j == new.len() || (i < old.len() && common[i + 1][j] >= common[i][j + 1]) {
			println!("-{}", old[i]);
			i += 1;
		} else {
			println!("+{}", new[j]);
			j += 1;
		}
	}
}

/// Shows the diff; whether to apply it: asked in a terminal, never when scripted (decision beautify-preview)
fn confirmed(shown: &str, old: &str, new: &str) -> bool {
	print_diff(shown, old, new);
	if !(std::io::stdin().is_terminal() && std::io::stdout().is_terminal()) {
		return false;
	}
	print!("apply? y/n ");
	let _ = std::io::stdout().flush();
	let mut answer = String::new();
	std::io::stdin().read_line(&mut answer).is_ok() && matches!(answer.trim(), "y" | "yes")
}

/// A name that means a file, never code: `x.warp`, `samples/hello`
fn looks_like_path(argument: &str) -> bool {
	argument.ends_with(&format!(".{WARP_EXTENSION}"))
		|| argument.ends_with(".wasp")
		|| (argument.contains('/') && !argument.contains(char::is_whitespace))
}

/// `warp lint "cnt=1 // one"` (card lint-use): code given instead of files: its hints, then its formatted text (in
/// `check`: the layout changes, exit 1 when there are some); nothing is written
fn format_code(
	code: &str,
	check: bool,
	options: Options,
	rewritten: impl Fn(&str) -> Result<Formatted, String>,
) -> i32 {
	printed_hints(CODE_LABEL, &format!("'{code}'"), code, options);
	let formatted = match rewritten(code) {
		Ok(formatted) => formatted,
		Err(reason) => {
			println!("{CODE_LABEL}: not formatted: {reason}");
			return 1;
		}
	};
	if check {
		formatted.changes.iter().for_each(|change| println!("{CODE_LABEL}:{}: layout: {}", change.line, change.rule));
		return !formatted.changes.is_empty() as i32;
	}
	print!("{}", formatted.text);
	0
}

/// The exit code: 1 when a file does not parse or, with `check`, would change. The option words come first; a file
/// named like one is written `./check`. Arguments naming no file, that read as code, are formatted as code
pub(crate) fn format_command(command: &str, arguments: &[String]) -> i32 {
	if arguments.iter().any(|argument| HELP_WORDS.contains(&argument.as_str())) {
		println!("  {FORMAT_USAGE}");
		return 0;
	}
	let words = arguments.iter().take_while(|argument| OPTION_WORDS.contains(&argument.as_str())).count();
	let (option_words, paths) = arguments.split_at(words);
	let check = option_words.iter().any(|word| word == CHECK_WORD);
	let beautify = BEAUTIFY_COMMANDS.contains(&command);
	let yes = option_words.iter().any(|word| word == YES_WORD);
	let options = Options { hash_comments: beautify || option_words.iter().any(|word| word == HASH_COMMENTS_WORD) };
	let rewritten = |code: &str| if beautify { warp::formatting::beautify(code) } else { format(code, options) };
	let names_code = !paths.is_empty() && paths.iter().all(|path| !Path::new(path).exists() && !looks_like_path(path));
	if names_code {
		return format_code(&paths.join(" "), check, options, rewritten);
	}
	let mut files = Vec::new();
	for path in paths {
		warp_files(Path::new(path), &mut files);
	}
	if files.is_empty() {
		eprintln!("warp {command}: name the warp files or folders to format, or give code\n  {FORMAT_USAGE}");
		return 2;
	}
	let (mut to_change, mut failed, mut hints_left, mut not_applied) = (0, 0, 0, 0);
	for file in &files {
		let shown = file.display().to_string();
		let Ok(code) = fs::read_to_string(file) else {
			println!("{shown}: not formatted: it cannot be read");
			failed += 1;
			continue;
		};
		let hints = printed_hints(&shown, &shown, &code, options);
		hints_left += hints;
		let formatted = match rewritten(&code) {
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
		let lines = formatted.changes.len();
		if check {
			to_change += 1;
			formatted.changes.iter().for_each(|change| println!("{shown}:{}: layout: {}", change.line, change.rule));
		} else if beautify && !yes && !confirmed(&shown, &code, &formatted.text) {
			println!("{shown}: not applied: `warp {BEAUTIFY_COMMAND} {YES_WORD} {shown}` applies it");
			not_applied += 1;
		} else if let Err(error) = fs::write(file, &formatted.text) {
			println!("{shown}: not formatted: {error}");
			failed += 1;
		} else if hints == 0 {
			to_change += 1;
			println!("formatted {shown}: {lines} lines");
		} else {
			to_change += 1;
			// "formatted" would read as done while the hints above still ask for something
			println!("{shown}: layout of {lines} lines fixed, {hints} hints left");
		}
	}
	let verb = if check { "to format" } else { "changed" };
	let previewed = if not_applied > 0 { format!(", {not_applied} not applied") } else { String::new() };
	println!(
		"warp {command}: {} files, {to_change} {verb}{previewed}, {failed} not formatted, {hints_left} hints left",
		files.len()
	);
	(failed > 0 || (check && to_change > 0)) as i32
}
