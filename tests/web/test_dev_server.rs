// card web-dev: `warp dev app.warp` serves the program's site from memory; /warp-dev/state tells the page (dev.js) the
// build's version and its failure, which the page shows as an overlay; a change of the file makes a new version
use std::time::Duration;

const PORT: u16 = 18439; // not 18432: test_web_server's RPC server listens there, and parallel tests met each other's server
const FOLDER: &str = "scratch/dev_server";

fn get(path: &str) -> String {
	ureq::get(&format!("http://127.0.0.1:{PORT}{path}"))
		.call()
		.expect("an answer")
		.body_mut()
		.read_to_string()
		.expect("a text")
}

fn state() -> serde_json::Value {
	serde_json::from_str(&get("/warp-dev/state")).expect("json")
}

#[test]
fn the_dev_server_rebuilds_a_changed_program() {
	std::fs::create_dir_all(FOLDER).expect("scratch folder");
	let program = std::path::Path::new(FOLDER).join("app.warp");
	std::fs::write(&program, "p{ \"one\" }").expect("the program");
	let served = program.clone();
	std::thread::spawn(move || {
		warp::web::web_server::stop_after(6);
		warp::web::dev_server::serve(&served, PORT)
	});
	let started = std::time::Instant::now();
	while std::net::TcpStream::connect(("127.0.0.1", PORT)).is_err() {
		assert!(started.elapsed() < Duration::from_secs(60), "the dev server did not start");
		std::thread::sleep(Duration::from_millis(50));
	}
	let page = get("/");
	assert!(page.contains("<p>one</p>") && page.contains("<script src=\"dev.js\">"), "{page}");
	assert!(get("/dev.js").contains("warp-dev/state"));
	let first = state();
	assert!(first["error"].is_null(), "{first}");

	std::fs::write(&program, "p{ \"two\" }").expect("the changed program");
	let changed = state();
	assert!(changed["version"].as_u64() > first["version"].as_u64(), "{changed}");
	assert!(get("/").contains("<p>two</p>"));

	std::fs::write(&program, "x = 1\np{ \"three\" + y }").expect("a failing program");
	let failed = state();
	let error = failed["error"].as_str().expect("the failure");
	assert!(error.contains("y") && error.contains("2 | p{ \"three\" + y }"), "{error}");
}

// a dev build keeps each main-level variable the program changes (as `stored` does, in the page's sessionStorage), so
// a reload after an edit shows the same state; one never changed takes its value from the edited source
#[test]
fn a_dev_build_keeps_the_values_the_program_changed() {
	let run = |code: &str| warp::pipeline::for_dev(|| warp::wasm_emitter::eval(code)).serialize();
	assert_eq!(run("dev_count = 0\ndev_count += 1\ndev_count"), "1");
	assert_eq!(run("dev_count = 0\ndev_count += 1\ndev_count"), "2");
	assert_eq!(run("dev_start = 5\ndev_start"), "5");
	assert_eq!(run("dev_start = 6\ndev_start"), "6");
	assert_eq!(run("dev_count = 0\ndev_count"), "0", "a variable the program no longer changes takes its new value");
}

// a dev build keeps each component instance's state too (card web-dev-instances): after a reload the second counter
// still shows its clicks, the first its own
#[test]
#[cfg(feature = "native")]
fn a_dev_build_keeps_component_state() {
	let code = "def DevTally(name) {\n\ttally = 0\n\tdiv{ button{ on click { tally += 1 } name } p{ name + \" \" + tally } }\n}\ndiv{ DevTally(\"a\") DevTally(\"b\") }";
	let reload = || warp::pipeline::for_dev(|| warp::web::headless::Page::render(code)).expect("the page");
	let mut page = reload();
	page.click("b").expect("clicked");
	page.click("b").expect("clicked");
	let reloaded = reload();
	let text = reloaded.text();
	assert!(text.contains("a 0") && text.contains("b 2"), "{}", reloaded.html());
}

// card web-dev-modules: a change of a module the program uses builds anew too, and a failure in the module shows the
// module's file and line, not the program's
#[test]
fn the_dev_server_watches_the_modules_used() {
	const MODULES_PORT: u16 = 18441;
	const MODULES_FOLDER: &str = "scratch/dev_modules";
	std::fs::create_dir_all(MODULES_FOLDER).expect("scratch folder");
	let folder = std::path::Path::new(MODULES_FOLDER);
	let (program, module) = (folder.join("app.warp"), folder.join("dev_greeting.warp"));
	std::fs::write(&module, "def greeting(){ \"one\" }").expect("the module");
	std::fs::write(&program, "use dev_greeting\np{ greeting() }").expect("the program");
	std::thread::spawn(move || {
		warp::web::web_server::stop_after(5);
		warp::web::dev_server::serve(&program, MODULES_PORT)
	});
	let get = |path: &str| {
		ureq::get(&format!("http://127.0.0.1:{MODULES_PORT}{path}"))
			.call()
			.expect("an answer")
			.body_mut()
			.read_to_string()
			.expect("a text")
	};
	let state = || serde_json::from_str::<serde_json::Value>(&get("/warp-dev/state")).expect("json");
	let started = std::time::Instant::now();
	while std::net::TcpStream::connect(("127.0.0.1", MODULES_PORT)).is_err() {
		assert!(started.elapsed() < Duration::from_secs(60), "the dev server did not start");
		std::thread::sleep(Duration::from_millis(50));
	}
	assert!(get("/").contains("<p>one</p>"));
	let first = state();

	std::fs::write(&module, "def greeting(){ \"two\" }").expect("the changed module");
	let changed = state();
	assert!(changed["version"].as_u64() > first["version"].as_u64(), "{changed}");
	assert!(get("/").contains("<p>two</p>"));

	std::fs::write(&module, "x = 1\ndef greeting(){ (\"three\" }").expect("a failing module");
	let error = state()["error"].as_str().expect("the failure").to_string();
	assert!(error.contains("dev_greeting.warp") && error.contains("2 | def greeting(){ (\"three\" }"), "{error}");
}

// card dev-opens-browser: `warp dev` opens its page in the default browser when run from a terminal; piped, as here,
// and under WARP_NO_WINDOW (tests, probes) it only prints the address
#[test]
#[cfg(feature = "native")]
fn warp_dev_opens_no_browser_when_piped() {
	use std::io::{BufRead, Read};
	// a port no other test or program holds: the system's free one, released for warp dev to take
	let port = std::net::TcpListener::bind(("127.0.0.1", 0))
		.and_then(|listener| listener.local_addr())
		.expect("a free port")
		.port();
	std::fs::create_dir_all(FOLDER).expect("scratch folder");
	let program = std::path::Path::new(FOLDER).join("opened.warp");
	std::fs::write(&program, "p{ \"hi\" }").expect("the program");
	let mut dev = crate::common::warp_command()
		.args(["dev", &program.to_string_lossy(), &port.to_string()])
		.stdout(std::process::Stdio::piped())
		.stderr(std::process::Stdio::piped())
		.spawn()
		.expect("warp dev");
	let mut first_line = String::new();
	std::io::BufReader::new(dev.stdout.take().expect("its output")).read_line(&mut first_line).expect("its first line");
	let _ = dev.kill();
	let _ = dev.wait();
	let mut failure = String::new();
	let _ = dev.stderr.take().expect("its errors").read_to_string(&mut failure);
	assert!(
		first_line.contains(&format!("http://localhost:{port}/")) && !first_line.contains("browser"),
		"first line {first_line:?}, errors {failure:?}"
	);
}
