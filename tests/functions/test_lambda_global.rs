//! A lambda declaring `global n` changes main's n as a def does (card closure-global, found by warp-ed)
use crate::is;

#[test]
fn a_lambda_with_global_changes_mains_variable() {
	is!("n = 0; f = () => { global n; n += 1 }; f(); f(); n", 2);
	is!("global n = 0; f = () => { global n; n += 1 }; f(); f(); f(); n", 3);
	is!("n = 0; def f() { global n; n += 1 }; f(); f(); n", 2);
}

// a stored table's typed declaration in main is the global a lambda declares, not a local besides it (a route handler of
// lib/router.warp route_add; the analysis looped forever on it)
#[cfg(feature = "native")]
#[test]
fn a_lambda_reads_a_stored_table_as_global() {
	let program = "class Todo{title: text; done: bool}\nstored todos: [Todo] in \"todos_lambda_global\"\ntodos.add(Todo(\"a\", no))\nf = (request) => { global todos\n todos }\ncount(f(0)) == count(todos)";
	assert_eq!(warp::wasm_emitter::eval(program).serialize().trim(), "yes");
}
