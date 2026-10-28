//! What `warp compile --aot` adds to the modules compiler.warp's build command writes: x.wasm's machine code
use super::*;

pub(crate) const DEFAULT_COMPILED_NAME: &str = "out.wasm";

/// x.wasm of a program file, out.wasm of inline code
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
