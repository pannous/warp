// card beautify-cli (the user's terminal): `warp beautify <file>` ran as code ("type error: int / text"), `warp format`
// said "formatted" while its own `//` hint stayed, and `warp format --help` read --help as a file
#![cfg(feature = "native")]

const SLASH_COMMENTED: &str = "// a note\nx=1+2\nx\n";

struct Run {
	status: Option<i32>,
	stdout: String,
	stderr: String,
}

fn warp(arguments: &[&str]) -> Run {
	let output = crate::common::warp_command().arg("--no-ask").args(arguments).output().expect("warp runs");
	Run {
		status: output.status.code(),
		stdout: String::from_utf8_lossy(&output.stdout).into(),
		stderr: String::from_utf8_lossy(&output.stderr).into(),
	}
}

/// A fresh file holding SLASH_COMMENTED
fn slash_commented_file(name: &str) -> String {
	let folder = crate::common::scratch_directory("format_cli");
	std::fs::create_dir_all(&folder).unwrap();
	let path = folder.join(format!("{name}.warp"));
	std::fs::write(&path, SLASH_COMMENTED).unwrap();
	path.to_string_lossy().into()
}

#[test]
fn beautify_formats_and_rewrites_slash_comments() {
	let file = slash_commented_file("beautify");
	let run = warp(&["beautify", "yes", &file]);
	assert_eq!(run.status, Some(0), "{}{}", run.stdout, run.stderr);
	assert_eq!(std::fs::read_to_string(&file).unwrap(), "# a note\nx = 1 + 2\nx\n");
	assert!(!run.stdout.contains("prefer # comments"), "a rewritten comment leaves no hint: {}", run.stdout);
}

// decision beautify-preview (card warp-format-strict): beautify shows its diff first; scripted, it applies nothing
#[test]
fn beautify_without_yes_shows_the_diff_and_applies_nothing_when_scripted() {
	let file = slash_commented_file("beautify_preview");
	let run = warp(&["beautify", &file]);
	assert_eq!(run.status, Some(0), "{}{}", run.stdout, run.stderr);
	assert!(run.stdout.contains("-// a note\n") && run.stdout.contains("+# a note\n"), "{}", run.stdout);
	assert!(run.stdout.contains(&format!("warp beautify yes {file}")), "{}", run.stdout);
	assert_eq!(std::fs::read_to_string(&file).unwrap(), SLASH_COMMENTED);
}

#[test]
fn an_unknown_command_before_a_file_is_no_expression() {
	let file = slash_commented_file("unknown_command");
	let run = warp(&["formt", &file]);
	assert_ne!(run.status, Some(0));
	assert!(run.stderr.contains("unknown command formt") && run.stderr.contains("format"), "{}", run.stderr);
	assert!(!run.stdout.contains("type error"), "{}", run.stdout);
	assert_eq!(std::fs::read_to_string(&file).unwrap(), SLASH_COMMENTED);
}

#[test]
fn format_leaving_its_hint_names_the_command_and_says_no_formatted() {
	let file = slash_commented_file("format_hint");
	let run = warp(&["format", &file]);
	assert_eq!(run.status, Some(0), "{}{}", run.stdout, run.stderr);
	assert_eq!(std::fs::read_to_string(&file).unwrap(), "// a note\nx = 1 + 2\nx\n");
	assert!(run.stdout.contains(&format!("warp beautify {file}")), "the hint names the exact command: {}", run.stdout);
	assert!(!run.stdout.contains(&format!("formatted {file}")), "{}", run.stdout);
}

#[test]
fn format_help_prints_its_usage() {
	for help in ["--help", "-h", "help"] {
		let run = warp(&["format", help]);
		assert_eq!(run.status, Some(0), "{help}: {}{}", run.stdout, run.stderr);
		assert!(run.stdout.contains("warp format [check]") && run.stdout.contains("warp beautify"), "{}", run.stdout);
		assert!(!run.stdout.contains("cannot be read"), "{}", run.stdout);
	}
}

// card lint-use: code given instead of a file is linted too: its hints and its formatted text, nothing written
#[test]
fn lint_of_inline_code_prints_hints_and_the_formatted_code() {
	let run = warp(&["lint", "cnt=1+2 // three"]);
	assert_eq!(run.status, Some(0), "{}{}", run.stdout, run.stderr);
	assert!(run.stdout.contains("cnt → count") && run.stdout.contains("prefer # comments"), "{}", run.stdout);
	assert!(run.stdout.contains("cnt = 1 + 2 // three"), "{}", run.stdout);
	assert!(!run.stdout.contains("cannot be read"), "{}", run.stdout);
	let beautified = warp(&["beautify", "use script; ls // the folder"]);
	assert!(beautified.stdout.contains("use script; ls # the folder"), "{}", beautified.stdout);
}
