#[cfg(feature = "native")] // wasmtime, files or network: not in the browser build
mod test_compile_only;
mod test_emitter;
#[cfg(feature = "native")] // wasmtime, files or network: not in the browser build
mod test_gc_name_registry;
#[cfg(feature = "native")] // wasmtime, files or network: not in the browser build
mod test_gc_struct;
mod test_name_subsection_order;
#[cfg(feature = "native")] // wasmtime, files or network: not in the browser build
mod test_wasm_emitter;
#[cfg(feature = "native")] // wasmtime, files or network: not in the browser build
mod test_wasm_reader;
mod test_wasm;
mod test_wast;
mod test_wasm_names_order;
mod test_wit_types;
mod test_wit;
mod wasm_optimizer_test;
mod test_optimizer_exceptions;
mod test_optimizer_extended_const;
mod test_read_bytes_plain_result;
