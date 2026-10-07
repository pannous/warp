// card web-server: `serve 8080 { get "/api/users" { users } post "/echo" { request.body } }` answers HTTP natively: a
// text as text/plain, any other value as JSON; the test stops the server after its requests (web_server::stop_after)
use std::time::Duration;

const PORT: u16 = 18431;
const PROGRAM: &str = r#"users = [{name: "Ann", age: 31}, {name: "Bo", age: 7}]
serve 18431 {
	get "/api/users" { users }
	post "/echo" { request.body }
	get "/hello" { "hello " + request.query.name }
}"#;

fn get(path: &str) -> String {
	ureq::get(&format!("http://127.0.0.1:{PORT}{path}")).call().expect("an answer").body_mut().read_to_string().expect("a text")
}

#[test]
fn serve_answers_routes() {
	let server = std::thread::spawn(|| {
		warp::web_server::stop_after(4);
		let value = warp::wasm_emitter::eval(PROGRAM);
		(value.first_error().is_none(), value.serialize())
	});
	let started = std::time::Instant::now();
	while std::net::TcpStream::connect(("127.0.0.1", PORT)).is_err() {
		assert!(started.elapsed() < Duration::from_secs(60), "the server did not start");
		std::thread::sleep(Duration::from_millis(50));
	}
	assert_eq!(get("/api/users"), r#"[{"name":"Ann","age":31},{"name":"Bo","age":7}]"#);
	let echoed = ureq::post(&format!("http://127.0.0.1:{PORT}/echo")).send("hello wasp").expect("an answer").body_mut().read_to_string().expect("a text");
	assert_eq!(echoed, "hello wasp");
	assert_eq!(get("/hello?name=Ann"), "hello Ann");
	let missing = ureq::get(&format!("http://127.0.0.1:{PORT}/nowhere")).call();
	assert!(matches!(missing, Err(ureq::Error::StatusCode(404))), "{missing:?}");
	let (fine, value) = server.join().expect("the server thread");
	assert!(fine, "{value}");
}
