//! Per-route modules of a built site (card web-bundle, notes/web_framework.md "Built sites"): each route's function
//! page·route·N and the functions only it reaches move into their own module app-route-N.wasm, which the page loads
//! when it shows that route (web/playground/site.js). binaryen's wasm-split does the splitting: the primary module
//! keeps a table slot and a placeholder import (module `placeholder.app-route-N`) per moved function, and the route's
//! module, instantiated with the primary's exports as `primary`, fills the slots. Without wasm-split on PATH the site
//! ships one module, with a note.
use crate::compiler::dead_functions::{function_names, CallGraph};
use crate::host::{TASK_SPAWN_VALUES, USER_FUNCTIONS_SECTION, USER_FUNCTIONS_SEPARATOR};
use crate::modules::binaryen::BINARYEN_FEATURES;
use std::process::Command;

const WASM_SPLIT: &str = "wasm-split";
/// The function of route N is ROUTE_FUNCTION_PREFIX + N (src/lowering/web/routes.rs)
const ROUTE_FUNCTION_PREFIX: &str = "page·route·";
/// The file of route N's module is ROUTE_MODULE_PREFIX + N + ".wasm"; site.js finds them by their placeholder imports
pub const ROUTE_MODULE_PREFIX: &str = "app-route-";
const WASM_EXTENSION: &str = ".wasm";
const PRIMARY_FILE: &str = "primary.wasm";
const MODULE_FILE: &str = "app.wasm";
const MANIFEST_FILE: &str = "routes.manifest";
/// wasm-split keeps the name section only when asked
const KEEP_NAMES: &str = "--debuginfo";

/// The primary module and each split route's module with its file name
pub struct RouteModules {
	pub primary: Vec<u8>,
	pub routes: Vec<(String, Vec<u8>)>,
}

/// `module` split by route, or None when it has no route whose functions could move (or wasm-split is missing)
pub fn split_by_route(module: &[u8]) -> Result<Option<RouteModules>, String> {
	let manifest = manifest(module)?;
	if manifest.is_empty() {
		return Ok(None);
	}
	if !crate::modules::binaryen::available(WASM_SPLIT) {
		eprintln!(
			"note: the site ships one module: install binaryen for wasm-split, so each route loads its own module"
		);
		return Ok(None);
	}
	wasm_split(module, &manifest).map(Some)
}

/// Per route with functions of its own: its module name and those functions' names, in route order
fn manifest(module: &[u8]) -> Result<Vec<(String, Vec<String>)>, String> {
	let graph = CallGraph::of(module).map_err(|failure| failure.to_string())?;
	let names = function_names(module).map_err(|failure| failure.to_string())?;
	let movable_exports = movable_exports(module)?;
	let mut routes: Vec<(usize, u32)> = names
		.iter()
		.filter_map(|(&function, name)| {
			name.strip_prefix(ROUTE_FUNCTION_PREFIX)?.parse().ok().map(|route| (route, function))
		})
		.collect();
	routes.sort();
	let route_functions: Vec<u32> = routes.iter().map(|&(_, function)| function).collect();
	let roots: Vec<u32> = graph
		.roots
		.iter()
		.copied()
		.filter(|root| {
			!route_functions.contains(root) && !names.get(root).is_some_and(|name| movable_exports.contains(name))
		})
		.collect();
	let shared = graph.reached(&roots, &route_functions);
	let reached: Vec<Vec<bool>> =
		route_functions.iter().map(|&function| graph.reached(&[function], &route_functions)).collect();
	let mut manifest = vec![];
	for (position, &(route, _)) in routes.iter().enumerate() {
		let own = (0..shared.len()).filter(|&defined| {
			reached[position][defined]
				&& !shared[defined]
				&& !reached.iter().enumerate().any(|(other, by)| other != position && by[defined])
		});
		let own_names: Vec<String> =
			own.filter_map(|defined| names.get(&(defined as u32 + graph.imported)).cloned()).collect();
		if !own_names.is_empty() {
			manifest.push((format!("{ROUTE_MODULE_PREFIX}{route}"), own_names));
		}
	}
	Ok(manifest)
}

/// The program's own functions, exported only for tasks (host.rs USER_FUNCTIONS_SECTION): a task runs them by name in
/// an instance of its own, without the route's module, so with tasks they stay
fn movable_exports(module: &[u8]) -> Result<Vec<String>, String> {
	let spawns_tasks = crate::host::host_parts::imports_of(module)?.iter().any(|(_, name)| name == TASK_SPAWN_VALUES);
	match spawns_tasks {
		true => Ok(vec![]),
		false => crate::host::host_parts::section_names(module, USER_FUNCTIONS_SECTION, USER_FUNCTIONS_SEPARATOR),
	}
}

/// wasm-split --multi-split in a folder of its own: the primary module and one module per manifest entry
fn wasm_split(module: &[u8], manifest: &[(String, Vec<String>)]) -> Result<RouteModules, String> {
	let folder =
		std::env::temp_dir().join(format!("warp-route-split-{}-{:?}", std::process::id(), std::thread::current().id()));
	std::fs::create_dir_all(&folder).map_err(|failure| format!("cannot make {}: {failure}", folder.display()))?;
	let written = |name: &str, bytes: &[u8]| {
		std::fs::write(folder.join(name), bytes).map_err(|failure| format!("cannot write {name}: {failure}"))
	};
	written(MODULE_FILE, module)?;
	let lines: Vec<String> =
		manifest.iter().map(|(name, functions)| format!("{name}\n{}\n", functions.join("\n"))).collect();
	written(MANIFEST_FILE, lines.join("\n").as_bytes())?;
	let output = Command::new(WASM_SPLIT)
		.current_dir(&folder)
		.args([
			"--multi-split",
			KEEP_NAMES,
			"--manifest",
			MANIFEST_FILE,
			"--out-prefix",
			"",
			"-o",
			PRIMARY_FILE,
			MODULE_FILE,
		])
		.args(BINARYEN_FEATURES)
		.output()
		.map_err(|failure| format!("cannot run {WASM_SPLIT}: {failure}"))?;
	let read = |name: &str| {
		std::fs::read(folder.join(name)).map_err(|failure| format!("{WASM_SPLIT} wrote no {name}: {failure}"))
	};
	let split = match output.status.success() {
		true => read(PRIMARY_FILE).and_then(|primary| {
			let routes = manifest.iter().map(|(name, _)| {
				read(&format!("{name}{WASM_EXTENSION}")).map(|bytes| (format!("{name}{WASM_EXTENSION}"), bytes))
			});
			Ok(RouteModules { primary, routes: routes.collect::<Result<_, _>>()? })
		}),
		false => Err(format!("{WASM_SPLIT} failed: {}", String::from_utf8_lossy(&output.stderr).trim())),
	};
	let _ = std::fs::remove_dir_all(&folder);
	split
}
