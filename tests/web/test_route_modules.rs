// card web-bundle: a built site with routes keeps each route's own code in a module of its own, app-route-N.wasm, which
// the page loads when it shows the route (src/route_split.rs, web/playground/site.js loadRouteModule); the main module
// imports a placeholder for each moved function. In a browser: probes/lazy_routes/check_in_browser.sh
use crate::common::WASM_SPLIT;
use crate::requires;

const ROUTED: &str = "route \"/\" { h1{ \"Home\" } }\nroute \"/about\" { p{ \"About \" + \"us\" } }\ndiv{ nav{ a{ href: \"/about\" \"About\" } } outlet }";

#[test]
fn each_route_comes_in_a_module_of_its_own() {
	requires!(WASM_SPLIT);
	let files = warp::site::files(ROUTED, "routed", false).expect("the site is built");
	let names: Vec<&str> = files.iter().map(|(name, _)| name.as_str()).collect();
	assert!(names.contains(&"app-route-0.wasm") && names.contains(&"app-route-1.wasm"), "{names:?}");
	let module = |name: &str| &files.iter().find(|(file, _)| file == name).unwrap().1;
	let placeholders: Vec<String> = wasmparser::Parser::new(0).parse_all(module("app.wasm")).filter_map(|payload| match payload.unwrap() {
		wasmparser::Payload::ImportSection(imports) => Some(imports.into_imports().map(|import| import.unwrap().module.to_string()).collect::<Vec<_>>()),
		_ => None,
	}).flatten().filter(|module| module.starts_with("placeholder.")).collect();
	assert!(placeholders.contains(&"placeholder.app-route-1".to_string()), "{placeholders:?}");
	for name in ["app.wasm", "app-route-0.wasm", "app-route-1.wasm"] {
		wasmparser::Validator::new_with_features(wasmparser::WasmFeatures::WASM3).validate_all(module(name)).unwrap_or_else(|failure| panic!("{name}: {failure}"));
	}
}

#[test]
fn a_dev_site_and_a_site_without_routes_ship_one_module() {
	let modules = |code: &str, dev: bool| warp::site::files(code, "one", dev).unwrap().into_iter().filter(|(name, _)| name.ends_with(".wasm")).count();
	assert_eq!(modules(ROUTED, true), 1);
	assert_eq!(modules("div{ p{ \"hi\" } }", false), 1);
}
