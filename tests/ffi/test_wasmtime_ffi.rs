// card wasmtime-ffi: `use wasmtime` reads Homebrew's wasmtime.h with the headers it includes, whose functions wasm.h
// declares through macros (`WASM_DECLARE_OWN(engine)`); skipped where libwasmtime is not installed
use crate::is;

const LIBWASMTIME: &str = "/opt/homebrew/lib/libwasmtime.dylib";

#[test]
fn a_macro_declared_function_is_called() {
	if !std::path::Path::new(LIBWASMTIME).exists() {
		return;
	}
	is!("use wasmtime\nwasm_engine_new() > 0", true);
}
