//! The files `warp build` writes: the module, machine code, WIT world, component, WAGI module, site
use super::*;

pub(crate) const DEFAULT_COMPILED_NAME: &str = "out.wasm";

/// `warp build`: the WebAssembly module next to the program file (out.wasm for inline code), with `--aot` its machine
/// code too; under WARP_SELF_HOSTED=pure or pure-only the warp compiler's module, as `warp <file>` runs it
pub(crate) fn write_module(code: &str, target: &str, ahead_of_time: bool) {
	let folder = std::path::Path::new(target)
		.parent()
		.filter(|_| file_exists(target))
		.map_or_else(|| env::current_dir().unwrap_or_default(), std::path::Path::to_path_buf);
	match warp::compiler::standalone::compiled_code(code, &folder, warp::compiler::standalone::Compiler::RustCompiled) {
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


/// `warp build --wit`: the component's WIT world next to the program file (out.wit for inline code)
pub(crate) fn write_wit(code: &str, target: &str) {
	let _ = code;
	fail(not_in_self_lane("warp build --wit", target));
}

/// `warp build --component`: the component next to the program file (out.component.wasm for inline code)
pub(crate) fn write_component(code: &str, target: &str) {
	let _ = code;
	fail(not_in_self_lane("warp build --component", target));
}

/// `warp build --wagi`: the WAGI module and its Spin manifest next to the program file
pub(crate) fn write_wagi(code: &str, target: &str) {
	let _ = code;
	fail(not_in_self_lane("warp build --wagi", target));
}

/// `warp compile --page`: the module next to the program file with the modules it imports by path in modules/
pub(crate) fn write_page_module(code: &str, target: &str) {
	let _ = code;
	fail(not_in_self_lane("warp compile --page", target));
}

/// `warp build --site`: the site next to the program file, its report or failure
pub(crate) fn write_site(code: &str, target: &str) {
	let _ = code;
	fail(not_in_self_lane("warp build --site", target));
}
