// card yes-fix (user): `yes y | warp fix *` applied nothing, the preview asked only in a terminal. Piped lines answer
// "apply? y/n", one per file; the end of the input answers no
#![cfg(feature = "native")]
use std::io::Write;
use std::process::Stdio;

const SLASH_COMMENTED: &str = "// a note\nx=1+2\nx\n";
const BEAUTIFIED: &str = "# a note\nx = 1 + 2\nx\n";

/// The files after `warp fix` on them with `answers` piped in
fn fixed_with_answers(name: &str, answers: &str) -> (Vec<String>, String) {
	let folder = crate::common::scratch_directory(name);
	std::fs::create_dir_all(&folder).unwrap();
	let files: Vec<_> = ["a.warp", "b.warp"].iter().map(|file| folder.join(file)).collect();
	files.iter().for_each(|file| std::fs::write(file, SLASH_COMMENTED).unwrap());
	let mut child = crate::common::warp_command()
		.args(["--no-ask", "fix"])
		.args(&files)
		.stdin(Stdio::piped())
		.stdout(Stdio::piped())
		.spawn()
		.expect("warp runs");
	child.stdin.take().unwrap().write_all(answers.as_bytes()).unwrap();
	let output = child.wait_with_output().unwrap();
	let contents = files.iter().map(|file| std::fs::read_to_string(file).unwrap()).collect();
	(contents, String::from_utf8_lossy(&output.stdout).into())
}

#[test]
fn piped_yes_answers_every_preview() {
	let (contents, stdout) = fixed_with_answers("piped_yes", "y\ny\ny\n");
	assert_eq!(contents, [BEAUTIFIED, BEAUTIFIED], "{stdout}");
}

#[test]
fn each_piped_line_answers_one_file_and_the_end_answers_no() {
	let (contents, stdout) = fixed_with_answers("piped_yes_then_end", "y\n");
	assert_eq!(contents, [BEAUTIFIED, SLASH_COMMENTED], "{stdout}");
	assert!(stdout.contains("not applied"), "{stdout}");
}
