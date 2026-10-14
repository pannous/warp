//! `warp deploy` (card cloud-deploy, src/web/deploy.rs): the program's routes as a Cloudflare Worker, run here by
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
	let mut reply =
		ureq::get(&format!("{base}{path}")).config().http_status_as_error(false).build().call().expect("an answer");
	let content_type =
		reply.headers().get("content-type").and_then(|value| value.to_str().ok()).unwrap_or_default().to_string();
	(reply.status().as_u16(), content_type, reply.body_mut().read_to_string().expect("a text"))
}

#[test]
fn a_deployed_program_answers_its_routes() {
	let directory = scratch_directory("worker");
	let files = warp::web::deploy::worker_files(PROGRAM, "worker").expect("the Worker is built");
	warp::web::site::write_files(&files, &directory).expect("written");
	let port = std::net::TcpListener::bind("127.0.0.1:0")
		.and_then(|listener| listener.local_addr())
		.expect("a free port")
		.port();
	let wrangler = std::process::Command::new("wrangler")
		.args(["dev", "--ip", "127.0.0.1", "--port", &port.to_string()])
		.current_dir(&directory)
		.stdout(std::process::Stdio::null())
		.stderr(std::process::Stdio::null())
		.spawn()
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
	let failure = warp::web::deploy::worker_files("x = 1", "none").expect_err("no routes");
	assert!(failure.contains("get \"/\""), "{failure}");
}

/// The playground's Deploy (src/web/mod.rs web_worker_bundle → warp-hosting): the module and the names of the host scripts
/// warp-hosting puts before cloud-worker.js; a program answering nothing (no routes, functions or value) is refused before
/// anything is sent
#[test]
fn the_playground_bundles_a_program_for_warp_hosting() {
	let (report, module) = warp::web::worker_bundle(include_str!("../../samples/hosting.warp"));
	assert_eq!(&module[..4], b"\0asm");
	assert_eq!(report["module_length"], module.len());
	let scripts: Vec<&str> =
		report["scripts"].as_array().expect("script names").iter().filter_map(|name| name.as_str()).collect();
	assert_eq!(scripts, ["reader.js", "host.js"], "a program of routes only imports no part of host.js");
	let (refused, nothing) = warp::web::worker_bundle("x = 1");
	assert!(refused["error"].as_str().is_some_and(|message| message.contains("answers no request")), "{refused}");
	assert!(nothing.is_empty());
}

/// The Worker of `program` run by `wrangler dev` on a free port, and its address
fn running_worker(program: &str, name: &str) -> (WranglerDev, String, std::path::PathBuf) {
	let directory = scratch_directory(name);
	let files = warp::web::deploy::worker_files(program, name).expect("the Worker is built");
	warp::web::site::write_files(&files, &directory).expect("written");
	let free_port = || {
		std::net::TcpListener::bind("127.0.0.1:0")
			.and_then(|listener| listener.local_addr())
			.expect("a free port")
			.port()
			.to_string()
	};
	let port = free_port();
	// its own inspector port too: two wrangler dev at once (the test above) clash on the default 9229
	let wrangler = std::process::Command::new("wrangler")
		.args(["dev", "--ip", "127.0.0.1", "--port", &port, "--inspector-port", &free_port()])
		.current_dir(&directory)
		.stdout(std::process::Stdio::null())
		.stderr(std::process::Stdio::null())
		.spawn()
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
	let (_running, base, directory) =
		running_worker("square(n) := n*n\nadd(a, b) := a + b\ntwice := it * 2\n\"squares and sums\"", "lambda");
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

/// A program painting, drawing or listening for frames runs in a browser page, not in a Worker (card
/// deploy-browser-site): the playground's Deploy bundles it as a site, which warp-hosting serves
#[test]
fn deploy_names_a_browser_only_feature() {
	let report = |code: &str| warp::web::worker_bundle(code).0;
	let painting = report(include_str!("../../samples/filled_circle.warp"));
	assert_eq!(painting["site"], true, "{painting}");
	let query = painting["query"].as_str().unwrap_or_default();
	assert!(
		query.starts_with("site=") && query.contains("canvas.js") && query.contains("&worker=reader.js"),
		"{query}"
	);
	let framing = report(include_str!("../../samples/shader_trails.warp"));
	assert_eq!(framing["site"], true, "{framing}");
	let routes = report(include_str!("../../samples/hosting.warp"));
	assert_eq!((&routes["site"], &routes["query"]), (&false.into(), &"scripts=reader.js,host.js".into()), "{routes}");
}

/// A program using the browser deploys as its site (card deploy-browser-site): the Worker serves the page, its scripts
/// and app.wasm, cross-origin isolated, and the visitor's browser runs the program (painting checked in CI)
#[test]
fn a_painting_program_deploys_as_its_site() {
	let (_running, base, directory) = running_worker(include_str!("../../samples/filled_circle.warp"), "painting");
	let (status, content_type, page) = answer(&base, "/");
	assert_eq!((status, content_type.as_str()), (200, "text/html; charset=utf-8"), "{page}");
	assert!(
		page.contains("<script src=\"canvas.js\">") && page.contains("data-warp-worker=\"reader.js,host.js"),
		"{page}"
	);
	let (status, content_type, _) = answer(&base, "/canvas.js");
	assert_eq!((status, content_type.as_str()), (200, "text/javascript; charset=utf-8"));
	let module = ureq::get(&format!("{base}/app.wasm")).call().expect("the module");
	assert_eq!(
		module.headers().get("cross-origin-embedder-policy").map(|value| value.to_str().unwrap()),
		Some("require-corp")
	);
	assert_eq!(&module.into_body().read_to_vec().expect("bytes")[..4], b"\0asm");
	assert_eq!(answer(&base, "/nope.js").0, 404);
	std::fs::remove_dir_all(directory).ok();
}
