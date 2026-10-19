mod test_browser_dollar_names; // card browser-interaction
mod test_builtin_lines;
#[cfg(feature = "native")] // cargo builds the crate: native only
mod test_cargo_crates; // card use-cargo
#[cfg(feature = "native")] // wasmtime components: not in the browser build
mod test_component_drop;
mod test_component_exports_anywhere; // card reflect-page-meta
#[cfg(feature = "native")] // wasmtime components: not in the browser build
mod test_component_operators;
#[cfg(feature = "native")] // wasmtime components: not in the browser build
mod test_component_short_use;
#[cfg(feature = "native")] // wasmtime components: not in the browser build
mod test_components;
mod test_components_anywhere;
#[cfg(feature = "native")] // cargo builds the crate: native only
mod test_crate_errors; // card try-crate-errors
mod test_download;
mod test_fetch_computed_url;
mod test_ffi;
#[cfg(feature = "native")] // the C headers: not in the browser build
mod test_ffi_float_results; // card raylib-float
mod test_ffi_gaps;
mod test_ffi_handles;
mod test_ffi_import_group;
#[cfg(all(feature = "native", target_os = "macos"))] // pinned on the C headers of macOS
mod test_ffi_one_parser; // card cleanup-ffi-headers
#[cfg(all(feature = "native", target_os = "macos"))] // libc's sranddev and the C headers of macOS
mod test_ffi_signatures; // card cleanup-ffi-f32
#[cfg(feature = "native")] // the C headers: not in the browser build
mod test_ffi_text_out_pointers; // card ffi-header
mod test_ffi_text_results;
mod test_ffi_warning_once;
#[cfg(feature = "native")] // a python3 child process: not in the browser build
mod test_foreign_bools; // card booleans-foreign
mod test_foreign_handles;
mod test_foreign_js;
mod test_foreign_lines; // decision foreign-signature, card standalone-math-units
mod test_foreign_member_assignment;
mod test_foreign_operators;
#[cfg(feature = "native")] // a python3 child process: not in the browser build
mod test_foreign_python;
#[cfg(feature = "native")] // a python3 child process: not in the browser build
mod test_foreign_type_tests; // card int-bool
mod test_glibc_math_header;
#[cfg(feature = "native")] // the C headers and the warp binary: not in the browser build
mod test_headless_windows;
#[cfg(feature = "native")] // wasmtime, files or network: not in the browser build
mod test_host_words;
mod test_js_callbacks;
mod test_libc_results;
#[cfg(feature = "native")] // the C headers: not in the browser build
mod test_libm_header_functions;
#[cfg(feature = "native")] // wasmtime, files or network: not in the browser build
mod test_libm_linking;
mod test_page_ids; // card page-ids
mod test_wasi;
mod test_web_idl; // card standalone-math-units step 3
