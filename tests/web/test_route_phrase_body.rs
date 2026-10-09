// card first-first, found serving `get "/api/newest" { last 2 of users }` (samples/server.warp): a one-line route block
// of words reading a table is one statement, not each word one ("undefined variable: last")
use std::io::{Read, Write};
use std::time::Duration;

const PORT: u16 = 18500;
const PROGRAM: &str = "class User{name: text}
stored users: [User]
if count(users) == 0 { users.add(User(\"Ann\")); users.add(User(\"Bo\")); users.add(User(\"Cy\")) }
get \"/api/newest\" { last 2 of users }
get \"/api/users/:id:int\" { users#id.name }";

/// The body of GET path
fn fetched(path: &str) -> String {
	let mut stream = std::net::TcpStream::connect(("127.0.0.1", PORT)).expect("the server");
	write!(stream, "GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n").expect("the request");
	let mut answer = String::new();
	stream.read_to_string(&mut answer).expect("the answer");
	answer.split_once("\r\n\r\n").map_or("", |(_, body)| body).to_string()
}

#[test]
fn a_one_line_route_block_reading_a_table_is_one_statement() {
	let folder = std::path::Path::new("scratch").join("route_phrase_body");
	std::fs::create_dir_all(&folder).expect("scratch folder");
	let _ = std::fs::remove_file(folder.join("app.database.sqlite"));
	let program = folder.join("app.warp");
	std::fs::write(&program, PROGRAM).expect("the program file");
	let server = std::thread::spawn(move || {
		warp::web_server::stop_after(2);
		warp::modules::with_program_file(&program, || warp::pipeline::serving_at(PORT, || warp::wasm_emitter::eval(PROGRAM))).serialize()
	});
	let started = std::time::Instant::now();
	while std::net::TcpStream::connect(("127.0.0.1", PORT)).is_err() {
		assert!(started.elapsed() < Duration::from_secs(60), "the server did not start");
		std::thread::sleep(Duration::from_millis(50));
	}
	assert_eq!(fetched("/api/newest"), r#"[{"name":"Bo","id":2},{"name":"Cy","id":3}]"#);
	assert_eq!(fetched("/api/users/3"), "Cy");
	server.join().expect("the server thread");
}
