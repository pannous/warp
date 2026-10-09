// card missing-row-404: a served page whose route looks up a row that is not there (`users#id` of /users/9) answers
// 404 with the readable error, not 500 "trap detail: 9:3" (found editing samples/server.warp)
use std::io::{Read, Write};
use std::time::Duration;

const PORT: u16 = 18497;
const PROGRAM: &str = "class User{name: text}
users: [User] = database.users_missing
route \"/users/:id:int\" { h1{ \"User \" + users#id.name } }";

/// The status line and body of GET path
fn fetched(path: &str) -> String {
	let mut stream = std::net::TcpStream::connect(("127.0.0.1", PORT)).expect("the server");
	write!(stream, "GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n").expect("the request");
	let mut answer = String::new();
	stream.read_to_string(&mut answer).expect("the answer");
	answer
}

#[test]
fn a_missing_row_is_not_found() {
	let folder = std::path::Path::new("scratch").join("missing_row");
	std::fs::create_dir_all(&folder).expect("scratch folder");
	let _ = std::fs::remove_file(folder.join("app.database.sqlite"));
	let program = folder.join("app.warp");
	warp::modules::with_program_file(&program, || warp::wasm_emitter::eval("class User{name: text}\nusers: [User] = database.users_missing\nusers.add(User(\"Ann\"))"));
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
	let missing = fetched("/users/9");
	assert!(missing.starts_with("HTTP/1.1 404") && missing.contains("index out of range: 9 not in 1…1"), "{missing}");
	let found = fetched("/users/1");
	assert!(found.starts_with("HTTP/1.1 200") && found.contains("<h1>User Ann</h1>"), "{found}");
	server.join().expect("the server thread");
}
