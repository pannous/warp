//! `warp app page.warp`, `warp page.html` (card read-chron, src/webview.rs): the page in the system's web view;
//! `--check` loads it in a hidden window and prints its text, so these run without a window on the screen
use std::path::PathBuf;

fn folder() -> PathBuf {
	let folder = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("test_webview");
	std::fs::create_dir_all(&folder).unwrap();
	folder
}

fn checked_text(file: &str, content: &str) -> String {
	let page = folder().join(file);
	std::fs::write(&page, content).unwrap();
	let output = crate::common::warp_command().args(["app", "--check"]).arg(&page).env("WARP_NO_WINDOW", "1").output().unwrap();
	assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
	String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn an_html_file_shows_in_the_web_view() {
	assert!(checked_text("page.html", "<html><body><p>hello <b>page</b></p></body></html>").contains("hello page"));
}

#[test]
fn a_program_shows_its_page_with_what_a_command_printed() {
	let text = checked_text("crontab.warp", "html{ lang:\"en\" body{ table{ tr{ td{ \"job\" } td{ exec sh \"echo printed\" } } } } }");
	assert!(text.contains("job\tprinted"), "{text}");
}

#[test]
fn without_check_no_window_opens_under_warp_no_window() {
	let page = folder().join("refused.html");
	std::fs::write(&page, "<p>x</p>").unwrap();
	let output = crate::common::warp_command().arg(&page).env("WARP_NO_WINDOW", "1").output().unwrap();
	assert!(!output.status.success());
	assert!(String::from_utf8_lossy(&output.stderr).contains("--check"));
}

// `warp crontab.warp` without `app` (user): a program whose value is an html{…} page shows it in a window when run
// from a terminal, as routes make `warp app.warp` serve; piped, redirected or under WARP_NO_WINDOW it prints the HTML
#[test]
fn a_program_ending_in_an_html_page_is_shown() {
	assert!(warp::serve::shows_page("jobs = [1]\nhtml{ lang:\"en\" body{ \"hi\" } }"));
	assert!(warp::serve::shows_page("html { body{ \"hi\" } }"));
	assert!(!warp::serve::shows_page("div{ \"hi\" }"));
	assert!(!warp::serve::shows_page("page = html{ body{ \"hi\" } }\ncount(page)"));
	let page = folder().join("printed.warp");
	std::fs::write(&page, "html{ body{ \"hi\" } }").unwrap();
	let output = crate::common::warp_command().arg("run").arg(&page).env("WARP_NO_WINDOW", "1").output().unwrap();
	assert!(String::from_utf8_lossy(&output.stdout).contains("<body>hi</body>"), "{}", String::from_utf8_lossy(&output.stderr));
}
