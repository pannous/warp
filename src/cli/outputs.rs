//! The files `warp build` writes: the module, machine code, WIT world, component, WAGI module, site
use super::*;

pub(crate) const DEFAULT_COMPILED_NAME: &str = "out.wasm";
/// The module with runtime.wasm copied in, untrimmed (x.full.wasm), and the one importing it (x.mod.wasm)
const FULL_VARIANT: &str = "full";
const SEPARATE_VARIANT: &str = "mod";

/// `warp build`: from one compile, three modules next to the program file (out for inline code): x.full.wasm with
/// runtime.wasm copied in, x.wasm that one trimmed, x.mod.wasm importing runtime.wasm's instance; with `--aot` the
/// machine code of x.wasm too
pub(crate) fn write_module(code: &str, target: &str, trim: Trim, ahead_of_time: bool) {
	let folder = std::path::Path::new(target)
		.parent()
		.filter(|_| file_exists(target))
		.map_or_else(|| env::current_dir().unwrap_or_default(), std::path::Path::to_path_buf);
	let output_path = compiled_output_path(target);
	let separate = variant_path(&output_path, SEPARATE_VARIANT);
	let compiler = warp::compiler::standalone::Compiler::RustCompiled;
	match warp::compiler::standalone::compiled_code_and_separate(code, &folder, compiler, Some(separate.as_ref())) {
		Ok(module) => {
			write_reported(&module.bytes, &variant_path(&output_path, FULL_VARIANT));
			report_written(&separate);
			write_compiled(module.bytes, target, trim, ahead_of_time);
		}
		Err(final_value) => fail(format!("nothing to compile: {}", final_value.serialize())),
	}
}

/// x.wasm as x.<variant>.wasm
fn variant_path(output_path: &str, variant: &str) -> String {
	std::path::Path::new(output_path).with_extension(format!("{variant}.wasm")).to_string_lossy().into_owned()
}

fn write_reported(bytes: &[u8], path: &str) {
	fs::write(path, bytes).unwrap_or_else(|failure| fail(format!("{path}: {failure}")));
	report_written(path);
}

fn report_written(path: &str) {
	let size = fs::metadata(path).map_or(0, |written| written.len());
	println!("compiled {size} bytes to {path}");
}

/// The compiled module written next to the program file (out.wasm for inline code), trimmed as asked, with `--aot` its
/// machine code too
pub(crate) fn write_compiled(bytes: Vec<u8>, target: &str, trim: Trim, ahead_of_time: bool) {
	let output_path = compiled_output_path(target);
	write_reported(&bytes, &output_path);
	let bytes = trim.apply(&output_path, bytes);
	if ahead_of_time {
		write_machine_code(&bytes, &output_path);
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
