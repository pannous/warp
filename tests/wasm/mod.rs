#[cfg(feature = "native")] // wasmtime, files or network: not in the browser build
mod test_build_emits_wasm; // card build-should
#[cfg(feature = "native")] // wasmtime, files or network: not in the browser build
mod test_compile_only;
#[cfg(feature = "native")] // wasmtime, files or network: not in the browser build
mod test_compiled_module_cache;
mod test_component_exports; // card reflection-components
mod test_component_worlds;
mod test_dead_functions;
mod test_emitter;
#[cfg(feature = "native")] // wasmtime, files or network: not in the browser build
mod test_executable_foreign; // card exe-foreign
#[cfg(feature = "native")] // wasmtime, files or network: not in the browser build
mod test_executable_output; // card executable-exit
#[cfg(feature = "native")] // wasmtime, files or network: not in the browser build
mod test_executable_size; // card g_gFs8
#[cfg(feature = "native")] // wasmtime, files or network: not in the browser build
mod test_gc_name_registry;
#[cfg(feature = "native")] // wasmtime, files or network: not in the browser build
mod test_gc_struct;
#[cfg(feature = "native")] // wasmtime stores: not in the browser build
mod test_memory_cap;
mod test_name_subsection_order;
#[cfg(feature = "native")] // a warp process in a temporary directory: not in the browser build
mod test_no_cwd_debris; // card cwd-artifacts
#[cfg(feature = "native")] // wasmtime, files or network: not in the browser build
mod test_own_runtime_stub; // card stub-race
mod test_parameter_names; // card reflection-classes
mod test_read_bytes_plain_result;
mod test_reproducible_builds;
mod test_runtime_stub_found;
mod test_runtime_stub_note; // card runtime-stub-note
#[cfg(feature = "native")] // wasmtime, files or network: not in the browser build
mod test_standalone_executable;
mod test_standalone_needs_runtime;
mod test_start_function_name; // card traps-task (reviewer)
mod test_wasm;
#[cfg(feature = "native")] // wasmtime, files or network: not in the browser build
mod test_wasm_emitter;
mod test_wasm_interop_rest;
mod test_wasm_names_order;
#[cfg(feature = "native")] // wasmtime, files or network: not in the browser build
mod test_wasm_reader;
mod test_wit;
mod test_wit_types;
