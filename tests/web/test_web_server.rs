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

// typed RPC (notes/web_framework.md "Server and page"): `server def f(…)` is also POST /rpc/f, the arguments a JSON
// array, the result JSON
// open (card rpc-arguments): the RPC route's call of f fails (500) while f's parameters, fed only by JSON values of
// unknown kind, default to Int; to be annotated any
#[test]
#[ignore = "next"]
fn a_server_function_is_called_over_http() {
	const RPC_PORT: u16 = 18432;
	let server = std::thread::spawn(|| {
		warp::web_server::stop_after(2);
		let value = warp::wasm_emitter::eval("server def add(a, b) { a + b }\nserver greet(name) := \"hi \" + name\nserve 18432 { get \"/\" { \"ok\" } }");
		(value.first_error().is_none(), value.serialize())
	});
	let started = std::time::Instant::now();
	while std::net::TcpStream::connect(("127.0.0.1", RPC_PORT)).is_err() {
		assert!(started.elapsed() < Duration::from_secs(60), "the server did not start");
		std::thread::sleep(Duration::from_millis(50));
	}
	let call = |name: &str, arguments: &str| ureq::post(&format!("http://127.0.0.1:{RPC_PORT}/rpc/{name}")).send(arguments).expect("an answer").body_mut().read_to_string().expect("a text");
	assert_eq!(call("add", "[2, 3]"), "5");
	assert_eq!(call("greet", "[\"Ann\"]"), "hi Ann");
	let (fine, value) = server.join().expect("the server thread");
	assert!(fine, "{value}");
}
