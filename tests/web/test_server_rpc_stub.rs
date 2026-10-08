// card rpc-stub (notes/server_routes.md step 2): the page calls a server function through POST /rpc/f, so the
// function's body, and any secret in it, stays on the server and out of app.wasm
const SECRET: &str = "s3cret-token";
const PROGRAM: &str = "server def greeting(name) { token = \"s3cret-token\"; \"hi \" + name + token#1 }\ng := greeting(\"Ann\")\np{ \"said: \" + (g ?? \"…\") }";

fn app_wasm(program: &str) -> Vec<u8> {
	let files = warp::site::files(program, "rpc", false).expect("the page builds");
	files.into_iter().find(|(name, _)| name == "app.wasm").expect("app.wasm").1
}

fn contains(bytes: &[u8], text: &str) -> bool {
	bytes.windows(text.len()).any(|window| window == text.as_bytes())
}

#[test]
fn a_server_function_body_stays_out_of_the_page() {
	let module = app_wasm(PROGRAM);
	assert!(!contains(&module, SECRET), "the server function's body shipped in app.wasm");
	assert!(contains(&module, "/rpc/greeting"), "the page calls POST /rpc/greeting");
}

// a server function the page never calls ships nothing at all
#[test]
fn an_uncalled_server_function_stays_out_of_the_page() {
	let module = app_wasm("server def secret() { \"s3cret-token\" }\np{ \"hello\" }");
	assert!(!contains(&module, SECRET));
}
