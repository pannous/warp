// A user function's parameters are named in the "name" section's local subsection (AGENTS.md: use all the name
// subsections), for debuggers and wasm-tools; a page leaves them out for size and is read through warp.meta
use warp::pipeline::{compile, for_a_page};
use wasmparser::{KnownCustom, Name, Parser, Payload};

const PROGRAM: &str = "def square(side):=side*side; square(3)";

/// Every local name in the module's name section
fn local_names(bytes: &[u8]) -> Vec<String> {
	let mut names = vec![];
	for payload in Parser::new(0).parse_all(bytes) {
		let Payload::CustomSection(section) = payload.expect("valid module") else { continue };
		let KnownCustom::Name(reader) = section.as_known() else { continue };
		for subsection in reader {
			let Ok(Name::Local(functions)) = subsection else { continue };
			for function in functions {
				for naming in function.expect("function locals").names {
					names.push(naming.expect("local name").name.to_string());
				}
			}
		}
	}
	names
}

fn compiled(program: &str) -> Vec<u8> {
	compile(program).unwrap_or_else(|answer| panic!("{program} compiled to constant {answer:?}")).bytes
}

#[test]
fn parameters_are_named_in_the_name_section() {
	assert!(local_names(&compiled(PROGRAM)).contains(&"side".to_string()));
}

#[test]
fn a_page_leaves_parameter_names_out() {
	assert!(!for_a_page(|| local_names(&compiled(PROGRAM))).contains(&"side".to_string()));
}
