mod test_folder_scope;
mod test_header_search;
mod test_include;
mod test_module_classes;
mod test_module_signals;
mod test_module_cache;
mod test_package_pin;
#[cfg(feature = "native")] // wasmtime, files or network: not in the browser build
mod test_package_tools;
mod test_packages;
mod test_std_collections;
mod test_std_file;
mod test_std_hash;
mod test_std_json;
mod test_std_list;
mod test_std_map;
mod test_std_math_text;
mod test_std_random;
mod test_std_time;
mod test_std_qualified;
mod test_std_module_uses_module;
mod test_std_prelude;
mod test_std_net;
mod test_std_regex;
mod test_std_matrix;
mod test_use_modules;
#[cfg(feature = "native")] // wasmtime, files or network: not in the browser build
mod test_versions;
mod test_wasm_modules;
mod test_std_aliases;
mod test_std_named_program; // card cli-std
mod test_std_file_copy; // card std-file
mod test_netbase_package;
mod test_dir;
mod test_use_several; // card std-use
mod test_from_import; // card std-import
mod test_std_shadowed_names; // card libm-function
mod test_std_args; // card std-args
mod test_std_implicit_use; // card std-implicit
#[cfg(feature = "native")] // reads lib/, tests/ and src/: not in the browser build
mod test_std_coverage; // card std-word
mod test_std_words; // card std-word
mod test_std_names_offline; // card playground-module
