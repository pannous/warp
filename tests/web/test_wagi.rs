// card fermyon-hosting: `warp build --wagi` makes a WASI command that answers one request as WAGI does: it reads the
// request from the environment (wagi_request, wagi_variable) and prints a CGI reply (wagi_answer, wagi_reply) whose body
// is the route's value as JSON (wagi_json, wagi_json_text, wagi_json_fields, wagi_holds_pairs) or the error (wagi_failed)
use wasmtime::{Engine, Linker, Module, Store};
use wasmtime_wasi::p1::{self, WasiP1Ctx};
use wasmtime_wasi::p2::pipe::MemoryOutputPipe;
use wasmtime_wasi::WasiCtxBuilder;

const OUTPUT_CAPACITY: usize = 1 << 16;
const ROUTES: &str = r#"fib(n) := if n < 2 then n else fib(n-1) + fib(n-2)
get "/" { "hello from the edge" }
get "/fib/:n:int" { {n: n, fib: fib(n)} }
get "/list" { [1, 2.5, "a", yes, [3]] }
get "/nested" { {name: "x", tags: ["a", "b"], inner: {k: no}} }"#;

/// the CGI reply the module prints for `GET path`
fn answer(module: &Module, path: &str) -> String {
	let stdout = MemoryOutputPipe::new(OUTPUT_CAPACITY);
	let mut context = WasiCtxBuilder::new();
	context.env("REQUEST_METHOD", "GET").env("PATH_INFO", path).stdout(stdout.clone());
	let mut store = Store::new(module.engine(), context.build_p1());
	let mut linker: Linker<WasiP1Ctx> = Linker::new(module.engine());
	p1::add_to_linker_sync(&mut linker, |context| context).unwrap();
	let start = linker.instantiate(&mut store, module).unwrap().get_typed_func::<(), ()>(&mut store, "_start").unwrap();
	start.call(&mut store, ()).unwrap_or_else(|trap| panic!("GET {path}: {trap:?}"));
	drop(store);
	String::from_utf8_lossy(&stdout.contents()).into_owned()
}

#[test]
fn each_route_answers_as_wagi() {
	let wasm = warp::pipeline::for_wagi(|| warp::wasm_emitter::compile(ROUTES)).expect("the WAGI module builds");
	let engine = Engine::new(&warp_runtime::engine::deterministic_config()).unwrap();
	let module = Module::new(&engine, &wasm.bytes).unwrap();
	let body = |path| answer(&module, path).rsplit("\n\n").next().unwrap().trim_end().to_string();
	assert_eq!(body("/"), "hello from the edge");
	assert_eq!(body("/fib/20"), r#"{"n":20,"fib":6765}"#);
	assert_eq!(body("/list"), r#"[1,2.5,"a",true,[3]]"#);
	assert_eq!(body("/nested"), r#"{"name":"x","tags":["a","b"],"inner":{"k":false}}"#);
	let missing = answer(&module, "/nowhere");
	assert!(missing.starts_with("Status: 404"), "{missing}");
}
