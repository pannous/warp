//! Effect inference, `effects of f`, declared constraints, effect-driven imports

use warp::effects::{effects_of, Effect::*, EffectReport, EffectSet};
use warp::wasm_emitter::eval;
use warp::wasp_parser::WaspParser;
use warp::{is, Node, WasmGcEmitter};

fn effects(code: &str, function: &str) -> EffectSet {
	effects_of(code, function).unwrap_or_else(|| panic!("{function} unresolved in {code}"))
}

/// (module, name) of every import the compiled program declares
fn imports_of(code: &str) -> Vec<(String, String)> {
	let mut emitter = WasmGcEmitter::new();
	emitter.emit_for_node(&WaspParser::parse(code));
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
fn test_effect_set_names() {
	assert_eq!(EffectSet::PURE.to_string(), "Pure");
	assert_eq!(EffectSet::of(&[FFI, IO]).to_string(), "IO, FFI");
	assert_eq!(EffectSet::named(&["IO", "FFI"]), Some(EffectSet::of(&[IO, FFI])));
	assert_eq!(EffectSet::named(&["Pure"]), Some(EffectSet::PURE));
	assert_eq!(EffectSet::named(&["Spooky"]), None);
	assert!(EffectSet::of(&[IO]).is_subset_of(EffectSet::of(&[IO, FFI])));
}

#[test]
fn test_effects_are_inferred_from_calls() {
	assert_eq!(effects("square(x) := x*x", "square"), EffectSet::PURE);
	assert_eq!(effects("greet(x) := puts x", "greet"), EffectSet::of(&[IO]));
	assert_eq!(effects("def greet(x){puts x};greet 'a'", "greet"), EffectSet::of(&[IO]));
	assert_eq!(effects("f := puts it", "f"), EffectSet::of(&[IO]));
	assert_eq!(effects("x=fetch https://a.com/t;x", "main"), EffectSet::of(&[IO]));
	assert_eq!(effects("use m;floor(4.5)", "main"), EffectSet::of(&[FFI]));
	assert_eq!(effects("puts", "fd_write"), EffectSet::of(&[IO, Unsafe]));
}

#[test]
fn test_effects_propagate_through_callers_and_recursion() {
	let code = "log(x) := puts x\nhelper(x) := log(x)\nloop(x) := x<1 ? helper(x) : loop(x-1)\nfib(n) := n<2 ? n : fib(n-1)+fib(n-2)";
	let report = EffectReport::of(&WaspParser::parse(code));
	assert_eq!(report.effects_of("helper"), Some(EffectSet::of(&[IO])));
	assert_eq!(report.effects_of("loop"), Some(EffectSet::of(&[IO])));
	assert_eq!(report.effects_of("fib"), Some(EffectSet::PURE));
	assert_eq!(report.call_chain("loop", IO), ["loop", "helper", "log", "puts"]);
	assert_eq!(report.entry_effects(), EffectSet::PURE); // defining is not calling
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
	assert_eq!(imports_of("use m;floor(4.5)"), [("m".into(), "floor".into())]);
	assert!(imports_of("x=fetch https://a.com/t;x").contains(&("host".into(), "fetch".into())));
}
