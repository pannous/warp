#[cfg(feature = "native")] // the process environment's key: the browser reads the page's key setting instead
mod test_agent; // card g_X_F0
mod test_agent_types; // card int-let
#[cfg(feature = "native")] // another checkout's lib/ on disk and file times: not in the browser build
mod test_checkout_std_lib; // card local-lib
mod test_dir;
mod test_dir_any_value;
mod test_dir_builtin_members; // card dir-lists
mod test_dir_value_methods; // card list-methods: dir([1 2]) lists its methods
mod test_english_phrases; // card set-set
mod test_folder_scope;
mod test_from_import; // card std-import
mod test_header_search;
mod test_import_compiled; // card import-compiled
mod test_import_words; // card cleanup-closed-lists
mod test_include;
mod test_json5; // card json5-parse
mod test_json_parse_form; // card g_ogQg
#[cfg(feature = "native")] // the CLI with a cache folder
mod test_lowered_modules; // card prelude-cache-runs
mod test_module_cache;
mod test_module_classes;
mod test_module_error_file; // card module-error-file
mod test_module_function_dir; // card use-math
mod test_module_map_keys;
mod test_module_meta; // card reflection-foreign-meta
mod test_module_refused_definition; // card module-error-hidden
mod test_module_signals;
mod test_netbase_package;
mod test_node_json; // card node-json
mod test_package_forms; // card packages
mod test_package_hints; // card hints-fire
mod test_package_pin;
#[cfg(feature = "native")] // files beside the program: not in the browser build
mod test_package_reexport; // card module-own
mod test_package_registry; // card build-sh: the default registry warp-packages
#[cfg(feature = "native")] // files beside the program: not in the browser build
mod test_package_shadow; // card sample-local
#[cfg(feature = "native")] // wasmtime, files or network: not in the browser build
mod test_package_tools;
mod test_packages;
mod test_prelude_own_names; // card lib-math
mod test_prelude_shadowing; // card program-own
mod test_replaced_module_function; // card use-sound-def
mod test_rewrites; // card compiler-modules
mod test_runtime_word_lines; // card retire-library-words
mod test_selective_package; // card import-hides
mod test_std_aliases;
mod test_std_args; // card std-args
mod test_std_collections;
#[cfg(feature = "native")] // reads lib/, tests/ and src/: not in the browser build
mod test_std_coverage; // card std-word
mod test_std_docs_comment_association; // card help-markdown
#[cfg(feature = "native")] // runs a process: not in the browser
mod test_std_exec;
mod test_std_extrema; // card min-max-lib
mod test_std_file;
mod test_std_file_bytes; // card path-list
mod test_std_file_copy; // card std-file
mod test_std_hash;
mod test_std_implicit_use; // card std-implicit
mod test_std_import_forms; // cards import-square include-math
mod test_std_json;
mod test_std_list;
mod test_std_map;
mod test_std_math_text;
mod test_std_matrix;
mod test_std_module_docs; // card std-module-docs
mod test_std_module_uses_module;
mod test_std_named_program; // card cli-std
mod test_std_names_offline; // card playground-module
mod test_std_net;
mod test_std_prelude;
mod test_std_qualified;
mod test_std_random;
mod test_std_reflection_hints; // card function-repeat
mod test_std_regex;
mod test_std_shadowed_names; // card libm-function
mod test_std_time;
mod test_std_words; // card std-word
mod test_use_modules;
mod test_use_several; // card std-use
mod test_used_module_bare_words; // card one-dispatch
#[cfg(feature = "native")] // wasmtime, files or network: not in the browser build
mod test_versions;
mod test_wasm_modules; // card read-chron // card map-key
