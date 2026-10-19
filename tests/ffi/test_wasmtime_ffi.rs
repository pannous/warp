// card wasmtime-ffi: `use wasmtime` reads Homebrew's wasmtime.h with the headers it includes, whose functions wasm.h
// declares through macros (`WASM_DECLARE_OWN(engine)`), and brings lib/wasmtime.warp, whose run(bytes) runs a module
// through them: its C structs cross as maps. Skipped where libwasmtime is not installed
use crate::is;

const LIBWASMTIME: &str = "/opt/homebrew/lib/libwasmtime.dylib";
/// (module (import "wasi_snapshot_preview1" "fd_write" …) (memory (export "memory") 1) (data (i32.const 16) "hi\n")
///  (func (export "main") (result i64) fd_write(1, iovec {16, 3}) i64.const 7))
const HI_MODULE: &str = "[0, 97, 115, 109, 1, 0, 0, 0, 1, 13, 2, 96, 4, 127, 127, 127, 127, 1, 127, 96, 0, 1, 126, 2, 35, 1, 22, 119, 97, 115, 105, 95, 115, 110, 97, 112, 115, 104, 111, 116, 95, 112, 114, 101, 118, 105, 101, 119, 49, 8, 102, 100, 95, 119, 114, 105, 116, 101, 0, 0, 3, 2, 1, 1, 5, 3, 1, 0, 1, 7, 17, 2, 6, 109, 101, 109, 111, 114, 121, 2, 0, 4, 109, 97, 105, 110, 0, 1, 10, 31, 1, 29, 0, 65, 0, 65, 16, 54, 2, 0, 65, 4, 65, 3, 54, 2, 0, 65, 1, 65, 0, 65, 1, 65, 8, 16, 0, 26, 66, 7, 11, 11, 9, 1, 0, 65, 16, 11, 3, 104, 105, 10]";

fn libwasmtime_installed() -> bool {
	std::path::Path::new(LIBWASMTIME).exists()
}

#[test]
fn a_macro_declared_function_is_called() {
	if !libwasmtime_installed() {
		return;
	}
	is!("use wasmtime\nwasm_engine_new() > 0", true);
}

#[test]
fn a_module_runs_through_libwasmtime() {
	if !libwasmtime_installed() {
		return;
	}
	is!(&format!("use wasmtime\nrun({HI_MODULE})"), 7);
}

#[test]
fn a_struct_output_reads_as_a_map() {
	if !libwasmtime_installed() {
		return;
	}
	let source = "use wasmtime
engine = wasm_engine_new()
context = wasmtime_store_context(wasmtime_store_new(engine, 0, 0))
seven = [0, 97, 115, 109, 1, 0, 0, 0, 1, 5, 1, 96, 0, 1, 126, 3, 2, 1, 0, 7, 8, 1, 4, 109, 97, 105, 110, 0, 0, 10, 6, 1, 4, 0, 66, 7, 11]
instance = wasmtime_linker_instantiate(wasmtime_linker_new(engine), context, wasmtime_module_new(engine, seven))
instance.store_id > 0";
	is!(source, true);
}

/// The module lib/compiler/emitter.warp emits (the self-hosted compile) runs through lib/wasmtime.warp too
#[test]
fn a_module_emitter_warp_emitted_runs_through_libwasmtime() {
	if !libwasmtime_installed() {
		return;
	}
	let folder = crate::common::scratch_directory("wasmtime_self_emitted");
	std::fs::create_dir_all(&folder).unwrap();
	std::fs::write(folder.join("program.warp"), "print \"hi from emitter.warp\"\n6 * 7").unwrap();
	let compiled = crate::common::warp_command()
		.args(["--no-ask", "compile", "--wasm", "program.warp"])
		.current_dir(&folder)
		.env("WARP_SELF_HOSTED", "1")
		.output()
		.unwrap();
	assert!(compiled.status.success(), "{}", String::from_utf8_lossy(&compiled.stderr));
	let run = crate::common::warp_command()
		.arg(concat!(env!("CARGO_MANIFEST_DIR"), "/probes/wasmtime_self_emitted.warp"))
		.arg(folder.join("program.wasm"))
		.output()
		.unwrap();
	assert_eq!(
		String::from_utf8_lossy(&run.stdout),
		"hi from emitter.warp\n42\n",
		"{}",
		String::from_utf8_lossy(&run.stderr)
	);
}
