// cards fix-rewrote, fix-laid (the user's `cd samples; warp fix *`): the shell's glob names every entry, so a JS file,
// a .wit file and the golf folder came as arguments and were rewritten. Only .warp files are formatted, and a golf
// folder keeps its layout also when named; each skip says why
#![cfg(feature = "native")]

const SLASH_COMMENTED: &str = "// a note\nx=1+2\nx\n";

fn folder_as_globbed() -> std::path::PathBuf {
	let folder = crate::common::scratch_directory("format_files_only");
	std::fs::create_dir_all(folder.join("golf")).unwrap();
	for name in ["plain.warp", "golf/short.warp", "examples.js", "types.wit"] {
		std::fs::write(folder.join(name), SLASH_COMMENTED).unwrap();
	}
	folder
}

#[test]
fn a_glob_of_a_folder_formats_only_its_warp_files_outside_golf() {
	let folder = folder_as_globbed();
	let named: Vec<String> = ["plain.warp", "golf", "examples.js", "types.wit"]
		.iter()
		.map(|name| folder.join(name).to_string_lossy().into())
		.collect();
	let output =
		crate::common::warp_command().args(["--no-ask", "fix", "yes"]).args(&named).output().expect("warp runs");
	let stdout = String::from_utf8_lossy(&output.stdout);
	assert_eq!(output.status.code(), Some(0), "{stdout}");
	let read = |name: &str| std::fs::read_to_string(folder.join(name)).unwrap();
	assert_eq!(read("plain.warp"), "# a note\nx = 1 + 2\nx\n", "{stdout}");
	for kept in ["golf/short.warp", "examples.js", "types.wit"] {
		assert_eq!(read(kept), SLASH_COMMENTED, "{kept}: {stdout}");
	}
	assert!(stdout.contains("examples.js: skipped: not a warp file"), "{stdout}");
	assert!(stdout.contains("golf: skipped: code golf counts bytes"), "{stdout}");
}
