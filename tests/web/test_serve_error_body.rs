// card serve-error-body: a route that fails answers 500 with the error's message (plain text; JSON {"error": …} for
// /api/ and /rpc/ routes), the backtrace goes only to the server's log; the server keeps answering
use std::time::Duration;

const PORT: u16 = 18496;
const PROGRAM: &str = r#"server def broken(x) { raise "no " + x }
serve 18496 {
	get "/fail" { raise "boom" }
	get "/index" { [1 2]#5 }
	get "/api/fail" { raise "boom" }
	get "/ok" { "fine" }
}"#;

/// status, content type and body of a request, a failing status read like any other
fn answer(method: &str, path: &str, body: &str) -> (u16, String, String) {
	let url = format!("http://127.0.0.1:{PORT}{path}");
	let reply = match method {
		"POST" => ureq::post(&url).config().http_status_as_error(false).build().send(body),
		_ => ureq::get(&url).config().http_status_as_error(false).build().call(),
	};
	let mut reply = reply.expect("an answer");
	let content_type = reply.headers().get("content-type").and_then(|value| value.to_str().ok()).unwrap_or_default().to_string();
	(reply.status().as_u16(), content_type, reply.body_mut().read_to_string().expect("a text"))
}

#[test]
fn a_failing_route_answers_its_message() {
	let server = std::thread::spawn(|| {
		warp::web_server::stop_after(5);
		warp::wasm_emitter::eval(PROGRAM).serialize()
	});
	let started = std::time::Instant::now();
	while std::net::TcpStream::connect(("127.0.0.1", PORT)).is_err() {
		assert!(started.elapsed() < Duration::from_secs(60), "the server did not start");
		std::thread::sleep(Duration::from_millis(50));
	}
	let (status, content_type, body) = answer("GET", "/fail", "");
	assert_eq!((status, body.as_str()), (500, "boom"));
	assert!(content_type.starts_with("text/plain"), "{content_type}");
	assert_eq!(answer("GET", "/index", "").2, "index out of range: 5 not in 1…2");
	let (status, content_type, body) = answer("GET", "/api/fail", "");
	assert_eq!((status, content_type.as_str(), body.as_str()), (500, "application/json", r#"{"error":"boom"}"#));
	let (status, _, body) = answer("POST", "/rpc/broken", "[\"luck\"]");
	assert_eq!((status, body.as_str()), (500, r#"{"error":"no luck"}"#));
	assert_eq!(answer("GET", "/ok", "").2, "fine");
	server.join().expect("the server thread");
}
