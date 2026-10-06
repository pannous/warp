use warp::dead_functions::without_dead_functions;

const MODULE_WITH_DEAD_CODE: &str = r#"(module
	(import "host" "clock" (func $clock (result i64)))
	(func $unused (result i64) (call $also_unused))
	(func $also_unused (result i64) (i64.const 7))
	(func $helper (param i64) (result i64) (i64.add (local.get 0) (i64.const 1)))
	(func $taken (result i64) (i64.const 3))
	(func (export "main") (result i64) (call $helper (i64.const 41)))
	(func (export "closure") (result funcref) (ref.func $taken))
	(elem declare func $taken))"#;

/// the names of a module's functions, from its name section
fn function_names(module: &[u8]) -> Vec<String> {
	let mut names = Vec::new();
	for payload in wasmparser::Parser::new(0).parse_all(module) {
		if let wasmparser::Payload::CustomSection(section) = payload.unwrap() {
			if let wasmparser::KnownCustom::Name(reader) = section.as_known() {
				for subsection in reader {
					if let wasmparser::Name::Function(map) = subsection.unwrap() {
						names.extend(map.into_iter().map(|naming| naming.unwrap().name.to_string()));
					}
				}
			}
		}
	}
	names
}

#[test]
fn unreachable_functions_are_dropped_and_the_rest_renumbered() {
	let module = wat::parse_str(MODULE_WITH_DEAD_CODE).unwrap();
	let pruned = without_dead_functions(&module).unwrap();
	wasmparser::validate(&pruned).unwrap();
	let names = function_names(&pruned);
	assert!(!names.iter().any(|name| name.contains("unused")), "{names:?}");
	for kept in ["clock", "helper", "taken"] {
		assert!(names.iter().any(|name| name == kept), "{kept} missing: {names:?}");
	}
	assert!(pruned.len() < module.len());
}

#[test]
fn a_module_without_dead_functions_stays_as_it_is() {
	let module = wat::parse_str(r#"(module (func $helper (result i64) (i64.const 1)) (func (export "main") (result i64) (call $helper)))"#).unwrap();
	assert_eq!(without_dead_functions(&module).unwrap(), module);
}

/// the emitter's modules carry no function that nothing reaches: pruning them again changes nothing
#[test]
fn compiled_programs_carry_no_dead_functions() {
	for program in ["42", "f(x) := x + 1; f(41)", "print \"hi\"; [1, 2, 3].map(x => x * 2)", "try 1/0 else 7"] {
		let module = warp::wasm_emitter::compile(program).unwrap().bytes;
		assert_eq!(without_dead_functions(&module).unwrap(), module, "{program}");
	}
}
