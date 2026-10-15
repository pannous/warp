//! card unix-words: `use script` has the basic Unix commands (lib/shell.warp) as warp words with warp results:
//! ls, pwd, cd, cat, echo, mkdir, cp, mv, touch, head, tail, wc, grep, which, find_files, file_size, info, sort_lines,
//! unique_lines, open and rm, which moves to the Trash with guard rails; paths without quotes
#![cfg(feature = "native")]
use crate::common::{fails_with, printed, warp_command};

const FOLDER: &str = "scratch/test_shell_words";

/// The program under `use script`, run in a fresh folder of its own (`cd` in it changes only that run's process)
pub(crate) fn script_in(folder: &str, code: &str) -> String {
	let folder = format!("{FOLDER}/{folder}");
	let _ = std::fs::remove_dir_all(&folder);
	std::fs::create_dir_all(&folder).unwrap();
	printed(&format!("use script\ncd ./{folder}\n{code}"))
}

#[test]
fn files_are_made_copied_moved_and_listed() {
	let code = "mkdir ./a/b\ntouch ./a/.hidden\nwrite(\"a/x.txt\", \"one\\ntwo fn\\nthree\\n\")\ncp ./a/x.txt ./a/b\nmv ./a/b/x.txt ./a/y.txt\nprint ls ./a";
	assert_eq!(script_in("files", code), "['b' \"x.txt\" \"y.txt\"]\n");
}

#[test]
fn a_file_is_read_by_lines() {
	let code = "write(\"t.txt\", \"one\\ntwo fn\\nthree\\n\")\nprint head(./t.txt, 2)\nprint tail ./t.txt\nprint wc ./t.txt\nprint sort_lines ./t.txt\nprint unique_lines ./t.txt\nprint grep(\"f.\", ./t.txt)\nprint cat ./t.txt";
	let expected = "[\"one\" \"two fn\"]\n[\"one\" \"two fn\" \"three\"]\n{\"lines\":3 \"words\":4 \"bytes\":17}\n[\"one\" \"three\" \"two fn\"]\n[\"one\" \"two fn\" \"three\"]\n[\"two fn\"]\none\ntwo fn\nthree\n\n";
	assert_eq!(script_in("lines", code), expected);
}

#[test]
fn cd_changes_the_working_directory() {
	let changed = script_in("cd", "mkdir ./sub\ncd ./sub\nprint pwd");
	let sub = std::path::Path::new(FOLDER).join("cd/sub").canonicalize().unwrap();
	assert_eq!(changed, format!("{}\n", sub.display()));
	let home = std::env::var("HOME").unwrap();
	assert_eq!(printed("use script\ncd ~\nprint pwd"), format!("{home}\n"));
	assert_eq!(printed("use script\nprint exists ~/"), "yes\n");
}

#[test]
fn a_shell_word_takes_an_absolute_path() {
	assert_eq!(printed("use script\nprint file_size /etc/hosts > 0"), "yes\n");
	assert_eq!(printed("use script\ni = info /etc/hosts\nprint i.kind"), "file\n");
	let folder = format!("{FOLDER}/copied");
	let _ = std::fs::create_dir_all(&folder);
	let target = std::fs::canonicalize(&folder).unwrap();
	let code = format!("use script\ncp /etc/hosts {}\nprint exists {}/hosts", target.display(), target.display());
	assert_eq!(printed(&code), "yes\n");
}

#[test]
#[ignore = "card pipes-after: a pipe after a braceless call (print cat \"/etc/hosts\" | count traps)"]
fn a_shell_word_pipes() {
	assert_eq!(printed("use script\nprint cat /etc/hosts | count > 0"), "yes\n");
}

#[test]
fn which_finds_a_program() {
	assert_eq!(printed("use script\nprint which \"sh\""), "/bin/sh\n");
	assert_eq!(printed("use script\nprint which \"no-such-program-here\""), "ø\n");
	assert_eq!(printed("use script\necho \"hi\""), "hi\n");
}

#[test]
fn files_are_found_and_measured() {
	let code = "mkdir ./a/b\nwrite(\"a/b/x.warp\", \"12345\")\nwrite(\"a/y.warp\", \"1\")\nwrite(\"a/z.txt\", \"\")\nprint find_files(./a, \"*.warp\")\nprint file_size ./a\ni = info ./a/b/x.warp\nprint i[\"size\"], i.kind, i.modified > 0";
	assert_eq!(script_in("find", code), "[\"./a/b/x.warp\" \"./a/y.warp\"]\n6\n5 file yes\n");
}

#[test]
fn rm_removes_a_file_with_guard_rails() {
	// a home of its own, whose Trash the test reads
	let home = std::path::Path::new(FOLDER).join("home");
	let _ = std::fs::remove_dir_all(&home);
	std::fs::create_dir_all(&home).unwrap();
	let home = home.canonicalize().unwrap();
	let code = format!(
		"use script\ncd {}\ntouch ./z.txt\nmkdir ./e\nrm ./z.txt\nrm ./e\nprint exists ./z.txt, exists ./e",
		home.display()
	);
	let output = warp_command().args(["--no-ask", &code]).env("HOME", &home).output().unwrap();
	assert_eq!(String::from_utf8_lossy(&output.stdout), "no no\n");
	let trash = home.join(if cfg!(target_os = "macos") { ".Trash" } else { ".local/share/Trash/files" });
	assert!(trash.join("z.txt").is_file() && trash.join("e").is_dir(), "{} holds z.txt and e", trash.display());
	let full = format!("{FOLDER}/full");
	std::fs::create_dir_all(&full).unwrap();
	std::fs::write(format!("{full}/a"), "").unwrap();
	let refused = |code: &str, reason: &str| fails_with(&format!("use script\n{code}"), reason);
	refused(&format!("rm ./{full}"), "only when empty");
	refused("rm \"*.txt\"", "no pattern");
	refused("rm ~", "never removes");
	refused("rm \".\"", "never removes");
}
