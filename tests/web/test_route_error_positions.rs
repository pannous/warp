//! card runtime-errors: a failing route answers its runtime error with the line and column of the statement
use std::time::Duration;

const PORT: u16 = 18661;
const PROGRAM: &str = "serve 18661 {\n\tget \"/index\" { [1 2]#5 }\n}";

#[test]
fn a_failing_route_names_its_statement() {
	let server = std::thread::spawn(|| {
		warp::web::web_server::stop_after(1);
		warp::wasm_emitter::eval(PROGRAM).serialize()
	});
	let started = std::time::Instant::now();
	while std::net::TcpStream::connect(("127.0.0.1", PORT)).is_err() {
		assert!(started.elapsed() < Duration::from_secs(60), "the server did not start");
		std::thread::sleep(Duration::from_millis(50));
	}
	let url = format!("http://127.0.0.1:{PORT}/index");
	let mut reply = ureq::get(&url).config().http_status_as_error(false).build().call().expect("an answer");
	let body = reply.body_mut().read_to_string().expect("a text");
	assert_eq!(body, "index out of range: 5 not in 1…2 at 2:17");
	server.join().expect("the server thread");
}
