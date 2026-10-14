//! Card build-links: `warp build page.warp` of a program that shows a page (as `warp page.warp` opens it in a window)
//! writes the executable `page`: the runtime stub carrying the page's site, which it shows in the system's web view
//! with no warp installed. `--check` loads it in a hidden window and prints its text, as `warp app --check` does
use std::path::PathBuf;

const PRESSES_Q: &str =
	"script{ \"setTimeout(() => document.dispatchEvent(new KeyboardEvent('keydown', {key: 'q'})), 100)\" }";

/// `warp build` of `code` as <name>.warp, then the built app run with `--check`
fn built_app_checked(name: &str, code: &str) -> std::process::Output {
	let folder = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("test_built_app");
	std::fs::create_dir_all(&folder).unwrap();
	let page = folder.join(format!("{name}.warp"));
	std::fs::write(&page, code).unwrap();
	let build = crate::common::warp_command()
		.env("WARP_PAGE_RUNTIME_STUB", crate::common::page_runtime_stub())
		.env("WARP_NO_WINDOW", "1")
		.arg("build")
		.arg(&page)
		.output()
		.unwrap();
	assert!(build.status.success(), "{}", String::from_utf8_lossy(&build.stderr));
	assert!(String::from_utf8_lossy(&build.stdout).contains("web view"), "{}", String::from_utf8_lossy(&build.stdout));
	std::process::Command::new(page.with_extension(std::env::consts::EXE_EXTENSION)).arg("--check").output().unwrap()
}

#[test]
fn a_built_page_program_is_an_app_showing_its_page() {
	let shown = built_app_checked("hello_app", "div{ h1{ \"hello\" } p{ \"built as an app\" } }");
	let text = String::from_utf8_lossy(&shown.stdout);
	assert!(shown.status.success(), "{}", String::from_utf8_lossy(&shown.stderr));
	assert!(text.contains("hello") && text.contains("built as an app"), "{text}");
	let exited =
		built_app_checked("exits_app", &format!("on key {{ exit 3 }}\ndiv{{ p{{ \"press a key\" }} {PRESSES_Q} }}"));
	assert_eq!(exited.status.code(), Some(3), "{}", String::from_utf8_lossy(&exited.stderr));
}
