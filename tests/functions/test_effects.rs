//! Effect inference, `effects of f`, declared constraints, effect-driven imports

use crate::common::effects_of;
use crate::is;
use warp::warp_parser::WarpParser;
use warp::wasm_emitter::eval;
use warp::{Node, WasmGcEmitter};

/// (module, name) of every import the compiled program declares
fn imports_of(code: &str) -> Vec<(String, String)> {
	let mut emitter = WasmGcEmitter::new();
	emitter.emit_for_node(&WarpParser::parse(code));
	let bytes = emitter.finish();
	let mut imports = vec![];
	for payload in wasmparser::Parser::new(0).parse_all(&bytes) {
		if let Ok(wasmparser::Payload::ImportSection(section)) = payload {
			for import in section.into_imports() {
				let import = import.expect("valid import");
				imports.push((import.module.to_string(), import.name.to_string()));
			}
		}
	}
	imports
}

fn error_text(result: Node) -> String {
	match result {
		Node::Error(message) => format!("{}", message),
		other => panic!("expected an effect diagnostic, got {other:?}"),
	}
}

#[test]
fn test_effects_are_inferred_from_calls() {
	assert_eq!(effects_of("square(x) := x*x", "square"), "Pure");
	assert_eq!(effects_of("greet(x) := puts x", "greet"), "IO");
	assert_eq!(effects_of("def greet(x){puts x}", "greet"), "IO");
	assert_eq!(effects_of("f := puts it", "f"), "IO");
	assert_eq!(effects_of("get(a) := fetch https://a.com/t", "get"), "IO");
	assert_eq!(effects_of("use m; down(x) := floor(x)", "down"), "Pure"); // libm is pure (P26)
	assert_eq!(effects_of("import strlen from \"c\"; size(s) := strlen(s)", "size"), "FFI");
	assert_eq!(effects_of("", "fd_write"), "(IO Unsafe)");
}

#[test]
fn test_effects_propagate_through_callers_and_recursion() {
	let code = "log(x) := puts x\nhelper(x) := log(x)\nloop(x) := x<1 ? helper(x) : loop(x-1)\nfib(n) := n<2 ? n : fib(n-1)+fib(n-2)";
	assert_eq!(effects_of(code, "helper"), "IO");
	assert_eq!(effects_of(code, "loop"), "IO");
	assert_eq!(effects_of(code, "fib"), "Pure");
}

#[test]
fn test_effects_of_query() {
	is!("square(x) := x*x\neffects of square", Node::Symbol("Pure".into()));
	is!("greet(x) := puts x\neffects of greet", Node::Symbol("IO".into()));
	is!("effects of puts", Node::Symbol("IO".into()));
	assert!(error_text(eval("effects of nowhere")).contains("unknown function"));
}

#[test]
fn test_declared_constraint_holds() {
	is!("square(x) := x*x ! Pure\nsquare(3)", 9);
	is!("next(x) := x+1 ! IO, FFI\nnext(2)", 3); // declared effects are an upper bound
}

#[test]
fn test_constraint_on_non_function_is_loud() {
	let message = error_text(eval("answer := 42 ! Pure\nanswer"));
	assert!(message.contains("answer is not a function"), "{message}");
}

#[test]
fn test_declared_constraint_violation_names_span_and_chain() {
	let message = error_text(eval("log(x) := puts x\nsquare(x) := log(x) ! Pure\nsquare(3)"));
	assert!(message.contains("square is declared ! Pure but performs IO"), "{message}");
	assert!(message.contains("square → log → puts"), "{message}");
	assert!(message.contains("at 2:"), "{message}");
	let message = error_text(eval("f(x) := puts x ! FFI\nf(1)"));
	assert!(message.contains("f → puts"), "{message}");
}

#[test]
fn test_imports_follow_effects() {
	assert!(imports_of("square(x) := x*x\nsquare(3)").is_empty());
	assert!(imports_of("x='puts ';x").is_empty(), "text mentioning puts is no IO call");
	assert!(imports_of("use m;3").is_empty(), "declared but uncalled FFI is not imported");
	assert_eq!(imports_of("puts('ok')"), [("wasi_snapshot_preview1".into(), "fd_write".into())]);
	assert_eq!(imports_of("use m;fmod(4.5, 2.0)"), [("m".into(), "fmod".into())]);
	assert!(imports_of("x=fetch https://a.com/t;x").contains(&("host".into(), "fetch".into())));
}

// card effects-infer: Allocation (construction) is reported but stays pure (wiki/pure.md), Async is a real effect
#[test]
fn test_allocation_and_async_are_inferred() {
	assert_eq!(effects_of("pair(x) := [x, x]", "pair"), "Allocation");
	assert_eq!(effects_of("person(n) := {name: n}", "person"), "Allocation");
	assert_eq!(effects_of("greet(n) := \"hi \" + n", "greet"), "Allocation");
	is!("first(x) := [x, x]#1 ! Pure\nfirst(3)", 3);
	is!("g() := 1\nf() := await go g()\neffects of f", Node::Symbol("Async".into()));
	let message = error_text(eval("g() := 1\nf() := await go g() ! Pure\nf()"));
	assert!(message.contains("f is declared ! Pure but performs Async"), "{message}");
}
