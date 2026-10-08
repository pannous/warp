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

fn read(path: &str) -> String {
	ureq::get(&format!("http://127.0.0.1:{PORT}{path}")).call().expect("an answer").body_mut().read_to_string().expect("a text")
}

#[test]
fn a_served_route_reads_the_table_at_each_request() {
	let folder = std::path::Path::new("scratch").join("served_tables");
	std::fs::create_dir_all(&folder).expect("scratch folder");
	let _ = std::fs::remove_file(folder.join("app.database.sqlite"));
	let program = folder.join("app.warp");
	std::fs::write(&program, PROGRAM).expect("the program file");
	let served = program.clone();
	let server = std::thread::spawn(move || {
		warp::web_server::stop_after(2);
		warp::modules::with_program_file(&served, || warp::pipeline::serving_at(PORT, || warp::wasm_emitter::eval(PROGRAM))).serialize()
	});
	let started = std::time::Instant::now();
	while std::net::TcpStream::connect(("127.0.0.1", PORT)).is_err() {
		assert!(started.elapsed() < Duration::from_secs(60), "the server did not start");
		std::thread::sleep(Duration::from_millis(50));
	}
	assert!(!read("/api/users").contains("Ann"));
	warp::modules::with_program_file(&program, || warp::wasm_emitter::eval(ADDING));
	assert!(read("/api/users").contains("Ann"));
	server.join().expect("the server thread");
}
