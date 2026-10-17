//! Showing what ran: values printed, exit statuses, pages in a window or the browser, parse trees
use super::*;

/// The exit status of a run that ends in an uncaught error (card cli-error-exit)
pub(crate) const ERROR_STATUS: i32 = 1;

/// A program file's exit status: its Int result; a float (a measurement such as elapsed seconds) is no status
pub(crate) fn node_to_i32(node: &Node) -> i32 {
	match node.drop_meta() {
		Node::Number(Number::Int(n)) => *n as i32,
		Node::Error(_) => ERROR_STATUS,
		_ => 0,
	}
}

/// The program's value printed, its Int the exit status, an uncaught error status 1
pub(crate) fn print_and_exit(result: Node) -> ! {
	exit_if_program_exited();
	show(&result, "");
	ended_well(&result);
	std::process::exit(node_to_i32(&result));
}

/// The value of inline code printed; an uncaught error exits with status 1, so scripts and CI see it failed
pub(crate) fn show_and_fail_on_error(result: &Node, mark: &str) {
	exit_if_program_exited();
	show(result, mark);
	if matches!(result.drop_meta(), Node::Error(_)) {
		std::process::exit(ERROR_STATUS);
	}
	ended_well(result);
}

/// The status of the program's `exit(code)` (P121), for a file run and inline code alike: it ends the run now, its
/// sounds stopped (card escape-window), where a program ending by itself lets them finish
fn exit_if_program_exited() {
	if let Some(code) = warp_runtime::system_signals::take_exit_code() {
		warp::sound::stop();
		let _ = io::stdout().flush();
		std::process::exit(code);
	}
}

/// A program that ended well keeps its painted window up until the user closes it; exit() or a failure closes it
pub(crate) fn ended_well(result: &Node) {
	if !matches!(result.drop_meta(), Node::Error(_)) {
		warp::graphics::paint_window::wait_until_closed();
	}
}

/// A program's value after what it printed, behind `mark`; nothing for ø, the value of `print` (issue #18) and of a
/// program that only acts, as Python's console shows nothing for None
pub(crate) fn show(result: &Node, mark: &str) {
	if !result.is_empty_node() {
		// markup (`html{ body{ … } }`) is printed as HTML: `warp run page.warp > page.html` (card web-dom)
		let shown = if markup::is_markup(result) { markup::to_html(result) } else { result.serialize() };
		println!("{mark}{shown}");
	}
	if let Some(excerpt) =
		diagnostic::error_position(result).and_then(|(line, column)| diagnostic::shown_excerpt(line, column))
	{
		let _ = io::stdout().flush();
		eprintln!("{excerpt}");
	}
}

/// A program ending in markup (`html{…}`, `div{…}`, card div-opens-app) run with a terminal for its output: shown in a
/// window as `warp app` shows it; piped, redirected or under WARP_NO_WINDOW it prints its HTML
pub(crate) fn shows_in_window(code: &str) -> bool {
	io::stdout().is_terminal() && warp::graphics::paint::shows_windows() && warp::serve::shows_page(code)
}

/// `warp dev` and a served program run from a terminal open their page (cards dev-opens-browser, samples-serving); piped, under WARP_NO_WINDOW or CI it doesn't
pub(crate) fn opens_browser() -> bool {
	io::stdout().is_terminal() && warp::graphics::paint::shows_windows()
}

pub(crate) fn is_html_file(target: &str) -> bool {
	target.ends_with(".html") || target.ends_with(".htm")
}

/// `warp page.html`, `warp app [--check] page.warp`: the page in a web view window (src/web/webview.rs); --check prints
/// its text from a hidden window instead
pub(crate) fn show_in_webview(target: &str, check: bool) {
	if !check && !warp::graphics::paint::shows_windows() {
		fail(format!("warp {APP_COMMAND}: no window under {} or CI; `warp {APP_COMMAND} {APP_CHECK} {target}` prints the page's text", warp::graphics::paint::NO_WINDOW_VARIABLE));
	}
	#[cfg(target_os = "macos")]
	let shown = if is_html_file(target) {
		warp::web::webview::show_file(std::path::Path::new(target), check)
	} else {
		let title = std::path::Path::new(target)
			.file_stem()
			.filter(|_| file_exists(target))
			.map_or(DEFAULT_SITE_NAME.into(), |stem| stem.to_string_lossy());
		warp::web::webview::show_program(&source_of(target), &title, check)
	};
	#[cfg(not(target_os = "macos"))]
	let shown: Result<(), String> = Err(format!(
		"warp is compiled without a web view here (macOS only for now); `warp build --site {target}` writes the page"
	));
	if let Err(failure) = shown {
		fail(format!("warp {APP_COMMAND}: {failure}"));
	}
}

/// Parse tree as s-expression: `(op left right)` for keys, `[items]` for lists
pub(crate) fn structure(node: &Node) -> String {
	match node.drop_meta() {
		Node::Key(left, op, right) => format!("({} {} {})", op, structure(left), structure(right)),
		Node::List(items, bracket, separator) => {
			let inner: Vec<String> = items.iter().map(structure).collect();
			let joiner = match separator.to_char() {
				Some(' ') | None => " ".to_string(),
				Some('\n') => "⏎ ".to_string(),
				Some(other) => format!("{other} "),
			};
			format!("{}{}{}", bracket.opening(), inner.join(&joiner), bracket.closing())
		}
		other => other.serialize(),
	}
}
