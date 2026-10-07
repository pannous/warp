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
// a server function's unannotated parameters take any value (card rpc-arguments)
#[test]
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

// a program serving its own page (card web-ssr, notes/web_framework.md "Built sites"): GET / is the page rendered by the
// program, the other files of its site (app.wasm, the scripts) beside it, its routes as before
#[test]
fn a_program_serves_its_own_page() {
	const PAGE_PORT: u16 = 18433;
	let server = std::thread::spawn(|| {
		warp::web_server::stop_after(4);
		let value = warp::wasm_emitter::eval("tests/fixtures/served_page.wasp");
		(value.first_error().is_none(), value.serialize())
	});
	let started = std::time::Instant::now();
	while std::net::TcpStream::connect(("127.0.0.1", PAGE_PORT)).is_err() {
		assert!(started.elapsed() < Duration::from_secs(60), "the server did not start");
		std::thread::sleep(Duration::from_millis(50));
	}
	let fetched = |path: &str| ureq::get(&format!("http://127.0.0.1:{PAGE_PORT}{path}")).call().expect("an answer");
	let page = fetched("/").body_mut().read_to_string().expect("a text");
	assert!(page.contains("<ul><li>users: 2</li></ul>") && page.contains("<script src=\"site.js\"></script>"), "{page}");
	let mut module = fetched("/app.wasm");
	assert_eq!(module.headers().get("content-type").and_then(|value| value.to_str().ok()), Some("application/wasm"));
	assert!(module.body_mut().read_to_vec().expect("bytes").starts_with(b"\0asm"));
	assert!(fetched("/site.js").body_mut().read_to_string().expect("a text").contains("hydrate"));
	assert_eq!(get_from(PAGE_PORT, "/api/users"), r#"["Ann","Bo"]"#);
	let (fine, value) = server.join().expect("the server thread");
	assert!(fine, "{value}");
}

fn get_from(port: u16, path: &str) -> String {
	ureq::get(&format!("http://127.0.0.1:{port}{path}")).call().expect("an answer").body_mut().read_to_string().expect("a text")
}

// its page's events come from the page it serves: no "a native run never raises it" warning
#[test]
fn a_served_page_s_events_draw_no_warning() {
	warp::diagnostic::take_warnings();
	let lowered = warp::pipeline::lower("clicks = 0\nserve 18434 { get \"/n\" { clicks } }\ndiv{ button{ on click { clicks += 1 } \"+\" } }");
	assert!(lowered.is_ok(), "{lowered:?}");
	let warnings = warp::diagnostic::take_warnings();
	assert!(warnings.iter().all(|warning| !warning.message.contains("never raises it")), "{warnings:?}");
}

// card serve-route: a served program with routes answers each path with that route's page, rendered for the path
#[test]
fn a_served_page_is_rendered_for_each_route() {
	const ROUTES_PORT: u16 = 18441;
	let server = std::thread::spawn(|| {
		warp::web_server::stop_after(4);
		let value = warp::wasm_emitter::eval("tests/fixtures/served_routes.wasp");
		(value.first_error().is_none(), value.serialize())
	});
	let started = std::time::Instant::now();
	while std::net::TcpStream::connect(("127.0.0.1", ROUTES_PORT)).is_err() {
		assert!(started.elapsed() < Duration::from_secs(60), "the server did not start");
		std::thread::sleep(Duration::from_millis(50));
	}
	let page = |path: &str| get_from(ROUTES_PORT, path);
	assert!(page("/").contains("<h1>Users</h1>"), "{}", page("/"));
	let user = page("/users/2");
	// its scripts, module and route modules come from the server's root (its <base>), not from /users/
	assert!(user.contains("<p>User Bo</p>") && user.contains("<base href=\"/\">") && user.contains("<script src=\"site.js\"></script>"), "{user}");
	assert!(page("/elsewhere").contains("<p>no such page</p>"));
	assert_eq!(page("/api/users"), r#"["Ann","Bo"]"#);
	let (fine, value) = server.join().expect("the server thread");
	assert!(fine, "{value}");
}
