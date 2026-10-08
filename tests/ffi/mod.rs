mod test_download;
mod test_fetch_computed_url;
mod test_ffi_import_group;
mod test_ffi_warning_once;
mod test_ffi;
#[cfg(feature = "native")] // a python3 child process: not in the browser build
mod test_foreign_python;
#[cfg(feature = "native")] // wasmtime components: not in the browser build
mod test_components;
#[cfg(feature = "native")] // wasmtime components: not in the browser build
mod test_component_short_use;
mod test_components_anywhere;
mod test_foreign_js;
mod test_web_idl;
mod test_js_callbacks;
mod test_foreign_handles;
mod test_foreign_operators;
#[cfg(feature = "native")] // wasmtime, files or network: not in the browser build
mod test_host;
mod test_host_words;
#[cfg(feature = "native")] // wasmtime, files or network: not in the browser build
mod test_libm_linking;
#[cfg(feature = "native")] // the C headers: not in the browser build
mod test_libm_header_functions;
mod test_wasi;
mod test_glibc_math_header;
mod test_libc_results;
mod test_ffi_gaps;
mod test_ffi_text_results;
mod test_ffi_handles;
