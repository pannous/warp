//! `warp serve`: the program and port served
use super::*;

/// The program `warp serve` serves when it is given none: the first of these in the current folder
pub(crate) const DEFAULT_PROGRAMS: [&str; 2] = ["app.warp", "main.warp"];

/// `warp serve [file] [port]`: the program file (app.warp or main.warp of the current folder when none is given) and
/// the port (8080 when none is given)
#[cfg(not(test))]
pub(crate) fn served_program(arguments: &[String]) -> Result<(&str, u16), String> {
	let (ports, files): (Vec<&String>, Vec<&String>) =
		arguments.iter().partition(|argument| argument.chars().all(|digit| digit.is_ascii_digit()));
	let port = match ports.as_slice() {
		[] => SERVE_PORT,
		[port] => port.parse().map_err(|_| format!("{port} is no port"))?,
		several => return Err(format!("one port, not {}", several.len())),
	};
	let file = match files.as_slice() {
		[] => DEFAULT_PROGRAMS
			.iter()
			.find(|file| file_exists(file))
			.copied()
			.ok_or_else(|| format!("no program given and no {} here", DEFAULT_PROGRAMS.join(" or ")))?,
		[file] => file.as_str(),
		several => return Err(format!("one program, not {}", several.len())),
	};
	Ok((file, port))
}

/// The program served at the port until it is stopped; `reason` says why a plain `warp app.warp` serves, which opens
/// its page in the browser from a terminal (`warp serve` doesn't)
#[cfg(not(test))]
pub(crate) fn serve_file(path: &str, port: u16, reason: &str, open: bool) -> ! {
	let code = source_of(path);
	if !warp::serve::serves_itself(&code) {
		println!("serving http://localhost:{port} ({reason}`warp run {path}` runs it once without serving)");
	}
	diagnostic::show_lines_of(&code);
	if open && opens_browser() {
		warp::web::web_server::when_listening(|address| {
			warp::web::dev_server::open_in_browser(address);
		});
	}
	print_and_exit(warp::pipeline::serving_at(port, || eval(path)))
}
