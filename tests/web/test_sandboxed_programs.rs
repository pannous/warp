// card ferron-hosting (user, 2026-10-09: hosted programs run sandboxed): a program run with `warp --sandbox` (the
// programs warp-lambda hosts on pannous.com) gets no FFI, no shell and no foreign runtime; the rest runs as always
use warp::pipeline::sandboxed;
use warp::wasm_emitter::eval;
use warp::Node;

fn refused(code: &str, capability: &str) {
	let result = sandboxed(|| eval(code));
	assert!(matches!(&result, Node::Error(message) if message.to_string().contains("capability denied") && message.to_string().contains(capability)), "{code}: {result:?}");
}

#[test]
fn a_sandboxed_program_calls_no_c_and_no_other_runtime() {
	refused("import strlen from \"c\"; strlen(\"ab\")", "ffi");
	refused("use python math; math.pi", "foreign");
	assert_eq!(sandboxed(|| eval("fib(n) := if n < 2 then n else fib(n-1) + fib(n-2); fib(10)")), Node::int(55));
	assert_eq!(sandboxed(|| eval("exp(0)")), Node::int(1));
	// outside the sandbox the grant is eval's again (P88: everything)
	assert_eq!(eval("import strlen from \"c\"; strlen(\"ab\")"), Node::int(2));
}

// warp's own host words are no C: a sandboxed program still serves its routes, sleeps, draws
#[test]
fn a_sandboxed_program_keeps_the_host_words() {
	assert_eq!(sandboxed(|| eval("sleep 1ms; 3")), Node::int(3));
}
