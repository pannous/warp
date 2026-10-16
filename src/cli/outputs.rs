//! The files `warp build` writes: the module, machine code, WIT world, component, WAGI module, site
use super::*;

pub(crate) const DEFAULT_COMPILED_NAME: &str = "out.wasm";
pub(crate) const SITE_SUFFIX: &str = "-site";
pub(crate) const WIT_EXTENSION: &str = "wit";
pub(crate) const COMPONENT_EXTENSION: &str = "component.wasm";
pub(crate) const WAGI_EXTENSION: &str = "wagi.wasm";
pub(crate) const SPIN_MANIFEST_EXTENSION: &str = "spin.toml";

/// `dir/program.warp` compiles to `dir/program.wasm`; inline code compiles to `out.wasm`
/// `warp build`: the WebAssembly module next to the program file (out.wasm for inline code), with `--aot` its machine
/// code too
pub(crate) fn write_module(code: &str, target: &str, ahead_of_time: bool) {
	match warp::pipeline::for_any_host(|| wasm_emitter::compile(code)) {
		Ok(module) => {
			let output_path = compiled_output_path(target);
			fs::write(&output_path, &module.bytes).expect("could not write the compiled module");
			println!("compiled {} bytes to {}", module.bytes.len(), output_path);
			if ahead_of_time {
				write_machine_code(&module.bytes, &output_path);
			}
		}
		Err(final_value) => fail(format!("nothing to compile: {}", final_value.serialize())),
	}
}

pub(crate) fn compiled_output_path(target: &str) -> String {
	if file_exists(target) {
		std::path::Path::new(target).with_extension("wasm").to_string_lossy().into_owned()
	} else {
		DEFAULT_COMPILED_NAME.to_string()
	}
}

/// `warp compile --aot`: the module compiled to machine code for this machine and wasmtime version, next to the .wasm
pub(crate) fn write_machine_code(bytes: &[u8], wasm_path: &str) {
	let path = std::path::Path::new(wasm_path).with_extension(COMPILED_EXTENSION);
	let (engine, _) = wasm_reader::engine_for(bytes);
	match engine.precompile_module(bytes) {
		Ok(machine_code) => {
			fs::write(&path, &machine_code).expect("could not write the machine code");
			println!("compiled {} bytes of machine code to {}", machine_code.len(), path.display());
		}
		Err(failure) => fail(format!("could not compile to machine code: {failure}")),
	}
}

/// What an import the standalone runtime lacks is for, in the user's words (card g-13Vk: "tasks need runtime."): the
/// host words of tasks, shared arrays and run-time blocks by their feature, any other import as itself. fetch,
/// fetch_within, read, warn and run are imported together (import_manager emit_host_imports): one feature.
pub(crate) fn feature_of(module: &str, name: &str) -> String {
	let feature = match name {
		"fetch" | "fetch_within" | "read" | "warn" | "run" if module == "host" => "fetch and files",
		_ if name.starts_with("task_") || name.starts_with("task·") => "tasks",
		_ if name.starts_with("shared_") => "shared arrays",
		"fetch_start" | "fetch_reply" => "async fetch",
		"run_block" | "block·value" => "run-time blocks",
		"parse_code" => "run-time parse",
		"foreign_call" => "foreign calls",
		_ => return format!("{module}.{name}"),
	};
	feature.to_string()
}

/// `warp build --wit`: the component's WIT world next to the program file (out.wit for inline code)
pub(crate) fn write_wit(code: &str, target: &str) {
	let path = std::path::Path::new(&compiled_output_path(target)).with_extension(WIT_EXTENSION);
	match warp::component_worlds::world_wit(&warp_parser::parse(code)) {
		Ok(wit) => {
			fs::write(&path, wit).expect("could not write the WIT world");
			println!("wrote {}", path.display());
		}
		Err(failure) => fail(format!("warp build --wit: {failure}")),
	}
}

/// `warp build --component`: the component next to the program file (out.component.wasm for inline code)
pub(crate) fn write_component(code: &str, target: &str) {
	let path = std::path::Path::new(&compiled_output_path(target)).with_extension(COMPONENT_EXTENSION);
	match warp::modules::component_builder::build(code) {
		Ok(component) => {
			fs::write(&path, &component).expect("could not write the component");
			println!("wrote {} ({} bytes)", path.display(), component.len());
		}
		Err(failure) => fail(format!("warp build --component: {failure}")),
	}
}

/// `warp build --wagi`: the WAGI module and its Spin manifest next to the program file
pub(crate) fn write_wagi(code: &str, target: &str) {
	let module_path = std::path::Path::new(&compiled_output_path(target)).with_extension(WAGI_EXTENSION);
	let manifest_path = module_path.with_extension("").with_extension(SPIN_MANIFEST_EXTENSION);
	let name = module_path
		.file_stem()
		.and_then(|stem| stem.to_str())
		.and_then(|stem| stem.split('.').next())
		.unwrap_or(DEFAULT_EXECUTABLE_NAME)
		.to_string();
	match warp::pipeline::for_wagi(|| wasm_emitter::compile(code)) {
		Ok(module) => {
			fs::write(&module_path, &module.bytes).expect("could not write the WAGI module");
			let source = module_path.file_name().unwrap_or_default().to_string_lossy();
			fs::write(&manifest_path, warp::web::deploy::spin_manifest(&name, &source))
				.expect("could not write the Spin manifest");
			println!("wrote {} ({} bytes) and {}", module_path.display(), module.bytes.len(), manifest_path.display());
		}
		Err(failure) => {
			eprintln!("warp build --wagi: {failure}");
			std::process::exit(1);
		}
	}
}

/// `warp compile --page`: the module next to the program file with the modules it imports by path in modules/
pub(crate) fn write_page_module(code: &str, target: &str) {
	let output_path = compiled_output_path(target);
	let written = warp::pipeline::for_any_host(|| wasm_emitter::compile(code))
		.map_err(|final_value| format!("nothing to compile: {}", final_value.serialize()))
		.and_then(|module| {
			warp::web::site_modules::write_page_module(&module.bytes, std::path::Path::new(&output_path))
		});
	match written {
		Ok(files) => println!("compiled {output_path} for a page, beside it: {}", files.join(", ")),
		Err(failure) => fail(format!("warp compile --page: {failure}")),
	}
}

/// `warp build --site`: the site next to the program file, its report or failure
pub(crate) fn write_site(code: &str, target: &str) {
	let file = std::path::Path::new(target);
	let (name, directory) = match file.file_stem().filter(|_| file_exists(target)) {
		Some(stem) => (
			stem.to_string_lossy().into_owned(),
			file.with_file_name(format!("{}{SITE_SUFFIX}", stem.to_string_lossy())),
		),
		None => (DEFAULT_SITE_NAME.to_string(), std::path::PathBuf::from(DEFAULT_SITE_NAME)),
	};
	match warp::web::site::build(code, &name, &directory) {
		Ok(site) => println!("built the site {} ({})", site.directory.display(), site.files.join(", ")),
		Err(failure) => fail(format!("warp build --site: {failure}")),
	}
}
