// P222 (user 2026-10-08): `warp serve [app.warp] [port]` serves any program (its page, its `server def`s as POST
// /rpc/f, its top-level `get`/`post` routes) without a `serve PORT {…}` statement; plain `warp app.warp` serves too when
// the program is obviously a server (serve::serves, a static check)
use std::time::Duration;

fn served(port: u16, requests: usize, code: &'static str) -> std::thread::JoinHandle<String> {
	let server = std::thread::spawn(move || {
		warp::web_server::stop_after(requests);
		warp::pipeline::serving_at(port, || warp::wasm_emitter::eval(code)).serialize()
	});
	let started = std::time::Instant::now();
	while std::net::TcpStream::connect(("127.0.0.1", port)).is_err() {
		assert!(started.elapsed() < Duration::from_secs(60), "the server did not start");
		std::thread::sleep(Duration::from_millis(50));
	}
	server
}

#[test]
fn a_program_without_serve_is_served_with_its_server_functions_and_routes() {
	const PORT: u16 = 18464;
	let server = served(PORT, 2, "server def add(a, b) { a + b }\nget \"/api/hi\" { \"hi\" }");
	let added = ureq::post(&format!("http://127.0.0.1:{PORT}/rpc/add")).send("[2, 3]").expect("an answer").body_mut().read_to_string().expect("a text");
	assert_eq!(added, "5");
	let hi = ureq::get(&format!("http://127.0.0.1:{PORT}/api/hi")).call().expect("an answer").body_mut().read_to_string().expect("a text");
	assert_eq!(hi, "hi");
	server.join().expect("the server thread");
}

#[test]
fn a_server_is_known_without_running_it() {
	assert!(warp::serve::serves("server def f() { 1 }"));
	assert!(warp::serve::serves("route \"/\" { p{ \"home\" } }"));
	assert!(warp::serve::serves("get \"/api\" { 1 }"));
	assert!(warp::serve::serves("x = 1\nserve 8080 { get \"/\" { x } }"));
	assert!(!warp::serve::serves("p{ \"just a page\" }"));
	assert!(!warp::serve::serves("get = 3\nget + 1"));
}
