// card web-dev: `warp dev app.wasp` serves the program's site from memory; /wasp-dev/state tells the page (dev.js) the
// build's version and its failure, which the page shows as an overlay; a change of the file makes a new version
use std::time::Duration;

const PORT: u16 = 18432;
const FOLDER: &str = "scratch/dev_server";

fn get(path: &str) -> String {
	ureq::get(&format!("http://127.0.0.1:{PORT}{path}")).call().expect("an answer").body_mut().read_to_string().expect("a text")
}

fn state() -> serde_json::Value {
	serde_json::from_str(&get("/wasp-dev/state")).expect("json")
}

#[test]
fn the_dev_server_rebuilds_a_changed_program() {
	std::fs::create_dir_all(FOLDER).expect("scratch folder");
	let program = std::path::Path::new(FOLDER).join("app.wasp");
	std::fs::write(&program, "p{ \"one\" }").expect("the program");
	let served = program.clone();
	std::thread::spawn(move || {
		warp::web_server::stop_after(6);
		warp::dev_server::serve(&served, PORT)
	});
	let started = std::time::Instant::now();
	while std::net::TcpStream::connect(("127.0.0.1", PORT)).is_err() {
		assert!(started.elapsed() < Duration::from_secs(60), "the dev server did not start");
		std::thread::sleep(Duration::from_millis(50));
	}
	let page = get("/");
	assert!(page.contains("<p>one</p>") && page.contains("<script src=\"dev.js\">"), "{page}");
	assert!(get("/dev.js").contains("wasp-dev/state"));
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
