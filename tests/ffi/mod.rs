mod test_download;
mod test_ffi_import_group;
mod test_ffi_warning_once;
mod test_ffi;
#[cfg(feature = "native")] // wasmtime, files or network: not in the browser build
mod test_host;
mod test_host_words;
#[cfg(feature = "native")] // wasmtime, files or network: not in the browser build
mod test_libm_linking;
mod test_wasi;
mod test_glibc_math_header;
