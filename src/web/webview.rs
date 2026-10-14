//! `warp app page.warp` and `warp page.html` (card read-chron): the page in a window of the system's web view
//! (WKWebView through wry), what `warp build --site` writes served from memory under warp://localhost/, an HTML file
//! with the files beside it. `--check` loads the page in a hidden window and prints its text instead, then ends
//! (probes and tests: no window on the user's screen).

use std::path::{Path, PathBuf};
use warp_runtime::site_files::served;
use warp_runtime::webview::show;

/// The page of the program `code` (its site, rendered and hydrated) in a window titled `title`
pub fn show_program(code: &str, title: &str, check: bool) -> Result<(), String> {
	let files = crate::web::site::files(code, title, false)?;
	show(title, check, Box::new(move |path| served(&files, path)))
}

/// The HTML file in a window, the files beside it at their relative paths
pub fn show_file(page: &Path, check: bool) -> Result<(), String> {
	let page = page.canonicalize().map_err(|failure| format!("cannot open {}: {failure}", page.display()))?;
	let folder = page.parent().map(Path::to_path_buf).unwrap_or_default();
	let title = page.file_stem().map(|stem| stem.to_string_lossy().into_owned()).unwrap_or_default();
	show(&title, check, Box::new(move |path| folder_file(&page, &folder, path)))
}

/// "/" is the page, any other path a file inside its folder (never outside: `..` is refused by the containment check)
fn folder_file(page: &Path, folder: &Path, path: &str) -> Option<(Vec<u8>, &'static str)> {
	let file: PathBuf =
		if path == "/" { page.to_path_buf() } else { folder.join(path.trim_start_matches('/')).canonicalize().ok()? };
	file.starts_with(folder).then_some(())?;
	let name = file.to_string_lossy().into_owned();
	std::fs::read(&file).ok().map(|bytes| (bytes, crate::web::site::content_type(&name)))
}
