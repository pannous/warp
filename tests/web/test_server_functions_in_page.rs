// card route-sample (notes/server_routes.md): one source holds the server and its page; a `server def` is also a
// function of the page build, which no longer fails on the word `server`
const PROGRAM: &str = "users = [\"Ann\", \"Bo\"]\nserver def user_count() { count(users) }\np{ \"users: \" + user_count() }";

#[test]
fn a_page_build_takes_a_server_function() {
	let files = warp::site::files(PROGRAM, "users", false).expect("the page builds");
	let (_, page) = files.iter().find(|(name, _)| name == "index.html").expect("index.html");
	assert!(String::from_utf8_lossy(page).contains("<p>users: 2</p>"), "{}", String::from_utf8_lossy(page));
}
