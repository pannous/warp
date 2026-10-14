// card web-route-header: in a route's body `header "name" "value"` adds a header to its HTTP answer and `status 418`
// sets the status (the CGI header Status); the next request starts without them
use std::time::Duration;

const PORT: u16 = 18452;
const PROGRAM: &str = r#"serve 18452 {
	get "/teapot" { status 418; header "X-Warp" "brewed"; "short and stout" }
	get "/plain" { "fine" }
}"#;

/// status, the X-Warp header and body of GET `path`, a failing status read like any other
fn answer(path: &str) -> (u16, Option<String>, String) {
	let url = format!("http://127.0.0.1:{PORT}{path}");
	let mut reply = ureq::get(&url).config().http_status_as_error(false).build().call().expect("an answer");
	let marked = reply.headers().get("x-warp").and_then(|value| value.to_str().ok()).map(str::to_string);
	(reply.status().as_u16(), marked, reply.body_mut().read_to_string().expect("a text"))
}

#[test]
fn a_route_sets_its_status_and_headers() {
	let server = std::thread::spawn(|| {
		warp::web::web_server::stop_after(2);
		warp::wasm_emitter::eval(PROGRAM).serialize()
	});
	let started = std::time::Instant::now();
	while std::net::TcpStream::connect(("127.0.0.1", PORT)).is_err() {
		assert!(started.elapsed() < Duration::from_secs(60), "the server did not start");
		std::thread::sleep(Duration::from_millis(50));
	}
	assert_eq!(answer("/teapot"), (418, Some("brewed".into()), "short and stout".into()));
	assert_eq!(answer("/plain"), (200, None, "fine".into()));
	server.join().expect("the server thread");
}
