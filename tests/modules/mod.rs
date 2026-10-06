mod test_folder_scope;
mod test_header_search;
mod test_include;
mod test_module_cache;
mod test_package_pin;
#[cfg(feature = "native")] // wasmtime, files or network: not in the browser build
mod test_package_tools;
mod test_packages;
mod test_use_modules;
#[cfg(feature = "native")] // wasmtime, files or network: not in the browser build
mod test_versions;
#[cfg(feature = "native")] // the browser host does not link imported modules yet (notes/wasm_modules.md)
mod test_wasm_modules;
