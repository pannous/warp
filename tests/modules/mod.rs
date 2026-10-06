mod test_folder_scope;
mod test_header_search;
mod test_include;
mod test_module_classes;
mod test_module_cache;
mod test_package_pin;
#[cfg(feature = "native")] // wasmtime, files or network: not in the browser build
mod test_package_tools;
mod test_packages;
mod test_std_collections;
mod test_std_list;
mod test_std_map;
mod test_std_math_text;
mod test_std_random;
mod test_std_qualified;
mod test_use_modules;
#[cfg(feature = "native")] // wasmtime, files or network: not in the browser build
mod test_versions;
mod test_wasm_modules;
