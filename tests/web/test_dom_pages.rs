//! A program against a page's real document (card dom-tests): `warp build --site` builds it, a local server serves the
//! site, headless Chrome (agent-browser) opens it, clicks and reads the DOM back. The DOM work sits in a click handler,
//! since main also runs at build time, natively, where there is no document.
use crate::common::scratch_directory;
use std::process::Command;

const SERVER_THREADS: usize = 4;
/// a busy machine (the full suite) may keep a page from loading in agent-browser's 25 s: longer, and relaunched
const ACTION_TIMEOUT_MS: &str = "60000";
const OPEN_ATTEMPTS: usize = 3;
const CLICK_WAIT_MS: &str = "10000";
/// the DOM work's outcome lands in the title, which the page's re-rendering after the handler leaves alone
const TITLE_SET: &str = "document.title != 'dom'";
/// two browsers launched at once time out opening their pages: one page at a time
static ONE_BROWSER: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// The site's files over HTTP on a free port, until the test process ends
fn serve_directory(directory: std::path::PathBuf) -> String {
	let server = std::sync::Arc::new(tiny_http::Server::http("127.0.0.1:0").expect("a free port"));
	let address = server.server_addr().to_ip().expect("an IP address");
	for _ in 0..SERVER_THREADS {
		let (server, directory) = (server.clone(), directory.clone());
		std::thread::spawn(move || {
			for request in server.incoming_requests() {
				let name = request.url().trim_start_matches('/').split('?').next().unwrap_or_default().to_string();
				let file = directory.join(if name.is_empty() { "index.html" } else { &name });
				let _ = match std::fs::read(&file) {
					Ok(bytes) => request.respond(tiny_http::Response::from_data(bytes).with_header(content_type(&name))),
					Err(_) => request.respond(tiny_http::Response::empty(404)),
				};
			}
		});
	}
	format!("http://{address}/")
}

fn content_type(name: &str) -> tiny_http::Header {
	let kind = match name.rsplit('.').next() {
		Some("js") => "text/javascript",
		Some("wasm") => "application/wasm",
		_ => "text/html; charset=utf-8",
	};
	tiny_http::Header::from_bytes("Content-Type", kind).expect("a header")
}

/// The page of `code` opened headless, its button `button` clicked: the title the handler set
fn title_after_clicking(name: &str, code: &str, button: &str) -> String {
	let directory = scratch_directory(name);
	warp::site::build(code, "dom", &directory).unwrap_or_else(|failure| panic!("the site of {name} is not built: {failure}"));
	title_on_page(name, directory, Some(button))
}

/// The built site in `directory` opened headless, its button `button` clicked (none: as it loaded): the title the
/// program set
fn title_on_page(name: &str, directory: std::path::PathBuf, button: Option<&str>) -> String {
	let _one_browser = ONE_BROWSER.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
	let url = serve_directory(directory.clone());
	let session = format!("warp-dom-{name}-{}", std::process::id());
	let run = |arguments: &[&str]| Command::new("agent-browser").env("AGENT_BROWSER_DEFAULT_TIMEOUT", ACTION_TIMEOUT_MS).args(["--session", &session]).args(arguments).output()
		.expect("agent-browser runs (npm install -g agent-browser && agent-browser install)");
	let browser = |arguments: &[&str]| {
		let output = run(arguments);
		assert!(output.status.success(), "agent-browser {arguments:?}: {}", String::from_utf8_lossy(&output.stderr));
		String::from_utf8_lossy(&output.stdout).trim().to_string()
	};
	// the browser first, then the page: a fresh browser asked for a page at once at times stays on about:blank
	browser(&["open"]);
	// an open that succeeds can still leave the page on about:blank: it counts once the page is at the url
	let opened = (1..=OPEN_ATTEMPTS).any(|attempt| {
		let output = run(&["open", &url]);
		let address = String::from_utf8_lossy(&run(&["eval", "location.href"]).stdout).to_string();
		let arrived = output.status.success() && address.contains(&url);
		if !arrived {
			eprintln!("agent-browser open {url}, attempt {attempt} of {OPEN_ATTEMPTS}: at {address} {}", String::from_utf8_lossy(&output.stderr));
		}
		arrived
	});
	assert!(opened, "agent-browser did not open {url} in {OPEN_ATTEMPTS} attempts");
	browser(&["wait", "--load", "networkidle"]);
	if let Some(button) = button {
		browser(&["find", "text", button, "click"]);
	}
	let waited = run(&["wait", "--fn", TITLE_SET, "--timeout", CLICK_WAIT_MS]).status.success();
	let title = browser(&["eval", "document.title"]);
	let console = browser(&["console"]);
	browser(&["close"]);
	std::fs::remove_dir_all(directory).ok();
	assert!(waited, "the click set no title; the console: {console}");
	serde_json::from_str(&title).unwrap_or(title)
}

#[test]
fn a_handler_sets_and_reads_elements_of_the_page() {
	if !crate::common::chrome_runs_here(module_path!()) {
		return;
	}
	let code = "use js document\ndiv{\n\tbutton{ on click {\n\t\te = document.getElementById(\"out\")\n\t\te.innerHTML = \"<b>set</b>\"\n\t\tdocument.getElementById(\"other\").innerHTML = \"<i>chained</i>\"\n\t\tdocument.title = e.textContent + \" \" + document.getElementById(\"other\").textContent\n\t} \"Go\" }\n\tp{ id: \"out\" \"before\" }\n\tp{ id: \"other\" \"before\" }\n}";
	assert_eq!(title_after_clicking("elements", code, "Go"), "set chained");
}

#[test]
fn a_handler_draws_on_a_canvas_of_the_page() {
	if !crate::common::chrome_runs_here(module_path!()) {
		return;
	}
	let code = "use js document\ndiv{\n\tbutton{ on click {\n\t\tctx = document.getElementById(\"c\").getContext(\"2d\")\n\t\tctx.fillStyle = \"red\"\n\t\tctx.fillRect(0, 0, 4, 4)\n\t\tpixel = ctx.getImageData(1, 1, 1, 1).data\n\t\tdocument.title = \"red \" + str(pixel#1) + \" alpha \" + str(pixel#4)\n\t} \"Draw\" }\n\tcanvas{ id: \"c\" width: 4 height: 4 }\n}";
	assert_eq!(title_after_clicking("canvas", code, "Draw"), "red 255 alpha 255");
}

/// card page-dom: a main calling into the page cannot run when the site is built, natively, so the page is rendered in
/// the browser. Built by the warp binary: a site built in this process leaves its node interpreter running here, and
/// a browser this process then starts never runs the page's main (seen 2026-10-09, cause unknown)
#[test]
fn main_sets_the_title_of_the_page() {
	if !crate::common::chrome_runs_here(module_path!()) {
		return;
	}
	let folder = scratch_directory("main");
	std::fs::create_dir_all(&folder).expect("a folder");
	let program = folder.join("dom.warp");
	std::fs::write(&program, "use js document\ndocument.title = \"main ran\"\ndiv{ p{ \"shown\" } }").expect("written");
	let built = crate::common::warp_command().args(["build", "--site"]).arg(&program).output().expect("warp runs");
	let warned = String::from_utf8_lossy(&built.stderr);
	assert!(built.status.success() && warned.contains("rendered there"), "{warned}");
	assert_eq!(title_on_page("main", folder.join("dom-site"), None), "main ran");
	std::fs::remove_dir_all(folder).ok();
}
