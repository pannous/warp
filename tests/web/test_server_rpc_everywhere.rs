// card rpc-everywhere (notes/server_routes.md): every call of a server function in the page asks the server (POST
// /rpc/f, fetched anew when its arguments change), so no server body ships in app.wasm; the first HTML holds the
// values the server's code gave at build time
const SECRET: &str = "s3cret-token";

fn site(program: &str) -> (String, Vec<u8>) {
	let files = warp::site::files(program, "rpc", false).expect("the page builds");
	let file = |wanted: &str| files.iter().find(|(name, _)| name == wanted).unwrap_or_else(|| panic!("{wanted}")).1.clone();
	(String::from_utf8_lossy(&file("index.html")).into_owned(), file("app.wasm"))
}

fn contains(bytes: &[u8], text: &str) -> bool {
	bytes.windows(text.len()).any(|window| window == text.as_bytes())
}

#[test]
fn a_call_in_markup_asks_the_server() {
	let (page, module) = site("server def motto() { token = \"s3cret-token\"; \"motto \" + token#1 }\np{ \"said: \" + motto() }");
	assert!(page.contains("<p>said: motto s</p>"), "{page}");
	assert!(!contains(&module, SECRET), "the server function's body shipped in app.wasm");
	assert!(contains(&module, "/rpc/motto"));
}

#[test]
fn a_call_with_a_main_level_argument_asks_the_server() {
	let (page, module) = site("name = \"Ann\"\nserver def greet(n) { token = \"s3cret-token\"; \"hi \" + n + token#1 }\np{ greet(name) }");
	assert!(page.contains("<p>hi Anns</p>"), "{page}");
	assert!(!contains(&module, SECRET));
}

#[test]
fn a_call_in_a_handler_asks_the_server() {
	let (page, module) = site("n = 1\nserver def doubled(x) { token = \"s3cret-token\"; x * 2 }\non click { n = doubled(n) }\np{ \"n: \" + n }");
	assert!(page.contains("<p>n: 1</p>"), "{page}");
	assert!(!contains(&module, SECRET));
}

// the page asks the server before its code runs, so an argument only a function of the page knows cannot be sent
#[test]
fn a_call_with_a_local_argument_is_refused() {
	let failure = warp::site::files("server def doubled(x) { x * 2 }\ndef show(k) { p{ doubled(k) } }\nshow(3)", "rpc", false).expect_err("refused");
	assert!(failure.contains("main-level"), "{failure}");
}

// the page's first value of a call: `x := fetch url ?? default` starts as the default and stays it when the fetch fails
#[test]
fn a_fetch_starts_from_its_default() {
	crate::is!("users := fetch \"http://127.0.0.1:9/users\" ?? [\"none\"]\ncount(users)", 1);
}
