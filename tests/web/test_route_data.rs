// P221 (user 2026-10-08, "both from your single line"): a route whose block reads server data (a database table) asks
// the server for what it shows: the page build sends the path to POST /rpc/route·data·N, which gives the value the
// block displays for that path (users#id for that id), so neither the table nor its other rows ship in app.wasm
use std::time::Duration;

const PROGRAM: &str = "class User{name: text}
users: [User] = database.users_p221
route \"/\" { p{ \"home\" } }
route \"/users/:id:int\" { h1{ \"User \" + users#id.name } }";

fn contains(bytes: &[u8], text: &str) -> bool {
	bytes.windows(text.len()).any(|window| window == text.as_bytes())
}

#[test]
fn a_route_reading_a_table_ships_without_it() {
	let files = warp::site::files(PROGRAM, "users", false).expect("the page builds");
	let file = |wanted: &str| files.iter().find(|(name, _)| name == wanted).unwrap_or_else(|| panic!("{wanted}")).1.clone();
	let page = String::from_utf8_lossy(&file("index.html")).into_owned();
	assert!(page.contains("<p>home</p>"), "{page}");
	let module = file("app.wasm");
	assert!(!contains(&module, "users_p221"), "the table shipped in app.wasm");
	assert!(contains(&module, "/rpc/route·data·0"));
}

#[test]
fn the_server_gives_only_what_the_route_shows() {
	const PORT: u16 = 18466;
	let folder = std::path::Path::new("scratch/route_data");
	std::fs::create_dir_all(folder).expect("scratch folder");
	let _ = std::fs::remove_file(folder.join("app.database.sqlite"));
	let program = folder.join("app.warp");
	warp::modules::with_program_file(&program, || warp::wasm_emitter::eval("class User{name: text}\nusers: [User] = database.users_p221\nusers.add(User(\"Ann\"))\nusers.add(User(\"Bo\"))"));
	let server = std::thread::spawn(move || {
		warp::web_server::stop_after(1);
		warp::modules::with_program_file(&program, || warp::pipeline::serving_at(PORT, || warp::wasm_emitter::eval(PROGRAM))).serialize()
	});
	let started = std::time::Instant::now();
	while std::net::TcpStream::connect(("127.0.0.1", PORT)).is_err() {
		assert!(started.elapsed() < Duration::from_secs(60), "the server did not start");
		std::thread::sleep(Duration::from_millis(50));
	}
	let shown = ureq::post(&format!("http://127.0.0.1:{PORT}/rpc/route%C2%B7data%C2%B70")).send("[\"/users/2\"]").expect("an answer").body_mut().read_to_string().expect("a text");
	assert_eq!(shown, "User Bo");
	server.join().expect("the server thread");
}
