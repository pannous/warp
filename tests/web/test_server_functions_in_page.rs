// card route-sample (notes/server_routes.md): one source holds the server and its page; a `server def` is also a
// function of the page build, which no longer fails on the word `server`
const PROGRAM: &str = "users = [\"Ann\", \"Bo\"]\nserver def user_count() { count(users) }\np{ \"users: \" + user_count() }";

#[test]
fn a_page_build_takes_a_server_function() {
	let files = warp::site::files(PROGRAM, "users", false).expect("the page builds");
	let (_, page) = files.iter().find(|(name, _)| name == "index.html").expect("index.html");
	assert!(String::from_utf8_lossy(page).contains("<p>users: 2</p>"), "{}", String::from_utf8_lossy(page));
}

// its body no longer ships in app.wasm (card rpc-everywhere: the page asks POST /rpc/user_count), so the build no
// longer warns that it does
#[test]
fn a_page_build_warns_that_a_server_function_ships() {
	warp::diagnostic::take_warnings();
	let files = warp::site::files(PROGRAM, "users", false).expect("the page builds");
	let warnings = warp::diagnostic::take_warnings();
	assert!(!warnings.iter().any(|warning| warning.message.contains("ships in its app.wasm")), "{warnings:?}");
	let (_, module) = files.iter().find(|(name, _)| name == "app.wasm").expect("app.wasm");
	assert!(module.windows("/rpc/user_count".len()).any(|window| window == b"/rpc/user_count"));
}
