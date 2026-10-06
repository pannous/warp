mod test_download;
mod test_ffi_import_group;
mod test_ffi_warning_once;
mod test_ffi;
#[cfg(feature = "native")] // a python3 child process: not in the browser build
mod test_foreign_python;
#[cfg(feature = "native")] // wasmtime components: not in the browser build
mod test_components;
mod test_components_anywhere;
mod test_foreign_js;
mod test_foreign_handles;
mod test_foreign_operators;
#[cfg(feature = "native")] // wasmtime, files or network: not in the browser build
mod test_host;
mod test_host_words;
#[cfg(feature = "native")] // wasmtime, files or network: not in the browser build
mod test_libm_linking;
mod test_wasi;
mod test_glibc_math_header;
mod test_libc_results;
mod test_ffi_gaps;
mod test_ffi_text_results;
mod test_ffi_handles;
