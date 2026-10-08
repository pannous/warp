// Card sample-server (found serving samples/server.warp): a served route reads the table as the database has it at the
// request, not as it was when the server started; a page render (its own instance) may have added rows since
use std::time::Duration;

const PROGRAM: &str = "class User{name: text}
stored users: [User]
get \"/api/users\" { users }";
const ADDING: &str = "class User{name: text}
stored users: [User]
users.add(User(\"Ann\"))";
const PORT: u16 = 18493;
const EMPTY_PORT: u16 = 18494;
const RPC_PORT: u16 = 18495;

fn read(port: u16, path: &str) -> String {
	ureq::get(&format!("http://127.0.0.1:{port}{path}")).call().expect("an answer").body_mut().read_to_string().expect("a text")
}

/// The `program` served at `port` from scratch/<folder>/app.warp with a fresh database, for `requests` requests: the server
/// thread and the program file
fn serving(program_code: &'static str, folder: &str, port: u16, requests: usize) -> (std::thread::JoinHandle<String>, std::path::PathBuf) {
	let folder = std::path::Path::new("scratch").join(folder);
	std::fs::create_dir_all(&folder).expect("scratch folder");
	let _ = std::fs::remove_file(folder.join("app.database.sqlite"));
	let program = folder.join("app.warp");
	std::fs::write(&program, program_code).expect("the program file");
	let served = program.clone();
	let server = std::thread::spawn(move || {
		warp::web_server::stop_after(requests);
		warp::modules::with_program_file(&served, || warp::pipeline::serving_at(port, || warp::wasm_emitter::eval(program_code))).serialize()
	});
	let started = std::time::Instant::now();
	while std::net::TcpStream::connect(("127.0.0.1", port)).is_err() {
		assert!(started.elapsed() < Duration::from_secs(60), "the server did not start");
		std::thread::sleep(Duration::from_millis(50));
	}
	(server, program)
}

#[test]
fn a_served_route_reads_the_table_at_each_request() {
	let (server, program) = serving(PROGRAM, "served_tables", PORT, 2);
	assert!(!read(PORT, "/api/users").contains("Ann"));
	warp::modules::with_program_file(&program, || warp::wasm_emitter::eval(ADDING));
	assert!(read(PORT, "/api/users").contains("Ann"));
	server.join().expect("the server thread");
}

// card served-empty: an empty table is the JSON array [], not null (ø, the empty list, reads back as nothing)
#[test]
fn an_empty_table_answers_an_empty_array() {
	let (server, _) = serving(PROGRAM, "served_empty", EMPTY_PORT, 1);
	assert_eq!(read(EMPTY_PORT, "/api/users"), "[]");
	server.join().expect("the server thread");
}

// a top-level `get` is the server's alone: the page leaves it out, and with it the table it reads
#[test]
fn a_get_reading_a_table_does_not_ship_it() {
	let program = "class User{name: text}\nstored people: [User]\nget \"/api/all\" { people }\nroute \"/\" { p{ \"home\" } }\nroute \"/u/:id:int\" { h1{ people#id.name } }";
	let files = warp::site::files(program, "people_app", false).expect("the page builds");
	let module = &files.iter().find(|(name, _)| name == "app.wasm").expect("app.wasm").1;
	assert!(!module.windows("people".len()).any(|window| window == b"people"), "the table shipped in app.wasm");
}

// card server-def-giving: a server function giving an empty table answers POST /rpc/f with [] too
#[test]
fn a_server_function_giving_an_empty_table_answers_an_empty_array() {
	const GIVING: &str = "class User{name: text}\nstored users: [User]\nserver def everyone() { users }";
	let (server, _) = serving(GIVING, "served_rpc_empty", RPC_PORT, 1);
	let answer = ureq::post(&format!("http://127.0.0.1:{RPC_PORT}/rpc/everyone")).send("[]").expect("an answer").body_mut().read_to_string().expect("a text");
	assert_eq!(answer, "[]");
	server.join().expect("the server thread");
}
