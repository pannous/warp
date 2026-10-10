//! `warp deploy` (card cloud-deploy, src/deploy.rs): the program's routes as a Cloudflare Worker, run here by
//! `wrangler dev` (workerd, the Workers runtime, which runs WASM GC) and asked over HTTP
use crate::common::scratch_directory;
use std::time::{Duration, Instant};

const PROGRAM: &str = include_str!("../../samples/worker.warp");
const START_LIMIT: Duration = Duration::from_secs(90);

struct WranglerDev(std::process::Child);

impl Drop for WranglerDev {
	// TERM, not KILL: wrangler then stops its workerd too
	fn drop(&mut self) {
		let _ = std::process::Command::new("kill").arg(self.0.id().to_string()).status();
		let _ = self.0.wait();
	}
}

/// status, content type and body of GET path
fn answer(base: &str, path: &str) -> (u16, String, String) {
	let mut reply = ureq::get(&format!("{base}{path}")).config().http_status_as_error(false).build().call().expect("an answer");
	let content_type = reply.headers().get("content-type").and_then(|value| value.to_str().ok()).unwrap_or_default().to_string();
	(reply.status().as_u16(), content_type, reply.body_mut().read_to_string().expect("a text"))
}

#[test]
fn a_deployed_program_answers_its_routes() {
	let directory = scratch_directory("worker");
	let files = warp::deploy::worker_files(PROGRAM, "worker").expect("the Worker is built");
	warp::site::write_files(&files, &directory).expect("written");
	let port = std::net::TcpListener::bind("127.0.0.1:0").and_then(|listener| listener.local_addr()).expect("a free port").port();
	let wrangler = std::process::Command::new("wrangler").args(["dev", "--ip", "127.0.0.1", "--port", &port.to_string()])
		.current_dir(&directory).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn()
		.expect("wrangler runs (npm install -g wrangler)");
	let _running = WranglerDev(wrangler);
	let base = format!("http://127.0.0.1:{port}");
	let started = Instant::now();
	while ureq::get(&base).call().is_err() {
		assert!(started.elapsed() < START_LIMIT, "wrangler dev did not answer on {base}");
		std::thread::sleep(Duration::from_millis(200));
	}
	let (status, content_type, body) = answer(&base, "/");
	assert_eq!((status, body.as_str()), (200, "hello from warp"));
	assert!(content_type.starts_with("text/plain"), "{content_type}");
	let (_, content_type, body) = answer(&base, "/square/12");
	assert_eq!(body, r#"{"n":12,"square":144}"#);
	assert!(content_type.starts_with("application/json"), "{content_type}");
	// the program's variables live across requests: the readiness check above asked "/" only
	assert_eq!(answer(&base, "/visits").2, "1");
	assert_eq!(answer(&base, "/visits").2, "2");
	assert_eq!(answer(&base, "/nope"), (404, "text/plain; charset=utf-8".into(), "no route answers GET /nope".into()));
	std::fs::remove_dir_all(directory).ok();
}

#[test]
fn a_program_without_routes_is_no_worker() {
	let failure = warp::deploy::worker_files("print 1", "none").expect_err("no routes");
	assert!(failure.contains("get \"/\""), "{failure}");
}

/// The playground's Deploy (src/web.rs web_worker_bundle → warp-hosting): the module and the names of the host scripts
/// warp-hosting puts before cloud-worker.js; a program without routes is refused before anything is sent
#[test]
fn the_playground_bundles_a_program_for_warp_hosting() {
	let (report, module) = warp::web::worker_bundle(include_str!("../../samples/hosting.warp"));
	assert_eq!(&module[..4], b"\0asm");
	assert_eq!(report["module_length"], module.len());
	let scripts: Vec<&str> = report["scripts"].as_array().expect("script names").iter().filter_map(|name| name.as_str()).collect();
	assert_eq!(scripts, ["reader.js", "host.js"], "a program of routes only imports no part of host.js");
	let (refused, nothing) = warp::web::worker_bundle("1 + 2");
	assert!(refused["error"].as_str().is_some_and(|message| message.contains("answers no request")), "{refused}");
	assert!(nothing.is_empty());
}

/// The Worker of `program` run by `wrangler dev` on a free port, and its address
fn running_worker(program: &str, name: &str) -> (WranglerDev, String, std::path::PathBuf) {
	let directory = scratch_directory(name);
	let files = warp::deploy::worker_files(program, name).expect("the Worker is built");
	warp::site::write_files(&files, &directory).expect("written");
	let free_port = || std::net::TcpListener::bind("127.0.0.1:0").and_then(|listener| listener.local_addr()).expect("a free port").port().to_string();
	let port = free_port();
	// its own inspector port too: two wrangler dev at once (the test above) clash on the default 9229
	let wrangler = std::process::Command::new("wrangler").args(["dev", "--ip", "127.0.0.1", "--port", &port, "--inspector-port", &free_port()])
		.current_dir(&directory).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn()
		.expect("wrangler runs (npm install -g wrangler)");
	let base = format!("http://127.0.0.1:{port}");
	let started = Instant::now();
	while ureq::get(&base).call().is_err() {
		assert!(started.elapsed() < START_LIMIT, "wrangler dev did not answer on {base}");
		std::thread::sleep(Duration::from_millis(200));
	}
	(WranglerDev(wrangler), base, directory)
}

/// A program without routes deploys anyway (card make-deploy): its value is the page at /, each function a route
/// calling it with the path's parts; a program of functions only lists their routes at /
#[test]
fn a_program_without_routes_answers_its_value_and_functions() {
	let (_running, base, directory) = running_worker("square(n) := n*n\nadd(a, b) := a + b\ntwice := it * 2\n\"squares and sums\"", "lambda");
	assert_eq!(answer(&base, "/"), (200, "text/plain; charset=utf-8".into(), "squares and sums".into()));
	assert_eq!(answer(&base, "/square/3").2, "9");
	assert_eq!(answer(&base, "/add/2/40").2, "42");
	assert_eq!(answer(&base, "/twice/21").2, "42");
	assert_eq!(answer(&base, "/nope").0, 404);
	std::fs::remove_dir_all(directory).ok();
	let (_running, base, directory) = running_worker("square(n) := n*n", "functions-only");
	assert_eq!(answer(&base, "/").2, r#"["/square/:n"]"#);
	assert_eq!(answer(&base, "/square/12").2, "144");
	std::fs::remove_dir_all(directory).ok();
}
