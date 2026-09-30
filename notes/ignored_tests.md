# Ignored tests (sweep 2, 2026-09-30)

Generated from `cargo test --offline --all-features -- --ignored` on main after un-ignoring test_types (the only test
that passed in both modes with real assertions). A test passing while ignored but asserting nothing stays ignored.

## Fails: feature or fix missing: 58

- test_angle.rs::test_function_params
- test_ffi.rs::test_ffi_math_pipeline
- test_functions.rs::test_wit_function
- test_functions.rs::test_stacked_lambdas
- test_functions.rs::test_modifiers
- test_lists.rs::test_array_operations
- test_lists.rs::test_array_creation
- test_lists.rs::test_while_nop_issue — while loop with nop needs investigation
- test_lists.rs::test_array_initialization_basics
- test_lists.rs::test_array_initialization
- test_math.rs::test_hyphen_units
- test_math.rs::test_primitive_types
- test_node.rs::test_replace
- test_operators.rs::test_add_pure_vec_data — shall this ever work?
- test_string.rs::test_string_operations
- test_todo.rs::test_array_type_generics — todo
- test_todo.rs::test_array_constructor — typed array constructor not yet implemented
- test_types.rs::test_go_types
- test_types.rs::test_return_types
- test_types.rs::test_emit_cast_tuple — tuple comparison with cast - complex
- test_wasm.rs::test_get_local
- test_wasm.rs::test_wasm_function_definiton
- test_wasm.rs::test_wasm_ternary
- test_wasm.rs::test_lazy_evaluation
- test_wasm.rs::test_wasm_function_calls
- test_wasm.rs::test_math_primitives
- test_wasm.rs::test_math_operators
- test_wasm.rs::test_math_operators_runtime
- test_wasm.rs::test_comparison_id
- test_wasm.rs::test_comparison_id_precedence
- test_wasm.rs::test_wasm_logic_primitives
- test_wasm.rs::test_wasm_variables0
- test_wasm.rs::test_wasm_logic_unary_variables
- test_wasm.rs::test_wasm_logic_unary
- test_wasm.rs::test_wasm_logic_on_objects
- test_wasm.rs::test_wasm_logic_negated
- test_wasm.rs::test_squares
- test_wasm.rs::test_wasm_runtime_extension
- test_wasm.rs::test_string_concat_wasm
- test_wasm.rs::test_object_properties_wasm
- test_wasm.rs::test_array_indices_wasm
- test_wasm.rs::test_wasm_stuff
- test_wasm.rs::test_recent_random_bugs
- test_wasm.rs::test_wasm_mutable_global_imports
- test_wasm.rs::test_import_wasm
- test_wasm.rs::test_logarithm2
- test_wasm.rs::test_for_loop_classic
- test_wasm.rs::test_for_loops
- test_wasm.rs::test_assert
- test_wasm.rs::test_arguments
- test_wasm.rs::test_sinus2
- test_wasm.rs::test_sinus
- test_wasm.rs::test_node_data_binary_reconstruction
- test_wasm.rs::test_all_wasm — NEVER TEST ALL again ;) each #test individually! (todo possible in wasm?)
- test_wasm.rs::test_dom_property
- test_wast.rs::test_parse
- test_wast.rs::test_wast
- test_web.rs::test_inner_html — later

## Host, library, filesystem scan or lost files (keep ignored): 28

- test_ffi.rs::test_ffi_sdl_init — requires SDL2 library and wasp files
- test_ffi.rs::test_ffi_sdl_window — requires SDL2 library and FFI signatures
- test_ffi.rs::test_ffi_sdl_version — requires SDL2 library and FFI signatures
- test_ffi.rs::test_ffi_sdl_combined — requires SDL2 library and FFI signatures
- test_ffi.rs::test_ffi_sdl_debug — requires SDL2 library and FFI signatures
- test_ffi.rs::test_ffi_sdl_red_square_demo — requires SDL2 library and display
- test_ffi.rs::test_ffi_raylib_combined — requires raylib library and FFI signatures
- test_todo.rs::test_polymorphic_dispatch — requires polymorphic function dispatch
- test_types.rs::test_typed_functions — TODO: requires complete type system and Signature implementation
- test_types.rs::test_empty_typed_functions — TODO: requires complete type system
- test_types.rs::test_polymorphism — TODO: requires complete type system
- test_types.rs::test_polymorphism2 — TODO: requires complete type system
- test_types.rs::test_polymorphism3 — TODO: requires complete type system
- test_types.rs::test_generics — TODO: requires Generics implementation
- test_types.rs::test_function_argument_cast — TODO: requires complete type system
- test_wasm.rs::test_merge_global — LOST files: main_global.wasm, lib_global.wasm
- test_wasm.rs::test_merge_memory — LOST files: main_memory.wasm, lib_memory.wasm
- test_wasm.rs::test_merge_runtime — LOST file: main_memory.wasm
- test_wasm.rs::test_merge_own — Types Module, Code, int not defined
- test_wasm.rs::test_merge_wabt_by_hand — WABT_MERGE types not defined
- test_wasm.rs::test_multi_value — Node constructor syntax not valid in Rust
- test_wasm.rs::test_get_element_by_id
- test_wasm.rs::test_canvas
- test_web.rs::test_js
- test_web.rs::test_canvas
- test_web.rs::test_dom
- test_web.rs::test_dom_property — WEBAPP feature required
- test_xml.rs::test_parse_all_xml_files — Run with: cargo test -- --ignored

## Passes but asserts nothing (empty body or commented-out assertions; keep ignored): 14

- test_ffi.rs::test_ffi_floor — floor is a built-in WASM instruction, FFI import is shadowed
- test_ffi.rs::test_ffi_combined — floor builtin conflicts with FFI floor
- test_ffi.rs::test_ffi_ceil — ceil is a built-in WASM instruction, FFI import is shadowed
- test_ffi.rs::test_ffi_atoi
- test_ffi.rs::test_ffi_atol
- test_ffi.rs::test_ffi_rand
- test_ffi.rs::test_ffi_trigonometry_combined
- test_ffi.rs::test_ffi_string_math_combined
- test_ffi.rs::test_ffi_string_comparison_logic
- test_ffi.rs::test_extract_function_signature
- test_ffi.rs::test_ffi_abs_from_c — abs is now a builtin keyword, use fabs from m for C import
- test_wasm.rs::test_is
- test_wasm.rs::test_host_download
- test_wasm.rs::test_host_integration

## Not run in the sweep: 4

- src/wisp_parser.rs::test_wisp_defn
- test_ffi.rs::test_dynlib_import_emit — use keyword with ceil/floor conflicts with builtins
- test_node.rs::test_mark_as_map
- test_wasm.rs::test_custom_operators
