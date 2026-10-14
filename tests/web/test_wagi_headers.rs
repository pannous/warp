// card wagi-build: a route's `status` and `header` (card web-route-header) reach the CGI answer of a WAGI build
// (lib/wagi.warp wagi_header, lowering/serve.rs with_answer_headers)
use super::test_wagi::answer;
use wasmtime::{Engine, Module};

const ROUTES: &str = r#"get "/teapot" {
	status 418
	header "X-Warp" "yes"
	"short and stout"
}
get "/page" {
	header "Content-Type" "text/html"
	"<b>hi</b>"
}
get "/broken" {
	status 201
	throw "no teapot"
}"#;

fn module() -> Module {
	let wasm = warp::pipeline::for_wagi(|| warp::wasm_emitter::compile(ROUTES)).expect("the WAGI module builds");
	let engine = Engine::new(&warp_runtime::engine::deterministic_config()).unwrap();
	Module::new(&engine, &wasm.bytes).unwrap()
}

/// The CGI header lines of the answer, before its blank line
fn headers(answer: &str) -> Vec<&str> {
	answer.split("\n\n").next().unwrap().lines().collect()
}

#[test]
fn a_routes_status_and_headers_reach_the_cgi_answer() {
	let module = module();
	let teapot = answer(&module, "/teapot");
	assert_eq!(headers(&teapot), ["Status: 418", "Content-Type: text/plain; charset=utf-8", "X-Warp: yes"], "{teapot}");
	assert!(teapot.trim_end().ends_with("\n\nshort and stout"), "{teapot}");
	let page = answer(&module, "/page");
	assert_eq!(headers(&page), ["Status: 200", "Content-Type: text/html"], "{page}");
	let broken = answer(&module, "/broken");
	assert!(headers(&broken).starts_with(&["Status: 500"]), "{broken}");
}
