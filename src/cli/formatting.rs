//! `warp format [check] [hash-comments] <file|folder>…`, also `warp lint` (decision warp-format): the canonical
//! layout written in place, or with `check` listed as `file:line: layout: <rule>` (exit 1 when something would
//! change); the soft hints (abbreviations, `//` comments) are listed either way and never fail. `warp beautify` is
//! the opinionated verb (card warp-format-strict): the layout plus the best-practice rewrites (hash-comments, synonyms
//! canonical), shown as a diff first (decision beautify-preview): applied after "apply? y/n" in a terminal, with the
//! word `yes` without asking, never when scripted.
//! Its options are words, never `--flags` (user, 2026-10-15: warp's own arguments have no minus-minus style); only
//! help is understood in every spelling
use std::path::Path;

pub(crate) const FORMAT_COMMANDS: [&str; 2] = ["format", "lint"];
pub(crate) const BEAUTIFY_COMMAND: &str = "beautify";
/// `warp fix <file>` = beautify 
pub(crate) const BEAUTIFY_COMMANDS: [&str; 2] = [BEAUTIFY_COMMAND, "fix"];
/// ⚠ WARP DOES THIS DIFFERENTLY: warp fix and warp beautify are lib/fix/command.warp, run with the command word and its
/// arguments (card fix-in-lib); src/formatting's beautify stays for its Rust callers
const FIX_COMMAND_PROGRAM: &str = "fix/command.warp";
const CHECK_WORD: &str = "check";
const HASH_COMMENTS_WORD: &str = "hash-comments";
/// `warp beautify yes <file>`: applied without the preview's question
const YES_WORD: &str = "yes";
const OPTION_WORDS: [&str; 3] = [CHECK_WORD, HASH_COMMENTS_WORD, YES_WORD];
const HELP_WORDS: [&str; 3] = ["help", "--help", "-h"];
pub(crate) const FORMAT_USAGE: &str = "warp format [check] [hash-comments] <file|folder|code>…  The canonical layout of warp code, only where the meaning stays (warp lint: the same; check: list what would change, exit 1; hash-comments: // comments become #)
  warp beautify [check|yes] <file|folder|code>…  The layout plus the best-practice rewrites (// comments become #, other languages' words the warp words), shown as a diff and applied after \"apply? y/n\" (yes: without asking; warp fix: the same)";
const WARP_EXTENSIONS: [&str; 2] = ["warp", "wasp"];
/// An extension-less script is warp code when its first line runs warp: `#!/usr/bin/env warp`
const SHEBANG: &str = "#!";
/// What hints on code given inline name instead of a file
const CODE_LABEL: &str = "code";
/// code.golf solutions count their bytes: a folder of them keeps its layout, also when named (`warp fix *` names it);
/// a file named explicitly is formatted
const SKIPPED_FOLDERS: [&str; 1] = ["golf"];
const SKIPPED_FOLDER_REASON: &str = "code golf counts bytes";
/// `warp fix *` names every file of a folder: only warp code is formatted (cards fix-rewrote, fix-laid)
const NOT_WARP_REASON: &str = "not a warp file";

pub(crate) fn is_format_command(word: &str) -> bool {
	FORMAT_COMMANDS.contains(&word) || BEAUTIFY_COMMANDS.contains(&word)
}

/// `x.warp`, `x.wasp`: a name that means a warp program
pub(crate) fn has_warp_extension(path: &str) -> bool {
	Path::new(path).extension().is_some_and(|extension| WARP_EXTENSIONS.iter().any(|warp| extension == *warp))
}

/// The exit code of lib/fix/command.warp, which ends warp itself with it; reached only when the program fails
fn lib_fix_command(command: &str, arguments: &[String]) -> i32 {
	let program = warp::modules::library_folder().join(FIX_COMMAND_PROGRAM);
	let words: Vec<String> = std::iter::once(command.to_string()).chain(arguments.iter().cloned()).collect();
	let ran = warp::compiler::standalone::run(&program, words, warp::compiler::standalone::Compiler::RustCompiled);
	println!("warp {command}: {}", ran.serialize());
	1
}

/// The exit code. `warp beautify` and `warp fix` are lib/fix/command.warp; `warp format` and `warp lint` were the Rust
/// formatter (src/formatting), which left `pure` with the Rust compiler (card rust-bootstrap): not in the self lane yet
pub(crate) fn format_command(command: &str, arguments: &[String]) -> i32 {
	if BEAUTIFY_COMMANDS.contains(&command) {
		return lib_fix_command(command, arguments);
	}
	if arguments.iter().any(|argument| HELP_WORDS.contains(&argument.as_str())) {
		println!("  {FORMAT_USAGE}");
		return 0;
	}
	let _ = (OPTION_WORDS, SHEBANG, CODE_LABEL, SKIPPED_FOLDERS, SKIPPED_FOLDER_REASON, NOT_WARP_REASON);
	eprintln!("warp {command}: not in the self lane yet; this build has no Rust formatter (card rust-bootstrap, branch pure)");
	1
}
