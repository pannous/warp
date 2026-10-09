// cards server-path and route-star and the plain-JSON default (supervisor 2026-10-09), found editing samples/server.warp:
// a get route's path takes typed parameters as a page route's; an /api/ path no route takes answers JSON 404; instances
// are plain JSON objects. A page path answers the one page (user 2026-10-07, single page), `route "*"` included, and
// 404 only when no route of the page matches it
use std::io::{Read, Write};
use std::time::Duration;

const PORT: u16 = 18498;
const UNCAUGHT_PORT: u16 = 18499;
const PROGRAM: &str = "class Team{name: text}
class User{name: text, team: Team}
red = Team(\"red\")
users = [User(\"Ann\", red), User(\"Bo\", red)]
get \"/api/users\" { users }
get \"/api/users/:id:int\" { users#id }
route \"/\" { h1{ \"Users\" } }
route \"/users/:id:int\" { p{ \"User \" + users#id.name } }
route \"*\" { p{ \"no such page\" } }";
const WITHOUT_CATCH_ALL: &str = "users = [\"Ann\"]
route \"/\" { h1{ \"Users\" } }
route \"/users/:id:int\" { p{ \"User \" + users#id } }";

/// `program` served at `port` from scratch/<folder>/app.warp (its page is the file's) for `requests` requests
fn serving(program: &'static str, folder: &str, port: u16, requests: usize) -> std::thread::JoinHandle<String> {
	let folder = std::path::Path::new("scratch").join(folder);
	std::fs::create_dir_all(&folder).expect("scratch folder");
	let file = folder.join("app.warp");
	std::fs::write(&file, program).expect("the program file");
	let server = std::thread::spawn(move || {
		warp::web_server::stop_after(requests);
		warp::modules::with_program_file(&file, || warp::pipeline::serving_at(port, || warp::wasm_emitter::eval(program))).serialize()
	});
	let started = std::time::Instant::now();
	while std::net::TcpStream::connect(("127.0.0.1", port)).is_err() {
		assert!(started.elapsed() < Duration::from_secs(60), "the server did not start");
		std::thread::sleep(Duration::from_millis(50));
	}
	server
}

/// The status code and body of GET path
fn fetched(port: u16, path: &str) -> (u16, String) {
	let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).expect("the server");
	write!(stream, "GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n").expect("the request");
	let mut answer = String::new();
	stream.read_to_string(&mut answer).expect("the answer");
	let status = answer.split(' ').nth(1).and_then(|code| code.parse().ok()).expect("a status line");
	let body = answer.split_once("\r\n\r\n").map_or("", |(_, body)| body).to_string();
	(status, body)
}

#[test]
fn a_get_route_takes_typed_parameters_and_answers_plain_objects() {
	let server = serving(PROGRAM, "route_paths", PORT, 6);
	let get = |path| fetched(PORT, path);
	assert_eq!(get("/api/users/2"), (200, r#"{"name":"Bo","team":{"name":"red"}}"#.to_string()));
	assert_eq!(get("/api/users"), (200, r#"[{"name":"Ann","team":{"name":"red"}},{"name":"Bo","team":{"name":"red"}}]"#.to_string()));
	assert_eq!(get("/api/users/abc"), (404, r#"{"error":"no route GET /api/users/abc"}"#.to_string()));
	// the one page, whose router shows `route "*"` in the browser
	let (status, page) = get("/users/abc");
	assert!(status == 200 && page.contains("<base href=\"/\">"), "{status} {page}");
	assert_eq!(get("/elsewhere").0, 200);
	assert_eq!(get("/users/1").0, 200);
	server.join().expect("the server thread");
}

#[test]
fn a_page_path_no_route_matches_is_not_found() {
	let server = serving(WITHOUT_CATCH_ALL, "uncaught_paths", UNCAUGHT_PORT, 3);
	assert_eq!(fetched(UNCAUGHT_PORT, "/users/abc").0, 404);
	assert_eq!(fetched(UNCAUGHT_PORT, "/elsewhere").0, 404);
	assert_eq!(fetched(UNCAUGHT_PORT, "/users/1").0, 200);
	server.join().expect("the server thread");
}
