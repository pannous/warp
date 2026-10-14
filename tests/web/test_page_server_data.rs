// card web-page-server-data: a main-level statement that only changes server data (seeding a table) runs on the
// server, which runs main; the shipped page leaves it out, with the table it reads, instead of opening a table of its
// own in the browser ("table.open: no such word in the browser" before)
const PROGRAM: &str = "class User{name: text}
stored users: [User] in \"users_seeded\"
if count(users) == 0 { users.add(User(\"Ann\")); users.add(User(\"Bo\")) }
route \"/\" { p{ \"home\" } }
route \"/users/:id:int\" { h1{ \"User \" + users#id.name } }";

fn contains(bytes: &[u8], text: &str) -> bool {
	bytes.windows(text.len()).any(|window| window == text.as_bytes())
}

#[test]
fn a_page_leaves_the_seeding_of_server_data_to_the_server() {
	let folder = std::path::Path::new("scratch").join("page_server_data");
	std::fs::create_dir_all(&folder).expect("scratch folder");
	let files =
		warp::modules::with_program_file(&folder.join("app.warp"), || warp::web::site::files(PROGRAM, "users", false))
			.expect("the page builds");
	let module = &files.iter().find(|(name, _)| name == "app.wasm").expect("app.wasm").1;
	assert!(!contains(module, "users_seeded"), "the table shipped in app.wasm");
	let std_modules = warp::host::host_parts::std_modules_of(module).expect("the std modules");
	assert!(!std_modules.contains(&"table".to_string()), "{std_modules:?}");
	assert!(contains(module, "/rpc/route·data·0"));
}
