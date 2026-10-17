// card web-split-helpers: a built site's function only one route calls moves into that route's module with the route
// (src/web/route_split.rs), unless a task may call it by name in a fresh instance (web/playground/host-tasks.js)
use crate::common::WASM_SPLIT;
use crate::requires;

const THREE_ROUTES: &str = include_str!("../../probes/lazy_routes/three_routes.warp");
const SPAWNS_FIB: &str = "answer = await go fib(10)\n";

/// The names of the functions a module defines (its name section names imported ones too: those are left out)
fn defined_functions(module: &[u8]) -> Vec<String> {
	let mut imported = 0;
	let mut names = vec![];
	for payload in wasmparser::Parser::new(0).parse_all(module) {
		match payload.unwrap() {
			wasmparser::Payload::ImportSection(imports) => {
				imported += imports
					.into_imports()
					.filter(|import| matches!(import.as_ref().unwrap().ty, wasmparser::TypeRef::Func(_)))
					.count() as u32
			}
			wasmparser::Payload::CustomSection(section) => {
				if let wasmparser::KnownCustom::Name(reader) = section.as_known() {
					for subsection in wasmparser::NameSectionReader::from(reader) {
						if let wasmparser::Name::Function(map) = subsection.unwrap() {
							names.extend(
								map.into_iter()
									.map(|naming| naming.unwrap())
									.filter(|naming| naming.index >= imported)
									.map(|naming| naming.name.to_string()),
							);
						}
					}
				}
			}
			_ => {}
		}
	}
	names
}

fn site_functions(code: &str, module: &str) -> Vec<String> {
	let files = warp::web::site::files(code, "routes", false).expect("the site is built");
	let (_, bytes) = files
		.iter()
		.find(|(name, _)| name == module)
		.unwrap_or_else(|| panic!("no {module} among {:?}", files.iter().map(|(name, _)| name).collect::<Vec<_>>()));
	defined_functions(bytes)
}

#[test]
fn a_function_one_route_calls_moves_with_the_route() {
	requires!(WASM_SPLIT);
	let primary = site_functions(THREE_ROUTES, "app.wasm");
	assert!(!primary.contains(&"primes".to_string()), "{primary:?}");
	let route = site_functions(THREE_ROUTES, "app-route-1.wasm");
	assert!(route.contains(&"primes".to_string()), "{route:?}");
}

#[test]
fn a_function_a_task_may_call_stays_in_the_main_module() {
	requires!(WASM_SPLIT);
	let primary = site_functions(&format!("{SPAWNS_FIB}{THREE_ROUTES}"), "app.wasm");
	assert!(primary.contains(&"primes".to_string()), "{primary:?}");
}
